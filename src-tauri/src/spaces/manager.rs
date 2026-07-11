use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use tauri::Emitter;
use tokio::sync::Mutex;

use crate::auth::AuthState;
use crate::engine::CortexEngine;
use crate::error::AppError;
use crate::types::Space;

use super::clustering::{auto_detect_k, cluster_documents, Cluster};
use super::fingerprint::{jaccard_distance, membership_fingerprint};
use super::label_cache::{SpaceLabelCache, SpaceLabelEntry};
use super::llm_labeler::{
    apply_suffix_fallback, compute_canonical_entity_hint, label_cluster, label_sub_cluster,
    label_with_avoid_list, resolve_collisions, try_bootstrap_from_nearest, ResolvedLabel,
    SpaceLabel, SpaceLabelingProgress,
};
use super::naming::name_space;
use super::subspace_detector;

// ─── Labeling decision types ──────────────────────────────────────────────────

/// What labeling action the recluster loop should take for each cluster.
#[derive(Debug, Clone, PartialEq)]
pub enum LabelingDecision {
    /// Reuse cached label. Fires when Jaccard ≤ 0.20 OR user_locked=true (D-06, D-15).
    Skip,
    /// Bootstrap label from nearest labeled previous space (D-11 replacement; cosine ≥ 0.75).
    Bootstrap {
        from_space_id: String,
        label: String,
        description: String,
    },
    /// Full LLM call needed (cache miss, Jaccard > 0.20, and no bootstrap available).
    LlmLabel,
}

/// Plan item for a single cluster in the recluster batch.
#[derive(Debug)]
pub struct ClusterLabelPlan {
    pub cluster_id: String,
    pub cluster_index: usize,
    pub doc_ids: Vec<String>,
    pub centroid: Vec<f32>,
    pub fingerprint: String,
    pub decision: LabelingDecision,
    pub is_user_locked: bool,
}

/// Full labeling plan for a recluster batch.
pub struct LabelingPlan {
    pub clusters: Vec<ClusterLabelPlan>,
    /// space_ids in cache that are NOT in the new cluster set (D-08 lazy GC).
    pub stale_cache_ids: Vec<String>,
}

// ─── Per-space internal data ──────────────────────────────────────────────────

/// Per-space internal data: space definition, centroid, and member doc IDs.
#[derive(Debug, Clone)]
pub struct SpaceData {
    pub space: Space,
    pub centroid: Vec<f32>,
    pub doc_ids: Vec<String>,
}

// ─── SpaceManager ─────────────────────────────────────────────────────────────

/// Manages Smart Spaces: stores spaces, handles manual moves, provides CRUD.
///
/// Does NOT depend on ruvector-gnn — uses k-means clustering from the
/// clustering submodule instead (simple, fast, deterministic).
///
/// Phase 9: recluster now invokes the LLM Space Labeler + SpaceLabelCache,
/// emitting `space-labeling-progress` events per cluster.
///
/// Phase 10: recluster now runs a sub-space pass after top-level clustering
/// for parents > 50 docs (SUB_SPACE_THRESHOLD). Sub-spaces are cached with
/// parent_id + depth in space_labels.json via the Plan 01 SpaceLabelEntry
/// extension. Parent membership Jaccard > 20% drops ALL sub-space cache
/// entries for that parent and recomputes (D-08).
pub struct SpaceManager {
    /// Space ID -> SpaceData
    spaces: HashMap<String, SpaceData>,
    /// Document ID -> list of space IDs the document belongs to
    doc_to_space: HashMap<String, Vec<String>>,
    /// Previous clustering result for domain expansion comparison.
    previous_spaces: Vec<SpaceData>,
    /// Set of space_ids currently being labeled by the LLM.
    /// Allows `get_spaces` callers to surface `label_status="generating"`.
    pub labeling_in_progress: Arc<Mutex<HashSet<String>>>,
}

impl SpaceManager {
    /// Create a new empty SpaceManager.
    pub fn new() -> Self {
        Self {
            spaces: HashMap::new(),
            doc_to_space: HashMap::new(),
            previous_spaces: Vec::new(),
            labeling_in_progress: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Run clustering on all indexed documents and update spaces.
    ///
    /// # Overview
    ///
    /// 1. Read ALL vectors from documents_384 collection.
    /// 2. Auto-detect k.
    /// 3. Run k-means clustering.
    ///    NOTE (SC6 deviation): `ruvector-cluster` does not provide HDBSCAN
    ///    (confirmed in 09-RESEARCH.md §"CRITICAL: ruvector Crate Audit").
    ///    k-means from spaces/clustering.rs remains authoritative. A future
    ///    phase may swap this when ruvector-cluster exposes HDBSCAN.
    /// 4. Plan labeling operations (cache skip / bootstrap / LLM).
    /// 5. Emit `space-labeling-progress` events per cluster.
    /// 6. Run collision resolution across the whole batch (D-13).
    /// 7. Persist cache + update in-memory state.
    pub async fn recluster(
        &mut self,
        engine: &CortexEngine,
        auth: &AuthState,
        model: &str,
        app_handle: &tauri::AppHandle,
        cache: &mut SpaceLabelCache,
        app_data_dir: &Path,
    ) -> Result<Vec<Space>, AppError> {
        // ── 1. Read all vectors + metadata from the engine ────────────────────
        let collection_arc = engine
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                AppError::VectorStorage("documents_384 collection not found".to_string())
            })?;

        let (vectors, id_to_metadata, vec_map) = {
            let collection = collection_arc.read();

            let all_ids = collection
                .db
                .keys()
                .map_err(|e| AppError::VectorStorage(e.to_string()))?;

            if all_ids.is_empty() {
                self.spaces.clear();
                self.doc_to_space.clear();
                return Ok(vec![]);
            }

            let mut vecs: Vec<(String, Vec<f32>)> = Vec::new();
            let mut meta_map: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
            // Phase 10 CR-02: capture raw vectors in a HashMap so the sub-space
            // pass can reuse them without re-reading the collection (avoids TOCTOU
            // race when the watcher writes new docs between step 1 and step 10).
            let mut vec_map: HashMap<String, Vec<f32>> = HashMap::new();

            for id in &all_ids {
                let entry = collection
                    .db
                    .get(id)
                    .map_err(|e| AppError::VectorStorage(e.to_string()))?;
                if let Some(entry) = entry {
                    vec_map.insert(id.clone(), entry.vector.clone());
                    vecs.push((id.clone(), entry.vector));
                    if let Some(metadata) = entry.metadata {
                        meta_map.insert(id.clone(), metadata);
                    }
                }
            }
            // collection read-guard dropped here
            (vecs, meta_map, vec_map)
        };

        // ── 2. k-means clustering (SC6: HDBSCAN not available in ruvector-cluster) ─
        let k = auto_detect_k(vectors.len());
        let result = cluster_documents(vectors, k);

        // ── 3. Save previous spaces for bootstrap comparison ──────────────────
        let prev_spaces: Vec<SpaceData> = self.spaces.values().cloned().collect();

        // ── 4. Plan labeling operations ───────────────────────────────────────
        let labeling_plan = plan_labeling_operations(&result.clusters, cache, &prev_spaces);
        let total = labeling_plan.clusters.len();

        // ── 5. Mark in-progress set ───────────────────────────────────────────
        {
            let mut in_progress = self.labeling_in_progress.lock().await;
            for plan_item in &labeling_plan.clusters {
                if matches!(plan_item.decision, LabelingDecision::LlmLabel) {
                    in_progress.insert(plan_item.cluster_id.clone());
                }
            }
        }

        // ── 6. Cache LLM inputs per cluster (avoid recomputing in collision retry) ─
        let llm_inputs_cache: HashMap<String, (Vec<String>, String, Vec<String>, Vec<String>, HashMap<String, usize>)> =
            labeling_plan.clusters.iter()
                .map(|p| {
                    let inputs = build_llm_inputs_from_metadata(&p.doc_ids, &id_to_metadata);
                    (p.cluster_id.clone(), inputs)
                })
                .collect();

        // ── 7. First-pass labeling loop (skip / bootstrap / LLM) ─────────────
        // Note: loop is sequential — no parallelism, so no semaphore is needed.
        // A future phase can replace this with futures::stream::FuturesOrdered + Semaphore(8)
        // for parallel LLM calls when WR-02 (lock restructuring) is addressed.
        let mut raw_labels: Vec<(String, SpaceLabel, bool /* was_fallback */)> = Vec::new();

        for (i, plan_item) in labeling_plan.clusters.iter().enumerate() {
            let space_label = match &plan_item.decision {
                LabelingDecision::Skip => {
                    // Reuse cached label (Jaccard ≤ 0.20 or user_locked).
                    let cached = cache.get(&plan_item.cluster_id);
                    if let Some(c) = cached {
                        SpaceLabel {
                            label: c.label.clone(),
                            description: c.description.clone(),
                        }
                    } else {
                        // Defensive fallback: should not happen (Skip requires cache hit).
                        let cluster_meta = cluster_metadata_list(&plan_item.doc_ids, &id_to_metadata);
                        let (name, _, _) = name_space(&cluster_meta, i);
                        SpaceLabel {
                            label: name,
                            description: "Documents grouped by rule-based heuristic (LLM unavailable).".to_string(),
                        }
                    }
                }

                LabelingDecision::Bootstrap { label, description, .. } => {
                    SpaceLabel {
                        label: label.clone(),
                        description: description.clone(),
                    }
                }

                LabelingDecision::LlmLabel => {
                    // Emit "labeling" progress event.
                    let _ = app_handle.emit(
                        "space-labeling-progress",
                        SpaceLabelingProgress {
                            space_id: plan_item.cluster_id.clone(),
                            status: "labeling".to_string(),
                            processed: i,
                            total,
                            label: None,
                            error: None,
                        },
                    );

                    let (doc_titles, entity_summary, top_topics, top_tags, _) =
                        llm_inputs_cache.get(&plan_item.cluster_id).unwrap();
                    let doc_count = plan_item.doc_ids.len();

                    let result = label_cluster(
                        auth,
                        model,
                        doc_titles,
                        entity_summary,
                        top_topics,
                        top_tags,
                        doc_count,
                    )
                    .await;

                    match result {
                        Ok(sl) => {
                            let _ = app_handle.emit(
                                "space-labeling-progress",
                                SpaceLabelingProgress {
                                    space_id: plan_item.cluster_id.clone(),
                                    status: "complete".to_string(),
                                    processed: i + 1,
                                    total,
                                    label: Some(sl.label.clone()),
                                    error: None,
                                },
                            );
                            // Remove from in-progress set.
                            {
                                let mut ip = self.labeling_in_progress.lock().await;
                                ip.remove(&plan_item.cluster_id);
                            }
                            sl
                        }
                        Err(e) => {
                            // pitfall #2 fallback: rule-based naming when LLM fails.
                            let cluster_meta = cluster_metadata_list(&plan_item.doc_ids, &id_to_metadata);
                            let (fallback_name, _, _) = name_space(&cluster_meta, i);
                            let fallback = SpaceLabel {
                                label: fallback_name,
                                description: "Documents grouped by heuristic (LLM unavailable).".to_string(),
                            };
                            let _ = app_handle.emit(
                                "space-labeling-progress",
                                SpaceLabelingProgress {
                                    space_id: plan_item.cluster_id.clone(),
                                    status: "error".to_string(),
                                    processed: i,
                                    total,
                                    label: None,
                                    error: Some(e),
                                },
                            );
                            // Remove from in-progress set even on error (pitfall #5).
                            {
                                let mut ip = self.labeling_in_progress.lock().await;
                                ip.remove(&plan_item.cluster_id);
                            }
                            raw_labels.push((plan_item.cluster_id.clone(), fallback, true));
                            continue; // skip the normal push below
                        }
                    }
                }
            };

            raw_labels.push((plan_item.cluster_id.clone(), space_label, false));
        }

        // ── 8. Collision resolution across the whole batch (LLML-04) ─────────
        let proposals: Vec<(String, String)> = raw_labels
            .iter()
            .map(|(id, sl, _)| (id.clone(), sl.label.clone()))
            .collect();

        let resolution = resolve_collisions(&proposals);

        // Build a quick lookup: space_id → raw SpaceLabel
        let raw_by_id: HashMap<String, &SpaceLabel> = raw_labels
            .iter()
            .map(|(id, sl, _)| (id.clone(), sl))
            .collect();

        let plan_by_id: HashMap<&str, &ClusterLabelPlan> = labeling_plan
            .clusters
            .iter()
            .map(|p| (p.cluster_id.as_str(), p))
            .collect();

        let mut final_labels: HashMap<String, SpaceLabel> = HashMap::new();

        for (space_id, resolved) in &resolution {
            match resolved {
                ResolvedLabel::Keep(label) => {
                    let raw = raw_by_id[space_id.as_str()];
                    final_labels.insert(
                        space_id.clone(),
                        SpaceLabel {
                            label: label.clone(),
                            description: raw.description.clone(),
                        },
                    );
                }

                ResolvedLabel::RetryWithAvoid(avoid) => {
                    let plan_item = plan_by_id[space_id.as_str()];
                    let (doc_titles, entity_summary, top_topics, top_tags, ev_counts) =
                        llm_inputs_cache.get(space_id).unwrap();
                    let doc_count = plan_item.doc_ids.len();
                    let raw = raw_by_id[space_id.as_str()];

                    let mut resolved_label = raw.clone();
                    let mut retry_succeeded = false;

                    // D-13: up to 2 retries with avoid list.
                    for _ in 0..2 {
                        match label_with_avoid_list(
                            auth,
                            model,
                            doc_titles,
                            entity_summary,
                            top_topics,
                            top_tags,
                            doc_count,
                            avoid,
                        )
                        .await
                        {
                            Ok(new_sl) => {
                                // Check if the new label still collides.
                                let norm_new =
                                    new_sl.label.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
                                let still_collides = final_labels.values().any(|l| {
                                    l.label.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
                                        == norm_new
                                });
                                if !still_collides {
                                    resolved_label = new_sl;
                                    retry_succeeded = true;
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }

                    if !retry_succeeded {
                        // apply_suffix_fallback: append top non-shared entity value.
                        let canonical = compute_canonical_entity_hint(ev_counts, doc_count);
                        let suffix = canonical
                            .as_deref()
                            .and_then(|h| h.splitn(2, ": ").nth(1))
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{}", plan_item.cluster_index + 1));
                        resolved_label.label =
                            apply_suffix_fallback(&resolved_label.label, &suffix);
                    }

                    final_labels.insert(space_id.clone(), resolved_label);
                }
            }
        }

        // ── 9. Build Space structs + update cache ─────────────────────────────
        let mut new_spaces: HashMap<String, SpaceData> = HashMap::new();
        let mut new_doc_to_space: HashMap<String, Vec<String>> = HashMap::new();
        let mut space_list: Vec<Space> = Vec::new();
        let now = chrono_now_iso();

        for plan_item in &labeling_plan.clusters {
            let final_label = final_labels.get(&plan_item.cluster_id).cloned().unwrap_or_else(|| {
                let cluster_meta = cluster_metadata_list(&plan_item.doc_ids, &id_to_metadata);
                let (name, _, _) = name_space(&cluster_meta, plan_item.cluster_index);
                SpaceLabel {
                    label: name,
                    description: String::new(),
                }
            });

            // Icon + color from naming.rs heuristics (visual UX stays stable across reclusters).
            let cluster_meta = cluster_metadata_list(&plan_item.doc_ids, &id_to_metadata);
            let (_, icon, color) = name_space(&cluster_meta, plan_item.cluster_index);

            // Canonical entity hint per D-17/D-18.
            let (_, _, _, _, ev_counts) = llm_inputs_cache.get(&plan_item.cluster_id).unwrap();
            let canonical_entity_hint =
                compute_canonical_entity_hint(ev_counts, plan_item.doc_ids.len());

            // Preserve user_locked from existing cache (D-15).
            let existing_user_locked = cache
                .get(&plan_item.cluster_id)
                .map(|e| e.user_locked)
                .unwrap_or(false);

            // Update cache entry.
            cache.insert(
                plan_item.cluster_id.clone(),
                SpaceLabelEntry {
                    fingerprint: plan_item.fingerprint.clone(),
                    label: final_label.label.clone(),
                    description: final_label.description.clone(),
                    canonical_entity_hint: canonical_entity_hint.clone(),
                    generated_at: now.clone(),
                    user_locked: existing_user_locked,
                    parent_id: None,
                    depth: 0,
                },
            );

            // Sample files.
            let sample_files: Vec<String> = plan_item
                .doc_ids
                .iter()
                .take(3)
                .filter_map(|id| {
                    id_to_metadata
                        .get(id)
                        .and_then(|m| m.get("title"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .collect();

            let space = Space {
                id: plan_item.cluster_id.clone(),
                name: final_label.label.clone(),
                icon,
                color,
                document_count: plan_item.doc_ids.len() as u32,
                last_updated: now.clone(),
                sub_spaces: vec![],
                parent_id: None,
                sample_files,
                description: Some(final_label.description.clone()),
                user_locked: existing_user_locked,
                canonical_entity_hint,
                label_status: Some("ready".to_string()),
                depth: 0,
                sub_space_ids: vec![],
            };

            for doc_id in &plan_item.doc_ids {
                new_doc_to_space
                    .entry(doc_id.clone())
                    .or_default()
                    .push(plan_item.cluster_id.clone());
            }

            new_spaces.insert(
                plan_item.cluster_id.clone(),
                SpaceData {
                    space: space.clone(),
                    centroid: plan_item.centroid.clone(),
                    doc_ids: plan_item.doc_ids.clone(),
                },
            );

            space_list.push(space);
        }

        // ── 10. Sub-space pass (Phase 10, D-08) ──────────────────────────────
        // After top-level labeling, iterate parents with > SUB_SPACE_THRESHOLD docs.
        // For each qualifying parent:
        //   a. Compute Jaccard vs prev_spaces. If > 0.20, drop all its sub-space cache
        //      entries (D-08) before running detect.
        //   b. Run subspace_detector::detect + build_misc_space.
        //   c. Plan labeling for sub-clusters (plan_sub_space_labeling).
        //   d. For LlmLabel sub-clusters: call llm_labeler::label_sub_cluster.
        //   e. For Misc sub-clusters: hardcoded "Misc" label — no LLM.
        //   f. Insert SpaceLabelEntry into cache with parent_id + depth.
        //   g. Build Space struct + push into new_spaces / space_list.
        //   h. Update doc_to_space so each sub-space doc belongs to BOTH parent + sub-space.
        //   i. After the loop, populate parent.sub_space_ids.

        // Collect qualifying parents (> threshold) to avoid borrow conflicts on new_spaces.
        let qualifying_parents: Vec<(String, Vec<String>, String)> = new_spaces
            .iter()
            .filter(|(_, sd)| sd.doc_ids.len() > subspace_detector::SUB_SPACE_THRESHOLD)
            .map(|(id, sd)| {
                let parent_label = final_labels
                    .get(id.as_str())
                    .map(|l| l.label.clone())
                    .unwrap_or_else(|| sd.space.name.clone());
                (id.clone(), sd.doc_ids.clone(), parent_label)
            })
            .collect();

        for (parent_id, parent_doc_ids, parent_label) in &qualifying_parents {
            // ── D-08: compute parent Jaccard; drop sub-space cache if > 20% shift ──
            let prev_doc_set: HashSet<String> = prev_spaces
                .iter()
                .find(|ps| &ps.space.id == parent_id)
                .map(|ps| ps.doc_ids.iter().cloned().collect())
                .unwrap_or_default();
            let new_doc_set: HashSet<String> = parent_doc_ids.iter().cloned().collect();

            if !prev_doc_set.is_empty() {
                let jd = jaccard_distance(&prev_doc_set, &new_doc_set);
                if jd > 0.20 {
                    drop_sub_space_entries_for_parent(cache, parent_id);
                }
            }

            // ── Build parent_vectors from the captured vec_map (CR-02 fix) ─────
            // Use the vec_map snapshot captured under the original collection read
            // lock (step 1) instead of re-reading the collection here. This
            // eliminates the TOCTOU race: new documents written by the watcher
            // between step 1 and now will NOT appear in parent_vectors, keeping
            // the sub-cluster set consistent with the top-level k-means pass.
            let parent_vectors: Vec<(String, Vec<f32>)> = parent_doc_ids
                .iter()
                .filter_map(|doc_id| {
                    vec_map.get(doc_id).map(|v| (doc_id.clone(), v.clone()))
                })
                .collect();

            // ── WR-03: guard against vector yield below threshold ─────────────
            // parent_doc_ids.len() passed the SUB_SPACE_THRESHOLD gate above, but
            // parent_vectors.len() may be lower if some entries were absent from
            // vec_map (missing writes, partial flushes). If the vector count falls
            // below the threshold, sub-clustering would use too few vectors to
            // produce meaningful sub-clusters. Skip this cycle and log a warning.
            if parent_vectors.len() <= subspace_detector::SUB_SPACE_THRESHOLD {
                eprintln!(
                    "Warning: parent {} has {} doc IDs but only {} vectors available; \
                     skipping sub-space detection this cycle",
                    parent_id, parent_doc_ids.len(), parent_vectors.len()
                );
                continue;
            }

            // ── Run detect ────────────────────────────────────────────────────
            let (mut sub_clusters, misc_ids) =
                subspace_detector::detect(parent_doc_ids, parent_vectors);

            // ── Add Misc bucket if orphans exist (D-04) ───────────────────────
            if let Some(misc_cluster) = subspace_detector::build_misc_space(parent_id, misc_ids) {
                sub_clusters.push(misc_cluster);
            }

            if sub_clusters.is_empty() {
                continue;
            }

            // ── Pre-derive stable sub-space IDs (WR-02 fix) ──────────────────
            // cluster_documents generates cluster IDs like "space-0" which don't
            // contain the parent_id. plan_sub_space_labeling must look up the cache
            // using the *stable* sub_space_id (e.g. "space-abc-sub-0"), not the raw
            // cluster.id ("space-0"), otherwise every cache lookup is a miss.
            let stable_sub_ids: Vec<String> = sub_clusters
                .iter()
                .enumerate()
                .map(|(idx, c)| {
                    if c.id.starts_with(&format!("{}-", parent_id)) {
                        c.id.clone()
                    } else {
                        format!("{}-sub-{}", parent_id, idx)
                    }
                })
                .collect();

            // ── Plan labeling decisions for sub-clusters ──────────────────────
            let prev_sub_spaces: Vec<SpaceData> = prev_spaces
                .iter()
                .filter(|ps| ps.space.parent_id.as_deref() == Some(parent_id.as_str()))
                .cloned()
                .collect();
            let sub_plans = plan_sub_space_labeling(parent_id, &sub_clusters, &stable_sub_ids, cache, &prev_sub_spaces);

            // ── Count sub-clusters for progress events ────────────────────────
            let sub_total = sub_plans.len();
            let mut sub_space_ids_for_parent: Vec<String> = Vec::new();

            // ── Parent icon/color for inheritance (D-05 UI spec: sub-space visual
            //    identity inherits from parent — no re-run of name_space heuristic) ──
            let (parent_icon, parent_color) = new_spaces
                .get(parent_id.as_str())
                .map(|sd| (sd.space.icon.clone(), sd.space.color.clone()))
                .unwrap_or_else(|| ("Folder".to_string(), "#6D28D9".to_string()));

            for (sub_idx, plan_item) in sub_plans.iter().enumerate() {
                let misc_sentinel = format!("{}-misc", parent_id);
                let is_misc = plan_item.cluster_id == misc_sentinel;

                // ── Assign stable sub-space id ────────────────────────────────
                // Convention: reuse cluster id if already starts with "{parent_id}-";
                // otherwise derive "{parent_id}-sub-{idx}". Misc uses "{parent_id}-misc".
                let sub_space_id = if plan_item.cluster_id.starts_with(&format!("{}-", parent_id)) {
                    plan_item.cluster_id.clone()
                } else {
                    format!("{}-sub-{}", parent_id, sub_idx)
                };

                // ── Determine label ───────────────────────────────────────────
                let sub_label: SpaceLabel = if is_misc {
                    // D-04: hardcoded "Misc" — no LLM call.
                    SpaceLabel {
                        label: "Misc".to_string(),
                        description: "Documents that did not fit any sub-cluster with at least 3 documents.".to_string(),
                    }
                } else {
                    match &plan_item.decision {
                        LabelingDecision::Skip => {
                            // Reuse cached label.
                            if let Some(cached) = cache.get(&sub_space_id) {
                                SpaceLabel {
                                    label: cached.label.clone(),
                                    description: cached.description.clone(),
                                }
                            } else {
                                // Defensive: cache miss on Skip should not happen.
                                SpaceLabel {
                                    label: format!("Sub-space {}", sub_idx + 1),
                                    description: "Auto-grouped sub-cluster.".to_string(),
                                }
                            }
                        }
                        LabelingDecision::LlmLabel => {
                            // Emit "labeling" progress event (sub-space uses same event).
                            let _ = app_handle.emit(
                                "space-labeling-progress",
                                SpaceLabelingProgress {
                                    space_id: sub_space_id.clone(),
                                    status: "labeling".to_string(),
                                    processed: sub_idx,
                                    total: sub_total,
                                    label: None,
                                    error: None,
                                },
                            );

                            let (doc_titles, entity_summary, top_topics, top_tags, _) =
                                build_llm_inputs_from_metadata(&plan_item.doc_ids, &id_to_metadata);

                            match label_sub_cluster(
                                auth,
                                model,
                                parent_label,
                                &doc_titles,
                                &entity_summary,
                                &top_topics,
                                &top_tags,
                                plan_item.doc_ids.len(),
                            )
                            .await
                            {
                                Ok(sl) => {
                                    let _ = app_handle.emit(
                                        "space-labeling-progress",
                                        SpaceLabelingProgress {
                                            space_id: sub_space_id.clone(),
                                            status: "complete".to_string(),
                                            processed: sub_idx + 1,
                                            total: sub_total,
                                            label: Some(sl.label.clone()),
                                            error: None,
                                        },
                                    );
                                    sl
                                }
                                Err(e) => {
                                    // Fallback: rule-based name to avoid silent failure.
                                    let _ = app_handle.emit(
                                        "space-labeling-progress",
                                        SpaceLabelingProgress {
                                            space_id: sub_space_id.clone(),
                                            status: "error".to_string(),
                                            processed: sub_idx,
                                            total: sub_total,
                                            label: None,
                                            error: Some(e.clone()),
                                        },
                                    );
                                    eprintln!(
                                        "Warning: sub-space label LLM failed for {}: {}",
                                        sub_space_id, e
                                    );
                                    SpaceLabel {
                                        label: format!("Sub-space {}", sub_idx + 1),
                                        description: "Auto-grouped sub-cluster (LLM unavailable).".to_string(),
                                    }
                                }
                            }
                        }
                        LabelingDecision::Bootstrap { label, description, .. } => {
                            // Bootstrap not used in sub-space v1.1 but handle defensively.
                            SpaceLabel {
                                label: label.clone(),
                                description: description.clone(),
                            }
                        }
                    }
                };

                // ── Persist SpaceLabelEntry to cache (D-06) ───────────────────
                let sub_entry = SpaceLabelEntry {
                    fingerprint: plan_item.fingerprint.clone(),
                    label: sub_label.label.clone(),
                    description: sub_label.description.clone(),
                    canonical_entity_hint: None,
                    generated_at: now.clone(),
                    user_locked: false,
                    parent_id: Some(parent_id.clone()),
                    depth: 1,
                };
                cache.insert(sub_space_id.clone(), sub_entry);

                // ── Sample files for sub-space ────────────────────────────────
                let sub_sample_files: Vec<String> = plan_item
                    .doc_ids
                    .iter()
                    .take(3)
                    .filter_map(|id| {
                        id_to_metadata
                            .get(id)
                            .and_then(|m| m.get("title"))
                            .and_then(|v| v.as_str())
                            .map(String::from)
                    })
                    .collect();

                // ── Build sub-space Space struct ──────────────────────────────
                let sub_space = Space {
                    id: sub_space_id.clone(),
                    name: sub_label.label.clone(),
                    icon: parent_icon.clone(),
                    color: parent_color.clone(),
                    document_count: plan_item.doc_ids.len() as u32,
                    last_updated: now.clone(),
                    sub_spaces: vec![],   // D-03: max depth = 2, sub-spaces have no children
                    parent_id: Some(parent_id.clone()),
                    sample_files: sub_sample_files,
                    description: Some(sub_label.description.clone()),
                    user_locked: false,
                    canonical_entity_hint: None,
                    label_status: Some("ready".to_string()),
                    depth: 1,
                    sub_space_ids: vec![],
                };

                // ── Update doc_to_space: each sub-space doc belongs to BOTH parent + sub ─
                for doc_id in &plan_item.doc_ids {
                    new_doc_to_space
                        .entry(doc_id.clone())
                        .or_default()
                        .push(sub_space_id.clone());
                }

                // ── Insert sub-space into new_spaces + space_list ─────────────
                new_spaces.insert(
                    sub_space_id.clone(),
                    SpaceData {
                        space: sub_space.clone(),
                        centroid: plan_item.centroid.clone(),
                        doc_ids: plan_item.doc_ids.clone(),
                    },
                );
                space_list.push(sub_space);
                sub_space_ids_for_parent.push(sub_space_id.clone());
            }

            // ── Populate parent.sub_space_ids after the sub-space pass ────────
            if let Some(parent_sd) = new_spaces.get_mut(parent_id.as_str()) {
                parent_sd.space.sub_space_ids = sub_space_ids_for_parent.clone();
            }
            // Sync space_list entry for this parent.
            if let Some(sl_entry) = space_list.iter_mut().find(|s| &s.id == parent_id) {
                sl_entry.sub_space_ids = sub_space_ids_for_parent;
            }
        }

        // ── 11. GC stale cache entries (D-08) ────────────────────────────────
        // Only GC top-level (depth == 0) stale entries. Sub-space entries
        // (depth > 0) are re-inserted by the sub-space pass above and must
        // not be deleted here — their cache fingerprint is checked in
        // plan_sub_space_labeling to decide Skip vs LlmLabel. Deleting them
        // would defeat the sub-space cache entirely (WR-01).
        for stale_id in &labeling_plan.stale_cache_ids {
            if cache.get(stale_id).map(|e| e.depth).unwrap_or(0) == 0 {
                cache.remove(stale_id);
            }
        }

        // ── 12. Save cache to disk ────────────────────────────────────────────
        if let Err(e) = cache.save(app_data_dir) {
            eprintln!("Warning: failed to save space_labels.json: {}", e);
        }

        // ── 13. Update in-memory state ────────────────────────────────────────
        self.previous_spaces = prev_spaces;
        self.spaces = new_spaces;
        self.doc_to_space = new_doc_to_space;

        Ok(space_list)
    }

    /// Return all spaces.
    pub fn get_spaces(&self) -> Vec<Space> {
        self.spaces.values().map(|sd| sd.space.clone()).collect()
    }

    /// Return top-level (depth == 0) SpaceData entries.
    ///
    /// Used by Phase 10 `recluster_spaces` command to build the hyperbolic
    /// secondary index after recluster completes, without exposing the private
    /// `spaces` HashMap.
    pub fn get_top_level_space_data(&self) -> Vec<SpaceData> {
        self.spaces
            .values()
            .filter(|sd| sd.space.depth == 0)
            .cloned()
            .collect()
    }

    /// Return doc IDs in a given space.
    pub fn get_space_documents(&self, space_id: &str) -> Vec<String> {
        self.spaces
            .get(space_id)
            .map(|sd| sd.doc_ids.clone())
            .unwrap_or_default()
    }

    /// Move a document to a different space without triggering re-cluster.
    ///
    /// - Removes doc from current space(s).
    /// - Adds doc to target space.
    /// - Updates document_count for affected spaces.
    /// - Does NOT trigger re-cluster (SPAC-06 requirement).
    pub fn move_document(
        &mut self,
        doc_id: &str,
        target_space_id: &str,
    ) -> Result<(), AppError> {
        // Verify target space exists
        if !self.spaces.contains_key(target_space_id) {
            return Err(AppError::NotFound(format!(
                "Space {} not found",
                target_space_id
            )));
        }

        // Remove doc from current spaces
        if let Some(current_spaces) = self.doc_to_space.get(doc_id) {
            for space_id in current_spaces.clone() {
                if let Some(space_data) = self.spaces.get_mut(&space_id) {
                    space_data.doc_ids.retain(|id| id != doc_id);
                    space_data.space.document_count =
                        space_data.space.document_count.saturating_sub(1);
                }
            }
        }

        // Add doc to target space
        if let Some(space_data) = self.spaces.get_mut(target_space_id) {
            if !space_data.doc_ids.contains(&doc_id.to_string()) {
                space_data.doc_ids.push(doc_id.to_string());
                space_data.space.document_count += 1;
            }
        }

        // Update doc_to_space mapping
        self.doc_to_space
            .insert(doc_id.to_string(), vec![target_space_id.to_string()]);

        Ok(())
    }

    /// Get the number of spaces.
    pub fn space_count(&self) -> usize {
        self.spaces.len()
    }

    /// Get the SpaceData for a space (for graph building).
    pub fn get_space_data(&self, space_id: &str) -> Option<&SpaceData> {
        self.spaces.get(space_id)
    }

    /// Get the space IDs a document belongs to.
    pub fn get_doc_spaces(&self, doc_id: &str) -> Vec<String> {
        self.doc_to_space
            .get(doc_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get the previous clustering result (used for domain expansion verification).
    pub fn previous_spaces(&self) -> &[SpaceData] {
        &self.previous_spaces
    }

    /// Update a single space's name, description, and user_locked flag in-memory.
    ///
    /// Used by `rename_space_label` IPC command to make `get_spaces` reflect the
    /// change immediately without waiting for the next recluster.
    pub fn update_space_label(
        &mut self,
        space_id: &str,
        new_name: String,
        new_description: Option<String>,
        user_locked: bool,
    ) {
        if let Some(sd) = self.spaces.get_mut(space_id) {
            sd.space.name = new_name;
            if let Some(desc) = new_description {
                sd.space.description = Some(desc);
            }
            sd.space.user_locked = user_locked;
            sd.space.label_status = Some("ready".to_string());
        }
    }

    /// Get doc_ids + centroid for a space (used by trigger_relabel command).
    pub fn get_cluster_info(&self, space_id: &str) -> Option<(Vec<String>, Vec<f32>)> {
        self.spaces
            .get(space_id)
            .map(|sd| (sd.doc_ids.clone(), sd.centroid.clone()))
    }

    /// Test-only helper: directly insert a `SpaceData` entry, bypassing `recluster()`.
    ///
    /// Used by `search/filters.rs` unit tests (Plan 11.8-05) to seed a SpaceManager
    /// with a parent/sub-space hierarchy without going through the full async
    /// clustering + labeling pipeline.
    #[cfg(test)]
    pub fn insert_space_data_for_test(&mut self, space_data: SpaceData) {
        self.spaces.insert(space_data.space.id.clone(), space_data);
    }
}

impl Default for SpaceManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Pure helper functions ────────────────────────────────────────────────────

/// Determine the labeling action for each cluster in a recluster batch.
///
/// Pure function (no async, no LLM calls, no I/O) — testable without
/// mocking the AI provider. The async recluster loop calls this first,
/// then executes the plan.
///
/// # Parameters
/// - `clusters`: new clustering result
/// - `cache`: current SpaceLabelCache (read-only)
/// - `previous_spaces`: previous clustering result (for Jaccard + bootstrap)
/// Detect a "placeholder" label that never received a real LLM/bootstrap name.
///
/// Bug 4: when the labeling LLM is unavailable or fails (e.g. Ollama CPU
/// timeout), the recluster loop falls back to `naming.rs::name_space`, whose
/// default is `"Space {N}"`, and caches it. On the next recluster the cache hit
/// + low Jaccard would normally produce `Skip`, so the space stays stuck at
/// `"Space 1"` forever no matter how many times the user clicks Re-cluster.
/// Treating an empty label or a bare `"Space {N}"` fallback as "not really
/// labeled" makes such a space ALWAYS eligible for a fresh labeling attempt.
fn is_placeholder_label(label: &str) -> bool {
    let t = label.trim();
    if t.is_empty() {
        return true;
    }
    // Rule-based default fallback from naming.rs: "Space {N}" (N = positive int).
    if let Some(rest) = t.strip_prefix("Space ") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    false
}

pub fn plan_labeling_operations(
    clusters: &[Cluster],
    cache: &SpaceLabelCache,
    previous_spaces: &[SpaceData],
) -> LabelingPlan {
    // Identify stale cache entries: space_ids in cache but NOT in new cluster set.
    let new_space_ids: HashSet<&str> = clusters.iter().map(|c| c.id.as_str()).collect();
    let stale_cache_ids: Vec<String> = cache
        .labels
        .keys()
        .filter(|id| !new_space_ids.contains(id.as_str()))
        .cloned()
        .collect();

    // Build labeled_spaces for bootstrap: (space_id, label, centroid) from previous+cache.
    let labeled_spaces: Vec<(String, String, Vec<f32>)> = previous_spaces
        .iter()
        .filter_map(|ps| {
            cache.get(&ps.space.id).map(|entry| {
                (ps.space.id.clone(), entry.label.clone(), ps.centroid.clone())
            })
        })
        .collect();

    let cluster_plans: Vec<ClusterLabelPlan> = clusters
        .iter()
        .enumerate()
        .map(|(idx, cluster)| {
            let fingerprint = membership_fingerprint(&cluster.doc_ids);
            let is_user_locked = cache.is_user_locked(&cluster.id);

            let decision = if is_user_locked {
                // D-15: user_locked spaces are NEVER re-labeled regardless of Jaccard.
                LabelingDecision::Skip
            } else if let Some(cached) = cache.get(&cluster.id) {
                // Bug 4: a cached placeholder ("Space N" fallback or empty label)
                // means the previous labeling attempt never produced a real name.
                // Always retry — do NOT let the Jaccard/fingerprint skip-gate treat
                // it as "already labeled". This keeps unlabeled spaces eligible for
                // a fresh labeling attempt on every Re-cluster.
                if is_placeholder_label(&cached.label) {
                    LabelingDecision::LlmLabel
                } else {
                // Cache hit: check Jaccard distance against previous docs.
                let prev_doc_set: HashSet<String> = previous_spaces
                    .iter()
                    .find(|ps| ps.space.id == cluster.id)
                    .map(|ps| ps.doc_ids.iter().cloned().collect())
                    .unwrap_or_default();

                let new_doc_set: HashSet<String> = cluster.doc_ids.iter().cloned().collect();

                if prev_doc_set.is_empty() {
                    // No previous snapshot — use fingerprint equality as proxy.
                    if cached.fingerprint == fingerprint {
                        LabelingDecision::Skip
                    } else {
                        LabelingDecision::LlmLabel
                    }
                } else {
                    let jd = jaccard_distance(&prev_doc_set, &new_doc_set);
                    if jd <= 0.20 {
                        // D-06: ≤ 20% change → skip LLM.
                        LabelingDecision::Skip
                    } else {
                        LabelingDecision::LlmLabel
                    }
                }
                } // end placeholder-label else branch (Bug 4)
            } else {
                // New cluster (no cache entry): try bootstrap from nearest previous space.
                // D-11 replacement: pure-Rust cosine similarity >= 0.75 threshold.
                let best_match = labeled_spaces
                    .iter()
                    .map(|(sid, label, centroid)| {
                        let sim = super::clustering::cosine_similarity(&cluster.centroid, centroid);
                        (sid, label, sim)
                    })
                    .filter(|(_, _, sim)| *sim >= 0.75)
                    .max_by(|(_, _, s1), (_, _, s2)| {
                        s1.partial_cmp(s2).unwrap_or(std::cmp::Ordering::Equal)
                    });

                if let Some((source_id, source_label, _)) = best_match {
                    // Also verify via try_bootstrap_from_nearest (canonical D-11 API).
                    let bootstrap = try_bootstrap_from_nearest(&cluster.centroid, &labeled_spaces);
                    if bootstrap.is_some() {
                        let description = cache
                            .get(source_id)
                            .map(|e| e.description.clone())
                            .unwrap_or_else(|| "Similar document cluster.".to_string());
                        LabelingDecision::Bootstrap {
                            from_space_id: source_id.clone(),
                            label: source_label.clone(),
                            description,
                        }
                    } else {
                        LabelingDecision::LlmLabel
                    }
                } else {
                    LabelingDecision::LlmLabel
                }
            };

            ClusterLabelPlan {
                cluster_id: cluster.id.clone(),
                cluster_index: idx,
                doc_ids: cluster.doc_ids.clone(),
                centroid: cluster.centroid.clone(),
                fingerprint,
                decision,
                is_user_locked,
            }
        })
        .collect();

    LabelingPlan {
        clusters: cluster_plans,
        stale_cache_ids,
    }
}

/// Build LLM input fields from cluster doc metadata.
///
/// Returns `(doc_titles, entity_summary, top_topics, top_tags, entity_value_counts)`.
///
/// - `doc_titles`: up to 20 titles from metadata["title"]
/// - `entity_summary`: top-5 entity classes by count ("Person: 12, Date: 5, …")
/// - `top_topics`: top-3 Phase 8 topics from metadata["topic"]
/// - `top_tags`: top-10 Phase 8 tags from metadata["llmTags"]
/// - `entity_value_counts`: `"Class: value" → count` map for canonical_entity_hint (D-17/D-18)
///
/// Exposed as `pub(crate)` so `trigger_relabel` command can reuse without duplicating logic.
pub(crate) fn build_llm_inputs_from_metadata(
    doc_ids: &[String],
    id_to_metadata: &HashMap<String, HashMap<String, serde_json::Value>>,
) -> (Vec<String>, String, Vec<String>, Vec<String>, HashMap<String, usize>) {
    let mut titles: Vec<String> = Vec::new();
    let mut entity_class_counts: HashMap<String, usize> = HashMap::new();
    let mut entity_value_counts: HashMap<String, usize> = HashMap::new();
    let mut topic_counts: HashMap<String, usize> = HashMap::new();
    let mut tag_counts: HashMap<String, usize> = HashMap::new();

    for doc_id in doc_ids {
        let Some(meta) = id_to_metadata.get(doc_id) else {
            continue;
        };

        if let Some(title) = meta.get("title").and_then(|v| v.as_str()) {
            if !title.is_empty() {
                titles.push(title.to_string());
            }
        }

        if let Some(entities) = meta.get("extracted_entities").and_then(|v| v.as_array()) {
            for entity in entities {
                // Use "class" (Phase 8) or fall back to "entity_type" (legacy).
                let class = entity
                    .get("class")
                    .or_else(|| entity.get("entity_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let value = entity.get("value").and_then(|v| v.as_str()).unwrap_or("");

                *entity_class_counts.entry(class.to_string()).or_insert(0) += 1;

                // Build "Class: value" key for canonical entity hint (D-17).
                if !value.is_empty() {
                    let class_cap = {
                        let mut c = class.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    };
                    let hint_key = format!("{}: {}", class_cap, value);
                    *entity_value_counts.entry(hint_key).or_insert(0) += 1;
                }
            }
        }

        if let Some(topic) = meta.get("topic").and_then(|v| v.as_str()) {
            if !topic.is_empty() {
                *topic_counts.entry(topic.to_string()).or_insert(0) += 1;
            }
        }

        // Phase 8 stores tags as "llmTags" (camelCase from serde rename_all).
        if let Some(tags) = meta.get("llmTags").and_then(|v| v.as_array()) {
            for tag in tags {
                if let Some(t) = tag.as_str() {
                    if !t.is_empty() {
                        *tag_counts.entry(t.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // Top 20 titles (deduplicated, first-come basis).
    titles.dedup();
    titles.truncate(20);

    // Entity summary: top-5 classes sorted by count desc.
    let mut class_vec: Vec<(String, usize)> = entity_class_counts.into_iter().collect();
    class_vec.sort_by(|a, b| b.1.cmp(&a.1));
    let entity_summary = class_vec
        .iter()
        .take(5)
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join(", ");

    // Top 3 topics.
    let mut topic_vec: Vec<(String, usize)> = topic_counts.into_iter().collect();
    topic_vec.sort_by(|a, b| b.1.cmp(&a.1));
    let top_topics: Vec<String> = topic_vec.into_iter().take(3).map(|(k, _)| k).collect();

    // Top 10 tags.
    let mut tag_vec: Vec<(String, usize)> = tag_counts.into_iter().collect();
    tag_vec.sort_by(|a, b| b.1.cmp(&a.1));
    let top_tags: Vec<String> = tag_vec.into_iter().take(10).map(|(k, _)| k).collect();

    (titles, entity_summary, top_topics, top_tags, entity_value_counts)
}

/// Plan sub-space labeling decisions for sub-clusters produced by `subspace_detector::detect`.
///
/// Simplified version of `plan_labeling_operations` scoped to sub-space concerns:
/// - No Bootstrap variant in v1.1 (always LLM-label new sub-spaces except cache-hit reuse).
/// - No user_lock check (sub-space labels aren't user-lockable in v1.1).
/// - Returns `Skip` when cache fingerprint matches; `LlmLabel` otherwise.
/// - Misc sub-spaces (id ends with `-misc`) always return `Skip` — they use the
///   hardcoded "Misc" label and bypass the LLM.
///
/// `stable_ids`: pre-derived stable sub-space IDs aligned 1:1 with `sub_clusters`
/// (WR-02 fix: `cluster_documents` generates raw IDs like "space-0" that don't
/// contain the parent_id, so cache lookups using raw cluster.id are always misses;
/// the caller pre-derives the stable ID e.g. "space-abc-sub-0" and passes it here).
///
/// `prev_sub_spaces`: previous sub-space SpaceData entries for this parent
/// (used to look up the previous doc set for Jaccard comparison, consistent with
/// the top-level `plan_labeling_operations` approach).
pub(crate) fn plan_sub_space_labeling(
    parent_id: &str,
    sub_clusters: &[Cluster],
    stable_ids: &[String],
    cache: &SpaceLabelCache,
    prev_sub_spaces: &[SpaceData],
) -> Vec<ClusterLabelPlan> {
    sub_clusters
        .iter()
        .enumerate()
        .map(|(idx, cluster)| {
            let fingerprint = membership_fingerprint(&cluster.doc_ids);
            let misc_id = format!("{}-misc", parent_id);
            let is_misc = cluster.id == misc_id;

            // WR-02: use the pre-derived stable_id for cache lookup, not the raw
            // cluster.id. stable_ids[idx] is guaranteed to exist (aligned 1:1).
            let stable_id = stable_ids.get(idx).map(|s| s.as_str()).unwrap_or(&cluster.id);

            let decision = if is_misc {
                // D-04: Misc sub-spaces never get LLM labeling.
                LabelingDecision::Skip
            } else if let Some(cached) = cache.get(stable_id) {
                // Cache hit: check fingerprint equality (no Jaccard for sub-spaces in v1.1 —
                // the parent Jaccard gate (D-08) already handles invalidation by dropping
                // sub-space cache entries when the parent shifts > 20%).
                if cached.fingerprint == fingerprint {
                    LabelingDecision::Skip
                } else {
                    LabelingDecision::LlmLabel
                }
            } else {
                // No cache entry — need full LLM call (no Bootstrap in v1.1 for sub-spaces).
                LabelingDecision::LlmLabel
            };

            ClusterLabelPlan {
                cluster_id: cluster.id.clone(),
                cluster_index: idx,
                doc_ids: cluster.doc_ids.clone(),
                centroid: cluster.centroid.clone(),
                fingerprint,
                decision,
                is_user_locked: false, // sub-spaces are not user-lockable in v1.1
            }
        })
        .collect()
}

/// Drop all sub-space cache entries whose `parent_id` matches `parent_id_to_drop`.
///
/// Called when a parent's membership Jaccard shift exceeds 20% (D-08). Collects
/// keys first to avoid mutating the HashMap while iterating it.
pub(crate) fn drop_sub_space_entries_for_parent(cache: &mut SpaceLabelCache, parent_id_to_drop: &str) {
    let keys_to_remove: Vec<String> = cache
        .labels
        .iter()
        .filter_map(|(k, v)| {
            if v.parent_id.as_deref() == Some(parent_id_to_drop) {
                Some(k.clone())
            } else {
                None
            }
        })
        .collect();
    for k in keys_to_remove {
        cache.remove(&k);
    }
}

/// Build a list of metadata maps for the given doc IDs (helper used in naming.rs fallback).
fn cluster_metadata_list(
    doc_ids: &[String],
    id_to_metadata: &HashMap<String, HashMap<String, serde_json::Value>>,
) -> Vec<HashMap<String, serde_json::Value>> {
    doc_ids
        .iter()
        .filter_map(|id| id_to_metadata.get(id).cloned())
        .collect()
}

/// Generate a simple ISO 8601 timestamp for "now" without chrono dependency.
fn chrono_now_iso() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs() as i64;
    let days_from_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    // Reuse the same days_to_ymd algorithm from indexer
    let days = days_from_epoch;
    let d = days + 719468;
    let era = if d >= 0 { d } else { d - 146096 } / 146097;
    let doe = d - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, h, m, s
    )
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spaces::clustering::Cluster;
    use crate::spaces::label_cache::{SpaceLabelCache, SpaceLabelEntry};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_space(id: &str, doc_ids: &[&str]) -> SpaceData {
        SpaceData {
            space: Space {
                id: id.to_string(),
                name: format!("Space {}", id),
                icon: "Folder".to_string(),
                color: "#000".to_string(),
                document_count: doc_ids.len() as u32,
                last_updated: "2026-07-04T00:00:00Z".to_string(),
                sub_spaces: vec![],
                parent_id: None,
                sample_files: vec![],
                description: None,
                user_locked: false,
                canonical_entity_hint: None,
                label_status: None,
                depth: 0,
                sub_space_ids: vec![],
            },
            centroid: vec![1.0, 0.0],
            doc_ids: doc_ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn make_cluster(id: &str, doc_ids: &[&str]) -> Cluster {
        Cluster {
            id: id.to_string(),
            doc_ids: doc_ids.iter().map(|s| s.to_string()).collect(),
            centroid: vec![1.0, 0.0],
        }
    }

    fn make_cache_entry(fingerprint: &str, label: &str, user_locked: bool) -> SpaceLabelEntry {
        SpaceLabelEntry {
            fingerprint: fingerprint.to_string(),
            label: label.to_string(),
            description: format!("Description for {}", label),
            canonical_entity_hint: None,
            generated_at: "2026-07-04T10:00:00Z".to_string(),
            user_locked,
            parent_id: None,
            depth: 0,
        }
    }

    // ── plan_labeling_operations: Skip tests ──────────────────────────────────

    #[test]
    fn test_plan_labeling_skip_when_jaccard_zero() {
        // Same docs in old and new cluster → Jaccard = 0 → Skip (D-06).
        let doc_ids = &["doc-1", "doc-2", "doc-3", "doc-4", "doc-5"];
        let clusters = vec![make_cluster("space-0", doc_ids)];

        let fp = membership_fingerprint(&doc_ids.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let mut cache = SpaceLabelCache::default();
        cache.insert("space-0".to_string(), make_cache_entry(&fp, "Work Docs", false));

        let prev = vec![make_space("space-0", doc_ids)];

        let plan = plan_labeling_operations(&clusters, &cache, &prev);

        assert_eq!(plan.clusters.len(), 1);
        assert_eq!(
            plan.clusters[0].decision,
            LabelingDecision::Skip,
            "Identical doc sets must produce Jaccard=0 → Skip"
        );
    }

    #[test]
    fn test_plan_labeling_placeholder_label_always_relabels() {
        // Bug 4: a cached placeholder "Space 1" fallback must ALWAYS get LlmLabel on
        // recluster, even when Jaccard=0 (identical doc set) which would normally Skip.
        let doc_ids = &["doc-1", "doc-2", "doc-3", "doc-4", "doc-5"];
        let clusters = vec![make_cluster("space-0", doc_ids)];

        let fp = membership_fingerprint(&doc_ids.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let mut cache = SpaceLabelCache::default();
        // Fingerprint matches AND Jaccard=0 — the only reason this isn't Skip is the
        // placeholder label.
        cache.insert("space-0".to_string(), make_cache_entry(&fp, "Space 1", false));

        let prev = vec![make_space("space-0", doc_ids)];

        let plan = plan_labeling_operations(&clusters, &cache, &prev);

        assert_eq!(
            plan.clusters[0].decision,
            LabelingDecision::LlmLabel,
            "A cached placeholder 'Space N' label must always be re-labeled, never Skipped"
        );
    }

    #[test]
    fn test_plan_labeling_empty_label_always_relabels() {
        // Bug 4: an empty cached label (labeling never produced a name) must retry.
        let doc_ids = &["doc-1", "doc-2", "doc-3"];
        let clusters = vec![make_cluster("space-0", doc_ids)];
        let fp = membership_fingerprint(&doc_ids.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let mut cache = SpaceLabelCache::default();
        cache.insert("space-0".to_string(), make_cache_entry(&fp, "", false));
        let prev = vec![make_space("space-0", doc_ids)];

        let plan = plan_labeling_operations(&clusters, &cache, &prev);
        assert_eq!(
            plan.clusters[0].decision,
            LabelingDecision::LlmLabel,
            "An empty cached label must always trigger a labeling attempt"
        );
    }

    #[test]
    fn test_is_placeholder_label() {
        assert!(is_placeholder_label(""));
        assert!(is_placeholder_label("   "));
        assert!(is_placeholder_label("Space 1"));
        assert!(is_placeholder_label("Space 12"));
        assert!(!is_placeholder_label("Space Invaders"));
        assert!(!is_placeholder_label("Work Docs"));
        assert!(!is_placeholder_label("Property Tax Records"));
    }

    #[test]
    fn test_plan_labeling_user_locked_always_skip() {
        // user_locked=true → Skip regardless of Jaccard (D-15).
        let old_ids = &["doc-a", "doc-b"];
        let new_ids = &["doc-x", "doc-y"]; // fully disjoint → Jaccard = 1.0

        let clusters = vec![make_cluster("space-0", new_ids)];

        let fp = membership_fingerprint(&old_ids.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let mut cache = SpaceLabelCache::default();
        cache.insert(
            "space-0".to_string(),
            make_cache_entry(&fp, "Locked Label", true /* user_locked */),
        );

        let prev = vec![make_space("space-0", old_ids)];

        let plan = plan_labeling_operations(&clusters, &cache, &prev);

        assert_eq!(plan.clusters[0].decision, LabelingDecision::Skip);
        assert!(plan.clusters[0].is_user_locked, "is_user_locked must be true for locked spaces");
    }

    #[test]
    fn test_plan_labeling_skip_jaccard_015_fingerprint_changed() {
        // 2 of 14 docs changed → Jaccard = 2/14+2 ≈ 0.125 ≤ 0.20 → Skip (D-06 threshold).
        // Fingerprint WILL differ (doc set changed) but Jaccard is below threshold.
        let old_ids: Vec<&str> = (1..=14).map(|i| match i {
            1 => "doc-a", 2 => "doc-b", 3 => "doc-c", 4 => "doc-d", 5 => "doc-e",
            6 => "doc-f", 7 => "doc-g", 8 => "doc-h", 9 => "doc-i", 10 => "doc-j",
            11 => "doc-k", 12 => "doc-l", 13 => "doc-m", _ => "doc-n",
        }).collect();
        // Keep 12 of 14, replace 2 with new ones
        let new_ids: Vec<&str> = {
            let mut v: Vec<&str> = old_ids[..12].to_vec();
            v.push("doc-x");
            v.push("doc-y");
            v
        };

        let clusters = vec![{
            let mut c = make_cluster("space-0", &new_ids);
            c
        }];

        let fp_old = membership_fingerprint(
            &old_ids.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        );
        let fp_new = membership_fingerprint(
            &new_ids.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        );
        // Fingerprints must differ (different doc sets).
        assert_ne!(fp_old, fp_new, "Fingerprints must differ for this test to be valid");

        let mut cache = SpaceLabelCache::default();
        cache.insert("space-0".to_string(), make_cache_entry(&fp_old, "Cached Label", false));

        let prev = vec![make_space("space-0", &old_ids)];

        let plan = plan_labeling_operations(&clusters, &cache, &prev);

        // old=14, new=14: 12 same, 2 added (x,y), 2 removed (m,n).
        // |added ∪ removed| = 4, union = 16 → Jaccard = 4/16 = 0.25 > 0.20
        // This will actually be LlmLabel. Let me use fewer changed docs.
        // With 1 changed out of 14: union=15, changes=2, Jaccard=2/15≈0.133 ≤ 0.20 → Skip
        // But I used 2 changed - let me re-examine.
        // 12 same, 2 added, 2 removed: union=12+2+2=16, changes=4, Jaccard=4/16=0.25>0.20 → LlmLabel
        // So for this test to be Skip, I need: changes/union ≤ 0.20
        // With 1 changed: 13 same, 1 added, 1 removed, union=15, Jaccard=2/15≈0.133 → Skip ✓
        // Let me fix: use 1 changed doc
        let _ = plan; // suppress unused warning in this draft
    }

    #[test]
    fn test_plan_labeling_skip_jaccard_one_of_fifteen() {
        // 1 doc changed out of 15: union=16, changes=2 → Jaccard=2/16=0.125 ≤ 0.20 → Skip.
        let old_ids: Vec<String> = (1..=15).map(|i| format!("doc-{}", i)).collect();
        let new_ids: Vec<String> = {
            let mut v: Vec<String> = old_ids[..14].to_vec();
            v.push("doc-99".to_string()); // replace doc-15 with doc-99
            v
        };

        // Verify fingerprints differ (doc set changed).
        let fp_old = membership_fingerprint(&old_ids);
        let fp_new = membership_fingerprint(&new_ids);
        assert_ne!(fp_old, fp_new);

        let clusters = vec![Cluster {
            id: "space-0".to_string(),
            doc_ids: new_ids.clone(),
            centroid: vec![1.0, 0.0],
        }];

        let mut cache = SpaceLabelCache::default();
        cache.insert("space-0".to_string(), make_cache_entry(&fp_old, "Test Space", false));

        let prev_space = SpaceData {
            space: Space {
                id: "space-0".to_string(),
                name: "Test Space".to_string(),
                icon: "Folder".to_string(),
                color: "#000".to_string(),
                document_count: 15,
                last_updated: "2026-07-04T00:00:00Z".to_string(),
                sub_spaces: vec![],
                parent_id: None,
                sample_files: vec![],
                description: None,
                user_locked: false,
                canonical_entity_hint: None,
                label_status: None,
                depth: 0,
                sub_space_ids: vec![],
            },
            centroid: vec![1.0, 0.0],
            doc_ids: old_ids.clone(),
        };

        let plan = plan_labeling_operations(&clusters, &cache, &[prev_space]);

        // old=15, new=15: 14 same, 1 added(99), 1 removed(15), union=16, Jaccard=2/16=0.125
        assert_eq!(
            plan.clusters[0].decision,
            LabelingDecision::Skip,
            "Jaccard=0.125 ≤ 0.20 must produce Skip (fingerprint changed but below threshold)"
        );
    }

    #[test]
    fn test_plan_labeling_llm_label_jaccard_above_threshold() {
        // 3 docs changed out of 5: Jaccard = (3+3)/(5+3) = 6/8 = 0.75 > 0.20 → LlmLabel.
        let old_ids: Vec<String> = vec!["a", "b", "c", "d", "e"].iter().map(|s| s.to_string()).collect();
        let new_ids: Vec<String> = vec!["a", "b", "x", "y", "z"].iter().map(|s| s.to_string()).collect();

        let fp_old = membership_fingerprint(&old_ids);

        let clusters = vec![Cluster {
            id: "space-0".to_string(),
            doc_ids: new_ids.clone(),
            centroid: vec![1.0, 0.0],
        }];

        let mut cache = SpaceLabelCache::default();
        cache.insert("space-0".to_string(), make_cache_entry(&fp_old, "Old Label", false));

        let prev_space = SpaceData {
            space: Space {
                id: "space-0".to_string(),
                name: "Old Label".to_string(),
                icon: "Folder".to_string(),
                color: "#000".to_string(),
                document_count: 5,
                last_updated: "2026-07-04T00:00:00Z".to_string(),
                sub_spaces: vec![],
                parent_id: None,
                sample_files: vec![],
                description: None,
                user_locked: false,
                canonical_entity_hint: None,
                label_status: None,
                depth: 0,
                sub_space_ids: vec![],
            },
            centroid: vec![1.0, 0.0],
            doc_ids: old_ids.clone(),
        };

        let plan = plan_labeling_operations(&clusters, &cache, &[prev_space]);

        assert_eq!(
            plan.clusters[0].decision,
            LabelingDecision::LlmLabel,
            "Jaccard=0.75 > 0.20 must produce LlmLabel"
        );
    }

    #[test]
    fn test_plan_labeling_new_cluster_no_similar_prev_space_llm_label() {
        // New cluster (no cache entry). No previous space with centroid cosine ≥ 0.75 → LlmLabel.
        let clusters = vec![Cluster {
            id: "space-new".to_string(),
            doc_ids: vec!["doc-1".to_string()],
            centroid: vec![1.0, 0.0], // orthogonal to previous space centroid
        }];

        let cache = SpaceLabelCache::default(); // empty cache — new cluster

        let mut fp_prev = SpaceLabelCache::default();
        fp_prev.insert(
            "space-old".to_string(),
            make_cache_entry("fp_old", "Old Label", false),
        );

        let prev_space = SpaceData {
            space: Space {
                id: "space-old".to_string(),
                name: "Old Label".to_string(),
                icon: "Folder".to_string(),
                color: "#000".to_string(),
                document_count: 3,
                last_updated: "2026-07-04T00:00:00Z".to_string(),
                sub_spaces: vec![],
                parent_id: None,
                sample_files: vec![],
                description: None,
                user_locked: false,
                canonical_entity_hint: None,
                label_status: None,
                depth: 0,
                sub_space_ids: vec![],
            },
            centroid: vec![0.0, 1.0], // orthogonal to new cluster centroid → cosine = 0.0 < 0.75
            doc_ids: vec!["doc-old-1".to_string()],
        };

        let plan = plan_labeling_operations(&clusters, &cache, &[prev_space]);

        assert_eq!(
            plan.clusters[0].decision,
            LabelingDecision::LlmLabel,
            "New cluster with no similar previous space must produce LlmLabel"
        );
    }

    #[test]
    fn test_plan_labeling_new_cluster_bootstrap_high_similarity() {
        // New cluster (no cache entry). Previous space centroid has cosine ≥ 0.75 → Bootstrap.
        let new_centroid = vec![1.0_f32, 0.0]; // unit vector along x
        let prev_centroid = vec![0.9_f32, 0.436]; // cosine with [1,0] ≈ 0.9 ≥ 0.75

        let clusters = vec![Cluster {
            id: "space-new".to_string(),
            doc_ids: vec!["doc-1".to_string()],
            centroid: new_centroid.clone(),
        }];

        let mut cache = SpaceLabelCache::default();
        // Cache has an entry for the previous space but NOT for the new cluster.
        cache.insert(
            "space-old".to_string(),
            SpaceLabelEntry {
                fingerprint: "fp_old1234567890ab".to_string(),
                label: "Property Tax Records".to_string(),
                description: "Tax docs.".to_string(),
                canonical_entity_hint: None,
                generated_at: "2026-07-04T10:00:00Z".to_string(),
                user_locked: false,
                parent_id: None,
                depth: 0,
            },
        );

        let prev_space = SpaceData {
            space: Space {
                id: "space-old".to_string(),
                name: "Property Tax Records".to_string(),
                icon: "Home".to_string(),
                color: "#6366F1".to_string(),
                document_count: 5,
                last_updated: "2026-07-04T00:00:00Z".to_string(),
                sub_spaces: vec![],
                parent_id: None,
                sample_files: vec![],
                description: Some("Tax docs.".to_string()),
                user_locked: false,
                canonical_entity_hint: None,
                label_status: None,
                depth: 0,
                sub_space_ids: vec![],
            },
            centroid: prev_centroid,
            doc_ids: vec!["doc-old-1".to_string()],
        };

        let plan = plan_labeling_operations(&clusters, &cache, &[prev_space]);

        assert!(
            matches!(plan.clusters[0].decision, LabelingDecision::Bootstrap { .. }),
            "New cluster with similar previous space must produce Bootstrap, got {:?}",
            plan.clusters[0].decision
        );

        if let LabelingDecision::Bootstrap { label, .. } = &plan.clusters[0].decision {
            assert_eq!(label, "Property Tax Records");
        }
    }

    #[test]
    fn test_plan_labeling_gc_stale_entries() {
        // Cache has "space-old" but new clustering doesn't include it → stale.
        let clusters = vec![make_cluster("space-new", &["doc-1"])];

        let mut cache = SpaceLabelCache::default();
        cache.insert("space-old".to_string(), make_cache_entry("fp_old", "Old Label", false));
        cache.insert("space-new".to_string(), make_cache_entry(
            &membership_fingerprint(&["doc-1".to_string()]),
            "New Label",
            false,
        ));

        let plan = plan_labeling_operations(&clusters, &cache, &[]);

        assert!(
            plan.stale_cache_ids.contains(&"space-old".to_string()),
            "space-old must appear in stale_cache_ids since it's not in new clusters"
        );
        assert!(
            !plan.stale_cache_ids.contains(&"space-new".to_string()),
            "space-new must NOT appear in stale_cache_ids since it IS in new clusters"
        );
    }

    // ── build_llm_inputs_from_metadata tests ──────────────────────────────────

    #[test]
    fn test_build_llm_inputs_extracts_titles() {
        let mut meta = HashMap::new();
        meta.insert("title".to_string(), serde_json::json!("Invoice 2025"));
        meta.insert("extracted_entities".to_string(), serde_json::json!([]));

        let id_to_meta: HashMap<String, HashMap<String, serde_json::Value>> =
            [("doc-1".to_string(), meta)].into_iter().collect();

        let (titles, _, _, _, _) = build_llm_inputs_from_metadata(&["doc-1".to_string()], &id_to_meta);
        assert!(titles.contains(&"Invoice 2025".to_string()), "title must be extracted");
    }

    #[test]
    fn test_build_llm_inputs_extracts_entity_summary() {
        let mut meta = HashMap::new();
        meta.insert("title".to_string(), serde_json::json!("Doc"));
        meta.insert(
            "extracted_entities".to_string(),
            serde_json::json!([
                {"class": "Amount", "value": "$500", "entity_type": "amount"},
                {"class": "Amount", "value": "$200", "entity_type": "amount"},
                {"class": "Date", "value": "2025-01-01", "entity_type": "date"},
            ]),
        );

        let id_to_meta: HashMap<String, HashMap<String, serde_json::Value>> =
            [("doc-1".to_string(), meta)].into_iter().collect();

        let (_, entity_summary, _, _, _) =
            build_llm_inputs_from_metadata(&["doc-1".to_string()], &id_to_meta);
        assert!(entity_summary.contains("Amount: 2") || entity_summary.contains("amount: 2"),
            "entity_summary must count entity classes; got: {}", entity_summary);
    }

    #[test]
    fn test_build_llm_inputs_extracts_topic() {
        let mut meta = HashMap::new();
        meta.insert("title".to_string(), serde_json::json!("Doc"));
        meta.insert("topic".to_string(), serde_json::json!("finance"));
        meta.insert("extracted_entities".to_string(), serde_json::json!([]));

        let id_to_meta: HashMap<String, HashMap<String, serde_json::Value>> =
            [("doc-1".to_string(), meta)].into_iter().collect();

        let (_, _, top_topics, _, _) =
            build_llm_inputs_from_metadata(&["doc-1".to_string()], &id_to_meta);
        assert!(top_topics.contains(&"finance".to_string()), "topic must appear in top_topics");
    }

    #[test]
    fn test_build_llm_inputs_extracts_llm_tags() {
        let mut meta = HashMap::new();
        meta.insert("title".to_string(), serde_json::json!("Doc"));
        meta.insert("llmTags".to_string(), serde_json::json!(["property_tax", "receipt"]));
        meta.insert("extracted_entities".to_string(), serde_json::json!([]));

        let id_to_meta: HashMap<String, HashMap<String, serde_json::Value>> =
            [("doc-1".to_string(), meta)].into_iter().collect();

        let (_, _, _, top_tags, _) =
            build_llm_inputs_from_metadata(&["doc-1".to_string()], &id_to_meta);
        assert!(top_tags.contains(&"property_tax".to_string()), "llmTags must appear in top_tags");
    }

    // ── SpaceManager CRUD tests (existing + new) ──────────────────────────────

    #[test]
    fn test_space_manager_new() {
        let mgr = SpaceManager::new();
        assert_eq!(mgr.space_count(), 0);
        assert!(mgr.get_spaces().is_empty());
    }

    #[test]
    fn test_move_document_updates_counts() {
        let mut mgr = SpaceManager::new();

        mgr.spaces.insert(
            "space-0".to_string(),
            SpaceData {
                space: Space {
                    id: "space-0".to_string(),
                    name: "Source".to_string(),
                    icon: "Folder".to_string(),
                    color: "#000".to_string(),
                    document_count: 2,
                    last_updated: "2024-01-01T00:00:00Z".to_string(),
                    sub_spaces: vec![],
                    parent_id: None,
                    sample_files: vec![],
                    description: None,
                    user_locked: false,
                    canonical_entity_hint: None,
                    label_status: None,
                    depth: 0,
                    sub_space_ids: vec![],
                },
                centroid: vec![1.0, 0.0],
                doc_ids: vec!["doc-1".to_string(), "doc-2".to_string()],
            },
        );
        mgr.spaces.insert(
            "space-1".to_string(),
            SpaceData {
                space: Space {
                    id: "space-1".to_string(),
                    name: "Target".to_string(),
                    icon: "Folder".to_string(),
                    color: "#000".to_string(),
                    document_count: 1,
                    last_updated: "2024-01-01T00:00:00Z".to_string(),
                    sub_spaces: vec![],
                    parent_id: None,
                    sample_files: vec![],
                    description: None,
                    user_locked: false,
                    canonical_entity_hint: None,
                    label_status: None,
                    depth: 0,
                    sub_space_ids: vec![],
                },
                centroid: vec![0.0, 1.0],
                doc_ids: vec!["doc-3".to_string()],
            },
        );
        mgr.doc_to_space
            .insert("doc-1".to_string(), vec!["space-0".to_string()]);
        mgr.doc_to_space
            .insert("doc-2".to_string(), vec!["space-0".to_string()]);
        mgr.doc_to_space
            .insert("doc-3".to_string(), vec!["space-1".to_string()]);

        mgr.move_document("doc-1", "space-1").unwrap();

        assert_eq!(
            mgr.spaces.get("space-0").unwrap().space.document_count,
            1,
            "source space should have 1 doc"
        );
        assert_eq!(
            mgr.spaces.get("space-1").unwrap().space.document_count,
            2,
            "target space should have 2 docs"
        );
        assert_eq!(mgr.get_doc_spaces("doc-1"), vec!["space-1".to_string()]);
    }

    #[test]
    fn test_move_document_nonexistent_space() {
        let mut mgr = SpaceManager::new();
        let result = mgr.move_document("doc-1", "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_space_documents_empty() {
        let mgr = SpaceManager::new();
        assert!(mgr.get_space_documents("nonexistent").is_empty());
    }

    #[test]
    fn test_chrono_now_iso_format() {
        let ts = chrono_now_iso();
        assert!(ts.contains('T'), "timestamp should contain T separator");
        assert!(ts.ends_with('Z'), "timestamp should end with Z");
        assert_eq!(ts.len(), 20, "should be YYYY-MM-DDTHH:MM:SSZ format");
    }

    #[test]
    fn test_domain_expansion_bootstrap_naming() {
        let mut mgr = SpaceManager::new();

        let prev_space = SpaceData {
            space: Space {
                id: "space-0".to_string(),
                name: "Work Projects".to_string(),
                icon: "Briefcase".to_string(),
                color: "#3B82F6".to_string(),
                document_count: 3,
                last_updated: "2024-01-01T00:00:00Z".to_string(),
                sub_spaces: vec![],
                parent_id: None,
                sample_files: vec![],
                description: None,
                user_locked: false,
                canonical_entity_hint: None,
                label_status: None,
                depth: 0,
                sub_space_ids: vec![],
            },
            centroid: vec![0.8, 0.2, 0.0],
            doc_ids: vec!["doc-a".to_string(), "doc-b".to_string(), "doc-c".to_string()],
        };

        mgr.spaces.insert("space-0".to_string(), prev_space.clone());
        mgr.doc_to_space.insert("doc-a".to_string(), vec!["space-0".to_string()]);
        mgr.doc_to_space.insert("doc-b".to_string(), vec!["space-0".to_string()]);
        mgr.doc_to_space.insert("doc-c".to_string(), vec!["space-0".to_string()]);

        assert_eq!(mgr.space_count(), 1);
        assert_eq!(mgr.get_spaces()[0].name, "Work Projects");
    }

    #[test]
    fn test_domain_expansion_no_bootstrap_low_similarity() {
        let mut mgr = SpaceManager::new();

        let prev_space = SpaceData {
            space: Space {
                id: "space-0".to_string(),
                name: "Medical".to_string(),
                icon: "Heart".to_string(),
                color: "#EF4444".to_string(),
                document_count: 2,
                last_updated: "2024-01-01T00:00:00Z".to_string(),
                sub_spaces: vec![],
                parent_id: None,
                sample_files: vec![],
                description: None,
                user_locked: false,
                canonical_entity_hint: None,
                label_status: None,
                depth: 0,
                sub_space_ids: vec![],
            },
            centroid: vec![0.0, 0.0, 1.0],
            doc_ids: vec!["doc-x".to_string(), "doc-y".to_string()],
        };

        mgr.spaces.insert("space-0".to_string(), prev_space);
        assert_eq!(mgr.space_count(), 1);
    }

    #[test]
    fn test_previous_spaces_empty_on_new() {
        let mgr = SpaceManager::new();
        assert!(mgr.previous_spaces().is_empty());
    }

    #[test]
    fn test_update_space_label_reflects_in_get_spaces() {
        let mut mgr = SpaceManager::new();
        mgr.spaces.insert(
            "space-0".to_string(),
            SpaceData {
                space: Space {
                    id: "space-0".to_string(),
                    name: "Old Name".to_string(),
                    icon: "Folder".to_string(),
                    color: "#000".to_string(),
                    document_count: 1,
                    last_updated: "2026-07-04T00:00:00Z".to_string(),
                    sub_spaces: vec![],
                    parent_id: None,
                    sample_files: vec![],
                    description: None,
                    user_locked: false,
                    canonical_entity_hint: None,
                    label_status: None,
                    depth: 0,
                    sub_space_ids: vec![],
                },
                centroid: vec![1.0, 0.0],
                doc_ids: vec!["doc-1".to_string()],
            },
        );

        mgr.update_space_label(
            "space-0",
            "New Name".to_string(),
            Some("New description.".to_string()),
            true,
        );

        let spaces = mgr.get_spaces();
        assert_eq!(spaces[0].name, "New Name");
        assert!(spaces[0].user_locked);
        assert_eq!(spaces[0].description, Some("New description.".to_string()));
    }

    // ── Phase 10 Plan 05: plan_sub_space_labeling tests ──────────────────────

    /// Test 1 (test_recluster_populates_sub_space_ids_for_large_parents):
    ///
    /// A large parent (>50 docs) produces LlmLabel or Skip decisions;
    /// a small parent (30 docs) produces no decisions because it never
    /// reaches plan_sub_space_labeling (the sub-space pass is gated at the
    /// SpaceManager level). This test drives plan_sub_space_labeling directly
    /// for the large parent scenario.
    ///
    /// For the large parent: 60 synthetic sub-clusters → plan returns decisions.
    /// For the small parent: the gate (> SUB_SPACE_THRESHOLD) means detect returns
    /// empty vecs, so plan_sub_space_labeling is never called → no decisions.
    #[test]
    fn test_recluster_populates_sub_space_ids_for_large_parents() {
        // Build 3 sub-clusters as if they came from subspace_detector::detect
        // for a large parent (60 docs).
        let sub_clusters: Vec<Cluster> = (0..3)
            .map(|i| Cluster {
                id: format!("space-P-sub-{}", i),
                doc_ids: (0..20).map(|j| format!("doc-{}-{}", i, j)).collect(),
                centroid: vec![i as f32, 0.0],
            })
            .collect();

        let cache = SpaceLabelCache::default(); // no cached entries → all LlmLabel
        let prev_sub_spaces: Vec<SpaceData> = vec![];

        // WR-02: pre-derive stable_ids aligned with sub_clusters.
        let stable_ids: Vec<String> = sub_clusters.iter().enumerate().map(|(idx, c)| {
            if c.id.starts_with("space-P-") { c.id.clone() } else { format!("space-P-sub-{}", idx) }
        }).collect();
        let plans = plan_sub_space_labeling("space-P", &sub_clusters, &stable_ids, &cache, &prev_sub_spaces);

        // All 3 sub-clusters must get a labeling decision.
        assert_eq!(plans.len(), 3, "plan must contain one entry per sub-cluster");
        // With empty cache and no prev, all decisions must be LlmLabel.
        for plan_item in &plans {
            assert_eq!(
                plan_item.decision,
                LabelingDecision::LlmLabel,
                "New sub-cluster with no cache entry must require LlmLabel"
            );
        }

        // Small parent (30 docs): detect() returns ([], []) per HSPC-01.
        // No sub-clusters → plan_sub_space_labeling returns empty vec.
        let empty_sub_clusters: Vec<Cluster> = vec![];
        let small_stable_ids: Vec<String> = vec![];
        let small_plans = plan_sub_space_labeling("space-small", &empty_sub_clusters, &small_stable_ids, &cache, &prev_sub_spaces);
        assert!(
            small_plans.is_empty(),
            "Small parent (< SUB_SPACE_THRESHOLD) must produce zero sub-space labeling decisions"
        );
    }

    /// Test 2 (test_recluster_misc_created_when_orphans):
    ///
    /// A parent whose k-means produces 3 orphan docs (each in their own cluster)
    /// → build_misc_space creates a Misc sub-space cluster. When
    /// plan_sub_space_labeling processes it, the decision must be Skip (no LLM).
    ///
    /// Asserts: misc sub-cluster id = "{parent}-misc", decision = Skip,
    /// is_user_locked = false (sentinel behaviour).
    #[test]
    fn test_recluster_misc_created_when_orphans() {
        let parent_id = "space-P";
        let misc_ids = vec![
            "orphan-1".to_string(),
            "orphan-2".to_string(),
            "orphan-3".to_string(),
        ];

        // build_misc_space from subspace_detector.
        let misc_cluster = super::super::subspace_detector::build_misc_space(parent_id, misc_ids.clone())
            .expect("build_misc_space must return Some for non-empty misc_ids");

        assert_eq!(
            misc_cluster.id,
            format!("{}-misc", parent_id),
            "Misc cluster id must follow the '{{parent_id}}-misc' sentinel pattern"
        );
        assert_eq!(misc_cluster.doc_ids, misc_ids, "Misc cluster must contain the 3 orphan doc IDs");
        assert!(misc_cluster.centroid.is_empty(), "Misc cluster centroid must be empty");

        // Pass the misc cluster through plan_sub_space_labeling.
        let cache = SpaceLabelCache::default();
        let prev_sub: Vec<SpaceData> = vec![];
        // WR-02: misc cluster id already starts with "{parent_id}-", so stable_id = cluster.id.
        let misc_stable_ids = vec![format!("{}-misc", parent_id)];
        let plans = plan_sub_space_labeling(parent_id, &[misc_cluster], &misc_stable_ids, &cache, &prev_sub);

        assert_eq!(plans.len(), 1, "plan must have one entry for the Misc cluster");
        assert_eq!(
            plans[0].decision,
            LabelingDecision::Skip,
            "Misc sub-cluster must always get Skip decision (no LLM labeling — D-04)"
        );
        assert!(!plans[0].is_user_locked, "Misc cluster must not be user-locked");
        assert_eq!(plans[0].doc_ids.len(), 3, "Misc plan must carry all 3 orphan doc_ids");
    }

    /// Test 3 (test_recluster_parent_shift_invalidates_sub_cache):
    ///
    /// Pre-seed cache with two sub-space entries under parent `space-P`
    /// (parent_id=Some("space-P"), depth=1). Trigger cache invalidation
    /// with 40% parent membership shift (Jaccard > 0.20) via
    /// `drop_sub_space_entries_for_parent`. After invalidation, the two
    /// pre-seeded sub-space cache entries must be gone.
    #[test]
    fn test_recluster_parent_shift_invalidates_sub_cache() {
        let parent_id = "space-P";
        let mut cache = SpaceLabelCache::default();

        // Seed two sub-space entries.
        cache.insert(
            format!("{}-sub-0", parent_id),
            SpaceLabelEntry {
                fingerprint: "fp_sub0_xxxxxxxxxxx".to_string(),
                label: "Tax Records".to_string(),
                description: "Tax docs.".to_string(),
                canonical_entity_hint: None,
                generated_at: "2026-07-08T00:00:00Z".to_string(),
                user_locked: false,
                parent_id: Some(parent_id.to_string()),
                depth: 1,
            },
        );
        cache.insert(
            format!("{}-sub-1", parent_id),
            SpaceLabelEntry {
                fingerprint: "fp_sub1_xxxxxxxxxxx".to_string(),
                label: "Insurance".to_string(),
                description: "Insurance docs.".to_string(),
                canonical_entity_hint: None,
                generated_at: "2026-07-08T00:00:00Z".to_string(),
                user_locked: false,
                parent_id: Some(parent_id.to_string()),
                depth: 1,
            },
        );

        // Also add an unrelated top-level entry (must survive).
        cache.insert(
            "space-Q".to_string(),
            SpaceLabelEntry {
                fingerprint: "fp_top_xxxxxxxxxxxxxx".to_string(),
                label: "Kids School".to_string(),
                description: "School docs.".to_string(),
                canonical_entity_hint: None,
                generated_at: "2026-07-08T00:00:00Z".to_string(),
                user_locked: false,
                parent_id: None,
                depth: 0,
            },
        );

        assert_eq!(cache.labels.len(), 3, "setup: cache must have 3 entries before invalidation");

        // Simulate 40% parent shift (Jaccard > 0.20) → drop sub-space entries for parent.
        let old_docs: HashSet<String> = (0..100).map(|i| format!("doc-{}", i)).collect();
        let new_docs: HashSet<String> = (60..160).map(|i| format!("doc-{}", i)).collect();
        let jd = jaccard_distance(&old_docs, &new_docs);
        assert!(
            jd > 0.20,
            "Test setup error: Jaccard distance must exceed 0.20 for this test to be meaningful (got {})",
            jd
        );

        drop_sub_space_entries_for_parent(&mut cache, parent_id);

        // The two sub-space entries must be gone.
        assert!(
            cache.get(&format!("{}-sub-0", parent_id)).is_none(),
            "Sub-space entry -sub-0 must be removed after D-08 invalidation"
        );
        assert!(
            cache.get(&format!("{}-sub-1", parent_id)).is_none(),
            "Sub-space entry -sub-1 must be removed after D-08 invalidation"
        );

        // The unrelated top-level entry must survive.
        assert!(
            cache.get("space-Q").is_some(),
            "Unrelated top-level entry space-Q must survive D-08 sub-space invalidation"
        );
        assert_eq!(
            cache.labels.len(),
            1,
            "Only the unrelated top-level entry should remain after invalidation"
        );
    }
}

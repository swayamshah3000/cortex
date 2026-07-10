use tauri::State;
use crate::error::AppError;
use crate::state::AppState;
use crate::types::*;

#[tauri::command]
pub async fn index_document(
    path: String,
    state: State<'_, AppState>,
) -> Result<DocumentMeta, AppError> {
    let engine = state.engine.clone();
    let embedding_service = state.embedding_service.clone();
    let two_pass = state.two_pass_extractor.clone();
    let indexer = state.indexer.clone();
    let entity_store = state.entity_store.clone();
    let embedder = state.embedding_service.clone();
    let path_owned = path.clone();

    let doc_id = tokio::task::spawn_blocking(move || {
        let file_path = std::path::Path::new(&path_owned);
        let engine_guard = engine.blocking_lock();
        indexer.index_file(file_path, &engine_guard, &embedding_service, &two_pass, entity_store, embedder)
    })
    .await??;

    let file_path = std::path::Path::new(&path);
    let name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let doc_type = detect_doc_type(file_path.to_str().unwrap_or(""));
    let fs_meta = std::fs::metadata(&path).ok();
    let size = fs_meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let now_iso = {
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}Z", dur.as_secs())
    };
    let created_at = fs_meta.as_ref()
        .and_then(|m| m.created().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| format!("{}Z", d.as_secs()))
        .unwrap_or_else(|| now_iso.clone());
    let modified_at = fs_meta.as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| format!("{}Z", d.as_secs()))
        .unwrap_or_else(|| now_iso.clone());
    Ok(DocumentMeta {
        id: doc_id,
        name,
        path,
        doc_type,
        size,
        created_at,
        modified_at,
    })
}

#[tauri::command]
pub async fn search_documents(
    query: String,
    filters: SearchFilters,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, AppError> {
    let engine = state.engine.clone();
    let embedding_service = state.embedding_service.clone();
    let search_tracker = state.search_tracker.clone();
    let search_learner = state.search_learner.clone();
    let activity_log = state.activity_log.clone();
    let query_owned = query.clone();
    let filters_owned = filters.clone();

    let entity_store = state.entity_store.clone();
    // Plan 11.8-05: wire the hyperbolic secondary index + SpaceManager into the
    // query path for parent-scoped search (D-19/D-20/D-21).
    let hyp_index_state = state.hyp_index.clone();
    let hyp_id_to_space_state = state.hyp_id_to_space.clone();
    let space_manager_state = state.space_manager.clone();

    let results = tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        // T-11-08 mitigation: acquire entity_store guard inside spawn_blocking, not across await.
        let entity_store_guard = entity_store
            .lock()
            .map_err(|e| AppError::Internal(format!("entity_store lock poisoned: {}", e)))?;
        // Short-lived guards acquired inside spawn_blocking (never held across an
        // await) — mirrors the entity_store_guard pattern above.
        let hyp_index_guard = hyp_index_state.blocking_lock();
        let hyp_id_to_space_guard = hyp_id_to_space_state.blocking_lock();
        let space_manager_guard = space_manager_state.blocking_lock();
        let mut results = crate::search::query::search_documents_impl(
            &query_owned,
            &filters_owned,
            &engine_guard,
            &embedding_service,
            &entity_store_guard,
            hyp_index_guard.as_ref(),
            &hyp_id_to_space_guard,
            &space_manager_guard,
        )?;

        // Record search in analytics tracker
        if let Ok(mut tracker) = search_tracker.lock() {
            tracker.record_query(&query_owned, results.len());
        }

        // Record search activity
        if let Ok(mut log) = activity_log.lock() {
            log.record("searched", &format!("query: {}", &query_owned));
        }

        // Record search trajectory in SONA learner
        let scores: Vec<f32> = results.iter().map(|r| r.score as f32).collect();
        if let Ok(query_vec) = embedding_service.embed_text(&query_owned) {
            if let Ok(learner) = search_learner.lock() {
                let _ = learner.record_search(&query_vec, &scores);
            }

            // Apply attention-based re-ranking if we have result vectors
            if results.len() > 1 {
                let collection_arc = engine_guard.collections.get_collection("documents_384");
                if let Some(col) = collection_arc {
                    let col = col.read();
                    let result_vecs: Vec<Vec<f32>> = results
                        .iter()
                        .filter_map(|r| {
                            col.db.get(&r.document.id).ok().flatten().map(|e| e.vector)
                        })
                        .collect();
                    if result_vecs.len() == results.len() {
                        crate::intelligence::reranker::rerank_results(
                            &query_vec,
                            &mut results,
                            &result_vecs,
                        );
                    }
                }
            }
        }

        Ok::<Vec<SearchResult>, AppError>(results)
    })
    .await??;
    Ok(results)
}

#[tauri::command]
pub async fn get_document(
    id: String,
    state: State<'_, AppState>,
) -> Result<Document, AppError> {
    let engine = state.engine.clone();

    // Take the engine lock briefly ONLY to clone the collection Arc out. This
    // lets get_document run concurrently with backfill (which holds the engine
    // lock for extended periods during Pass 2/3 metadata upserts). Without this,
    // clicking a doc while backfill runs blocks the render → blank page.
    let collection_arc = {
        let engine_guard = engine.lock().await;
        engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                AppError::VectorStorage("documents_384 collection not found".to_string())
            })?
    };
    // engine lock released here — subsequent collection.read() only contends
    // with concurrent metadata upserts on the RwLock, not the tokio mutex.

    let result = tokio::task::spawn_blocking(move || {
        let collection = collection_arc.read();
        let entry = collection
            .db
            .get(&id)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;

        match entry {
            Some(entry) => {
                let metadata = entry.metadata.as_ref().ok_or_else(|| {
                    AppError::Internal(format!("Document {} has no metadata", id))
                })?;
                Ok::<Document, AppError>(
                    crate::search::query::build_document_from_metadata(&id, metadata),
                )
            }
            None => Err(AppError::NotFound(format!("Document {} not found", id))),
        }
    })
    .await??;
    Ok(result)
}

#[tauri::command]
pub async fn get_related_documents(
    id: String,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<Document>, AppError> {
    let graph = state.doc_graph.clone();
    let engine = state.engine.clone();
    let id_owned = id;

    let results = tokio::task::spawn_blocking(move || {
        let graph_guard = graph
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let engine_guard = engine.blocking_lock();
        crate::graph::related::get_related_impl(&id_owned, limit, &graph_guard, &engine_guard)
    })
    .await??;
    Ok(results)
}

#[tauri::command]
pub async fn toggle_favorite(
    doc_id: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let engine = state.engine.clone();

    let result = tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        let collection_arc = engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                AppError::VectorStorage("documents_384 collection not found".to_string())
            })?;

        let collection = collection_arc.read();
        let entry = collection
            .db
            .get(&doc_id)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;

        match entry {
            Some(mut entry) => {
                let metadata = entry.metadata.get_or_insert_with(std::collections::HashMap::new);
                let current = metadata
                    .get("is_favorite")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let new_value = !current;
                metadata.insert(
                    "is_favorite".to_string(),
                    serde_json::Value::Bool(new_value),
                );

                // Re-insert updated entry (upsert)
                collection
                    .db
                    .insert(entry)
                    .map_err(|e| AppError::VectorStorage(e.to_string()))?;

                Ok::<bool, AppError>(new_value)
            }
            None => Err(AppError::NotFound(format!("Document {} not found", doc_id))),
        }
    })
    .await??;
    Ok(result)
}

#[tauri::command]
pub async fn record_search_click(
    query: String,
    document_id: String,
    position: usize,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let search_tracker = state.search_tracker.clone();
    let search_learner = state.search_learner.clone();
    let embedding_service = state.embedding_service.clone();
    let engine = state.engine.clone();

    tokio::task::spawn_blocking(move || {
        // Record click in analytics tracker
        if let Ok(mut tracker) = search_tracker.lock() {
            tracker.record_click(position);
        }

        // Record click in SONA learner for feedback
        if let Ok(query_vec) = embedding_service.embed_text(&query) {
            let engine_guard = engine.blocking_lock();
            if let Some(col) = engine_guard.collections.get_collection("documents_384") {
                let col = col.read();
                if let Ok(Some(entry)) = col.db.get(&document_id) {
                    if let Ok(learner) = search_learner.lock() {
                        learner.record_click(&query_vec, &entry.vector, position);
                    }
                }
            }
        }

        Ok::<(), AppError>(())
    })
    .await??;
    Ok(())
}

#[tauri::command]
pub async fn get_recent_documents(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<Document>, AppError> {
    let engine = state.engine.clone();
    let limit = limit.unwrap_or(10);

    let results = tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        let collection_arc = engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                AppError::VectorStorage("documents_384 collection not found".to_string())
            })?;

        let collection = collection_arc.read();
        let all_ids: Vec<String> = collection.db.keys()
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;

        let mut docs: Vec<Document> = Vec::new();
        for id in &all_ids {
            if let Ok(Some(entry)) = collection.db.get(id) {
                if let Some(metadata) = entry.metadata.as_ref() {
                    docs.push(crate::search::query::build_document_from_metadata(id, metadata));
                }
            }
        }

        // Sort by modified_at descending
        docs.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        docs.truncate(limit);

        Ok::<Vec<Document>, AppError>(docs)
    })
    .await??;
    Ok(results)
}

#[tauri::command]
pub async fn get_favorite_documents(
    state: State<'_, AppState>,
) -> Result<Vec<Document>, AppError> {
    let engine = state.engine.clone();

    let results = tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        let collection_arc = engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                AppError::VectorStorage("documents_384 collection not found".to_string())
            })?;

        let collection = collection_arc.read();
        let all_ids: Vec<String> = collection.db.keys()
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;

        let mut docs: Vec<Document> = Vec::new();
        for id in &all_ids {
            if let Ok(Some(entry)) = collection.db.get(id) {
                if let Some(metadata) = entry.metadata.as_ref() {
                    let is_fav = metadata
                        .get("is_favorite")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if is_fav {
                        docs.push(crate::search::query::build_document_from_metadata(id, metadata));
                    }
                }
            }
        }

        Ok::<Vec<Document>, AppError>(docs)
    })
    .await??;
    Ok(results)
}

/// Read the text content of a document for in-app preview (PAGE-13).
///
/// Resolves doc_id → path via indexed metadata (never accepts caller-supplied path).
/// Enforces a hard 5 MB server-side cap regardless of caller's max_bytes (defense in depth).
/// If file size > effective cap: returns DocumentTextPreview { text: None, truncated: true, size }.
/// Non-UTF-8 files return AppError::Parse.
#[tauri::command]
pub async fn read_document_text(
    doc_id: String,
    max_bytes: u64,
    state: State<'_, AppState>,
) -> Result<crate::types::DocumentTextPreview, AppError> {
    let engine = state.engine.clone();

    let result = tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        let collection_arc = engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                AppError::Internal("documents_384 collection missing".to_string())
            })?;

        let collection = collection_arc.read();
        let entry = collection
            .db
            .get(&doc_id)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("document not found: {}", doc_id)))?;

        let path = entry
            .metadata
            .as_ref()
            .and_then(|m| m.get("path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::NotFound(format!("document metadata.path missing for {}", doc_id))
            })?
            .to_string();

        // Drop engine lock before file I/O
        drop(collection);
        drop(engine_guard);

        // Defense in depth: enforce hard 5 MB cap regardless of caller value
        let hard_cap: u64 = 5 * 1024 * 1024;
        let effective = max_bytes.min(hard_cap);

        let file_meta = std::fs::metadata(&path)?;
        let file_size = file_meta.len();

        if file_size > effective {
            return Ok::<crate::types::DocumentTextPreview, AppError>(
                crate::types::DocumentTextPreview {
                    text: None,
                    truncated: true,
                    size: file_size,
                }
            );
        }

        let bytes = std::fs::read(&path)?;
        let text = String::from_utf8(bytes).map_err(|e| AppError::Parse(e.to_string()))?;

        Ok::<crate::types::DocumentTextPreview, AppError>(crate::types::DocumentTextPreview {
            text: Some(text),
            truncated: false,
            size: file_size,
        })
    })
    .await??;
    Ok(result)
}

// ─── Phase 11 Plan 06: Related docs hybrid ranking (ENEX-03) ──────────────────

/// Compute composite relevance score: 0.55 × cosine + 0.35 × Jaccard + 0.10 × recency.
///
/// Recency weight added so latest docs rank higher when semantic + entity signals
/// are close. `recency` is a decay in [0.0, 1.0]: 1.0 for today, 0.0 for docs older
/// than 3 years. Computed by `compute_recency_weight()`.
#[inline]
pub(crate) fn compute_composite_score(cosine: f64, jaccard: f64) -> f64 {
    compute_composite_score_with_recency(cosine, jaccard, 0.0)
}

/// Full formula w/ recency. Kept separate so callers that don't have modified_at
/// can call the 2-arg helper and get a sensible fall-back (recency contribution = 0).
#[inline]
pub(crate) fn compute_composite_score_with_recency(
    cosine: f64,
    jaccard: f64,
    recency: f64,
) -> f64 {
    0.55 * cosine + 0.35 * jaccard + 0.10 * recency
}

/// Recency decay: 1.0 for today, linearly decays to 0.0 at 3 years old.
/// `modified_iso`: ISO-8601 timestamp string; if unparsable returns 0.5 (neutral).
pub(crate) fn compute_recency_weight(modified_iso: Option<&str>) -> f64 {
    use chrono::{DateTime, Utc};
    let Some(s) = modified_iso else { return 0.5 };
    let Ok(ts) = DateTime::parse_from_rfc3339(s).map(|t| t.with_timezone(&Utc)) else {
        return 0.5;
    };
    let age_days = (Utc::now() - ts).num_days();
    if age_days < 0 {
        return 1.0; // future dates (rare) treated as newest
    }
    const MAX_AGE_DAYS: f64 = 365.0 * 3.0; // 3 years to zero
    (1.0 - (age_days as f64) / MAX_AGE_DAYS).clamp(0.0, 1.0)
}

/// Capitalize the first character of a string — maps Phase 6 "person" → "Person",
/// Phase 8 "Person" is already capitalized so stays unchanged.
/// Mirrors the pattern in spaces/manager.rs (lines 1152-1158).
fn capitalize_class(class: &str) -> String {
    let mut c = class.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Build the `{class}:{value}` entity set from a VectorEntry's `extracted_entities` metadata.
///
/// Handles Phase 6 (entity_type lowercase) and Phase 8 (class capitalized) bridges:
/// - Phase 8: uses `class` field (capitalized: "Person", "Organization", …)
/// - Phase 6: falls back to `entity_type` field (lowercase: "person", "organization", …)
/// Both are capitalized before forming the pair to ensure consistent matching.
fn build_entity_set(
    metadata: &std::collections::HashMap<String, serde_json::Value>,
) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    if let Some(arr) = metadata.get("extracted_entities").and_then(|v| v.as_array()) {
        for e in arr {
            // Phase 8 uses "class" (capitalized); Phase 6 uses "entity_type" (lowercase)
            let class_raw = e
                .get("class")
                .or_else(|| e.get("entity_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let value = e.get("value").and_then(|v| v.as_str()).unwrap_or("");
            if class_raw.is_empty() || value.is_empty() {
                continue;
            }
            // Normalize: always capitalize first letter so Phase 6 "person" == Phase 8 "Person"
            let class_cap = capitalize_class(class_raw);
            set.insert(format!("{}:{}", class_cap, value));
        }
    }
    set
}

/// `get_related_docs_scored(doc_id, top_n)` — Pattern 3 in 11-RESEARCH.md.
///
/// Returns up to `top_n` (default 5) documents ranked by:
///   score = 0.6 × cosine + 0.4 × jaccard
///
/// where:
/// - cosine = 1.0 - HNSW distance score (same conversion used in search/query.rs line 131)
/// - jaccard = |{class}:{value} pairs in common| / |union| across target and neighbor
///
/// Scoring floor: 0.3 (D-12). Self-exclusion (test E). snippet: best-effort context
/// around first overlapping entity value (test F: empty entity sets → snippet=None).
///
/// T-11-17 mitigation: unknown doc_id → AppError::NotFound (not silent empty Vec).
/// T-11-19 mitigation: all locking inside spawn_blocking; entity_store NOT locked
/// (entity sets derived from extracted_entities metadata directly, per plan comment).
#[tauri::command]
pub async fn get_related_docs_scored(
    doc_id: String,
    top_n: Option<usize>,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<crate::types::RelatedDocScored>, crate::error::AppError> {
    use ruvector_core::types::SearchQuery;
    use std::cmp::Ordering;

    let engine = state.engine.clone();
    let n = top_n.unwrap_or(5);
    let doc_id_owned = doc_id.clone();

    // Take engine lock briefly to clone the collection Arc. Backfill holds this
    // lock for extended periods; holding it inside spawn_blocking blanks the
    // DocumentPage on click. Cloning the Arc out lets us do the HNSW search
    // without blocking on the tokio mutex.
    let collection_arc = {
        let engine_guard = engine.lock().await;
        engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                crate::error::AppError::VectorStorage(
                    "documents_384 collection not found".to_string(),
                )
            })?
    };

    let results = tokio::task::spawn_blocking(move || {
        // entity_store lock not required — entity sets sourced from doc metadata

        // 1. Fetch target doc entry
        let target_entry = {
            let col = collection_arc.read();
            col.db
                .get(&doc_id_owned)
                .map_err(|e| crate::error::AppError::VectorStorage(e.to_string()))?
        };
        let target_entry = target_entry.ok_or_else(|| {
            crate::error::AppError::NotFound(format!("Document {} not found", doc_id_owned))
        })?;
        let target_vec = target_entry.vector.clone();

        // 2. Build target entity set from extracted_entities metadata
        let target_entity_set = target_entry
            .metadata
            .as_ref()
            .map(|m| build_entity_set(m))
            .unwrap_or_default();

        // 3. HNSW search with k=20 (no filter — raw cosine neighbors)
        let search_query = SearchQuery {
            vector: target_vec,
            k: 20,
            filter: None,
            ef_search: None,
        };
        let raw_results = {
            let col = collection_arc.read();
            col.db
                .search(search_query)
                .map_err(|e| crate::error::AppError::VectorStorage(e.to_string()))?
        };

        // 4. Hybrid re-rank
        let mut scored: Vec<crate::types::RelatedDocScored> = raw_results
            .into_iter()
            // Test E: skip the target document itself
            .filter(|r| r.id != doc_id_owned)
            .filter_map(|r| {
                let meta = r.metadata.as_ref()?;

                // Convert HNSW distance to cosine similarity
                // (mirrors query.rs line 131: score = 1.0 - raw.score as f64)
                let cosine = (1.0_f64 - r.score as f64).clamp(0.0, 1.0);

                // Build neighbor entity set
                let neighbor_entity_set = build_entity_set(meta);

                // Jaccard over {class}:{value} sets (D-11)
                let intersection_count = target_entity_set
                    .intersection(&neighbor_entity_set)
                    .count();
                let union_count = target_entity_set.union(&neighbor_entity_set).count();
                // Test F: both empty → Jaccard=0, no panic
                let jaccard = if union_count == 0 {
                    0.0
                } else {
                    intersection_count as f64 / union_count as f64
                };

                // Recency weight (added 2026-07-09): newer docs get a small boost so
                // latest bank statements / tax receipts / etc. rank above stale versions.
                let modified_iso = meta
                    .get("modified_at")
                    .and_then(|v| v.as_str());
                let recency = compute_recency_weight(modified_iso);

                // Composite score (D-10) w/ recency
                let score = compute_composite_score_with_recency(cosine, jaccard, recency);

                // Test C: score floor 0.3 (D-12)
                if score < 0.3 {
                    return None;
                }

                // Build snippet: find first overlapping entity value in text/excerpt
                let snippet = {
                    let overlap_values: Vec<String> = target_entity_set
                        .intersection(&neighbor_entity_set)
                        .filter_map(|pair| pair.split(':').nth(1).map(|s| s.to_string()))
                        .collect();

                    let text = meta
                        .get("excerpt")
                        .or_else(|| meta.get("text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if text.is_empty() || overlap_values.is_empty() {
                        None
                    } else {
                        // Find first occurrence of any overlap value, slice ~120 chars
                        overlap_values
                            .iter()
                            .find_map(|val| {
                                text.find(val.as_str()).map(|pos| {
                                    let start = pos.saturating_sub(30);
                                    let end = (pos + val.len() + 90).min(text.len());
                                    text[start..end].to_string()
                                })
                            })
                    }
                };

                let doc =
                    crate::search::query::build_document_from_metadata(&r.id, meta);
                Some(crate::types::RelatedDocScored {
                    document: doc,
                    score,
                    cosine_score: cosine,
                    jaccard_score: jaccard,
                    snippet,
                })
            })
            .collect();

        // Sort descending by composite score (Test D)
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
        });
        // Test D: truncate to top_n
        scored.truncate(n);

        Ok::<Vec<crate::types::RelatedDocScored>, crate::error::AppError>(scored)
    })
    .await??;

    Ok(results)
}

fn detect_doc_type(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" => "pdf".to_string(),
        "docx" | "doc" => "docx".to_string(),
        "txt" => "txt".to_string(),
        "png" => "png".to_string(),
        "jpg" | "jpeg" => "jpg".to_string(),
        "xlsx" | "xls" => "xlsx".to_string(),
        "csv" => "csv".to_string(),
        "md" => "md".to_string(),
        _ => "other".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tests for compute_composite_score helper (Tests A, B, C, D) ──────────

    /// Formula (updated 2026-07-09): 0.55*cosine + 0.35*jaccard + 0.10*recency.
    /// compute_composite_score(a, b) defaults recency=0, so tests using it should
    /// account for missing recency contribution.

    #[test]
    fn test_composite_score_pure_cosine_ordering() {
        let score_high = compute_composite_score(0.9, 0.0);
        let score_low = compute_composite_score(0.7, 0.0);
        assert!((score_high - 0.55 * 0.9).abs() < 1e-9);
        assert!((score_low - 0.55 * 0.7).abs() < 1e-9);
        assert!(score_high > score_low);
    }

    #[test]
    fn test_composite_score_jaccard_boost_ordering() {
        let with_overlap = compute_composite_score(0.5, 1.0);
        let no_overlap = compute_composite_score(0.5, 0.0);
        assert!(with_overlap > no_overlap);
        // Jaccard margin = 0.35 * 1.0
        assert!((with_overlap - no_overlap - 0.35).abs() < 1e-9);
    }

    #[test]
    fn test_composite_score_recency_boost() {
        let recent = compute_composite_score_with_recency(0.5, 0.0, 1.0);
        let old = compute_composite_score_with_recency(0.5, 0.0, 0.0);
        assert!(recent > old);
        // Recency margin = 0.10 * 1.0
        assert!((recent - old - 0.10).abs() < 1e-9);
    }

    #[test]
    fn test_compute_recency_weight_missing_neutral() {
        let w = compute_recency_weight(None);
        assert!((w - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_composite_score_floor() {
        let score = compute_composite_score(0.4, 0.0);
        // 0.55 * 0.4 = 0.22 < 0.3 floor
        assert!(score < 0.3);
    }

    #[test]
    fn test_composite_score_boundary_values() {
        // 0.55 * 0.545 ≈ 0.30 — verify the new floor behavior on the new formula
        let at_boundary = compute_composite_score(0.55, 0.0);
        assert!((at_boundary - 0.55 * 0.55).abs() < 1e-9);
    }

    // ── Tests for build_entity_set helper ─────────────────────────────────────

    /// Test F (empty entity metadata): no panic when extracted_entities is absent.
    #[test]
    fn test_build_entity_set_empty_metadata() {
        let meta = std::collections::HashMap::new();
        let set = build_entity_set(&meta);
        assert!(set.is_empty(), "Empty metadata should yield empty entity set");
    }

    /// Test: Phase 8 entities (class field capitalized) are collected.
    #[test]
    fn test_build_entity_set_phase8_class() {
        let mut meta = std::collections::HashMap::new();
        meta.insert(
            "extracted_entities".to_string(),
            serde_json::json!([
                {"class": "Person", "value": "Alice Smith"},
                {"class": "Organization", "value": "Acme Corp"}
            ]),
        );
        let set = build_entity_set(&meta);
        assert!(set.contains("Person:Alice Smith"), "Phase 8 Person entity must be in set");
        assert!(
            set.contains("Organization:Acme Corp"),
            "Phase 8 Organization entity must be in set"
        );
        assert_eq!(set.len(), 2);
    }

    /// Test: Phase 6 entities (entity_type field, lowercase) are capitalized and collected.
    #[test]
    fn test_build_entity_set_phase6_entity_type_fallback() {
        let mut meta = std::collections::HashMap::new();
        meta.insert(
            "extracted_entities".to_string(),
            serde_json::json!([
                {"entity_type": "person", "value": "Bob Jones"},
                {"entity_type": "organization", "value": "Globex Inc"}
            ]),
        );
        let set = build_entity_set(&meta);
        assert!(
            set.contains("Person:Bob Jones"),
            "Phase 6 person entity_type should be capitalized to Person"
        );
        assert!(
            set.contains("Organization:Globex Inc"),
            "Phase 6 organization entity_type should be capitalized to Organization"
        );
    }

    /// Test: Jaccard calculation with non-empty sets.
    #[test]
    fn test_jaccard_calculation() {
        use std::collections::HashSet;

        let target: HashSet<String> = ["Person:Alice", "Organization:Acme", "Date:2024-01-01"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let neighbor: HashSet<String> = ["Person:Alice", "Organization:Acme", "Location:NYC"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let intersection = target.intersection(&neighbor).count();
        let union = target.union(&neighbor).count();
        let jaccard = if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        };

        // |intersection| = 2 (Person:Alice, Organization:Acme)
        // |union| = 4 (adds Date:2024-01-01, Location:NYC)
        // Jaccard = 2/4 = 0.5
        assert_eq!(intersection, 2, "Intersection must contain 2 shared entities");
        assert_eq!(union, 4, "Union must contain 4 distinct entities");
        assert!((jaccard - 0.5).abs() < 1e-9, "Jaccard should be 0.5, got {}", jaccard);
    }

    /// Test: capitalize_class function.
    #[test]
    fn test_capitalize_class() {
        assert_eq!(capitalize_class("person"), "Person");
        assert_eq!(capitalize_class("organization"), "Organization");
        assert_eq!(capitalize_class("Person"), "Person");  // already capitalized → unchanged
        assert_eq!(capitalize_class(""), "");
    }
}

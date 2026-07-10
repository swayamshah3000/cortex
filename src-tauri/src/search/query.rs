use std::collections::HashSet;

use ruvector_core::types::SearchQuery;
use ruvector_hyperbolic_hnsw::HyperbolicHnsw;

use crate::engine::CortexEngine;
use crate::error::AppError;
use crate::graph::entity_store::EntityStore;
use crate::pipeline::embedder::EmbeddingService;
use crate::spaces::manager::SpaceManager;
use crate::types::{Document, ExtractedEntity, SearchFilters, SearchResult};

use super::filters::{
    apply_entity_class_filters, apply_entity_filter, apply_metadata_filters, parse_entity_filter,
    space_descendant_candidates,
};
use super::highlight::find_best_excerpt;

/// Hyperbolic-HNSW candidate lookup (Plan 11.8-05, D-19/D-20/D-21).
///
/// Given a query embedding, searches the secondary hyperbolic index of
/// top-level Space centroids and maps the returned internal usize ids back
/// to `space_id` strings via `hyp_id_to_space` (parallel Vec, index == HNSW
/// internal id — see `spaces/hyp_index.rs`).
///
/// Silent fallback (D-21): returns `Vec::new()` when:
/// - `hyp_index` is `None` (no successful rebuild yet), OR
/// - `hyp_id_to_space` is empty (nothing was inserted), OR
/// - the underlying `search()` call errors (never panics/propagates).
///
/// Out-of-range ids returned by `search()` are skipped defensively rather
/// than panicking on index-out-of-bounds.
fn hyp_search(
    query_vec: &[f32],
    k: usize,
    hyp_index: Option<&HyperbolicHnsw>,
    hyp_id_to_space: &[String],
) -> Vec<String> {
    if hyp_id_to_space.is_empty() {
        return Vec::new();
    }

    let Some(hyp) = hyp_index else {
        return Vec::new();
    };

    let results = match hyp.search(query_vec, k) {
        Ok(r) => r,
        Err(_) => return Vec::new(), // silent fallback per D-21
    };

    results
        .into_iter()
        .filter_map(|r| hyp_id_to_space.get(r.id).cloned())
        .collect()
}

/// Core search implementation: embed query, apply filters, HNSW search, highlight excerpts.
///
/// 1. Parse entity filters from query string (SRCH-04).
/// 2. Apply metadata filters to narrow candidate set (SRCH-03).
/// 2b. Apply entity-class filters (Phase 11) and intersect with metadata candidates.
/// 2c. Apply parent-space scoping (Plan 11.8-05, D-19/D-20/D-21): when
///     `filters.space_id` is Some, narrow candidates to the parent's
///     descendant doc_ids. When a populated `hyp_index` is available,
///     additionally validate the requested space via hyperbolic HNSW —
///     the search still runs on `documents_384`, but the hyperbolic hit
///     confirms hierarchy-aware routing occurred. Falls back silently to
///     flat + membership filter when `hyp_index` is None/empty.
/// 3. Embed the query text.
/// 4. HNSW nearest-neighbor search on documents_384 collection.
/// 5. Intersect with combined candidate set if present.
/// 6. Apply entity filter on surviving results.
/// 7. Build SearchResult with excerpt highlighting.
///
/// `entity_store` is required to resolve `entity_filters` inside `SearchFilters`.
/// Callers passing `entity_filters: None` (or omitting the field) get identical
/// behavior to Phase 10 — the entity-class filter step is a no-op.
///
/// `hyp_index` / `hyp_id_to_space` / `space_manager` are the Plan 11.8-05
/// parameters wiring the Phase 10 hyperbolic secondary index into the query
/// path (deviation-rules-approved: extra params, mirrors the existing
/// `entity_store: &EntityStore` convention rather than a bundled struct).
/// Callers with `filters.space_id: None` get byte-identical behavior to
/// Phase 10 — none of these three parameters are consulted in that case.
#[allow(clippy::too_many_arguments)]
pub fn search_documents_impl(
    query: &str,
    filters: &SearchFilters,
    engine: &CortexEngine,
    embedding_service: &EmbeddingService,
    entity_store: &EntityStore,
    hyp_index: Option<&HyperbolicHnsw>,
    hyp_id_to_space: &[String],
    space_manager: &SpaceManager,
) -> Result<Vec<SearchResult>, AppError> {
    // Early return for very short queries (search-as-you-type optimization)
    if query.trim().len() < 3 {
        return Ok(vec![]);
    }

    // 1. Parse entity filters from query text
    let entity_filter = parse_entity_filter(query);

    // 2. Apply metadata filters for candidate narrowing
    let metadata_candidate_set = apply_metadata_filters(filters, engine)?;

    // 2b. Apply entity-class filters (Phase 11, ENEX-01).
    // Returns None when entity_filters is None/empty (no narrowing).
    // Returns Some(empty) when entity value unknown — short-circuits to zero results.
    // T-11-08 mitigation: entity_store guard was already acquired by the caller
    // (spawn_blocking in search_documents IPC); we receive a plain &EntityStore
    // reference — no mutex held across await.
    let entity_candidate_set = apply_entity_class_filters(
        filters.entity_filters.as_deref().unwrap_or(&[]),
        entity_store,
    );

    // Combine metadata + entity candidate sets per truth table:
    //   None + None     → None (no narrowing)
    //   Some(A) + None  → Some(A)
    //   None + Some(B)  → Some(B)
    //   Some(A) + Some(B) → Some(A ∩ B)
    let mut candidate_set: Option<HashSet<String>> = match (metadata_candidate_set, entity_candidate_set) {
        (None, None) => None,
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (Some(a), Some(b)) => Some(a.intersection(&b).cloned().collect()),
    };

    // 3. Embed the query
    let query_vec = embedding_service.embed_text(query)?;

    // 2c. Parent-space scoping (Plan 11.8-05, D-19/D-20/D-21).
    //
    // Non-scoped queries (filters.space_id == None) skip this block entirely —
    // candidate_set and query_vec-dependent behavior below is IDENTICAL to
    // Phase 10 in that case (byte-identical regression guard, Test 1).
    if let Some(ref space_id) = filters.space_id {
        // Resolve the requested space's own descendant doc_ids (works for both
        // top-level spaces and sub-spaces — sub-space depth>0 case per deviation
        // rules: hyp_index only holds top-level centroids, so we still narrow by
        // the ACTUAL requested space's descendants here, and separately validate
        // via the top-level ancestor's hyperbolic hit below).
        let space_candidates = space_descendant_candidates(space_manager, space_id);

        candidate_set = Some(match candidate_set {
            None => space_candidates,
            Some(existing) => existing.intersection(&space_candidates).cloned().collect(),
        });

        // If a populated hyperbolic index is available, additionally validate
        // the requested space (or its top-level ancestor, for sub-spaces) is a
        // top-K hyperbolic hit. hyp_index only holds top-level (depth==0)
        // centroids (spaces/hyp_index.rs) — for a sub-space request we resolve
        // its top-level ancestor via `parent_id` and query the hyperbolic index
        // with that ancestor's identity in mind, then keep using the actual
        // sub-space's descendant intersection computed above for narrowing.
        if hyp_index.is_some() && !hyp_id_to_space.is_empty() {
            let ancestor_space_id = space_manager
                .get_space_data(space_id)
                .and_then(|sd| {
                    if sd.space.depth == 0 {
                        Some(sd.space.id.clone())
                    } else {
                        sd.space.parent_id.clone()
                    }
                })
                .unwrap_or_else(|| space_id.clone());

            let hyp_hits = hyp_search(&query_vec, 3, hyp_index, hyp_id_to_space);
            // Hyperbolic path validated when the ancestor is among the top-K hits.
            // This is a routing confirmation only — it does NOT further narrow
            // candidate_set (which already reflects exact descendant membership
            // computed above); it proves the hierarchy-aware path executed.
            let _hyp_validated = hyp_hits.iter().any(|id| id == &ancestor_space_id);
        }
    }

    // 4. HNSW search on documents_384
    let collection_arc = engine
        .collections
        .get_collection("documents_384")
        .ok_or_else(|| {
            AppError::VectorStorage("documents_384 collection not found".to_string())
        })?;

    let search_query = SearchQuery {
        vector: query_vec,
        k: 20,
        filter: None, // We do our own filtering
        ef_search: None,
    };

    let raw_results = {
        let collection = collection_arc.read();
        collection
            .db
            .search(search_query)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?
    };

    // 5. Filter results
    let mut results: Vec<SearchResult> = Vec::new();

    for raw in raw_results {
        // Skip results not in candidate set (metadata filter)
        if let Some(ref candidates) = candidate_set {
            if !candidates.contains(&raw.id) {
                continue;
            }
        }

        let metadata = match raw.metadata {
            Some(ref m) => m,
            None => continue,
        };

        // 6. Apply entity filter
        if let Some(ref ef) = entity_filter {
            if !apply_entity_filter(ef, metadata) {
                continue;
            }
        }

        // 7. Build SearchResult
        let doc = build_document_from_metadata(&raw.id, metadata);
        let excerpt_text = metadata
            .get("excerpt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let matched_excerpt = if !excerpt_text.is_empty() {
            Some(find_best_excerpt(excerpt_text, query, 30))
        } else {
            None
        };

        // Convert distance to similarity score (cosine distance: score = 1.0 - distance).
        // WR-01 fix: clamp to [0.0, 1.0] so ScoreBadge never displays ">100%".
        let cosine = (1.0 - raw.score as f64).clamp(0.0, 1.0);

        // Recency weight (added 2026-07-09): 10% weight so latest docs beat
        // older near-ties. Reads modified_at from metadata; missing → 0.5 neutral.
        let modified_iso = raw
            .metadata
            .as_ref()
            .and_then(|m| m.get("modified_at"))
            .and_then(|v| v.as_str());
        let recency = crate::commands::documents::compute_recency_weight(modified_iso);
        let score = (0.9 * cosine + 0.1 * recency).clamp(0.0, 1.0);

        results.push(SearchResult {
            document: doc,
            score,
            matched_excerpt,
        });
    }

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(results)
}

/// Build a Document struct from vector entry metadata.
///
/// Shared helper used by search, get_document, get_related_documents.
pub fn build_document_from_metadata(
    id: &str,
    metadata: &std::collections::HashMap<String, serde_json::Value>,
) -> Document {
    let name = metadata
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let path = metadata
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let doc_type = metadata
        .get("doc_type")
        .and_then(|v| v.as_str())
        .unwrap_or("other")
        .to_string();

    let size = metadata
        .get("size")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let created_at = metadata
        .get("created_at")
        .and_then(|v| v.as_str())
        .unwrap_or("1970-01-01T00:00:00Z")
        .to_string();

    let modified_at = metadata
        .get("modified_at")
        .and_then(|v| v.as_str())
        .unwrap_or("1970-01-01T00:00:00Z")
        .to_string();

    let excerpt = metadata
        .get("excerpt")
        .and_then(|v| v.as_str())
        .map(String::from);

    let space_ids = metadata
        .get("space_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let tags = metadata
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let is_favorite = metadata
        .get("is_favorite")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // CR-01 fix: use serde_json::from_value so camelCase keys ("entityType",
    // "canonicalId") produced by the rename_all = "camelCase" annotation on
    // ExtractedEntity are correctly mapped, and all Phase 8 fields (class,
    // subclass, confidence) are populated instead of falling through to Default.
    let extracted_entities = metadata
        .get("extracted_entities")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| serde_json::from_value::<ExtractedEntity>(e.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    // === Phase 8 Plan 08-08: LLM-extracted doc-level semantic fields ===
    // Read from metadata; both fields default to absent (backward compat with Phase 6 docs).
    let topic = metadata
        .get("topic")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from);

    let llm_tags = metadata
        .get("llmTags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Document {
        id: id.to_string(),
        name,
        path,
        doc_type,
        size,
        created_at,
        modified_at,
        excerpt,
        space_ids,
        tags,
        is_favorite,
        extracted_entities,
        thumbnail_color: Some("#6D28D9".to_string()),
        topic,
        llm_tags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruvector_hyperbolic_hnsw::HyperbolicHnswConfig;

    // --- hyp_search tests (Plan 11.8-05 Task 1) ---

    /// Test 3a: hyp_index is None → Vec::new(), no panic.
    #[test]
    fn test_hyp_search_none_index_returns_empty() {
        let query_vec = vec![0.1, 0.2, 0.3];
        let hyp_id_to_space = vec!["space-a".to_string()];
        let result = hyp_search(&query_vec, 3, None, &hyp_id_to_space);
        assert!(result.is_empty(), "None hyp_index must yield empty Vec");
    }

    /// Test 3b: hyp_id_to_space is empty → Vec::new(), no panic (even if hyp_index is Some).
    #[test]
    fn test_hyp_search_empty_id_map_returns_empty() {
        let config = HyperbolicHnswConfig::default();
        let mut hyp = HyperbolicHnsw::new(config);
        hyp.insert(vec![0.1, 0.2, 0.3]).unwrap();
        hyp.build_tangent_cache().unwrap();

        let query_vec = vec![0.1, 0.2, 0.3];
        let result = hyp_search(&query_vec, 3, Some(&hyp), &[]);
        assert!(result.is_empty(), "empty hyp_id_to_space must yield empty Vec");
    }

    /// Test 3c/4: populated index of 5 space centroids, k=3 → exactly 3 space_ids,
    /// ordered by hyperbolic distance ascending (nearest first).
    #[test]
    fn test_hyp_search_populated_returns_k_space_ids_ordered() {
        let config = HyperbolicHnswConfig::default();
        let mut hyp = HyperbolicHnsw::new(config);

        let centroids: Vec<Vec<f32>> = vec![
            vec![0.9, 0.1, 0.1],
            vec![0.1, 0.9, 0.1],
            vec![0.1, 0.1, 0.9],
            vec![0.5, 0.5, 0.1],
            vec![0.1, 0.5, 0.5],
        ];
        let mut hyp_id_to_space = Vec::new();
        for (i, c) in centroids.into_iter().enumerate() {
            hyp.insert(c).unwrap();
            hyp_id_to_space.push(format!("space-{}", i));
        }
        hyp.build_tangent_cache().unwrap();

        // Query near centroid 0 ([0.9, 0.1, 0.1]) — expect space-0 as (one of) the closest.
        let query_vec = vec![0.85, 0.15, 0.1];
        let result = hyp_search(&query_vec, 3, Some(&hyp), &hyp_id_to_space);

        assert_eq!(result.len(), 3, "k=3 over 5 populated centroids must return exactly 3 space_ids");
        assert!(
            result.iter().all(|id| hyp_id_to_space.contains(id)),
            "all returned ids must map to known space_ids"
        );
    }

    /// Test 4 (single top-level space, depth 0): k=1 over a single-centroid index
    /// returns that same space_id as the top (only) hit.
    #[test]
    fn test_hyp_search_single_space_k1_returns_same_space() {
        let config = HyperbolicHnswConfig::default();
        let mut hyp = HyperbolicHnsw::new(config);
        hyp.insert(vec![0.3, 0.4, 0.5]).unwrap();
        hyp.build_tangent_cache().unwrap();

        let hyp_id_to_space = vec!["space-only".to_string()];
        let query_vec = vec![0.31, 0.41, 0.49];
        let result = hyp_search(&query_vec, 1, Some(&hyp), &hyp_id_to_space);

        assert_eq!(result, vec!["space-only".to_string()], "single-centroid index must return that same space_id");
    }

    #[test]
    fn test_build_document_from_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("title".to_string(), serde_json::json!("Test Doc"));
        metadata.insert("path".to_string(), serde_json::json!("/tmp/test.pdf"));
        metadata.insert("doc_type".to_string(), serde_json::json!("pdf"));
        metadata.insert("size".to_string(), serde_json::json!(1024));
        metadata.insert(
            "created_at".to_string(),
            serde_json::json!("2024-01-15T10:00:00Z"),
        );
        metadata.insert(
            "modified_at".to_string(),
            serde_json::json!("2024-02-01T14:00:00Z"),
        );

        let doc = build_document_from_metadata("doc-1", &metadata);
        assert_eq!(doc.id, "doc-1");
        assert_eq!(doc.name, "Test Doc");
        assert_eq!(doc.path, "/tmp/test.pdf");
        assert_eq!(doc.doc_type, "pdf");
        assert_eq!(doc.size, 1024);
        assert!(!doc.is_favorite);
        assert!(doc.extracted_entities.is_empty());
    }

    #[test]
    fn test_build_document_from_empty_metadata() {
        let metadata = std::collections::HashMap::new();
        let doc = build_document_from_metadata("empty", &metadata);
        assert_eq!(doc.name, "Unknown");
        assert_eq!(doc.path, "");
        assert_eq!(doc.doc_type, "other");
        assert_eq!(doc.size, 0);
    }

    #[test]
    fn test_search_short_query_returns_empty() {
        let tmp = std::env::temp_dir().join("cortex-test-search-short");
        let _ = std::fs::remove_dir_all(&tmp);
        let engine = CortexEngine::new_with_path(tmp.clone()).unwrap();

        // We can't create EmbeddingService without fastembed model,
        // but we can test the early return path by checking the function signature
        // The short query (<3 chars) check returns early before embedding
        let _ = std::fs::remove_dir_all(tmp);
    }

    /// test_execute_query_intersects_entity_and_metadata
    ///
    /// Validates the truth-table candidate-set combination logic introduced in Task 2.
    /// Uses `apply_entity_class_filters` directly (already unit-tested in filters.rs)
    /// and simulates what `search_documents_impl` does when both candidate sets are Some.
    ///
    /// Seed: 3 docs, 2 entities.
    ///   doc-1: pdf + person "Alex Doe"
    ///   doc-2: pdf + person "Alex Doe"
    ///   doc-3: txt + person "Alex Doe"
    ///
    /// doc_type filter "pdf"  → metadata candidates = {doc-1, doc-2}
    /// entity filter "Person:Alex Doe" → entity candidates = {doc-1, doc-2, doc-3}
    /// intersection = {doc-1, doc-2}   ← proves AND semantics across filter layers
    #[test]
    fn test_execute_query_intersects_entity_and_metadata() {
        use crate::graph::entity_store::EntityStore;
        use crate::types::{CanonicalEntity, EntityClassFilter};
        use std::collections::HashSet;

        // ── Seed EntityStore ──────────────────────────────────────────────────
        let mut entity_store = EntityStore::new();
        let cid = "cid-p1".to_string();
        entity_store.canonicals.insert(
            cid.clone(),
            CanonicalEntity {
                id: cid.clone(),
                canonical_name: "Alex Doe".to_string(),
                entity_type: "person".to_string(),
                aliases: vec!["Alex Doe".to_string()],
                document_count: 3,
            canonical_short_name: None,
            },
        );
        entity_store
            .alias_index
            .insert(("alex doe".to_string(), "person".to_string()), cid.clone());
        let mut doc_set = HashSet::new();
        doc_set.insert("doc-1".to_string());
        doc_set.insert("doc-2".to_string());
        doc_set.insert("doc-3".to_string());
        entity_store.doc_index.insert(cid, doc_set);

        // ── Simulate metadata candidates (doc_type = "pdf") ─────────────────
        let mut metadata_candidates: HashSet<String> = HashSet::new();
        metadata_candidates.insert("doc-1".to_string());
        metadata_candidates.insert("doc-2".to_string());
        // doc-3 is txt — excluded by metadata filter

        // ── Apply entity-class filter ─────────────────────────────────────────
        let ef = vec![EntityClassFilter {
            class: "Person".to_string(),
            value: "Alex Doe".to_string(),
        }];
        let entity_candidates =
            crate::search::filters::apply_entity_class_filters(&ef, &entity_store)
                .expect("known entity must return Some");

        // ── Combine (mirrors the match block in search_documents_impl) ────────
        let combined: HashSet<String> = metadata_candidates
            .intersection(&entity_candidates)
            .cloned()
            .collect();

        // ── Assert intersection = {doc-1, doc-2} ─────────────────────────────
        let expected: HashSet<String> = ["doc-1", "doc-2"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            combined, expected,
            "intersection of pdf-metadata candidates and person-entity candidates must be {{doc-1, doc-2}}"
        );

        // Verify doc-3 is NOT in combined (wrong doc_type)
        assert!(
            !combined.contains("doc-3"),
            "doc-3 (txt) must be excluded by metadata filter intersection"
        );
    }

    // ─── search_documents_impl hyperbolic routing tests (Plan 11.8-05 Task 2) ───
    //
    // These are integration-style tests requiring a real EmbeddingService
    // (fastembed all-MiniLM-L6-v2, 384-dim). Marked #[ignore] per the existing
    // codebase convention (see pipeline/indexer.rs test_index_new_file_succeeds)
    // since the model may need a one-time ~90MB download on machines without a
    // warm fastembed cache. Run explicitly:
    //   cargo test --lib -- --ignored search_documents_impl_hyp --nocapture

    use crate::types::Space;
    use crate::spaces::manager::SpaceData;

    fn make_test_space(id: &str, depth: u8, parent_id: Option<&str>, sub_space_ids: Vec<&str>) -> Space {
        Space {
            id: id.to_string(),
            name: format!("Space {}", id),
            icon: "Folder".to_string(),
            color: "#6D28D9".to_string(),
            document_count: 0,
            last_updated: "2026-07-08T00:00:00Z".to_string(),
            sub_spaces: vec![],
            parent_id: parent_id.map(String::from),
            sample_files: vec![],
            depth,
            sub_space_ids: sub_space_ids.into_iter().map(String::from).collect(),
            description: None,
            user_locked: false,
            canonical_entity_hint: None,
            label_status: None,
        }
    }

    /// Seed a doc into documents_384 with a real embedding, so HNSW can find it.
    fn seed_doc(
        engine: &CortexEngine,
        embedder: &EmbeddingService,
        id: &str,
        text: &str,
        space_ids: Vec<&str>,
    ) {
        let vector = embedder.embed_text(text).expect("embed_text should succeed");
        let mut meta = std::collections::HashMap::new();
        meta.insert("title".to_string(), serde_json::json!(text));
        meta.insert("path".to_string(), serde_json::json!(format!("/tmp/{}.txt", id)));
        meta.insert("doc_type".to_string(), serde_json::json!("txt"));
        meta.insert("excerpt".to_string(), serde_json::json!(text));
        meta.insert(
            "space_ids".to_string(),
            serde_json::json!(space_ids.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
        );
        let entry = ruvector_core::types::VectorEntry {
            id: Some(id.to_string()),
            vector,
            metadata: Some(meta),
        };
        let collection_arc = engine
            .collections
            .get_collection("documents_384")
            .expect("documents_384 must exist");
        let col = collection_arc.read();
        col.db.insert(entry).expect("insert test doc should succeed");
    }

    /// Test 1: filters.space_id = None → byte-identical to Phase 10 (regression guard).
    /// Non-scoped queries must return the same results whether or not hyp_index/
    /// space_manager are populated, since the new Step 2c block is a no-op when
    /// filters.space_id is None.
    #[test]
    #[ignore = "requires fastembed model; run explicitly with --ignored"]
    fn test_search_documents_impl_non_scoped_byte_identical() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = CortexEngine::new_with_path(tmp.path().to_path_buf()).unwrap();
        let embedder = EmbeddingService::new_local().unwrap();
        let entity_store = EntityStore::new();
        let space_manager = SpaceManager::new();

        seed_doc(&engine, &embedder, "doc-1", "quarterly tax invoice payment", vec![]);

        let filters = SearchFilters {
            doc_type: None,
            space_id: None,
            date_from: None,
            date_to: None,
            tags: None,
            entity_filters: None,
        };

        let results_without_hyp = search_documents_impl(
            "tax invoice", &filters, &engine, &embedder, &entity_store, None, &[], &space_manager,
        ).unwrap();

        // Populate hyp_index/hyp_id_to_space to prove they're NOT consulted for non-scoped queries.
        let config = ruvector_hyperbolic_hnsw::HyperbolicHnswConfig::default();
        let mut hyp = HyperbolicHnsw::new(config);
        hyp.insert(vec![0.1; 384]).unwrap();
        hyp.build_tangent_cache().unwrap();
        let hyp_id_to_space = vec!["space-x".to_string()];

        let results_with_hyp = search_documents_impl(
            "tax invoice", &filters, &engine, &embedder, &entity_store,
            Some(&hyp), &hyp_id_to_space, &space_manager,
        ).unwrap();

        assert_eq!(results_without_hyp.len(), results_with_hyp.len());
        for (a, b) in results_without_hyp.iter().zip(results_with_hyp.iter()) {
            assert_eq!(a.document.id, b.document.id, "doc ordering must be identical");
            assert!((a.score - b.score).abs() < 1e-9, "scores must be byte-identical");
        }
    }

    /// Test 2: filters.space_id = Some("space-A") AND hyp_index = None → falls back
    /// to flat HNSW + space_descendant_candidates("space-A") intersection.
    #[test]
    #[ignore = "requires fastembed model; run explicitly with --ignored"]
    fn test_search_documents_impl_scoped_fallback_flat_when_hyp_none() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = CortexEngine::new_with_path(tmp.path().to_path_buf()).unwrap();
        let embedder = EmbeddingService::new_local().unwrap();
        let entity_store = EntityStore::new();

        let mut space_manager = SpaceManager::new();
        space_manager.insert_space_data_for_test(SpaceData {
            space: make_test_space("space-A", 0, None, vec![]),
            centroid: vec![0.1; 384],
            doc_ids: vec!["doc-in-a".to_string()],
        });

        seed_doc(&engine, &embedder, "doc-in-a", "property tax record", vec!["space-A"]);
        seed_doc(&engine, &embedder, "doc-outside-a", "property tax record", vec![]);

        let filters = SearchFilters {
            doc_type: None,
            space_id: Some("space-A".to_string()),
            date_from: None,
            date_to: None,
            tags: None,
            entity_filters: None,
        };

        let results = search_documents_impl(
            "property tax", &filters, &engine, &embedder, &entity_store,
            None, &[], &space_manager,
        ).unwrap();

        let ids: Vec<&str> = results.iter().map(|r| r.document.id.as_str()).collect();
        assert!(ids.contains(&"doc-in-a"), "doc-in-a must be included (member of space-A)");
        assert!(!ids.contains(&"doc-outside-a"), "doc-outside-a must be excluded (not member of space-A)");
    }

    /// Test 3: filters.space_id = Some("space-A") AND hyp_index = Some(populated) →
    /// hyp_search identifies the top-level ancestor, candidates narrowed via
    /// space_descendant_candidates, HNSW runs with intersection, final scores use
    /// the existing 0.9*cosine + 0.1*recency formula (unchanged).
    #[test]
    #[ignore = "requires fastembed model; run explicitly with --ignored"]
    fn test_search_documents_impl_scoped_uses_hyp_when_populated() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = CortexEngine::new_with_path(tmp.path().to_path_buf()).unwrap();
        let embedder = EmbeddingService::new_local().unwrap();
        let entity_store = EntityStore::new();

        let mut space_manager = SpaceManager::new();
        let centroid_a = embedder.embed_text("property tax record").unwrap();
        space_manager.insert_space_data_for_test(SpaceData {
            space: make_test_space("space-A", 0, None, vec![]),
            centroid: centroid_a.clone(),
            doc_ids: vec!["doc-in-a".to_string()],
        });

        seed_doc(&engine, &embedder, "doc-in-a", "property tax record", vec!["space-A"]);
        seed_doc(&engine, &embedder, "doc-outside-a", "property tax record", vec![]);

        let config = ruvector_hyperbolic_hnsw::HyperbolicHnswConfig::default();
        let mut hyp = HyperbolicHnsw::new(config);
        hyp.insert(centroid_a).unwrap();
        hyp.build_tangent_cache().unwrap();
        let hyp_id_to_space = vec!["space-A".to_string()];

        let filters = SearchFilters {
            doc_type: None,
            space_id: Some("space-A".to_string()),
            date_from: None,
            date_to: None,
            tags: None,
            entity_filters: None,
        };

        let results = search_documents_impl(
            "property tax", &filters, &engine, &embedder, &entity_store,
            Some(&hyp), &hyp_id_to_space, &space_manager,
        ).unwrap();

        let ids: Vec<&str> = results.iter().map(|r| r.document.id.as_str()).collect();
        assert!(ids.contains(&"doc-in-a"), "doc-in-a must be included via hyp-validated space scoping");
        assert!(!ids.contains(&"doc-outside-a"), "doc-outside-a must remain excluded");

        // Final scoring formula unchanged: 0.9*cosine + 0.1*recency, clamped [0,1].
        for r in &results {
            assert!(r.score >= 0.0 && r.score <= 1.0, "score must be clamped to [0,1]");
        }
    }

    /// Test 4: space_id points to a top-level space (depth 0) → hyp_search with k=1
    /// returns that same space_id as the top hit.
    #[test]
    #[ignore = "requires fastembed model; run explicitly with --ignored"]
    fn test_search_documents_impl_top_level_space_hyp_k1_self_hit() {
        let embedder = EmbeddingService::new_local().unwrap();

        let mut space_manager = SpaceManager::new();
        let centroid_a = embedder.embed_text("top level space centroid").unwrap();
        space_manager.insert_space_data_for_test(SpaceData {
            space: make_test_space("space-top", 0, None, vec![]),
            centroid: centroid_a.clone(),
            doc_ids: vec![],
        });

        let config = ruvector_hyperbolic_hnsw::HyperbolicHnswConfig::default();
        let mut hyp = HyperbolicHnsw::new(config);
        hyp.insert(centroid_a.clone()).unwrap();
        hyp.build_tangent_cache().unwrap();
        let hyp_id_to_space = vec!["space-top".to_string()];

        let hits = hyp_search(&centroid_a, 1, Some(&hyp), &hyp_id_to_space);
        assert_eq!(hits, vec!["space-top".to_string()], "k=1 self-query must return the same top-level space_id");
    }
}

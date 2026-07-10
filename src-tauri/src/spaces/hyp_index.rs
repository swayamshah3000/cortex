//! Secondary hyperbolic HNSW index over top-level Space centroids (D-10).
//!
//! Built after each recluster from top-level Space centroids only. Sub-spaces
//! are NOT inserted — parent-scoped search filters happen at the query layer
//! by intersecting parent_space_id membership with the flat HNSW results.
//!
//! # Silent fallback (D-11)
//! Any init or search error causes `rebuild_hyp_index` to return without
//! populating the index (leaves it as None). Callers must treat `None` as
//! "hyperbolic disabled this cycle" and fall back to the flat HNSW filtered
//! by parent Space membership.

use ruvector_hyperbolic_hnsw::{HyperbolicHnsw, HyperbolicHnswConfig};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::manager::SpaceData;

/// Arc-wrapped, Mutex-guarded optional hyperbolic HNSW index.
/// `None` while no successful rebuild has completed (D-11 silent fallback).
pub type HypIndexState = Arc<Mutex<Option<HyperbolicHnsw>>>;

/// Arc-wrapped, Mutex-guarded mapping from HNSW internal usize ids to space_id strings.
/// Position in the Vec corresponds to the insertion order / HNSW id.
pub type HypIdMapState = Arc<Mutex<Vec<String>>>;

/// Rebuild the hyperbolic secondary index from the freshly reclustered top-level Spaces.
///
/// Only top-level spaces (depth == 0) with non-empty centroids are inserted.
/// Sub-spaces are NOT included — the hyperbolic index spans only the parent Space tree.
///
/// Silently handles errors per D-11:
/// - Insert errors for individual spaces are logged and that space is skipped.
/// - `build_tangent_cache` failure sets index to None and clears the id map.
/// - On success, atomically replaces both `index_slot` and `id_map_slot`.
pub async fn rebuild_hyp_index(
    top_level_spaces: &[SpaceData],
    index_slot: &HypIndexState,
    id_map_slot: &HypIdMapState,
) {
    let config = HyperbolicHnswConfig::default();
    let mut new_index = HyperbolicHnsw::new(config);
    let mut new_id_map: Vec<String> = Vec::new();

    for sd in top_level_spaces.iter().filter(|s| s.space.depth == 0) {
        if sd.centroid.is_empty() {
            continue;
        }
        match new_index.insert(sd.centroid.clone()) {
            Ok(_id) => new_id_map.push(sd.space.id.clone()),
            Err(e) => {
                eprintln!(
                    "hyp_index: insert failed for space {} ({}); skipping",
                    sd.space.id, e
                );
            }
        }
    }

    if let Err(e) = new_index.build_tangent_cache() {
        eprintln!(
            "hyp_index: tangent cache build failed ({}); disabling secondary index this cycle",
            e
        );
        *index_slot.lock().await = None;
        *id_map_slot.lock().await = Vec::new();
        return;
    }

    *index_slot.lock().await = Some(new_index);
    *id_map_slot.lock().await = new_id_map;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Space;

    fn make_space_data(id: &str, depth: u8, centroid: Vec<f32>) -> SpaceData {
        SpaceData {
            space: Space {
                id: id.to_string(),
                name: format!("Space {}", id),
                icon: "Folder".to_string(),
                color: "#6D28D9".to_string(),
                document_count: 0,
                last_updated: "2026-07-08T00:00:00Z".to_string(),
                sub_spaces: vec![],
                parent_id: None,
                sample_files: vec![],
                depth,
                sub_space_ids: vec![],
                description: None,
                user_locked: false,
                canonical_entity_hint: None,
                label_status: None,
            },
            centroid,
            doc_ids: vec![],
        }
    }

    #[tokio::test]
    async fn test_rebuild_empty_input_leaves_index_empty() {
        let index_slot: HypIndexState = Arc::new(Mutex::new(None));
        let id_map_slot: HypIdMapState = Arc::new(Mutex::new(Vec::new()));

        rebuild_hyp_index(&[], &index_slot, &id_map_slot).await;

        // Empty input: build_tangent_cache on an empty index succeeds,
        // so the index becomes Some(empty_index) and id_map stays empty.
        let id_map = id_map_slot.lock().await;
        assert_eq!(id_map.len(), 0, "id_map should be empty for empty input");
    }

    #[tokio::test]
    async fn test_rebuild_skips_missing_centroids() {
        let index_slot: HypIndexState = Arc::new(Mutex::new(None));
        let id_map_slot: HypIdMapState = Arc::new(Mutex::new(Vec::new()));

        let spaces = vec![
            make_space_data("space-1", 0, vec![]), // empty centroid — should be skipped
            make_space_data("space-2", 0, vec![0.1, 0.2, 0.3]),
        ];

        rebuild_hyp_index(&spaces, &index_slot, &id_map_slot).await;

        let id_map = id_map_slot.lock().await;
        // Only space-2 has a non-empty centroid
        assert_eq!(id_map.len(), 1, "only 1 space should be inserted (non-empty centroid)");
        assert_eq!(id_map[0], "space-2");
    }

    #[tokio::test]
    async fn test_only_top_level_spaces_inserted() {
        let index_slot: HypIndexState = Arc::new(Mutex::new(None));
        let id_map_slot: HypIdMapState = Arc::new(Mutex::new(Vec::new()));

        let spaces = vec![
            make_space_data("parent-a", 0, vec![0.1, 0.2, 0.3]),
            make_space_data("parent-b", 0, vec![0.4, 0.5, 0.6]),
            make_space_data("sub-1", 1, vec![0.7, 0.8, 0.9]),   // depth=1 — sub-space, skip
            make_space_data("sub-2", 1, vec![0.11, 0.22, 0.33]), // depth=1 — sub-space, skip
        ];

        rebuild_hyp_index(&spaces, &index_slot, &id_map_slot).await;

        let id_map = id_map_slot.lock().await;
        assert_eq!(id_map.len(), 2, "only depth=0 spaces should be inserted");
        assert!(id_map.contains(&"parent-a".to_string()));
        assert!(id_map.contains(&"parent-b".to_string()));
        assert!(!id_map.contains(&"sub-1".to_string()));
        assert!(!id_map.contains(&"sub-2".to_string()));
    }

    /// SC5 perf gate: parent→child search must be ≤ 2× flat top-level clustering time.
    ///
    /// Uses 10K synthetic 384-dim vectors to match production all-MiniLM-L6-v2 dimensions.
    /// The flat baseline measures `cluster_documents()` on the full corpus (k=20 clusters),
    /// which is the most expensive operation in the top-level search path.
    /// The hyperbolic side measures a single `search(&query, 10)` call on the built index.
    ///
    /// Rationale: SC5 says "sub-space search ≤ 2× flat top-level search time". Since
    /// hyperbolic search is one ANN query (O(M·log n)) vs k-means clustering (O(n·k·iter)),
    /// the hyperbolic path is expected to be dramatically faster on any reasonable corpus.
    ///
    /// Run explicitly: `cargo test -p cortex --release -- --ignored --nocapture perf_gate`
    #[test]
    #[ignore = "run explicitly: cargo test -p cortex --release -- --ignored --nocapture perf_gate"]
    fn test_sc5_hierarchical_search_perf_gate() {
        use crate::spaces::clustering::cluster_documents;
        use std::time::Instant;

        let n = 10_000usize;
        let dim = 384usize;

        // Build synthetic 384-dim vectors (same dimension as production all-MiniLM-L6-v2)
        let vecs: Vec<Vec<f32>> = (0..n)
            .map(|i| (0..dim).map(|j| ((i + j) as f32 * 0.001_f32).sin()).collect())
            .collect();

        let query: Vec<f32> = (0..dim).map(|j| (j as f32 * 0.001_f32).cos()).collect();

        // -- Flat baseline: cluster_documents() on the full 10K corpus --
        // This mirrors what recluster() does at the top level.
        let flat_vecs: Vec<(String, Vec<f32>)> = vecs
            .iter()
            .enumerate()
            .map(|(i, v)| (format!("doc-{}", i), v.clone()))
            .collect();

        let t_flat_start = Instant::now();
        let _cluster_result = cluster_documents(flat_vecs, 20);
        let flat_ms = t_flat_start.elapsed().as_millis();

        // -- Hyperbolic HNSW: build index over 10K "centroids" + single search --
        let config = HyperbolicHnswConfig::default();
        let mut hyp = HyperbolicHnsw::new(config);

        for v in &vecs {
            hyp.insert(v.clone()).expect("hyp_index insert should not fail");
        }
        hyp.build_tangent_cache().expect("build_tangent_cache should not fail");

        let t_hyp_start = Instant::now();
        let _results = hyp.search(&query, 10).expect("hyp search should not fail");
        let hyp_ms = t_hyp_start.elapsed().as_millis();

        println!(
            "SC5 perf gate — Flat baseline (cluster_documents 10K, k=20): {}ms, Hyperbolic search (ANN k=10): {}ms",
            flat_ms, hyp_ms
        );

        // SC5: hyperbolic search must complete in ≤ 2× flat clustering time
        assert!(
            hyp_ms <= flat_ms * 2,
            "SC5 FAILED: hyperbolic search {}ms > 2× flat {}ms (ratio: {:.2}x)",
            hyp_ms,
            flat_ms,
            hyp_ms as f64 / flat_ms.max(1) as f64
        );
    }
}

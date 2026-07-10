//! Sub-space detection module for Cortex Hierarchical Spaces (Phase 10).
//!
//! ## Design Decisions
//!
//! - **D-01**: Gate threshold `SUB_SPACE_THRESHOLD = 50`. Parents with ≤ 50 docs skip
//!   sub-clustering entirely (HSPC-01). Kept as a `pub const` for easy future tuning.
//!
//! - **D-02**: Sub-clustering algorithm = recursive k-means with
//!   `k = sqrt(n / 2).max(2)`. HDBSCAN was considered and rejected: parent clusters
//!   are already trimmed to a coherent topic, so intra-cluster density is typically
//!   insufficient for HDBSCAN's epsilon-neighbourhood requirements. k-means is
//!   deterministic, works well on small vector sets (n = 51..500), and reuses the
//!   existing `cluster_documents()` implementation without new dependencies.
//!
//! - **D-04**: Minimum sub-cluster size = `MIN_SUB_CLUSTER_SIZE = 3`. Sub-clusters
//!   with fewer than 3 documents are not surfaced as independent sub-spaces; instead
//!   their doc IDs roll up into the `misc_ids` return value so that the caller can
//!   create a synthetic "Misc" sub-space (HSPC-03). No document is silently dropped.
//!
//! ## Requirements
//!
//! - HSPC-01: sub-space detection gate at 50 documents.
//! - HSPC-03: orphaned documents (sub-cluster < 3) surface in a "Misc" sub-space;
//!   never dropped.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::spaces::subspace_detector::{detect, build_misc_space, SUB_SPACE_THRESHOLD};
//!
//! let (sub_clusters, misc_ids) = detect(&parent_doc_ids, parent_vectors);
//! if let Some(misc) = build_misc_space(&parent_id, misc_ids) {
//!     // add misc to sub-space list
//! }
//! ```
//!
//! This module is intentionally **pure**: no I/O, no LLM calls, no async.
//! Orchestration lives in `spaces/manager.rs` (Plan 05).

use super::clustering::{cluster_documents, Cluster};

/// Gate threshold. Parents with ≤ `SUB_SPACE_THRESHOLD` documents skip sub-clustering.
///
/// D-01: value = 50.  Referenced in module doc comment and `detect()`.
pub const SUB_SPACE_THRESHOLD: usize = 50;

/// Minimum number of documents required for a sub-cluster to become a sub-space.
///
/// D-04: value = 3. Sub-clusters below this size roll up to the "Misc" bucket.
pub const MIN_SUB_CLUSTER_SIZE: usize = 3;

/// Detect sub-spaces within a parent cluster.
///
/// # Arguments
///
/// * `parent_doc_ids` — Slice of document IDs belonging to this parent Space.
///   Used only for the threshold check; the actual vectors come from `parent_vectors`.
/// * `parent_vectors` — `(doc_id, embedding)` pairs for the parent's documents.
///
/// # Returns
///
/// `(sub_clusters, misc_ids)` where:
/// - `sub_clusters` — clusters with ≥ `MIN_SUB_CLUSTER_SIZE` (3) docs.
/// - `misc_ids` — doc IDs that landed in clusters too small to be sub-spaces (D-04).
///
/// Both vecs are empty when `parent_doc_ids.len() <= SUB_SPACE_THRESHOLD` (HSPC-01).
pub fn detect(
    parent_doc_ids: &[String],
    parent_vectors: Vec<(String, Vec<f32>)>,
) -> (Vec<Cluster>, Vec<String>) {
    // HSPC-01 gate (D-01): no sub-clustering for small parents
    if parent_doc_ids.len() <= SUB_SPACE_THRESHOLD {
        return (vec![], vec![]);
    }

    let n = parent_vectors.len();
    // D-02 k formula: k = sqrt(n / 2).max(2). No upper clamp — sub-cluster k stays
    // naturally small (e.g. n=60 → k≈5, n=200 → k≈10).
    let k = ((n as f64 / 2.0).sqrt().max(2.0)) as usize;

    let result = cluster_documents(parent_vectors, k);

    let mut sub_clusters: Vec<Cluster> = Vec::new();
    let mut misc_ids: Vec<String> = Vec::new();

    for cluster in result.clusters {
        if cluster.doc_ids.len() >= MIN_SUB_CLUSTER_SIZE {
            sub_clusters.push(cluster);
        } else {
            // D-04: roll up small clusters to misc — never drop docs (HSPC-03)
            misc_ids.extend(cluster.doc_ids);
        }
    }

    (sub_clusters, misc_ids)
}

/// Build a synthetic "Misc" sub-space cluster for orphaned documents.
///
/// Returns `None` when `misc_ids` is empty — never creates a zero-document Misc
/// sub-space (pitfall #3 in 10-RESEARCH.md).
///
/// The returned `Cluster.id` uses the `"{parent_id}-misc"` sentinel that the
/// labeling path (Plan 05) detects to skip LLM labeling and use the literal
/// name "Misc" directly (D-04).
///
/// The `centroid` is intentionally empty: Misc has no meaningful semantic center
/// and the SpaceManager will not compute a hyperbolic index entry for it.
pub fn build_misc_space(parent_id: &str, misc_ids: Vec<String>) -> Option<Cluster> {
    if misc_ids.is_empty() {
        return None;
    }
    Some(Cluster {
        id: format!("{}-misc", parent_id),
        doc_ids: misc_ids,
        centroid: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create n synthetic 2-D vectors clustered around a given center.
    // Adds small per-index noise to avoid identical vectors.
    fn make_vectors(prefix: &str, count: usize, center: [f32; 2]) -> Vec<(String, Vec<f32>)> {
        (0..count)
            .map(|i| {
                let noise = (i as f32) * 0.001;
                (
                    format!("{}-{}", prefix, i),
                    vec![center[0] + noise, center[1] - noise],
                )
            })
            .collect()
    }

    // Helper: generate a list of doc IDs.
    fn make_ids(prefix: &str, count: usize) -> Vec<String> {
        (0..count).map(|i| format!("{}-{}", prefix, i)).collect()
    }

    /// Test 1 (HSPC-01): detect() returns empty vecs for parent with ≤ 50 documents.
    ///
    /// Sub-space detection must be a no-op at low doc counts.
    #[test]
    fn test_detect_below_threshold() {
        let ids: Vec<String> = make_ids("doc", 30);
        let vectors: Vec<(String, Vec<f32>)> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), vec![(i as f32) * 0.01, 1.0 - (i as f32) * 0.01]))
            .collect();

        let (sub_clusters, misc_ids) = detect(&ids, vectors);

        assert!(
            sub_clusters.is_empty(),
            "expected no sub-clusters for 30 docs (below threshold {})",
            SUB_SPACE_THRESHOLD
        );
        assert!(
            misc_ids.is_empty(),
            "expected no misc ids for 30 docs (below threshold)"
        );
    }

    /// Test 2: detect() produces sub-clusters when parent has > 50 documents.
    ///
    /// 60 vectors split evenly across 3 well-separated 2-D centers.
    /// k = sqrt(60/2).max(2) = sqrt(30) ≈ 5.
    /// Result should contain in [2, 5] clusters, each with >= MIN_SUB_CLUSTER_SIZE docs.
    #[test]
    fn test_detect_above_threshold_produces_clusters() {
        // Three tight groups far apart in 2-D space
        let mut vectors: Vec<(String, Vec<f32>)> = Vec::new();
        vectors.extend(make_vectors("a", 20, [1.0, 0.0]));
        vectors.extend(make_vectors("b", 20, [0.0, 1.0]));
        vectors.extend(make_vectors("c", 20, [-1.0, 0.0]));

        let ids: Vec<String> = vectors.iter().map(|(id, _)| id.clone()).collect();

        let (sub_clusters, misc_ids) = detect(&ids, vectors);

        assert!(
            !sub_clusters.is_empty(),
            "expected sub-clusters for 60 docs (above threshold {})",
            SUB_SPACE_THRESHOLD
        );
        assert!(
            sub_clusters.len() >= 2 && sub_clusters.len() <= 5,
            "expected between 2 and 5 sub-clusters (k=sqrt(30)≈5), got {}",
            sub_clusters.len()
        );
        for cluster in &sub_clusters {
            assert!(
                cluster.doc_ids.len() >= MIN_SUB_CLUSTER_SIZE,
                "sub-cluster {} has only {} docs, expected >= {}",
                cluster.id,
                cluster.doc_ids.len(),
                MIN_SUB_CLUSTER_SIZE
            );
        }
        // Every doc must appear somewhere (sub_clusters + misc_ids)
        let total_accounted = sub_clusters.iter().map(|c| c.doc_ids.len()).sum::<usize>()
            + misc_ids.len();
        assert_eq!(
            total_accounted, 60,
            "all 60 docs must be accounted for (no silent drops)"
        );
    }

    /// Test 3 (D-04 / HSPC-03): small sub-clusters roll up to misc_ids.
    ///
    /// Create 60 docs: 2 large tight clusters (25 each) + 10 outliers spread evenly
    /// across 10 different far-apart positions (each outlier → its own cluster of 1).
    /// After filtering by MIN_SUB_CLUSTER_SIZE (3), outlier clusters land in misc_ids.
    #[test]
    fn test_misc_rollup_when_small_clusters() {
        // Two large clusters: "pos" and "neg" poles
        let mut vectors: Vec<(String, Vec<f32>)> = Vec::new();
        vectors.extend(make_vectors("pos", 25, [0.99, 0.01]));
        vectors.extend(make_vectors("neg", 25, [-0.99, 0.01]));

        // 10 widely-separated outliers (each unique direction in 2-D)
        for i in 0..10 {
            let angle = (i as f32) * std::f32::consts::PI / 5.0;
            vectors.push((
                format!("outlier-{}", i),
                vec![angle.cos() * 0.5, angle.sin() * 0.5],
            ));
        }

        let ids: Vec<String> = vectors.iter().map(|(id, _)| id.clone()).collect();
        assert_eq!(ids.len(), 60);

        let (sub_clusters, misc_ids) = detect(&ids, vectors);

        // Total coverage check (HSPC-03: no doc silently dropped)
        let total = sub_clusters.iter().map(|c| c.doc_ids.len()).sum::<usize>() + misc_ids.len();
        assert_eq!(total, 60, "all 60 docs must be accounted for");

        // The two large clusters must survive the MIN_SUB_CLUSTER_SIZE filter
        assert!(
            sub_clusters.len() >= 1,
            "at least 1 sub-cluster should survive the size filter"
        );

        // At least 3 outliers should end up in misc (they form clusters of size 1)
        assert!(
            misc_ids.len() >= 3,
            "expected at least 3 misc ids from outlier docs, got {}",
            misc_ids.len()
        );
    }

    /// Test 4 (pitfall #3): build_misc_space returns None for an empty misc_ids vec.
    ///
    /// A zero-document Misc sub-space must never be created.
    #[test]
    fn test_no_misc_on_empty() {
        let result = build_misc_space("parent-x", vec![]);
        assert!(
            result.is_none(),
            "build_misc_space must return None for empty misc_ids (pitfall #3)"
        );
    }

    /// Test 5 (D-04): build_misc_space returns a Cluster with the expected id format.
    ///
    /// id must be `"{parent_id}-misc"` — the sentinel the labeling path checks.
    /// centroid must be empty — Misc has no meaningful semantic center.
    #[test]
    fn test_misc_id_format() {
        let misc_ids = vec!["doc-1".to_string(), "doc-2".to_string()];
        let result = build_misc_space("parent-abc", misc_ids.clone());

        assert!(result.is_some(), "expected Some(Cluster) for non-empty misc_ids");
        let cluster = result.unwrap();

        assert_eq!(
            cluster.id, "parent-abc-misc",
            "id must be '{{parent_id}}-misc', got '{}'",
            cluster.id
        );
        assert_eq!(
            cluster.doc_ids, misc_ids,
            "doc_ids must match the provided misc_ids"
        );
        assert!(
            cluster.centroid.is_empty(),
            "centroid must be empty vec for Misc sub-space"
        );
    }
}

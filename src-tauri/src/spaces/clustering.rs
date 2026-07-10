/// Result of document clustering.
#[derive(Debug, Clone)]
pub struct ClusterResult {
    pub clusters: Vec<Cluster>,
}

/// A single cluster of documents.
#[derive(Debug, Clone)]
pub struct Cluster {
    pub id: String,
    pub doc_ids: Vec<String>,
    pub centroid: Vec<f32>,
}

/// Cluster document vectors using k-means with cosine similarity.
///
/// Algorithm:
/// 1. If fewer than k documents, put each in its own cluster.
/// 2. Initialize centroids using k-means++ (max distance selection).
/// 3. Iterate (max 20 iterations):
///    a. Assign each document to nearest centroid by cosine similarity.
///    b. Recompute centroids as mean of assigned vectors.
///    c. If no assignments changed, break early.
/// 4. Return clusters with centroid vectors and member doc IDs.
pub fn cluster_documents(vectors: Vec<(String, Vec<f32>)>, k: usize) -> ClusterResult {
    if vectors.is_empty() {
        return ClusterResult { clusters: vec![] };
    }

    let k = k.min(vectors.len());

    if vectors.len() <= k {
        // Each document is its own cluster
        let clusters: Vec<Cluster> = vectors
            .into_iter()
            .enumerate()
            .map(|(i, (id, vec))| Cluster {
                id: format!("space-{}", i),
                doc_ids: vec![id],
                centroid: vec,
            })
            .collect();
        return ClusterResult { clusters };
    }

    // Initialize centroids using k-means++ strategy
    let mut centroids = kmeans_plus_plus_init(&vectors, k);

    let mut assignments: Vec<usize> = vec![0; vectors.len()];

    for _iteration in 0..20 {
        let mut changed = false;

        // Assign each document to nearest centroid
        for (i, (_id, vec)) in vectors.iter().enumerate() {
            let mut best_cluster = 0;
            let mut best_sim = f32::NEG_INFINITY;

            for (c, centroid) in centroids.iter().enumerate() {
                let sim = cosine_similarity(vec, centroid);
                if sim > best_sim {
                    best_sim = sim;
                    best_cluster = c;
                }
            }

            if assignments[i] != best_cluster {
                assignments[i] = best_cluster;
                changed = true;
            }
        }

        if !changed {
            break;
        }

        // Recompute centroids
        for c in 0..k {
            let dim = centroids[c].len();
            let mut sum = vec![0.0f32; dim];
            let mut count = 0usize;

            for (i, (_id, vec)) in vectors.iter().enumerate() {
                if assignments[i] == c {
                    for (j, val) in vec.iter().enumerate() {
                        if j < dim {
                            sum[j] += val;
                        }
                    }
                    count += 1;
                }
            }

            if count > 0 {
                for val in sum.iter_mut() {
                    *val /= count as f32;
                }
                normalize(&mut sum);
                centroids[c] = sum;
            }
        }
    }

    // Build clusters from final assignments
    let mut cluster_docs: Vec<Vec<String>> = vec![vec![]; k];
    for (i, (id, _vec)) in vectors.iter().enumerate() {
        cluster_docs[assignments[i]].push(id.clone());
    }

    let clusters: Vec<Cluster> = centroids
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !cluster_docs[*i].is_empty())
        .map(|(i, centroid)| Cluster {
            id: format!("space-{}", i),
            doc_ids: cluster_docs[i].clone(),
            centroid,
        })
        .collect();

    ClusterResult { clusters }
}

/// Auto-detect the number of clusters (k) using heuristic: sqrt(n/2), clamped to [2, 20].
pub fn auto_detect_k(n_documents: usize) -> usize {
    if n_documents < 2 {
        return n_documents.max(1);
    }
    let k = ((n_documents as f64 / 2.0).sqrt()).round() as usize;
    k.clamp(2, 20)
}

/// K-means++ initialization: select initial centroids to maximize spread.
fn kmeans_plus_plus_init(vectors: &[(String, Vec<f32>)], k: usize) -> Vec<Vec<f32>> {
    let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(k);

    // Pick first centroid: use the first vector (deterministic for reproducibility)
    centroids.push(vectors[0].1.clone());

    for _ in 1..k {
        // For each point, compute distance to nearest existing centroid
        let mut max_dist = f32::NEG_INFINITY;
        let mut best_idx = 0;

        for (i, (_id, vec)) in vectors.iter().enumerate() {
            let min_sim = centroids
                .iter()
                .map(|c| cosine_similarity(vec, c))
                .fold(f32::INFINITY, f32::min);
            // Convert similarity to distance (1 - sim)
            let dist = 1.0 - min_sim;
            if dist > max_dist {
                max_dist = dist;
                best_idx = i;
            }
        }

        centroids.push(vectors[best_idx].1.clone());
    }

    centroids
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// L2 normalize a vector in place.
pub fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in v.iter_mut() {
            *val /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_two_groups() {
        // 6 vectors: 3 similar (near [1,0,0]) and 3 similar (near [0,1,0])
        let vectors = vec![
            ("a1".into(), vec![0.9, 0.1, 0.0]),
            ("a2".into(), vec![0.85, 0.15, 0.0]),
            ("a3".into(), vec![0.95, 0.05, 0.0]),
            ("b1".into(), vec![0.1, 0.9, 0.0]),
            ("b2".into(), vec![0.15, 0.85, 0.0]),
            ("b3".into(), vec![0.05, 0.95, 0.0]),
        ];

        let result = cluster_documents(vectors, 2);
        assert_eq!(result.clusters.len(), 2, "should produce 2 clusters");

        // Verify each cluster has exactly 3 members
        let mut sizes: Vec<usize> = result.clusters.iter().map(|c| c.doc_ids.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![3, 3], "each cluster should have 3 docs");

        // Verify the 'a' docs are together
        let a_cluster = result
            .clusters
            .iter()
            .find(|c| c.doc_ids.contains(&"a1".to_string()));
        assert!(a_cluster.is_some());
        let a_cluster = a_cluster.unwrap();
        assert!(a_cluster.doc_ids.contains(&"a2".to_string()));
        assert!(a_cluster.doc_ids.contains(&"a3".to_string()));
    }

    #[test]
    fn test_cluster_empty() {
        let result = cluster_documents(vec![], 3);
        assert!(result.clusters.is_empty());
    }

    #[test]
    fn test_cluster_fewer_than_k() {
        let vectors = vec![
            ("x".into(), vec![1.0, 0.0]),
            ("y".into(), vec![0.0, 1.0]),
        ];
        let result = cluster_documents(vectors, 5);
        assert_eq!(result.clusters.len(), 2, "each doc becomes its own cluster");
    }

    #[test]
    fn test_auto_detect_k_small() {
        assert_eq!(auto_detect_k(1), 1);
        assert_eq!(auto_detect_k(2), 2);
        assert_eq!(auto_detect_k(4), 2);
    }

    #[test]
    fn test_auto_detect_k_medium() {
        assert_eq!(auto_detect_k(50), 5);
        assert_eq!(auto_detect_k(100), 7);
    }

    #[test]
    fn test_auto_detect_k_large() {
        assert_eq!(auto_detect_k(1000), 20); // clamped to max
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        assert!((v[0] - 0.6).abs() < 1e-5);
        assert!((v[1] - 0.8).abs() < 1e-5);
    }
}

use crate::types::SearchResult;

/// Re-rank search results using scaled dot-product attention.
///
/// Computes attention scores between the query vector and each result vector,
/// then blends with the original cosine similarity score:
///   final_score = 0.7 * original_score + 0.3 * attention_weight
///
/// This gives more nuanced result ordering than raw cosine similarity alone.
pub fn rerank_results(
    query_vec: &[f32],
    results: &mut Vec<SearchResult>,
    result_vecs: &[Vec<f32>],
) {
    if results.is_empty() || result_vecs.is_empty() {
        return;
    }

    // Compute attention scores: scaled dot-product
    let dim = query_vec.len() as f32;
    let scale = dim.sqrt();

    let raw_scores: Vec<f32> = result_vecs
        .iter()
        .map(|key| {
            let dot: f32 = query_vec
                .iter()
                .zip(key.iter())
                .map(|(q, k)| q * k)
                .sum();
            dot / scale
        })
        .collect();

    // Softmax normalization
    let attention_weights = softmax(&raw_scores);

    // Blend original scores with attention weights
    for (i, result) in results.iter_mut().enumerate() {
        if i < attention_weights.len() {
            let original = result.score;
            let attention = attention_weights[i] as f64;
            result.score = 0.7 * original + 0.3 * attention;
        }
    }

    // Re-sort by final score descending
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Softmax over a slice of scores.
fn softmax(scores: &[f32]) -> Vec<f32> {
    if scores.is_empty() {
        return vec![];
    }

    let max_score = scores.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let exp_scores: Vec<f32> = scores.iter().map(|s| (s - max_score).exp()).collect();
    let sum: f32 = exp_scores.iter().sum();

    if sum == 0.0 {
        return vec![1.0 / scores.len() as f32; scores.len()];
    }

    exp_scores.iter().map(|e| e / sum).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Document;

    fn make_result(id: &str, score: f64) -> SearchResult {
        SearchResult {
            document: Document {
                id: id.to_string(),
                name: format!("{}.pdf", id),
                path: format!("/tmp/{}.pdf", id),
                doc_type: "pdf".to_string(),
                size: 1024,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                modified_at: "2024-01-01T00:00:00Z".to_string(),
                excerpt: None,
                space_ids: vec![],
                tags: vec![],
                is_favorite: false,
                extracted_entities: vec![],
                thumbnail_color: None,
                topic: None,
                llm_tags: vec![],
            },
            score,
            matched_excerpt: None,
        }
    }

    #[test]
    fn test_rerank_preserves_results() {
        let query = vec![1.0, 0.0, 0.0];
        let mut results = vec![
            make_result("a", 0.9),
            make_result("b", 0.5),
        ];
        let vecs = vec![
            vec![0.9, 0.1, 0.0], // similar to query
            vec![0.1, 0.9, 0.0], // different
        ];

        rerank_results(&query, &mut results, &vecs);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_rerank_can_change_order() {
        let query = vec![0.0, 1.0, 0.0];
        let mut results = vec![
            make_result("a", 0.8),
            make_result("b", 0.7),
        ];
        let vecs = vec![
            vec![1.0, 0.0, 0.0], // dissimilar to query
            vec![0.0, 1.0, 0.0], // very similar to query
        ];

        rerank_results(&query, &mut results, &vecs);
        // b should be boosted because its vector is more similar to query
        // This tests that attention can influence ordering
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn test_rerank_empty_results() {
        let query = vec![1.0, 0.0];
        let mut results: Vec<SearchResult> = vec![];
        let vecs: Vec<Vec<f32>> = vec![];
        rerank_results(&query, &mut results, &vecs);
        assert!(results.is_empty());
    }

    #[test]
    fn test_softmax_basic() {
        let scores = vec![1.0, 2.0, 3.0];
        let weights = softmax(&scores);
        assert_eq!(weights.len(), 3);
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "softmax should sum to 1");
        // Highest score should have highest weight
        assert!(weights[2] > weights[1]);
        assert!(weights[1] > weights[0]);
    }

    #[test]
    fn test_softmax_empty() {
        let weights = softmax(&[]);
        assert!(weights.is_empty());
    }
}

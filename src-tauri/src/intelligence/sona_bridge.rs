use ruvector_sona::engine::SonaEngine;

use crate::error::AppError;

/// Wrapper around SONA engine for search-specific learning signals.
///
/// Records search trajectories and click-through data to improve
/// future search result ranking.
pub struct SearchLearner {
    engine: SonaEngine,
    embedding_dim: usize,
}

impl SearchLearner {
    /// Create a new SearchLearner with the given embedding dimension.
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            engine: SonaEngine::new(embedding_dim),
            embedding_dim,
        }
    }

    /// Record a search query and its result scores as a SONA trajectory.
    ///
    /// The query embedding becomes the trajectory context; each result
    /// score is a step with its score as reward.
    pub fn record_search(
        &self,
        query_embedding: &[f32],
        result_scores: &[f32],
    ) -> Result<(), AppError> {
        let mut builder = self.engine.begin_trajectory(query_embedding.to_vec());

        for score in result_scores {
            // Use query embedding as activations, empty weights, score as reward
            builder.add_step(
                query_embedding.to_vec(),
                vec![*score],
                *score,
            );
        }

        // Overall quality = mean of result scores
        let quality = if result_scores.is_empty() {
            0.0
        } else {
            result_scores.iter().sum::<f32>() / result_scores.len() as f32
        };

        self.engine.end_trajectory(builder, quality);
        Ok(())
    }

    /// Record a click-through event as a learning signal.
    ///
    /// Position-weighted: clicking a lower-ranked result generates a
    /// stronger learning signal (the system should learn from corrections).
    pub fn record_click(
        &self,
        query_embedding: &[f32],
        clicked_doc_embedding: &[f32],
        position: usize,
    ) {
        let mut builder = self.engine.begin_trajectory(query_embedding.to_vec());

        let reward = 1.0 / (1 + position) as f32;
        builder.add_step(
            clicked_doc_embedding.to_vec(),
            vec![reward],
            reward,
        );

        self.engine.end_trajectory(builder, reward);
    }

    /// Apply micro-LoRA boost to a query embedding.
    ///
    /// If SONA has learned patterns from previous searches, it applies
    /// a learned transformation to the query embedding. If nothing has
    /// been learned yet, returns the original embedding unchanged.
    pub fn apply_boost(&self, query_embedding: &[f32]) -> Vec<f32> {
        let mut output = query_embedding.to_vec();
        self.engine.apply_micro_lora(query_embedding, &mut output);
        output
    }

    /// Get the embedding dimension.
    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_learner_creation() {
        let learner = SearchLearner::new(384);
        assert_eq!(learner.embedding_dim(), 384);
    }

    #[test]
    fn test_record_search_does_not_panic() {
        let learner = SearchLearner::new(4);
        let query = vec![0.1, 0.2, 0.3, 0.4];
        let scores = vec![0.9, 0.7, 0.5];
        let result = learner.record_search(&query, &scores);
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_click_does_not_panic() {
        let learner = SearchLearner::new(4);
        let query = vec![0.1, 0.2, 0.3, 0.4];
        let doc = vec![0.5, 0.6, 0.7, 0.8];
        learner.record_click(&query, &doc, 2);
    }

    #[test]
    fn test_apply_boost_returns_same_dim() {
        let learner = SearchLearner::new(4);
        let query = vec![0.1, 0.2, 0.3, 0.4];
        let boosted = learner.apply_boost(&query);
        assert_eq!(boosted.len(), 4);
    }

    #[test]
    fn test_record_search_empty_scores() {
        let learner = SearchLearner::new(4);
        let query = vec![0.1, 0.2, 0.3, 0.4];
        let result = learner.record_search(&query, &[]);
        assert!(result.is_ok());
    }
}

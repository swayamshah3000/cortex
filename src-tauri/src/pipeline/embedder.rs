use crate::error::AppError;

/// Local embedding service wrapping fastembed's TextEmbedding model.
///
/// Uses all-MiniLM-L6-v2 (384-dim) for local embeddings.
///
/// API embedding path (DPIP-07): OpenAI text-embedding-3-small (1536-dim) will use
/// ruvector-core's ApiEmbedding::openai(), activated via settings toggle in Phase 4.
/// The `documents_1536` collection already exists from Phase 1 initialization.
pub struct EmbeddingService {
    model: std::sync::Mutex<fastembed::TextEmbedding>,
    pub dimensions: usize,
}

impl EmbeddingService {
    /// Initialize the local fastembed model (all-MiniLM-L6-v2, 384-dim).
    /// NOTE: First call downloads ~90MB model to ~/.cache/fastembed/. This is expected.
    pub fn new_local() -> Result<Self, AppError> {
        let model = fastembed::TextEmbedding::try_new(
            fastembed::InitOptions::new(fastembed::EmbeddingModel::AllMiniLML6V2),
        )
        .map_err(|e| AppError::Embedding(e.to_string()))?;

        Ok(Self {
            model: std::sync::Mutex::new(model),
            dimensions: 384,
        })
    }

    /// Embed a text string, returning a 384-dimensional vector.
    /// Long text is truncated to 2000 chars before embedding (MiniLM token limit).
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, AppError> {
        let chunk = truncate_to_chars(text, 2000);
        let mut model = self
            .model
            .lock()
            .map_err(|e| AppError::Embedding(e.to_string()))?;
        let mut results = model
            .embed(vec![chunk.as_str()], None)
            .map_err(|e| AppError::Embedding(e.to_string()))?;
        if results.is_empty() {
            return Err(AppError::Embedding("Empty embedding result".to_string()));
        }
        Ok(results.remove(0))
    }
}

/// Truncate text to at most `max_chars` Unicode characters.
fn truncate_to_chars(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        text.chars().take(max_chars).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_to_chars_short() {
        let s = "hello world";
        assert_eq!(truncate_to_chars(s, 2000), s.to_string());
    }

    #[test]
    fn test_truncate_to_chars_long() {
        let s: String = "a".repeat(3000);
        let result = truncate_to_chars(&s, 2000);
        assert_eq!(result.len(), 2000);
    }

    #[test]
    fn test_truncate_to_chars_exact() {
        let s: String = "x".repeat(2000);
        assert_eq!(truncate_to_chars(&s, 2000).len(), 2000);
    }

    /// Integration test: initializes fastembed and embeds text.
    /// Downloads ~90MB model on first run. Marked #[ignore] for CI.
    #[test]
    #[ignore]
    fn test_new_local_succeeds() {
        let svc = EmbeddingService::new_local().expect("EmbeddingService should initialize");
        assert_eq!(svc.dimensions, 384);
    }

    #[test]
    #[ignore]
    fn test_embed_text_returns_384_dims() {
        let svc = EmbeddingService::new_local().unwrap();
        let vec = svc.embed_text("hello world").unwrap();
        assert_eq!(vec.len(), 384);
    }

    #[test]
    #[ignore]
    fn test_embed_empty_string_returns_384_dims() {
        let svc = EmbeddingService::new_local().unwrap();
        let vec = svc.embed_text("").unwrap();
        assert_eq!(vec.len(), 384);
    }

    #[test]
    #[ignore]
    fn test_embed_long_text_truncates_and_embeds() {
        let svc = EmbeddingService::new_local().unwrap();
        let long_text: String = "word ".repeat(1000); // ~5000 chars
        let vec = svc.embed_text(&long_text).unwrap();
        assert_eq!(vec.len(), 384);
    }
}

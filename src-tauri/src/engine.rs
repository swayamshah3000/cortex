use ruvector_collections::{CollectionManager, CollectionConfig};
use ruvector_core::types::{DistanceMetric, HnswConfig};
use ruvector_filter::{PayloadIndexManager, IndexType};
use std::path::PathBuf;

/// CortexEngine holds all backend state: RuVector collections and metadata filter indices.
///
/// - `collections`: multi-collection vector storage (384-dim for local ONNX, 1536-dim for OpenAI API)
/// - `filter_index`: payload indices for pre-search filtering by doc_type, created_at, space_ids, tags
pub struct CortexEngine {
    pub collections: CollectionManager,
    pub filter_index: PayloadIndexManager,
}

impl CortexEngine {
    /// Initialize CortexEngine with RuVector backed by the given data directory.
    ///
    /// Creates two vector collections:
    /// - `documents_384`: 384-dimensional for local ONNX embeddings (all-MiniLM-L6-v2)
    /// - `documents_1536`: 1536-dimensional for OpenAI API embeddings (opt-in, Phase 2)
    ///
    /// Creates four metadata filter indices:
    /// - `doc_type` (Keyword): filter by document type (pdf, docx, txt, etc.)
    /// - `created_at` (Integer): filter by Unix timestamp
    /// - `space_ids` (Keyword): filter by Smart Space membership
    /// - `tags` (Keyword): filter by document tags
    pub fn new_with_path(data_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        // CollectionManager::new() creates the directory if it doesn't exist
        let collections = CollectionManager::new(data_dir)?;

        // 384-dim: local ONNX embeddings (all-MiniLM-L6-v2)
        collections.create_collection("documents_384", CollectionConfig {
            dimensions: 384,
            distance_metric: DistanceMetric::Cosine,
            hnsw_config: Some(HnswConfig::default()),
            quantization: None,
            on_disk_payload: true,
        }).or_else(|e| {
            // Ignore AlreadyExists on app restart; propagate other errors
            if format!("{}", e).contains("already exists") {
                Ok(())
            } else {
                Err(e)
            }
        })?;

        // 1536-dim: OpenAI API embeddings (opt-in, Phase 2)
        collections.create_collection("documents_1536", CollectionConfig {
            dimensions: 1536,
            distance_metric: DistanceMetric::Cosine,
            hnsw_config: Some(HnswConfig::default()),
            quantization: None,
            on_disk_payload: true,
        }).or_else(|e| {
            if format!("{}", e).contains("already exists") {
                Ok(())
            } else {
                Err(e)
            }
        })?;

        // Metadata filter indices for pre-search filtering
        let mut filter_index = PayloadIndexManager::new();
        filter_index.create_index("doc_type", IndexType::Keyword)?;
        filter_index.create_index("created_at", IndexType::Integer)?;
        filter_index.create_index("space_ids", IndexType::Keyword)?;
        filter_index.create_index("tags", IndexType::Keyword)?;

        Ok(Self { collections, filter_index })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_initializes_with_temp_dir() {
        let tmp = std::env::temp_dir().join("cortex-test-engine");
        let engine = CortexEngine::new_with_path(tmp.clone());
        assert!(engine.is_ok(), "Engine failed to initialize: {:?}", engine.err());
        // Cleanup
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn test_engine_initializes_twice_same_dir() {
        // Second initialization should succeed (AlreadyExists is ignored)
        let tmp = std::env::temp_dir().join("cortex-test-engine-restart");
        let _ = std::fs::remove_dir_all(&tmp);

        let engine1 = CortexEngine::new_with_path(tmp.clone());
        assert!(engine1.is_ok(), "First init failed: {:?}", engine1.err());
        drop(engine1);

        let engine2 = CortexEngine::new_with_path(tmp.clone());
        assert!(engine2.is_ok(), "Second init (restart sim) failed: {:?}", engine2.err());

        // Cleanup
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn test_engine_has_four_filter_indices() {
        // Use a thread-unique directory to avoid redb lock conflicts in parallel tests
        let thread_id = format!("{:?}", std::thread::current().id())
            .replace(['(', ')', ' '], "");
        let tmp = std::env::temp_dir().join(format!("cortex-test-filters-{}", thread_id));
        let _ = std::fs::remove_dir_all(&tmp);

        let engine = CortexEngine::new_with_path(tmp.clone()).expect("Engine init failed");
        assert!(engine.filter_index.has_index("doc_type"), "doc_type index missing");
        assert!(engine.filter_index.has_index("created_at"), "created_at index missing");
        assert!(engine.filter_index.has_index("space_ids"), "space_ids index missing");
        assert!(engine.filter_index.has_index("tags"), "tags index missing");
        assert_eq!(engine.filter_index.index_count(), 4, "Expected 4 filter indices");

        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn test_engine_collections_exist() {
        // Use a thread-unique directory to avoid redb lock conflicts in parallel tests
        let thread_id = format!("{:?}", std::thread::current().id())
            .replace(['(', ')', ' '], "");
        let tmp = std::env::temp_dir().join(format!("cortex-test-collections-{}", thread_id));
        let _ = std::fs::remove_dir_all(&tmp);

        let engine = CortexEngine::new_with_path(tmp.clone()).expect("Engine init failed");
        assert!(engine.collections.collection_exists("documents_384"), "documents_384 missing");
        assert!(engine.collections.collection_exists("documents_1536"), "documents_1536 missing");

        let _ = std::fs::remove_dir_all(tmp);
    }
}

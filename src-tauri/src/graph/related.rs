use crate::engine::CortexEngine;
use crate::error::AppError;
use crate::search::query::build_document_from_metadata;
use crate::types::Document;

use super::edges::DocumentGraph;

/// Get documents related to a given document, ordered by edge weight.
///
/// Traverses the document graph, looks up metadata from RuVector,
/// and builds Document structs for the frontend.
pub fn get_related_impl(
    doc_id: &str,
    limit: usize,
    graph: &DocumentGraph,
    engine: &CortexEngine,
) -> Result<Vec<Document>, AppError> {
    let neighbors = graph.get_neighbors(doc_id, limit);

    if neighbors.is_empty() {
        return Ok(vec![]);
    }

    let collection_arc = engine
        .collections
        .get_collection("documents_384")
        .ok_or_else(|| {
            AppError::VectorStorage("documents_384 collection not found".to_string())
        })?;
    let collection = collection_arc.read();

    let mut documents: Vec<Document> = Vec::new();

    for edge in neighbors {
        let entry = collection
            .db
            .get(&edge.target)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;

        if let Some(entry) = entry {
            if let Some(ref metadata) = entry.metadata {
                documents.push(build_document_from_metadata(&edge.target, metadata));
            }
        }
    }

    Ok(documents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_related_empty_graph() {
        let graph = DocumentGraph::new();
        let tmp = std::env::temp_dir().join("cortex-test-related-empty");
        let _ = std::fs::remove_dir_all(&tmp);
        let engine = CortexEngine::new_with_path(tmp.clone()).unwrap();

        let result = get_related_impl("nonexistent", 5, &graph, &engine).unwrap();
        assert!(result.is_empty());
        let _ = std::fs::remove_dir_all(tmp);
    }
}

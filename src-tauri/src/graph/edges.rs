use std::collections::HashMap;

use ruvector_core::types::SearchQuery;

use crate::engine::CortexEngine;
use crate::error::AppError;
use crate::spaces::clustering::cosine_similarity;
use crate::spaces::manager::SpaceManager;
use crate::types::{SpaceGraph, SpaceGraphEdge, SpaceGraphNode};

/// An edge connecting two documents with a weight and edge type labels.
#[derive(Debug, Clone)]
pub struct DocumentEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
    pub edge_types: Vec<String>,
}

/// In-memory adjacency list graph of document relationships.
///
/// Edges are created from:
/// - Content similarity (cosine > 0.7)
/// - Shared space membership
/// - Shared tags
/// - Shared entities (same person/organization)
pub struct DocumentGraph {
    /// doc_id -> list of edges from that doc
    edges: HashMap<String, Vec<DocumentEdge>>,
}

impl DocumentGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }

    /// Build edges between documents using HNSW top-N neighbors.
    ///
    /// For each document, finds top-10 nearest neighbors via HNSW search
    /// (O(n log n) instead of O(n^2)), then creates edges for:
    /// - Content similarity > 0.7
    /// - Shared space membership
    /// - Shared tags
    /// - Shared entities
    pub fn build_edges(
        &mut self,
        engine: &CortexEngine,
        space_manager: &SpaceManager,
    ) -> Result<(), AppError> {
        self.edges.clear();

        let collection_arc = engine
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                AppError::VectorStorage("documents_384 collection not found".to_string())
            })?;

        let collection = collection_arc.read();

        let all_ids = collection
            .db
            .keys()
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;

        if all_ids.len() < 2 {
            return Ok(());
        }

        // Collect all vectors and metadata
        let mut id_vec: HashMap<String, Vec<f32>> = HashMap::new();
        let mut id_meta: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();

        for id in &all_ids {
            let entry = collection
                .db
                .get(id)
                .map_err(|e| AppError::VectorStorage(e.to_string()))?;
            if let Some(entry) = entry {
                id_vec.insert(id.clone(), entry.vector);
                if let Some(metadata) = entry.metadata {
                    id_meta.insert(id.clone(), metadata);
                }
            }
        }

        // For each document, find nearest neighbors via HNSW
        for id in &all_ids {
            let vec = match id_vec.get(id) {
                Some(v) => v.clone(),
                None => continue,
            };

            let search_result = collection
                .db
                .search(SearchQuery {
                    vector: vec,
                    k: 11, // top-10 + self
                    filter: None,
                    ef_search: None,
                })
                .map_err(|e| AppError::VectorStorage(e.to_string()))?;

            let my_meta = id_meta.get(id);
            let my_spaces = space_manager.get_doc_spaces(id);
            let my_tags = extract_tags(my_meta);
            let my_entities = extract_entity_values(my_meta);

            for result in &search_result {
                if result.id == *id {
                    continue; // Skip self
                }

                let similarity = 1.0 - result.score as f64; // cosine distance -> similarity
                let mut edge_types: Vec<String> = Vec::new();
                let mut weight = 0.0f64;

                // Content similarity edge (> 0.7)
                if similarity > 0.7 {
                    edge_types.push("content_similarity".to_string());
                    weight = weight.max(similarity);
                }

                // Shared space edge
                let other_spaces = space_manager.get_doc_spaces(&result.id);
                let shared_spaces = my_spaces
                    .iter()
                    .any(|s| other_spaces.contains(s));
                if shared_spaces {
                    edge_types.push("shared_space".to_string());
                    weight = weight.max(0.5);
                }

                // Shared tags edge
                let other_meta = id_meta.get(&result.id);
                let other_tags = extract_tags(other_meta);
                let shared_tag_count = my_tags
                    .iter()
                    .filter(|t| other_tags.contains(t))
                    .count();
                if shared_tag_count > 0 {
                    let max_tags = my_tags.len().max(other_tags.len()).max(1);
                    let tag_weight = 0.3 * (shared_tag_count as f64 / max_tags as f64);
                    edge_types.push("shared_tags".to_string());
                    weight = weight.max(tag_weight);
                }

                // Shared entities edge
                let other_entities = extract_entity_values(other_meta);
                let shared_entities = my_entities
                    .iter()
                    .any(|e| other_entities.contains(e));
                if shared_entities {
                    edge_types.push("shared_entity".to_string());
                    weight = weight.max(0.4);
                }

                // Only create edge if there's any relationship
                if !edge_types.is_empty() {
                    let edge = DocumentEdge {
                        source: id.clone(),
                        target: result.id.clone(),
                        weight,
                        edge_types,
                    };
                    self.edges
                        .entry(id.clone())
                        .or_default()
                        .push(edge);
                }
            }
        }

        Ok(())
    }

    /// Get neighbors of a document, sorted by edge weight descending.
    pub fn get_neighbors(&self, doc_id: &str, limit: usize) -> Vec<&DocumentEdge> {
        match self.edges.get(doc_id) {
            Some(edges) => {
                let mut sorted: Vec<&DocumentEdge> = edges.iter().collect();
                sorted.sort_by(|a, b| {
                    b.weight
                        .partial_cmp(&a.weight)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                sorted.truncate(limit);
                sorted
            }
            None => vec![],
        }
    }

    /// Build a SpaceGraph for visualization from the document graph.
    ///
    /// Creates one node per space and edges between spaces that share
    /// document connections.
    pub fn build_space_graph(&self, space_manager: &SpaceManager) -> SpaceGraph {
        let spaces = space_manager.get_spaces();

        let nodes: Vec<SpaceGraphNode> = spaces
            .iter()
            .map(|s| SpaceGraphNode {
                id: s.id.clone(),
                name: s.name.clone(),
                color: s.color.clone(),
                document_count: s.document_count,
            })
            .collect();

        // Count connections between spaces
        let mut space_edges: HashMap<(String, String), f64> = HashMap::new();

        for (doc_id, edges) in &self.edges {
            let source_spaces = space_manager.get_doc_spaces(doc_id);
            for edge in edges {
                let target_spaces = space_manager.get_doc_spaces(&edge.target);
                for src_space in &source_spaces {
                    for tgt_space in &target_spaces {
                        if src_space != tgt_space {
                            let key = if src_space < tgt_space {
                                (src_space.clone(), tgt_space.clone())
                            } else {
                                (tgt_space.clone(), src_space.clone())
                            };
                            *space_edges.entry(key).or_insert(0.0) += edge.weight;
                        }
                    }
                }
            }
        }

        let edges: Vec<SpaceGraphEdge> = space_edges
            .into_iter()
            .map(|((source, target), weight)| SpaceGraphEdge {
                source,
                target,
                weight,
            })
            .collect();

        SpaceGraph { nodes, edges }
    }
}

impl Default for DocumentGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract tags from document metadata.
fn extract_tags(meta: Option<&HashMap<String, serde_json::Value>>) -> Vec<String> {
    meta.and_then(|m| m.get("tags"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Extract entity values (person names, organizations) from metadata.
fn extract_entity_values(meta: Option<&HashMap<String, serde_json::Value>>) -> Vec<String> {
    meta.and_then(|m| m.get("extracted_entities"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|e| {
                    let et = e.get("entity_type").and_then(|v| v.as_str());
                    et == Some("person") || et == Some("organization")
                })
                .filter_map(|e| e.get("value").and_then(|v| v.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_graph_new() {
        let graph = DocumentGraph::new();
        assert!(graph.get_neighbors("nonexistent", 10).is_empty());
    }

    #[test]
    fn test_get_neighbors_sorted_by_weight() {
        let mut graph = DocumentGraph::new();
        graph.edges.insert(
            "doc-1".to_string(),
            vec![
                DocumentEdge {
                    source: "doc-1".to_string(),
                    target: "doc-2".to_string(),
                    weight: 0.5,
                    edge_types: vec!["shared_space".to_string()],
                },
                DocumentEdge {
                    source: "doc-1".to_string(),
                    target: "doc-3".to_string(),
                    weight: 0.9,
                    edge_types: vec!["content_similarity".to_string()],
                },
                DocumentEdge {
                    source: "doc-1".to_string(),
                    target: "doc-4".to_string(),
                    weight: 0.3,
                    edge_types: vec!["shared_tags".to_string()],
                },
            ],
        );

        let neighbors = graph.get_neighbors("doc-1", 2);
        assert_eq!(neighbors.len(), 2);
        assert_eq!(neighbors[0].target, "doc-3"); // highest weight
        assert_eq!(neighbors[1].target, "doc-2"); // second highest
    }

    #[test]
    fn test_build_space_graph_empty() {
        let graph = DocumentGraph::new();
        let space_manager = SpaceManager::new();
        let space_graph = graph.build_space_graph(&space_manager);
        assert!(space_graph.nodes.is_empty());
        assert!(space_graph.edges.is_empty());
    }

    #[test]
    fn test_extract_tags() {
        let mut meta = HashMap::new();
        meta.insert(
            "tags".to_string(),
            serde_json::json!(["invoice", "tax"]),
        );
        let tags = extract_tags(Some(&meta));
        assert_eq!(tags, vec!["invoice", "tax"]);
    }

    #[test]
    fn test_extract_tags_missing() {
        let meta = HashMap::new();
        let tags = extract_tags(Some(&meta));
        assert!(tags.is_empty());
    }

    #[test]
    fn test_extract_entity_values() {
        let mut meta = HashMap::new();
        meta.insert(
            "extracted_entities".to_string(),
            serde_json::json!([
                {"entity_type": "person", "value": "John Smith", "label": "Person"},
                {"entity_type": "date", "value": "2024-01-01", "label": "Date"},
                {"entity_type": "organization", "value": "Acme Corp", "label": "Org"},
            ]),
        );
        let entities = extract_entity_values(Some(&meta));
        assert_eq!(entities, vec!["John Smith", "Acme Corp"]);
    }
}

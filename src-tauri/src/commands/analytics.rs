use tauri::State;
use crate::engine::CortexEngine;
use crate::error::AppError;
use crate::state::AppState;
use crate::types::*;

#[tauri::command]
pub async fn get_stats(
    state: State<'_, AppState>,
) -> Result<Stats, AppError> {
    let engine = state.engine.clone();
    let space_mgr = state.space_manager.clone();
    let registry = state.registry.clone();

    let engine_guard = engine.lock().await;

    // Count total documents from collection
    let total_documents = match engine_guard.collections.get_collection("documents_384") {
        Some(col) => {
            let col = col.read();
            col.db.keys().map(|k| k.len() as u32).unwrap_or(0)
        }
        None => 0,
    };

    // Count smart spaces (Phase 9: tokio::sync::Mutex)
    let smart_spaces = space_mgr.lock().await.space_count() as u32;

    // Get last scan from registry (still std::sync::Mutex)
    let last_scan = match registry.lock() {
        Ok(reg) => {
            reg.folders
                .values()
                .filter_map(|f| f.last_scan.as_deref())
                .max()
                .unwrap_or("never")
                .to_string()
        }
        Err(_) => "never".to_string(),
    };

    // Estimate index size: total_documents * 384 dimensions * 4 bytes per f32
    let index_size = total_documents as u64 * 384 * 4;

    Ok(Stats {
        total_documents,
        smart_spaces,
        last_scan,
        index_size,
    })
}

#[tauri::command]
pub async fn get_space_graph(
    state: State<'_, AppState>,
) -> Result<SpaceGraph, AppError> {
    let graph = state.doc_graph.clone();
    let space_mgr = state.space_manager.clone();

    // Phase 9: space_manager is tokio::sync::Mutex. Take async guard first,
    // then briefly acquire the still-sync doc_graph lock.
    let space_guard = space_mgr.lock().await;
    let graph_guard = graph
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(graph_guard.build_space_graph(&space_guard))
}

#[tauri::command]
pub async fn get_search_analytics(
    state: State<'_, AppState>,
) -> Result<SearchAnalytics, AppError> {
    let tracker = state.search_tracker.clone();

    let result = tokio::task::spawn_blocking(move || {
        let tracker_guard = tracker
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok::<SearchAnalytics, AppError>(tracker_guard.get_analytics())
    })
    .await??;
    Ok(result)
}

#[tauri::command]
pub async fn get_tags(
    state: State<'_, AppState>,
) -> Result<Vec<Tag>, AppError> {
    let engine = state.engine.clone();

    let results = tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        let collection_arc = match engine_guard.collections.get_collection("documents_384") {
            Some(col) => col,
            None => return Ok::<Vec<Tag>, AppError>(vec![]),
        };

        let collection = collection_arc.read();
        let all_ids = collection
            .db
            .keys()
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;

        // Collect all tags and count documents per tag
        let mut tag_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

        for id in &all_ids {
            if let Ok(Some(entry)) = collection.db.get(id) {
                if let Some(ref metadata) = entry.metadata {
                    if let Some(tags) = metadata.get("tags").and_then(|v| v.as_array()) {
                        for tag_val in tags {
                            if let Some(tag_name) = tag_val.as_str() {
                                *tag_counts.entry(tag_name.to_string()).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
        }

        // Tag color palette
        let colors = [
            "#6D28D9", "#3B82F6", "#10B981", "#F59E0B", "#EF4444",
            "#8B5CF6", "#EC4899", "#14B8A6", "#F97316", "#6366F1",
        ];

        let mut tags: Vec<Tag> = tag_counts
            .into_iter()
            .enumerate()
            .map(|(i, (name, count))| Tag {
                id: format!("tag-{}", name.to_lowercase().replace(' ', "-")),
                name,
                color: colors[i % colors.len()].to_string(),
                document_count: count,
                tag_type: "auto".to_string(),
            })
            .collect();

        // Sort by document count descending
        tags.sort_by(|a, b| b.document_count.cmp(&a.document_count));

        Ok::<Vec<Tag>, AppError>(tags)
    })
    .await??;
    Ok(results)
}

/// Aggregate topic counts from all docs in the documents_384 collection.
///
/// Rules:
/// - Docs with missing, empty, or "other" topic are excluded (D-36: "other" is the
///   LLM-optional default and adds no discovery value to the filter bar).
/// - Results are sorted by count DESC then topic ASC for stable ordering.
///
/// This function is `pub(crate)` so unit tests in the `tests` module below can call it
/// directly without going through the full Tauri `State<'_, AppState>` machinery.
pub(crate) fn aggregate_topics(engine: &CortexEngine) -> Vec<TopicCount> {
    let collection_arc = match engine.collections.get_collection("documents_384") {
        Some(col) => col,
        None => return vec![],
    };

    let collection = collection_arc.read();
    let all_ids = match collection.db.keys() {
        Ok(ids) => ids,
        Err(_) => return vec![],
    };

    let mut topic_counts: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();

    for id in &all_ids {
        if let Ok(Some(entry)) = collection.db.get(id) {
            if let Some(ref metadata) = entry.metadata {
                if let Some(topic_val) = metadata.get("topic").and_then(|v| v.as_str()) {
                    let topic = topic_val.trim();
                    // Exclude missing, empty, or "other" topics (D-36)
                    if !topic.is_empty() && topic != "other" {
                        *topic_counts.entry(topic.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut topics: Vec<TopicCount> = topic_counts
        .into_iter()
        .map(|(topic, count)| TopicCount { topic, count })
        .collect();

    // Sort by count DESC, then topic ASC for deterministic order (T-08-28)
    topics.sort_by(|a, b| b.count.cmp(&a.count).then(a.topic.cmp(&b.topic)));

    topics
}

/// Returns the top topics by document count from the indexed corpus.
///
/// Scans the `documents_384` collection metadata for `topic` fields and aggregates
/// into a `Vec<TopicCount>` sorted by count DESC then topic ASC.
///
/// Complexity: O(N) scan — acceptable for v1.1 index sizes (~10K docs). Future
/// improvement: cached topic-count index invalidated on index changes (T-08-27).
#[tauri::command]
pub async fn get_topics(
    state: State<'_, AppState>,
) -> Result<Vec<TopicCount>, AppError> {
    let engine = state.engine.clone();

    let results = tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        Ok::<Vec<TopicCount>, AppError>(aggregate_topics(&engine_guard))
    })
    .await??;
    Ok(results)
}

#[tauri::command]
pub async fn get_activity_feed(
    state: State<'_, AppState>,
) -> Result<Vec<ActivityItem>, AppError> {
    let activity_log = state.activity_log.clone();

    let results = tokio::task::spawn_blocking(move || {
        let log = activity_log
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok::<Vec<ActivityItem>, AppError>(log.recent(50))
    })
    .await??;
    Ok(results)
}

// ─── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ruvector_core::types::VectorEntry;
    use std::collections::HashMap;

    /// Test: insert 3 VectorEntries with topics ["finance", "finance", "other"].
    /// aggregate_topics must return exactly [{topic:"finance",count:2}] and must
    /// NOT include "other" (D-36: "other" is the no-provider default, excluded).
    #[test]
    fn test_get_topics() {
        let tmp = std::env::temp_dir().join("cortex-test-get-topics");
        let _ = std::fs::remove_dir_all(&tmp);
        let engine = crate::engine::CortexEngine::new_with_path(tmp.clone())
            .expect("CortexEngine for test");

        let collection_arc = engine
            .collections
            .get_collection("documents_384")
            .expect("documents_384 collection");

        // Insert: finance (x2), other (x1)
        for (doc_id, topic) in [
            ("doc-t-1", "finance"),
            ("doc-t-2", "finance"),
            ("doc-t-3", "other"),
        ] {
            let mut meta: HashMap<String, serde_json::Value> = HashMap::new();
            meta.insert("topic".to_string(), serde_json::Value::String(topic.to_string()));
            let entry = VectorEntry {
                id: Some(doc_id.to_string()),
                vector: vec![0.0f32; 384],
                metadata: Some(meta),
            };
            let col = collection_arc.read();
            col.db.insert(entry).expect("insert test entry");
        }

        let results = aggregate_topics(&engine);

        assert_eq!(results.len(), 1, "only 'finance' must be returned; 'other' must be excluded");
        assert_eq!(results[0].topic, "finance", "returned topic must be 'finance'");
        assert_eq!(results[0].count, 2, "finance count must be 2 (two docs)");

        // Verify "other" is absent
        let has_other = results.iter().any(|tc| tc.topic == "other");
        assert!(!has_other, "'other' must NOT appear in get_topics results (D-36)");

        let _ = std::fs::remove_dir_all(tmp);
    }
}

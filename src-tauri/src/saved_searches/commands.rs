//! IPC commands for the Saved Searches feature (Plan 04, Phase 11).
//!
//! Provides four `#[tauri::command] async fn`s:
//! - `get_saved_searches`       — return all persisted saved searches
//! - `save_search`              — persist a new saved search (ss-{uuid} id)
//! - `delete_saved_search`      — remove a saved search by id
//! - `get_saved_search_counts`  — batch doc-count refresh for sidebar (ENEX-04)
//!
//! All mutation commands use `tokio::task::spawn_blocking` + `SavedSearchStore`
//! held under `Arc<tokio::sync::Mutex<>>` to avoid holding a std::sync guard
//! across an `.await`. The `entity_store` (std::sync::Mutex) is only accessed
//! inside `spawn_blocking` where no `.await` is present (T-11-12 mitigation).
//!
//! Security: `save_search` rejects empty/whitespace-only names (T-11-10).
//! Concurrency: single Arc<tokio::Mutex<SavedSearchStore>> serializes all
//! mutations preventing torn writes (T-11-13).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tauri::State;

use crate::engine::CortexEngine;
use crate::error::AppError;
use crate::graph::entity_store::EntityStore;
use crate::saved_searches::store::SavedSearchStore;
use crate::state::AppState;
use crate::types::{EntityClassFilter, SavedSearch, SavedSearchFilters, SearchFilters};

// ── Private helper ───────────────────────────────────────────────────────────

/// Apply entity-class filters against the EntityStore.
///
/// Returns:
/// - `None`              — empty filter list; no narrowing.
/// - `Some(non-empty)`   — doc-ids that mention every filter entity (AND).
/// - `Some(empty)`       — at least one filter entity unknown; zero candidates.
///
/// Key lookup: `(value.to_lowercase(), class_key)` where `class_key` is tried
/// first as-is (Phase 8 capitalized class), then as `class.to_lowercase()`
/// (Phase 6 entity_type convention). This bridges both storage generations
/// (pitfall #5 in 11-RESEARCH.md).
fn apply_entity_class_filters_local(
    entity_filters: &[EntityClassFilter],
    entity_store: &EntityStore,
) -> Option<HashSet<String>> {
    if entity_filters.is_empty() {
        return None;
    }

    let mut result: Option<HashSet<String>> = None;

    for filter in entity_filters {
        let lower_value = filter.value.to_lowercase();

        // Try Phase-8 capitalized class first, then Phase-6 lowercase entity_type.
        let canonical_id = entity_store
            .alias_index
            .get(&(lower_value.clone(), filter.class.clone()))
            .or_else(|| {
                entity_store
                    .alias_index
                    .get(&(lower_value.clone(), filter.class.to_lowercase()))
            })
            .cloned();

        match canonical_id {
            None => {
                // Unknown entity → AND-semantics zeroes the intersection immediately.
                return Some(HashSet::new());
            }
            Some(cid) => {
                let doc_set: HashSet<String> = entity_store
                    .doc_index
                    .get(&cid)
                    .map(|s| s.iter().cloned().collect())
                    .unwrap_or_default();

                result = Some(match result {
                    None => doc_set,
                    Some(existing) => existing.intersection(&doc_set).cloned().collect(),
                });
            }
        }
    }

    result
}

/// Apply metadata filters and entity class filters to count matching documents.
///
/// Metadata-only, no HNSW call — keeps sidebar mount performant on personal
/// corpus scale (Open Q 1 resolution from 11-RESEARCH.md).
///
/// Combine rules:
/// | metadata | entity | combined   |
/// |----------|--------|------------|
/// | None     | None   | count all  |
/// | Some(A)  | None   | |A|        |
/// | None     | Some(B)| |B|        |
/// | Some(A)  | Some(B)| |A ∩ B|    |
fn count_matching_docs(
    search_filters: &SearchFilters,
    entity_store: &EntityStore,
    engine: &CortexEngine,
) -> Result<u32, AppError> {
    // Metadata filter (doc_type, space_id, date_from, date_to, tags).
    let metadata_set = crate::search::filters::apply_metadata_filters(search_filters, engine)?;

    // Entity-class filter (Phase 11 URL-format "{class}:{value}" list).
    let entity_set = search_filters
        .entity_filters
        .as_deref()
        .map(|ef| apply_entity_class_filters_local(ef, entity_store))
        .unwrap_or(None);

    // Combine sets.
    let count = match (metadata_set, entity_set) {
        (None, None) => {
            // No filters — count ALL docs in the collection.
            let collection_arc = engine
                .collections
                .get_collection("documents_384")
                .ok_or_else(|| {
                    AppError::VectorStorage("documents_384 collection not found".to_string())
                })?;
            let collection = collection_arc.read();
            collection
                .db
                .keys()
                .map_err(|e| AppError::VectorStorage(e.to_string()))?
                .len() as u32
        }
        (Some(a), None) => a.len() as u32,
        (None, Some(b)) => b.len() as u32,
        (Some(a), Some(b)) => a.intersection(&b).count() as u32,
    };

    Ok(count)
}

/// Parse `SavedSearchFilters.entities` (Vec<String> of "{class}:{value}") into
/// a `Vec<EntityClassFilter>` suitable for `SearchFilters.entity_filters`.
fn parse_entity_class_filters(entities: &[String]) -> Vec<EntityClassFilter> {
    entities
        .iter()
        .filter_map(|s| {
            s.split_once(':').map(|(class, value)| EntityClassFilter {
                class: class.to_string(),
                value: value.to_string(),
            })
        })
        .collect()
}

/// Build a `SearchFilters` from a `SavedSearchFilters` (the persisted filter
/// shape, D-06) by converting the entities list into `entity_filters`.
fn build_search_filters(saved_filters: &SavedSearchFilters) -> SearchFilters {
    let entity_filters = if saved_filters.entities.is_empty() {
        None
    } else {
        Some(parse_entity_class_filters(&saved_filters.entities))
    };

    SearchFilters {
        doc_type: saved_filters.doc_type.clone(),
        space_id: saved_filters.space_id.clone(),
        date_from: saved_filters.date_from.clone(),
        date_to: saved_filters.date_to.clone(),
        tags: saved_filters.tags.clone(),
        entity_filters,
    }
}

// ── IPC commands ─────────────────────────────────────────────────────────────

/// Return all persisted saved searches (pure in-memory clone, no filesystem I/O).
///
/// Called by `useSavedSearches()` hook on initial render (D-07 Sidebar section).
#[tauri::command]
pub async fn get_saved_searches(
    state: State<'_, AppState>,
) -> Result<Vec<SavedSearch>, AppError> {
    let store = state.saved_search_store.lock().await;
    Ok(store.all().to_vec())
}

/// Persist a new saved search and return it with the generated `ss-{uuid}` id.
///
/// Security: rejects empty/whitespace-only names (T-11-10).
/// Initial `doc_count_cache` is computed inline via `count_matching_docs` so
/// the sidebar renders the correct count without a follow-up IPC round-trip.
#[tauri::command]
pub async fn save_search(
    name: String,
    query: String,
    filters: SavedSearchFilters,
    state: State<'_, AppState>,
) -> Result<SavedSearch, AppError> {
    // T-11-10: reject empty/whitespace names.
    if name.trim().is_empty() {
        return Err(AppError::Internal(
            "saved search name must not be empty".to_string(),
        ));
    }

    // CR-02: reject malformed entity filter strings that lack a ':' separator.
    // Entries without ':' would be silently dropped by parse_entity_class_filters on every
    // count refresh, causing a permanently incorrect (too-high) doc count with no error.
    for entity_str in &filters.entities {
        if !entity_str.contains(':') {
            return Err(AppError::Internal(format!(
                "malformed entity filter '{}': expected 'Class:value' format",
                entity_str
            )));
        }
    }

    let store_arc = state.saved_search_store.clone();
    let entity_store_arc = state.entity_store.clone();
    let engine_arc = state.engine.clone();
    let app_data_dir = state.app_data_dir.clone();

    let filters_clone = filters.clone();
    let name_clone = name.clone();
    let query_clone = query.clone();

    let new_search = tokio::task::spawn_blocking(move || {
        // Compute doc_count_cache inline (metadata-only, no HNSW).
        // CR-01 fix: acquire engine first, then entity_store — consistent with
        // search_documents (documents.rs) which also acquires engine before entity_store.
        // Reversed ordering (entity_store → engine) caused a potential deadlock when
        // save_search and search_documents ran concurrently.
        let doc_count_cache = {
            let engine_guard = engine_arc.blocking_lock();
            let entity_store_guard = entity_store_arc
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let search_filters = build_search_filters(&filters_clone);
            count_matching_docs(&search_filters, &entity_store_guard, &engine_guard)?
        };

        let id = format!("ss-{}", uuid::Uuid::new_v4());
        let created_at = chrono::Utc::now().to_rfc3339();

        let new_search = SavedSearch {
            id,
            name: name_clone,
            query: query_clone,
            filters: filters_clone,
            created_at,
            doc_count_cache,
        };

        // Persist to disk.
        let mut store = store_arc.blocking_lock();
        store.insert(new_search.clone());
        store
            .save(&app_data_dir)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok::<SavedSearch, AppError>(new_search)
    })
    .await??;

    Ok(new_search)
}

/// Remove the saved search with the given id from the store and persist.
///
/// Returns `AppError::NotFound` if the id does not exist.
#[tauri::command]
pub async fn delete_saved_search(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let store_arc = state.saved_search_store.clone();
    let app_data_dir = state.app_data_dir.clone();

    tokio::task::spawn_blocking(move || {
        let mut store = store_arc.blocking_lock();
        if !store.remove(&id) {
            return Err(AppError::NotFound(format!("saved search not found: {}", id)));
        }
        store
            .save(&app_data_dir)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    })
    .await?
}

/// Batch doc-count refresh for the Sidebar (ENEX-04, D-08).
///
/// Returns a `HashMap<id, count>` in a single IPC round-trip so the Sidebar
/// can refresh all saved-search counts without N separate calls.
/// Uses metadata-only counting — no embedding call, no HNSW (Open Q 1).
#[tauri::command]
pub async fn get_saved_search_counts(
    ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<HashMap<String, u32>, AppError> {
    let store_arc = state.saved_search_store.clone();
    let entity_store_arc = state.entity_store.clone();
    let engine_arc = state.engine.clone();

    tokio::task::spawn_blocking(move || {
        let store = store_arc.blocking_lock();
        // CR-01 fix: acquire engine before entity_store — consistent with search_documents.
        let engine_guard = engine_arc.blocking_lock();
        let entity_store_guard = entity_store_arc
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut result: HashMap<String, u32> = HashMap::new();

        for id in &ids {
            let count = match store.get(id) {
                None => 0,
                Some(saved_search) => {
                    let search_filters = build_search_filters(&saved_search.filters);
                    count_matching_docs(&search_filters, &entity_store_guard, &engine_guard)?
                }
            };
            result.insert(id.clone(), count);
        }

        Ok::<HashMap<String, u32>, AppError>(result)
    })
    .await?
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::entity_store::EntityStore;
    use crate::types::CanonicalEntity;
    use std::collections::{HashMap, HashSet};

    // ── helpers ──────────────────────────────────────────────────────────────

    fn make_seeded_store() -> EntityStore {
        let mut store = EntityStore::new();

        // Canonical: Person "Alex Doe"
        let cid_person = "cid-p1".to_string();
        store.canonicals.insert(
            cid_person.clone(),
            CanonicalEntity {
                id: cid_person.clone(),
                canonical_name: "Alex Doe".to_string(),
                entity_type: "person".to_string(),
                aliases: vec![],
                document_count: 2,
            canonical_short_name: None,
            },
        );
        store
            .alias_index
            .insert(("alex doe".to_string(), "person".to_string()), cid_person.clone());
        // Also index under Phase-8 capitalized class for bridge test
        store
            .alias_index
            .insert(("alex doe".to_string(), "Person".to_string()), cid_person.clone());
        let mut docs_person: HashSet<String> = HashSet::new();
        docs_person.insert("d1".to_string());
        docs_person.insert("d2".to_string());
        store.doc_index.insert(cid_person, docs_person);

        // Canonical: Location "AlphaComplex"
        let cid_loc = "cid-loc1".to_string();
        store.canonicals.insert(
            cid_loc.clone(),
            CanonicalEntity {
                id: cid_loc.clone(),
                canonical_name: "AlphaComplex".to_string(),
                entity_type: "location".to_string(),
                aliases: vec![],
                document_count: 2,
            canonical_short_name: None,
            },
        );
        store
            .alias_index
            .insert(("alphacomplex".to_string(), "location".to_string()), cid_loc.clone());
        store
            .alias_index
            .insert(("alphacomplex".to_string(), "Location".to_string()), cid_loc.clone());
        let mut docs_loc: HashSet<String> = HashSet::new();
        docs_loc.insert("d2".to_string());
        docs_loc.insert("d3".to_string());
        store.doc_index.insert(cid_loc, docs_loc);

        store
    }

    // ── apply_entity_class_filters_local tests ────────────────────────────────

    /// Empty filter list → None (no narrowing).
    #[test]
    fn test_entity_filter_empty_returns_none() {
        let store = make_seeded_store();
        let result = apply_entity_class_filters_local(&[], &store);
        assert!(result.is_none(), "empty filter list must return None");
    }

    /// Single filter for a known entity → correct doc set.
    #[test]
    fn test_entity_filter_single_known() {
        let store = make_seeded_store();
        let filters = vec![EntityClassFilter {
            class: "Person".to_string(),
            value: "Alex Doe".to_string(),
        }];
        let result = apply_entity_class_filters_local(&filters, &store).expect("should be Some");
        assert_eq!(result.len(), 2);
        assert!(result.contains("d1"));
        assert!(result.contains("d2"));
    }

    /// Two filters → intersection (AND semantics).
    #[test]
    fn test_entity_filter_two_filters_and_semantics() {
        let store = make_seeded_store();
        let filters = vec![
            EntityClassFilter {
                class: "Person".to_string(),
                value: "Alex Doe".to_string(),
            },
            EntityClassFilter {
                class: "Location".to_string(),
                value: "AlphaComplex".to_string(),
            },
        ];
        let result = apply_entity_class_filters_local(&filters, &store).expect("should be Some");
        assert_eq!(result.len(), 1, "intersection should be {{d2}}");
        assert!(result.contains("d2"));
    }

    /// Unknown entity → Some(empty) — no panic.
    #[test]
    fn test_entity_filter_unknown_entity_returns_empty() {
        let store = make_seeded_store();
        let filters = vec![EntityClassFilter {
            class: "Person".to_string(),
            value: "Unknown Person".to_string(),
        }];
        let result = apply_entity_class_filters_local(&filters, &store).expect("should be Some");
        assert!(result.is_empty(), "unknown entity must return empty set, not panic");
    }

    /// Case-insensitive value lookup (alias_index stores lowercase values).
    #[test]
    fn test_entity_filter_case_insensitive_value() {
        let store = make_seeded_store();
        let filters = vec![EntityClassFilter {
            class: "Person".to_string(),
            value: "ALEX DOE".to_string(), // uppercase — should still resolve
        }];
        let result = apply_entity_class_filters_local(&filters, &store).expect("should be Some");
        assert_eq!(result.len(), 2, "uppercase value must resolve via lowercase lookup");
    }

    /// Phase 6/8 class bridge: filter with lowercase class must resolve same canonical
    /// as uppercase class (because alias_index uses both forms in seeded store).
    #[test]
    fn test_entity_filter_class_case_bridge() {
        let mut store = EntityStore::new();
        // Simulate Phase-6 storage: alias_index keyed with lowercase entity_type
        let cid = "cid-org1".to_string();
        store.canonicals.insert(
            cid.clone(),
            CanonicalEntity {
                id: cid.clone(),
                canonical_name: "Acme Corp".to_string(),
                entity_type: "organization".to_string(),
                aliases: vec![],
                document_count: 1,
            canonical_short_name: None,
            },
        );
        store.alias_index.insert(
            ("acme corp".to_string(), "organization".to_string()),
            cid.clone(),
        );
        let mut docs: HashSet<String> = HashSet::new();
        docs.insert("d4".to_string());
        store.doc_index.insert(cid, docs);

        // Filter with Phase-8 capitalized class "Organization" should fall back to lowercase
        let filters = vec![EntityClassFilter {
            class: "Organization".to_string(),
            value: "Acme Corp".to_string(),
        }];
        let result = apply_entity_class_filters_local(&filters, &store).expect("should be Some");
        assert_eq!(result.len(), 1, "Phase-8 class must resolve via lowercase fallback");
        assert!(result.contains("d4"));
    }

    // ── parse_entity_class_filters tests ──────────────────────────────────────

    /// Parse "Class:value" strings into EntityClassFilter list.
    #[test]
    fn test_parse_entity_class_filters_basic() {
        let entities = vec![
            "Person:Alex Doe".to_string(),
            "Location:AlphaComplex".to_string(),
        ];
        let filters = parse_entity_class_filters(&entities);
        assert_eq!(filters.len(), 2);
        assert_eq!(filters[0].class, "Person");
        assert_eq!(filters[0].value, "Alex Doe");
        assert_eq!(filters[1].class, "Location");
        assert_eq!(filters[1].value, "AlphaComplex");
    }

    /// Entries without ':' are silently skipped (no panic).
    #[test]
    fn test_parse_entity_class_filters_skips_invalid() {
        let entities = vec!["NoColonHere".to_string(), "Person:Valid".to_string()];
        let filters = parse_entity_class_filters(&entities);
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].value, "Valid");
    }

    // ── save_search id format test ────────────────────────────────────────────

    /// Verify ss-{uuid} id format by parsing the UUID portion.
    #[test]
    fn test_save_search_id_format() {
        // Simulate id generation logic from save_search command
        let id = format!("ss-{}", uuid::Uuid::new_v4());
        assert!(id.starts_with("ss-"), "id must start with 'ss-'");
        let uuid_part = &id["ss-".len()..];
        uuid::Uuid::parse_str(uuid_part).expect("remainder must be a valid UUID v4");
    }
}

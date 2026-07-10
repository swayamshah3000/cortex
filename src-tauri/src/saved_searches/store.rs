//! Persistent store for user-created Saved Searches.
//!
//! Writes `{app_data_dir}/saved_searches.json` — the same JSON-sidecar pattern
//! used by `spaces/label_cache.rs` for `space_labels.json`.
//!
//! # Schema (D-06 from 11-CONTEXT.md)
//! ```json
//! {
//!   "savedSearches": [
//!     { "id": "ss-uuid", "name": "Property Tax 2024", "query": "property tax 2024",
//!       "filters": { "entities": ["Location:AlphaComplex"], "topic": "property" },
//!       "createdAt": "2026-07-08T10:00Z", "docCountCache": 12 }
//!   ]
//! }
//! ```
//!
//! # Thread-safety (T-11-06)
//! `load` / `save` are synchronous. Callers in Plan 04 wrap this in
//! `Arc<tokio::sync::Mutex<SavedSearchStore>>` inside `AppState`.
//!
//! # Error resilience (T-11-04)
//! `load` never panics. Any I/O or JSON parse error silently returns the
//! `Default` (empty store). The worst-case outcome is empty saved searches,
//! not a crash. Mirror of SpaceLabelCache resilience contract.

use crate::types::SavedSearch;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// In-memory mirror of `{app_data_dir}/saved_searches.json`.
///
/// The outer field uses camelCase serde (`savedSearches`) to match the D-06
/// JSON shape. Arc<tokio::sync::Mutex<SavedSearchStore>> wrapping is done by
/// Plan 04 at the AppState layer — not here.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SavedSearchStore {
    /// Ordered list of user-saved searches. Append-only via `insert`; removable via `remove`.
    pub saved_searches: Vec<SavedSearch>,
}

impl SavedSearchStore {
    /// Load the store from `{app_data_dir}/saved_searches.json`.
    ///
    /// Returns `Default::default()` (empty store) on any I/O or JSON parse
    /// error — never panics (D-05 / T-11-04 mitigation).
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join("saved_searches.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist the store to `{app_data_dir}/saved_searches.json`.
    ///
    /// Creates the file if it does not exist; overwrites if it does.
    /// Uses the `serde_json::to_string_pretty` + `std::fs::write` pattern
    /// from `spaces/label_cache.rs` (same atomicity guarantees on POSIX).
    ///
    /// # Concurrency note (T-11-06)
    /// Wrap in `Arc<Mutex<>>` at the call site (Plan 04 AppState) to prevent
    /// concurrent writes corrupting the file.
    pub fn save(&self, app_data_dir: &Path) -> std::io::Result<()> {
        let path = app_data_dir.join("saved_searches.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// Read accessor — returns the SavedSearch with the given `id`, if present.
    ///
    /// Linear scan: the number of saved searches is user-bounded (< 1000 realistic;
    /// T-11-05 accepts DoS from huge lists as out-of-scope for v1).
    pub fn get(&self, id: &str) -> Option<&SavedSearch> {
        self.saved_searches.iter().find(|s| s.id == id)
    }

    /// Write accessor — inserts `search` if its `id` is absent; replaces in-place
    /// if the `id` already exists (upsert semantics).
    ///
    /// Upsert ensures Plan 04's `save_search` command can update `doc_count_cache`
    /// without duplicating entries.
    pub fn insert(&mut self, search: SavedSearch) {
        if let Some(pos) = self.saved_searches.iter().position(|s| s.id == search.id) {
            self.saved_searches[pos] = search;
        } else {
            self.saved_searches.push(search);
        }
    }

    /// Remove the saved search with the given `id`.
    ///
    /// Returns `true` if a removal happened, `false` if no entry matched.
    /// Used by Plan 04's `delete_saved_search` IPC command.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.saved_searches.len();
        self.saved_searches.retain(|s| s.id != id);
        self.saved_searches.len() < before
    }

    /// Returns the full slice of saved searches (all entries).
    ///
    /// Used by Plan 04's `get_saved_searches` IPC command and the Sidebar
    /// count-refresh path (ENEX-04 / D-08).
    pub fn all(&self) -> &[SavedSearch] {
        &self.saved_searches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SavedSearchFilters;
    use tempfile::TempDir;

    // Helper: build a SavedSearch with entity-only filters.
    fn entity_only_search(id: &str, name: &str, entities: Vec<String>) -> SavedSearch {
        SavedSearch {
            id: id.to_string(),
            name: name.to_string(),
            query: "".to_string(),
            filters: SavedSearchFilters {
                entities,
                ..Default::default()
            },
            created_at: "2026-07-08T10:00:00Z".to_string(),
            doc_count_cache: 0,
        }
    }

    // Helper: build a SavedSearch with entity + topic + date_from filters.
    fn full_filter_search(id: &str, name: &str) -> SavedSearch {
        SavedSearch {
            id: id.to_string(),
            name: name.to_string(),
            query: "property tax 2024".to_string(),
            filters: SavedSearchFilters {
                entities: vec!["Location:AlphaComplex".to_string()],
                topic: Some("finance".to_string()),
                doc_type: Some("pdf".to_string()),
                space_id: None,
                date_from: Some("2024-01-01".to_string()),
                date_to: None,
                tags: Some(vec!["tax".to_string()]),
            },
            created_at: "2026-07-08T12:00:00Z".to_string(),
            doc_count_cache: 12,
        }
    }

    /// Test 1: `SavedSearchStore::default()` produces an empty saved_searches vec.
    #[test]
    fn test_default_empty() {
        let store = SavedSearchStore::default();
        assert!(
            store.saved_searches.is_empty(),
            "Default store must be empty"
        );
    }

    /// Test 2: `SavedSearchStore::load()` on a nonexistent path returns empty default
    /// without panicking (T-11-04 / D-05 resilience contract).
    #[test]
    fn test_load_missing_returns_default() {
        let store = SavedSearchStore::load(Path::new("/nonexistent/path/cortex-test-11-02"));
        assert!(
            store.saved_searches.is_empty(),
            "Load from missing path must return empty default, not panic"
        );
    }

    /// Test 3: Round-trip preserves every field for two SavedSearch entries with
    /// distinct filter shapes — one entity-only, one with entity+topic+date_from.
    #[test]
    fn test_roundtrip_preserves_all_fields() {
        let dir = TempDir::new().unwrap();

        let s1 = entity_only_search(
            "ss-001",
            "Entity Only Search",
            vec!["Person:Alex Doe".to_string()],
        );
        let s2 = full_filter_search("ss-002", "Full Filter Search");

        let mut store = SavedSearchStore::default();
        store.insert(s1.clone());
        store.insert(s2.clone());

        store.save(dir.path()).unwrap();
        let loaded = SavedSearchStore::load(dir.path());

        // --- s1 field-by-field assertions ---
        let r1 = loaded.get("ss-001").expect("ss-001 must survive round-trip");
        assert_eq!(r1.id,    s1.id,    "id");
        assert_eq!(r1.name,  s1.name,  "name");
        assert_eq!(r1.query, s1.query, "query");
        assert_eq!(r1.filters.entities, s1.filters.entities, "filters.entities");
        assert_eq!(r1.filters.topic,    s1.filters.topic,    "filters.topic");
        assert_eq!(r1.filters.doc_type, s1.filters.doc_type, "filters.doc_type");
        assert_eq!(r1.created_at,       s1.created_at,       "created_at");
        assert_eq!(r1.doc_count_cache,  s1.doc_count_cache,  "doc_count_cache");

        // --- s2 field-by-field assertions ---
        let r2 = loaded.get("ss-002").expect("ss-002 must survive round-trip");
        assert_eq!(r2.id,    s2.id,    "id s2");
        assert_eq!(r2.name,  s2.name,  "name s2");
        assert_eq!(r2.query, s2.query, "query s2");
        assert_eq!(r2.filters.entities,  s2.filters.entities,  "filters.entities s2");
        assert_eq!(r2.filters.topic,     s2.filters.topic,     "filters.topic s2");
        assert_eq!(r2.filters.doc_type,  s2.filters.doc_type,  "filters.doc_type s2");
        assert_eq!(r2.filters.date_from, s2.filters.date_from, "filters.date_from s2");
        assert_eq!(r2.filters.tags,      s2.filters.tags,      "filters.tags s2");
        assert_eq!(r2.created_at,        s2.created_at,        "created_at s2");
        assert_eq!(r2.doc_count_cache,   s2.doc_count_cache,   "doc_count_cache s2");
    }

    /// Test 4: Pre-writing garbage to `saved_searches.json` and calling `load()`
    /// must return empty default without panicking (T-11-04 malformed-JSON resilience).
    #[test]
    fn test_malformed_json_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("saved_searches.json");
        std::fs::write(&path, b"{{{not valid JSON!!!}}}").unwrap();

        let store = SavedSearchStore::load(dir.path());
        assert!(
            store.saved_searches.is_empty(),
            "Malformed JSON must yield empty default, not panic"
        );
    }

    /// Test 5: Calling `save()` twice with different content — the second `load()`
    /// must return only the second content (overwrite semantics).
    #[test]
    fn test_save_overwrites() {
        let dir = TempDir::new().unwrap();

        let mut store_v1 = SavedSearchStore::default();
        store_v1.insert(entity_only_search("ss-v1", "First Search", vec![]));
        store_v1.save(dir.path()).unwrap();

        let mut store_v2 = SavedSearchStore::default();
        store_v2.insert(entity_only_search("ss-v2", "Second Search", vec![]));
        store_v2.save(dir.path()).unwrap();

        let loaded = SavedSearchStore::load(dir.path());
        assert_eq!(
            loaded.saved_searches.len(),
            1,
            "Only the second save's single entry must be present"
        );
        assert_eq!(
            loaded.saved_searches[0].id,
            "ss-v2",
            "Loaded entry must be from the second save"
        );
        assert!(
            loaded.get("ss-v1").is_none(),
            "First save's entry must not appear after second save overwrites"
        );
    }

    /// Test 6: After `save()`, `{tmp_dir}/saved_searches.json` must exist at the
    /// exact path (not a subdirectory, not a differently-named file).
    #[test]
    fn test_file_at_correct_path() {
        let dir = TempDir::new().unwrap();
        let mut store = SavedSearchStore::default();
        store.insert(entity_only_search("ss-path-check", "Path Check", vec![]));

        store.save(dir.path()).unwrap();

        let expected = dir.path().join("saved_searches.json");
        assert!(
            expected.exists(),
            "Expected file at {}, but it does not exist",
            expected.display()
        );
    }

    /// Bonus: insert idempotency — inserting a search with the same id replaces
    /// in-place (upsert) and does not create a duplicate entry.
    #[test]
    fn test_insert_upsert_no_duplicate() {
        let mut store = SavedSearchStore::default();
        store.insert(entity_only_search("ss-dup", "Original", vec![]));
        store.insert(entity_only_search("ss-dup", "Updated", vec![]));

        assert_eq!(
            store.saved_searches.len(),
            1,
            "Upserting same id must not create duplicate"
        );
        assert_eq!(
            store.get("ss-dup").unwrap().name,
            "Updated",
            "Upserted entry must have the new name"
        );
    }

    /// Bonus: remove returns true when entry found, false when absent.
    #[test]
    fn test_remove_returns_bool() {
        let mut store = SavedSearchStore::default();
        store.insert(entity_only_search("ss-rm", "To Remove", vec![]));

        assert!(
            store.remove("ss-rm"),
            "remove must return true when entry exists"
        );
        assert!(
            !store.remove("ss-rm"),
            "remove must return false when entry already gone"
        );
        assert!(
            store.saved_searches.is_empty(),
            "Store must be empty after removal"
        );
    }
}

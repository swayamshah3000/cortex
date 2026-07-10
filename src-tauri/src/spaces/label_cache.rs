//! Persistent cache for LLM-generated Space labels.
//!
//! Writes `{app_data_dir}/space_labels.json` — the same sidecar pattern
//! used by `commands/settings.rs` for `settings.json`.
//!
//! # Schema (D-07)
//! ```json
//! {
//!   "labels": {
//!     "space-abc123": {
//!       "fingerprint": "d41d8cd98f00b204",
//!       "label": "Property Tax Records",
//!       "description": "...",
//!       "canonicalEntityHint": "Property: AlphaComplex",
//!       "generatedAt": "2026-07-04T10:00:00Z",
//!       "userLocked": false
//!     }
//!   }
//! }
//! ```
//!
//! # Thread-safety (T-09-03)
//! `load` / `save` are synchronous. Callers in Plan 04 wrap this in
//! `Arc<tokio::sync::Mutex<SpaceLabelCache>>` inside `AppState`.
//!
//! # Error resilience (T-09-04 / D-08)
//! `load` never panics. Any I/O or JSON parse error silently returns the
//! `Default` (empty cache). The worst-case outcome is a fresh label batch,
//! not a crash.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// One cached label entry for a single Space (keyed by `space_id` in the
/// outer `SpaceLabelCache::labels` map — **not** by fingerprint).
///
/// Pitfall #4: Two spaces can share the same `fingerprint` value (if their
/// doc-id membership happens to be identical). Keying the map by `space_id`
/// ensures both entries survive a save → load round-trip independently.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpaceLabelEntry {
    /// 16-char hex SHA-256 fingerprint of the sorted doc-id set (D-05).
    /// Used to detect whether a re-label is needed (Jaccard > 20%, D-06).
    pub fingerprint: String,
    /// 2-4 word LLM-generated category label (e.g. "Property Tax Records").
    pub label: String,
    /// 1-sentence description of the space's content (LLML-02).
    pub description: String,
    /// Highest-count entity across the space's docs (Phase 11/13 hook, D-17).
    /// Format: `"{ClassName}: {value}"` e.g. `"Property: AlphaComplex"`.
    /// `None` when no single entity dominates (< 20% of doc count, D-18).
    pub canonical_entity_hint: Option<String>,
    /// ISO-8601 timestamp of when the label was last generated.
    pub generated_at: String,
    /// When `true`, the user manually renamed this space. LLM re-labeling
    /// skips user-locked spaces (D-15 / pitfall #5 backend enforcement).
    pub user_locked: bool,

    // === Phase 10 Plan 01: Hierarchical Space cache fields (D-06) ===
    // Both fields use #[serde(default)] so existing Phase-9 space_labels.json entries
    // (which lack these keys) still deserialize cleanly without wiping the cache.
    // T-10-01 mitigation — pitfall #4 in 10-RESEARCH.md.

    /// ID of the parent Space for sub-space cache entries; None for top-level spaces.
    /// Enables invalidation of all sub-space entries when a parent membership shifts
    /// > 20% (D-08: recluster invalidates sub-spaces).
    #[serde(default)]
    pub parent_id: Option<String>,

    /// Hierarchy depth: 0 = top-level, 1 = sub-space (D-06, D-03).
    /// Mirrors Space.depth in types.rs for cache-side hierarchy awareness.
    #[serde(default)]
    pub depth: u8,
}

/// In-memory mirror of `{app_data_dir}/space_labels.json`.
///
/// Keyed by `space_id` (not fingerprint — see pitfall #4 in RESEARCH.md).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpaceLabelCache {
    /// Map of `space_id → SpaceLabelEntry`.
    pub labels: HashMap<String, SpaceLabelEntry>,
}

impl SpaceLabelCache {
    /// Load the cache from `{app_data_dir}/space_labels.json`.
    ///
    /// Returns `Default::default()` (empty cache) on any I/O or JSON parse
    /// error — never panics (D-08 / T-09-04 mitigation).
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join("space_labels.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist the cache to `{app_data_dir}/space_labels.json`.
    ///
    /// Creates the file if it does not exist; overwrites if it does.
    /// Uses the `serde_json::to_string_pretty` + `std::fs::write` pattern
    /// from `commands/settings.rs` (same atomicity guarantees).
    ///
    /// # Concurrency note (T-09-03)
    /// Wrap in `Arc<Mutex<>>` at the call site (Plan 04 AppState) to prevent
    /// concurrent writes.
    pub fn save(&self, app_data_dir: &Path) -> std::io::Result<()> {
        let path = app_data_dir.join("space_labels.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// Read accessor — returns the entry for the given `space_id`, if any.
    pub fn get(&self, space_id: &str) -> Option<&SpaceLabelEntry> {
        self.labels.get(space_id)
    }

    /// Write accessor — inserts or replaces the entry for `space_id`.
    pub fn insert(&mut self, space_id: String, entry: SpaceLabelEntry) {
        self.labels.insert(space_id, entry);
    }

    /// Remove a Space's cache entry (D-08 lazy garbage-collection of deleted
    /// spaces on next recluster).
    pub fn remove(&mut self, space_id: &str) {
        self.labels.remove(space_id);
    }

    /// Returns `true` when the user has manually locked this Space's label
    /// (D-15 / pitfall #5). LLM re-labeling calls check this before
    /// overwriting.
    pub fn is_user_locked(&self, space_id: &str) -> bool {
        self.labels
            .get(space_id)
            .map(|e| e.user_locked)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Helper: build a complete SpaceLabelEntry with every Phase 9 field set (Phase 10 fields default).
    fn full_entry(
        fingerprint: &str,
        label: &str,
        hint: Option<&str>,
        user_locked: bool,
    ) -> SpaceLabelEntry {
        SpaceLabelEntry {
            fingerprint: fingerprint.to_string(),
            label: label.to_string(),
            description: format!("Description for {}", label),
            canonical_entity_hint: hint.map(|s| s.to_string()),
            generated_at: "2026-07-04T10:00:00Z".to_string(),
            user_locked,
            parent_id: None,
            depth: 0,
        }
    }

    // Helper: build a SpaceLabelEntry with Phase 10 parent_id + depth fields set.
    fn full_entry_with_hierarchy(
        fingerprint: &str,
        label: &str,
        hint: Option<&str>,
        user_locked: bool,
        parent_id: Option<&str>,
        depth: u8,
    ) -> SpaceLabelEntry {
        SpaceLabelEntry {
            fingerprint: fingerprint.to_string(),
            label: label.to_string(),
            description: format!("Description for {}", label),
            canonical_entity_hint: hint.map(|s| s.to_string()),
            generated_at: "2026-07-08T00:00:00Z".to_string(),
            user_locked,
            parent_id: parent_id.map(|s| s.to_string()),
            depth,
        }
    }

    // ===== Phase 10 Plan 01 tests: SpaceLabelEntry extension (D-06) =====

    /// Test 1: SpaceLabelEntry with parent_id=Some("parent-abc"), depth=1 round-trips
    /// through SpaceLabelCache::save → load inside a TempDir, preserving both new fields.
    /// D-06: new fields must persist to space_labels.json and reload intact.
    #[test]
    fn test_phase10_fields_roundtrip() {
        let dir = TempDir::new().unwrap();
        let entry = full_entry_with_hierarchy(
            "fp_sub_abc0000000",
            "Tax Records",
            None,
            false,
            Some("parent-abc"),
            1,
        );

        let mut cache = SpaceLabelCache::default();
        cache.insert("sub-space-tax".to_string(), entry);

        cache.save(dir.path()).unwrap();
        let loaded = SpaceLabelCache::load(dir.path());

        let e = loaded.get("sub-space-tax")
            .expect("sub-space-tax entry must survive save/load round-trip");
        assert_eq!(
            e.parent_id,
            Some("parent-abc".to_string()),
            "parent_id must survive save/load round-trip"
        );
        assert_eq!(e.depth, 1, "depth=1 must survive save/load round-trip");
        assert_eq!(e.label, "Tax Records");
    }

    /// Test 2: A hand-written space_labels.json with Phase-9 shape (no parentId or depth keys)
    /// deserializes cleanly, producing parent_id=None and depth=0.
    /// Prevents pitfall #4: wiping cache on app upgrade from Phase 9 to Phase 10.
    #[test]
    fn test_phase10_backward_compat() {
        // Phase-9 JSON shape — no parentId or depth keys in the label entry.
        let json_str = r#"{
            "labels": {
                "space-1": {
                    "fingerprint": "abc1234567890abc",
                    "label": "Foo",
                    "description": "Bar",
                    "canonicalEntityHint": null,
                    "generatedAt": "2026-07-08T00:00:00Z",
                    "userLocked": false
                }
            }
        }"#;
        let cache: SpaceLabelCache = serde_json::from_str(json_str)
            .expect("Phase-9 space_labels.json must deserialize without parentId/depth (backward compat)");
        let entry = cache.get("space-1")
            .expect("space-1 must be present after deserialization");
        assert_eq!(
            entry.parent_id, None,
            "missing parentId must default to None (top-level)"
        );
        assert_eq!(entry.depth, 0, "missing depth must default to 0 (top-level)");
    }

    /// Test 3: A top-level SpaceLabelEntry (parent_id=None, depth=0) still serializes
    /// without runtime error and reloads with the same default values.
    /// Validates that serde defaults survive when both fields are at their default values.
    #[test]
    fn test_phase10_default_top_level_omits_parent_id() {
        let dir = TempDir::new().unwrap();
        let entry = full_entry_with_hierarchy(
            "fp_top_level000000",
            "Property",
            Some("Property: AlphaComplex"),
            false,
            None,    // parent_id = None (top-level)
            0,       // depth = 0 (top-level)
        );

        let mut cache = SpaceLabelCache::default();
        cache.insert("space-property".to_string(), entry);

        cache.save(dir.path()).unwrap();
        let loaded = SpaceLabelCache::load(dir.path());

        let e = loaded.get("space-property")
            .expect("space-property must survive save/load");
        assert_eq!(e.parent_id, None, "top-level parent_id must remain None after roundtrip");
        assert_eq!(e.depth, 0, "top-level depth must remain 0 after roundtrip");
    }

    #[test]
    fn test_cache_default_empty() {
        // Default must produce an empty labels map (no entries, no panic).
        let cache = SpaceLabelCache::default();
        assert!(
            cache.labels.is_empty(),
            "Default cache must be empty"
        );
    }

    #[test]
    fn test_load_missing_dir_returns_default() {
        // load() on a path that does not exist must return empty default
        // without panicking (D-08 / T-09-04).
        let cache = SpaceLabelCache::load(Path::new("/nonexistent/path/cortex-test-xyz"));
        assert!(
            cache.labels.is_empty(),
            "Load from missing dir must return empty default"
        );
    }

    #[test]
    fn test_roundtrip_preserves_all_fields() {
        // Full round-trip: build cache with two entries, save, reload, assert
        // all six fields are preserved identically.
        let dir = TempDir::new().unwrap();

        let entry1 = SpaceLabelEntry {
            fingerprint: "d41d8cd98f00b204".to_string(),
            label: "Property Tax Records".to_string(),
            description: "Municipal property tax assessments, receipts, and demand notices."
                .to_string(),
            canonical_entity_hint: Some("Property: AlphaComplex".to_string()),
            generated_at: "2026-07-04T10:00:00Z".to_string(),
            user_locked: true,
            parent_id: None,
            depth: 0,
        };

        let entry2 = SpaceLabelEntry {
            fingerprint: "abc1234567890abc".to_string(),
            label: "Kids School Docs".to_string(),
            description: "School enrollment forms, progress reports, and activity records."
                .to_string(),
            canonical_entity_hint: None,
            generated_at: "2026-07-04T11:00:00Z".to_string(),
            user_locked: false,
            parent_id: None,
            depth: 0,
        };

        let mut cache = SpaceLabelCache::default();
        cache.insert("space-prop-001".to_string(), entry1.clone());
        cache.insert("space-kids-002".to_string(), entry2.clone());

        cache.save(dir.path()).unwrap();
        let loaded = SpaceLabelCache::load(dir.path());

        // --- entry1 (user_locked=true, canonical_entity_hint=Some(...)) ---
        let e1 = loaded.get("space-prop-001").expect("entry1 must survive round-trip");
        assert_eq!(e1.fingerprint, entry1.fingerprint, "fingerprint");
        assert_eq!(e1.label, entry1.label, "label");
        assert_eq!(e1.description, entry1.description, "description");
        assert_eq!(
            e1.canonical_entity_hint, entry1.canonical_entity_hint,
            "canonical_entity_hint"
        );
        assert_eq!(e1.generated_at, entry1.generated_at, "generated_at");
        assert_eq!(e1.user_locked, entry1.user_locked, "user_locked");

        // --- entry2 (user_locked=false, canonical_entity_hint=None) ---
        let e2 = loaded.get("space-kids-002").expect("entry2 must survive round-trip");
        assert_eq!(e2.label, entry2.label, "label entry2");
        assert_eq!(e2.canonical_entity_hint, None, "hint must be None");
        assert!(!e2.user_locked, "user_locked must be false");
    }

    #[test]
    fn test_pitfall4_keying_by_space_id() {
        // Two DIFFERENT spaces that happen to have the SAME fingerprint
        // (identical doc-id membership) must both survive a save → load cycle
        // because the cache is keyed by space_id, not fingerprint (pitfall #4).
        let dir = TempDir::new().unwrap();
        let shared_fp = "aaaa1111bbbb2222";

        let mut cache = SpaceLabelCache::default();
        cache.insert(
            "space-alpha".to_string(),
            full_entry(shared_fp, "Space Alpha", None, false),
        );
        cache.insert(
            "space-beta".to_string(),
            full_entry(shared_fp, "Space Beta", None, false),
        );

        cache.save(dir.path()).unwrap();
        let loaded = SpaceLabelCache::load(dir.path());

        let alpha = loaded.get("space-alpha").expect("space-alpha must exist");
        let beta = loaded.get("space-beta").expect("space-beta must exist");

        assert_eq!(alpha.label, "Space Alpha");
        assert_eq!(beta.label, "Space Beta");
        assert_eq!(alpha.fingerprint, shared_fp);
        assert_eq!(beta.fingerprint, shared_fp);
    }

    #[test]
    fn test_load_malformed_json_returns_default() {
        // Corrupt file content must not panic — returns empty default (T-09-04 / D-08).
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("space_labels.json");
        std::fs::write(&path, b"{{{not valid JSON at all!!!}}}").unwrap();

        let cache = SpaceLabelCache::load(dir.path());
        assert!(
            cache.labels.is_empty(),
            "Malformed JSON must yield empty default, not panic"
        );
    }

    #[test]
    fn test_file_at_correct_path() {
        // The JSON file must be written to `{app_data_dir}/space_labels.json`
        // exactly (not a subdirectory, not a differently-named file).
        let dir = TempDir::new().unwrap();
        let mut cache = SpaceLabelCache::default();
        cache.insert(
            "space-001".to_string(),
            full_entry("fp16charsxxxxxx0", "Test Space", None, false),
        );

        cache.save(dir.path()).unwrap();

        let expected = dir.path().join("space_labels.json");
        assert!(
            expected.exists(),
            "Expected file at {}, but it does not exist",
            expected.display()
        );
    }

    #[test]
    fn test_save_overwrites_on_subsequent_call() {
        // A second `save()` must overwrite the first so a subsequent `load()`
        // reads the new content (settings.rs sidecar pattern).
        let dir = TempDir::new().unwrap();

        let mut cache_v1 = SpaceLabelCache::default();
        cache_v1.insert(
            "space-001".to_string(),
            full_entry("fp_v1xxxxxxxxxxx0", "Original Label", None, false),
        );
        cache_v1.save(dir.path()).unwrap();

        let mut cache_v2 = SpaceLabelCache::default();
        cache_v2.insert(
            "space-001".to_string(),
            full_entry("fp_v2xxxxxxxxxxx0", "Updated Label", None, false),
        );
        cache_v2.save(dir.path()).unwrap();

        let loaded = SpaceLabelCache::load(dir.path());
        assert_eq!(
            loaded.get("space-001").unwrap().label,
            "Updated Label",
            "Second save must overwrite first"
        );
    }

    #[test]
    fn test_is_user_locked() {
        let mut cache = SpaceLabelCache::default();

        // Non-existent space_id → must return false (not panic).
        assert!(
            !cache.is_user_locked("nonexistent"),
            "Unknown space must not be locked"
        );

        cache.insert(
            "space-locked".to_string(),
            full_entry("fp_locked00000000", "Locked Space", None, true),
        );
        cache.insert(
            "space-unlocked".to_string(),
            full_entry("fp_unlocked000000", "Unlocked Space", None, false),
        );

        assert!(
            cache.is_user_locked("space-locked"),
            "user_locked=true space must return true"
        );
        assert!(
            !cache.is_user_locked("space-unlocked"),
            "user_locked=false space must return false"
        );
    }
}

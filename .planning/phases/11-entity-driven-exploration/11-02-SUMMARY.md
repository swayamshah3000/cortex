---
phase: 11-entity-driven-exploration
plan: 02
subsystem: saved-searches-persistence
tags: [rust, persistence, json-sidecar, tdd, saved-searches]
dependency_graph:
  requires:
    - "src-tauri/src/types.rs::SavedSearch (provided by Plan 11-01)"
  provides:
    - "SavedSearchStore JSON-sidecar persistence layer"
    - "src-tauri/src/saved_searches/store.rs::SavedSearchStore"
  affects:
    - "Plan 04 (AppState IPC wiring — will wrap in Arc<Mutex<>>)"
tech_stack:
  added: []
  patterns:
    - "SpaceLabelCache mirror pattern for JSON sidecar persistence"
    - "TDD: RED tests first, then GREEN implementation"
key_files:
  created:
    - "src-tauri/src/saved_searches/mod.rs"
    - "src-tauri/src/saved_searches/store.rs"
  modified:
    - "src-tauri/src/lib.rs (added pub mod saved_searches)"
decisions:
  - "Mirrored SpaceLabelCache load/save resilience contract exactly: .ok().and_then().unwrap_or_default()"
  - "insert() uses upsert semantics (replace in-place if id exists) — matches Plan 04 CRUD requirement"
  - "remove() returns bool — true if removed, false if not found (Plan 04 delete_saved_search needs this)"
  - "8 tests written (6 required + 2 bonus: upsert idempotency + remove bool return)"
  - "SavedSearchFilters and SavedSearch types were already present in types.rs (Plan 11-01 executed)"
metrics:
  duration: "4 minutes"
  completed: "2026-07-09T04:25:00Z"
  tasks_completed: 1
  files_created: 2
  files_modified: 1
---

# Phase 11 Plan 02: SavedSearchStore JSON-sidecar module Summary

**One-liner:** SavedSearchStore Vec-backed JSON sidecar mirroring SpaceLabelCache with load/save/get/insert/remove/all + 8 TDD tests.

## What Was Built

Created `src-tauri/src/saved_searches/` module:

- `mod.rs` — single line `pub mod store;` (Plan 04 will add `pub mod commands;`)
- `store.rs` — `SavedSearchStore` struct with:
  - `load(app_data_dir)` — reads `{app_data_dir}/saved_searches.json`; returns `Default::default()` on any I/O or parse error (T-11-04 mitigation)
  - `save(app_data_dir)` — writes pretty-printed JSON to `{app_data_dir}/saved_searches.json`
  - `get(id)` — linear scan O(n), returns `Option<&SavedSearch>`
  - `insert(search)` — upsert: replace in-place if id exists, push if new
  - `remove(id)` — retains all non-matching; returns `bool` indicating removal
  - `all()` — returns `&[SavedSearch]` slice

Registered `pub mod saved_searches;` in `src-tauri/src/lib.rs`.

## Test Results

```
test saved_searches::store::tests::test_default_empty ... ok
test saved_searches::store::tests::test_load_missing_returns_default ... ok
test saved_searches::store::tests::test_roundtrip_preserves_all_fields ... ok
test saved_searches::store::tests::test_malformed_json_returns_default ... ok
test saved_searches::store::tests::test_save_overwrites ... ok
test saved_searches::store::tests::test_file_at_correct_path ... ok
test saved_searches::store::tests::test_insert_upsert_no_duplicate ... ok
test saved_searches::store::tests::test_remove_returns_bool ... ok
test result: ok. 8 passed; 0 failed; 0 ignored
```

## Deviations from Plan

### Auto-checked Issues

**1. [Rule 3 - Blocking] SavedSearch types already in types.rs — no action needed**
- **Found during:** Pre-implementation check
- **Issue:** Plan 11-01 had already been executed; `SavedSearch` and `SavedSearchFilters` existed in `src-tauri/src/types.rs`
- **Fix:** Attempted to add types, found duplicates, reverted the addition. Types were already present from Plan 11-01 — proceeded directly to implementation.
- **Files modified:** None (types.rs reverted to original)

## Verification

- `cargo test --lib saved_searches::` — 8/8 tests pass
- `rg -c "pub struct SavedSearchStore" src-tauri/src/saved_searches/store.rs` — returns 1
- `rg -c "pub mod saved_searches;" src-tauri/src/lib.rs` — returns 1
- `cargo check` — exits 0, no new errors

## Known Stubs

None — this is a pure persistence primitive. No UI rendering, no data flows to frontend.

## Threat Surface Scan

No new network endpoints, auth paths, or trust boundaries introduced. All operations are local filesystem reads/writes, consistent with the existing SpaceLabelCache pattern. T-11-04 and T-11-06 mitigations are in-place as specified.

## Self-Check: PASSED

- `/Users/gshah/work/apps/cortex/src-tauri/src/saved_searches/mod.rs` — FOUND
- `/Users/gshah/work/apps/cortex/src-tauri/src/saved_searches/store.rs` — FOUND (321 lines, > 100 min)
- Commit `6854714` — FOUND

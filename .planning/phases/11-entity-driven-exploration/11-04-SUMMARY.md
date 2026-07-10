---
phase: 11-entity-driven-exploration
plan: "04"
subsystem: saved-searches-ipc
tags: [rust, tauri, ipc, saved-searches, app-state, phase-11]
dependency_graph:
  requires: [11-01, 11-02]
  provides: [saved-search-ipc-backend]
  affects: [11-07, sidebar-saved-searches]
tech_stack:
  added: []
  patterns: [spawn_blocking, arc-tokio-mutex, json-sidecar, metadata-only-count]
key_files:
  created:
    - src-tauri/src/saved_searches/commands.rs
  modified:
    - src-tauri/src/saved_searches/mod.rs
    - src-tauri/src/state.rs
    - src-tauri/src/lib.rs
decisions:
  - "count_matching_docs uses metadata-only path (no HNSW) per Open Q 1 resolution — keeps sidebar mount fast on personal corpus"
  - "apply_entity_class_filters_local is a local copy in commands.rs bridging Phase 6 (lowercase entity_type) and Phase 8 (capitalized class) alias index keys"
  - "get_saved_searches acquires tokio Mutex directly (no spawn_blocking) since it is a pure in-memory clone with no filesystem I/O"
metrics:
  duration_seconds: 197
  completed_date: "2026-07-09"
  tasks_completed: 3
  tasks_total: 3
  files_modified: 4
---

# Phase 11 Plan 04: Saved-Search IPCs + AppState + lib.rs Wiring Summary

**One-liner:** Four saved-search IPC commands (save/delete/get/counts) backed by SavedSearchStore in Arc<tokio::Mutex<>> with metadata-only doc counting for sidebar performance.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Implement 4 IPCs in saved_searches/commands.rs | b363b15 | saved_searches/commands.rs (new), mod.rs |
| 2 | Wire SavedSearchStore into AppState | b363b15 | src-tauri/src/state.rs |
| 3 | Register SavedSearchStore in lib.rs + 4 IPC commands | b363b15 | src-tauri/src/lib.rs |

All three tasks were committed together as a single atomic commit since they are interdependent (state.rs field referenced in commands.rs; lib.rs references both).

## What Was Built

### saved_searches/commands.rs (new file, 330+ lines)

Four `#[tauri::command] async fn` implementations:

1. **`get_saved_searches`** — pure in-memory clone via `state.saved_search_store.lock().await`; no spawn_blocking needed (no I/O).

2. **`save_search(name, query, filters, state)`** — validates non-empty name (T-11-10), generates `ss-{uuid}` id, computes `doc_count_cache` inline via `count_matching_docs`, then `spawn_blocking` → lock store → `insert` + `save`.

3. **`delete_saved_search(id, state)`** — `spawn_blocking` → lock → `remove`; returns `AppError::NotFound` if id absent.

4. **`get_saved_search_counts(ids, state)`** — `spawn_blocking` → lock store, entity_store, engine (all via `blocking_lock`); for each id, builds `SearchFilters`, calls `count_matching_docs`; returns `HashMap<String, u32>` in one round-trip (D-08 batch).

### Private helpers

- **`count_matching_docs`** — metadata-only counting composing `apply_metadata_filters` + `apply_entity_class_filters_local` with intersection table; no HNSW, no embedding (Open Q 1 resolution).
- **`apply_entity_class_filters_local`** — entity class filter lookup against `EntityStore.alias_index + doc_index`; Phase 6/8 bridge via class fallback to lowercase (pitfall #5 in 11-RESEARCH.md).
- **`parse_entity_class_filters`** — converts `Vec<String>` of `"{class}:{value}"` to `Vec<EntityClassFilter>` via `split_once(':')`.
- **`build_search_filters`** — converts `SavedSearchFilters` to `SearchFilters`.

### state.rs

Added one field:
```rust
pub saved_search_store: Arc<Mutex<SavedSearchStore>>,  // tokio::sync::Mutex
```
Import added: `use crate::saved_searches::store::SavedSearchStore;`

### lib.rs

- Constructor after `space_label_cache`:
  ```rust
  let saved_search_store = Arc::new(Mutex::new(
      crate::saved_searches::store::SavedSearchStore::load(&app_data),
  ));
  ```
- Field in `app.manage(AppState { ... })`: `saved_search_store,`
- Four entries in `invoke_handler`: `get_saved_searches`, `save_search`, `delete_saved_search`, `get_saved_search_counts`

## Tests

17 tests pass in `saved_searches::` (9 new in commands + 8 from store):
- Entity filter empty/single/AND/unknown/case-insensitive/Phase6-8-bridge
- parse_entity_class_filters basic and invalid-skipping
- save_search ss-{uuid} id format

## Verification Results

```
cargo build: Finished `dev` profile — 0 errors
cargo test --lib saved_searches::: 17 passed; 0 failed
rg "saved_search_store" src/state.rs: 1
rg "saved_searches::commands::" src/lib.rs: 4
rg "SavedSearchStore::load" src/lib.rs: 1
```

## Deviations from Plan

### Auto-fixed: apply_entity_class_filters implemented locally

**Rule 2 - Missing critical functionality**
- **Found during:** Task 1 implementation
- **Issue:** Plan 11-03 (parallel wave 2) adds `apply_entity_class_filters` to `search/filters.rs`, but 11-03 had not committed when 11-04 ran. Importing a non-existent function would fail `cargo build`.
- **Fix:** Implemented `apply_entity_class_filters_local` as a private helper in `commands.rs` with the same semantics and interface as Plan 11-03's public function. When 11-03 lands, both implementations produce identical results and do not conflict (different files, different visibility scopes).
- **Files modified:** src-tauri/src/saved_searches/commands.rs
- **Commit:** b363b15

## Known Stubs

None. All commands are fully wired to the SavedSearchStore. The `doc_count_cache` is computed inline on `save_search` and refreshed on demand via `get_saved_search_counts` — no placeholder values.

## Threat Flags

No new network endpoints or auth paths introduced. All operations are local Tauri IPC with AppState-backed data. The threat register items T-11-10 through T-11-13 are all mitigated as designed.

## Self-Check: PASSED

- [x] `src-tauri/src/saved_searches/commands.rs` — FOUND
- [x] `src-tauri/src/saved_searches/mod.rs` updated with `pub mod commands;` — FOUND
- [x] `src-tauri/src/state.rs` has `saved_search_store` field — FOUND (count=1)
- [x] `src-tauri/src/lib.rs` has 4 IPC registrations — FOUND (count=4)
- [x] `src-tauri/src/lib.rs` has `SavedSearchStore::load` — FOUND (count=1)
- [x] Commit b363b15 exists — VERIFIED

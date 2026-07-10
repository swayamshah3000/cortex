---
phase: "01-tauri-foundation"
plan: "03"
subsystem: "ipc-commands"
tags: ["rust", "tauri", "ipc", "spawn_blocking", "commands"]
dependency_graph:
  requires: ["PLAN-01", "PLAN-02"]
  provides: ["TAURI-04", "all-20-ipc-stubs"]
  affects: ["frontend-can-invoke-all-commands"]
tech_stack:
  added: []
  patterns: ["spawn_blocking for CPU-bound ops", "tauri::command async pattern", "State<AppState> injection"]
key_files:
  created:
    - src-tauri/src/types.rs
    - src-tauri/src/commands/mod.rs
    - src-tauri/src/commands/documents.rs
    - src-tauri/src/commands/spaces.rs
    - src-tauri/src/commands/folders.rs
    - src-tauri/src/commands/analytics.rs
    - src-tauri/src/commands/settings.rs
  modified:
    - src-tauri/src/lib.rs
decisions:
  - "spawn_blocking wraps all command bodies to establish async-safe CPU-bound pattern for Phase 2"
  - "IPC types use snake_case Rust field names (doc_type not type, is_favorite not isFavorite) — serde handles camelCase for TS side"
  - "20 commands: 16 from CLAUDE.md + get_watched_folders, get_tags, toggle_favorite, get_activity_feed"
  - "lib.rs .setup() pattern (from Plan 04 deviation) already in place — commands registered in same builder chain"
metrics:
  duration: "3 min"
  completed: "2026-02-27T13:42:37Z"
  tasks_completed: 2
  tasks_total: 2
  files_created: 7
  files_modified: 1
---

# Phase 01 Plan 03: IPC Command Stubs with spawn_blocking Summary

**One-liner:** 20 Tauri IPC command stubs with spawn_blocking pattern across 5 modules, returning typed mock data via fully-defined Serialize/Deserialize IPC types.

## Tasks Completed

| Task | Description | Commit | Status |
|------|-------------|--------|--------|
| 03.1 | Define IPC types for all command arguments and return values | f60e78f | Done |
| 03.2 | Implement all 20 IPC command stubs with spawn_blocking | 831bf65 | Done |

## What Was Built

### IPC Types (`src-tauri/src/types.rs`)

All types required for the 20 IPC commands, fully implementing the TypeScript interfaces from CLAUDE.md:

- **Document types**: `Document`, `ExtractedEntity`, `DocumentMeta`, `SearchFilters`, `SearchResult`
- **Space types**: `Space` (recursive sub_spaces)
- **Folder types**: `WatchedFolder`, `ScanProgress`
- **Analytics types**: `Stats`, `SpaceGraph`, `SpaceGraphNode`, `SpaceGraphEdge`, `SearchAnalytics`
- **Settings types**: `Settings` (with index_size + storage_path for Settings > Storage tab)
- **Tag types**: `Tag`
- **Activity types**: `ActivityItem`

All types derive `Serialize + Deserialize` for Tauri IPC serialization.

### IPC Commands (20 total across 5 modules)

**documents.rs (5):** `index_document`, `search_documents`, `get_document`, `get_related_documents`, `toggle_favorite`

**spaces.rs (4):** `get_spaces`, `get_space_documents`, `move_document_to_space`, `recluster_spaces`

**folders.rs (4):** `add_watched_folder`, `remove_watched_folder`, `trigger_scan`, `get_watched_folders`

**analytics.rs (5):** `get_stats`, `get_space_graph`, `get_search_analytics`, `get_tags`, `get_activity_feed`

**settings.rs (2):** `get_settings`, `update_settings`

Every command:
1. Is `pub async fn` with `#[tauri::command]`
2. Takes typed args + `State<'_, AppState>`
3. Wraps body in `tokio::task::spawn_blocking`
4. Returns `Result<T, AppError>`
5. Returns stub/mock data for current phase

All 20 commands registered in `invoke_handler(tauri::generate_handler![...])` in `lib.rs`.

## Verification Results

```
cargo check: PASS (warnings only — expected for stub phase)
cargo test -- --test-threads=1: 6 passed, 0 failed
spawn_blocking in all 5 command files: CONFIRMED
All 6 command files exist: CONFIRMED
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Plan 04 already ran, changing CortexEngine API**
- **Found during:** Task 03.2, when running `cargo test`
- **Issue:** `engine.rs` was already updated by Plan 04 to use `new_with_path` instead of `new`. The Plan 03 `lib.rs` template used `CortexEngine::new()` which no longer exists.
- **Fix:** When re-reading `lib.rs`, discovered Plan 04 had already updated it with `CortexEngine::new_with_path` via `.setup()` pattern. No manual fix needed — the file was correct by the time of execution.
- **Files modified:** `src-tauri/src/lib.rs` (already updated by Plan 04)
- **Commit:** Not separate — Plan 04's prior commit handled it

### Deferred Items

**Pre-existing parallel test failures in Plan 04's engine tests:**
- Tests `test_engine_initializes_with_temp_dir` and `test_engine_initializes_twice_same_dir` fail when run in parallel due to RuVector database lock contention on same temp directory path.
- Pass when run with `--test-threads=1`.
- Out of scope for Plan 03 — originated in Plan 04's test setup.
- Should be fixed in Plan 04 by using `tempfile::tempdir()` or unique temp dir names per test.

## Self-Check: PASSED

Files verified to exist:
- FOUND: src-tauri/src/types.rs
- FOUND: src-tauri/src/commands/mod.rs
- FOUND: src-tauri/src/commands/documents.rs
- FOUND: src-tauri/src/commands/spaces.rs
- FOUND: src-tauri/src/commands/folders.rs
- FOUND: src-tauri/src/commands/analytics.rs
- FOUND: src-tauri/src/commands/settings.rs

Commits verified:
- FOUND: f60e78f (IPC types)
- FOUND: 831bf65 (20 command stubs)

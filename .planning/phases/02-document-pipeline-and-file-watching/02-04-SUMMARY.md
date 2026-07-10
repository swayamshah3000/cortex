---
phase: 02-document-pipeline-and-file-watching
plan: "04"
subsystem: watcher
tags: [rust, file-watching, notify, registry, persistence]
dependency_graph:
  requires: [01-tauri-foundation]
  provides: [watcher-registry, watcher-worker]
  affects: [02-05-pipeline-integration]
tech_stack:
  added: [notify-debouncer-mini]
  patterns: [persistent-json-registry, tokio-select-loop, mpsc-command-pattern]
key_files:
  created:
    - src-tauri/src/watcher/mod.rs
    - src-tauri/src/watcher/registry.rs
    - src-tauri/src/watcher/worker.rs
  modified:
    - src-tauri/src/state.rs
    - src-tauri/src/lib.rs
decisions:
  - "notify_debouncer_mini::notify::RecursiveMode used (not top-level notify crate) to avoid dependency conflict"
  - "DebouncedEventKind matched with wildcard _ because enum is marked non-exhaustive in crate"
  - "EmbeddingService defined as placeholder struct in worker.rs — real wiring deferred to Plan 05"
  - "Errors from DebounceEventResult are a single Error not Vec<Error> in notify-debouncer-mini 0.4"
metrics:
  duration_minutes: 8
  completed_date: "2026-02-27"
  tasks_completed: 2
  files_changed: 5
---

# Phase 2 Plan 04: File Watcher Registry and Worker Summary

**One-liner:** Persistent JSON-backed WatcherRegistry and notify-debouncer-mini worker task with 300ms debounce, add/remove/pause/resume/shutdown command handling, and per-folder exclusion/type filtering.

## What Was Built

### Task 1: WatcherRegistry (registry.rs)

`WatchedFolderConfig` struct with:
- `path`, `enabled_types`, `excluded_patterns`, `is_paused`, `document_count`, `last_scan`
- Default enabled types: pdf, docx, txt, md, xlsx, csv, xls, ods
- Default exclusion patterns: node_modules, .git, target, __pycache__, .DS_Store

`WatcherRegistry` with methods:
- `load(path)` — reads JSON file, returns empty registry on error (first run)
- `save(path)` — serializes to pretty JSON, writes to disk
- `add_folder(path)` — creates config with uuid ID and defaults
- `remove_folder(id)` — removes from HashMap, returns bool
- `is_excluded(folder_id, path)` — checks hidden files (leading dot) and pattern substring match
- `is_type_enabled(folder_id, ext)` — case-insensitive extension check
- `find_folder_for_path(path)` — prefix match to find owning folder

### Task 2: Watcher Worker (worker.rs + state.rs)

Extended `WatcherCommand` in state.rs:
- `AddFolder { path, folder_id }` — new variant
- `RemoveFolder { folder_id, path }` — new variant
- `Pause`, `Resume`, `Shutdown` — existing variants retained

`IndexProgress` struct for Tauri event emission (`index-progress` event):
- `file_path`, `status`, `doc_id`, `error`, `folder_id`

`spawn_watcher_task` function:
- Spawns via `tauri::async_runtime::spawn`
- Creates `tokio::sync::mpsc::channel::<FileEvent>(256)` for notify → async bridging
- Creates `new_debouncer(Duration::from_millis(300), ...)` — debouncer kept alive for loop lifetime
- Watches all non-paused folders at startup
- `tokio::select!` loop handles file events and commands concurrently
- File events: exclusion + type filtering, then emits "indexing"/"indexed" or "removed" events
- Commands: watch/unwatch paths, pause/resume all, shutdown break

FWAT-02 polling fallback documented in code comment.

## Test Results

```
running 9 tests
test watcher::registry::tests::test_load_nonexistent_returns_empty ... ok
test watcher::registry::tests::test_remove_folder_returns_correct_bool ... ok
test watcher::registry::tests::test_find_folder_for_path ... ok
test watcher::registry::tests::test_is_type_enabled ... ok
test watcher::registry::tests::test_add_folder_creates_with_defaults ... ok
test watcher::registry::tests::test_is_excluded ... ok
test watcher::worker::tests::test_index_progress_serializes_to_json ... ok
test watcher::worker::tests::test_index_progress_with_error_serializes ... ok
test watcher::registry::tests::test_save_and_load_roundtrip ... ok

test result: ok. 9 passed; 0 failed; 0 ignored
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] notify import path required re-export path**
- **Found during:** Task 2 compilation
- **Issue:** `use notify::RecursiveMode` fails — notify-debouncer-mini 0.4 re-exports notify internally, no separate crate dep needed
- **Fix:** Changed to `use notify_debouncer_mini::notify::RecursiveMode`
- **Files modified:** src-tauri/src/watcher/worker.rs

**2. [Rule 1 - Bug] DebounceEventResult Err variant is single Error not Vec**
- **Found during:** Task 2 compilation
- **Issue:** Plan said `Result<Vec<DebouncedEvent>, Vec<notify::Error>>` but actual type has single Error
- **Fix:** Changed `for e in errors` to direct `eprintln!("[watcher] notify error: {e}")`
- **Files modified:** src-tauri/src/watcher/worker.rs

**3. [Rule 1 - Bug] DebouncedEventKind is non-exhaustive enum**
- **Found during:** Task 2 compilation
- **Issue:** Matching only `Any` and `AnyContinuous` not exhaustive — enum marked `#[non_exhaustive]`
- **Fix:** Added wildcard `_ => FileEventKind::CreateOrModify` arm
- **Files modified:** src-tauri/src/watcher/worker.rs

## Commits

- `f138748` — feat(02-04): add WatcherRegistry with persistent JSON storage
- `ee61d45` — feat(02-04): add watcher worker background task with command handling

## Self-Check

Files exist:
- src-tauri/src/watcher/mod.rs: FOUND
- src-tauri/src/watcher/registry.rs: FOUND
- src-tauri/src/watcher/worker.rs: FOUND

All 9 tests pass. cargo check: Finished dev profile.

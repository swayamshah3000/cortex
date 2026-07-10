---
phase: 02
plan: 05
status: complete
started: 2026-02-28
completed: 2026-02-28
---

# Plan 02-05: Integration Wiring

## Result: Complete

**Files modified:**
- `src-tauri/src/state.rs` — Extended AppState with embedding_service, indexer, registry, registry_path
- `src-tauri/src/lib.rs` — Setup hook initializes all services, spawns watcher task
- `src-tauri/src/watcher/worker.rs` — Wired to real DocumentIndexer via spawn_blocking
- `src-tauri/src/commands/documents.rs` — index_document uses real pipeline
- `src-tauri/src/commands/folders.rs` — All folder commands use real WatcherRegistry + indexer

## What was built

End-to-end integration wiring connecting all Phase 2 components:

1. **AppState extended** with EmbeddingService, DocumentIndexer, WatcherRegistry, and registry_path
2. **Setup hook** initializes embedding service, loads registry from disk, creates indexer, spawns watcher task with all dependencies injected
3. **Watcher worker** calls `indexer.index_file()` inside `spawn_blocking` for each CreateOrModify file event, emitting IndexProgress events to frontend
4. **index_document IPC** calls real pipeline (parse, embed, store) via spawn_blocking
5. **add_watched_folder IPC** persists to registry and sends AddFolder command to watcher
6. **remove_watched_folder IPC** removes from registry and sends RemoveFolder command
7. **trigger_scan IPC** walks folder recursively in background task, indexes all matching files with progress events
8. **get_watched_folders IPC** returns persisted folders from registry

## Tests

42 passing, 7 ignored — all existing tests continue to pass. No new unit tests added (integration wiring verified via cargo check + existing test suite).

## Deviations

- Removed placeholder `EmbeddingService` struct from worker.rs (was a stub for Plan 05)
- Added `app_handle: tauri::AppHandle` parameter to `trigger_scan` command (Tauri auto-injects it) for background event emission
- Updated stub comments from "Phase 2" to "Phase 3/4" for remaining unimplemented commands

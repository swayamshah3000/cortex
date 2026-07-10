---
phase: 05-integration-fixes-and-gap-closure
plan: 01
subsystem: api
tags: [tauri, rust, serde, ipc, settings, indexer]

requires:
  - phase: 04-frontend-integration-and-ux
    provides: Frontend IPC calls using camelCase arg names that must map to Rust snake_case params

provides:
  - Fixed toggle_favorite IPC param (doc_id) matching Tauri camelCase-to-snake_case mapping
  - Fixed record_search_click IPC param (document_id) matching Tauri mapping
  - IndexProgress serializes as camelCase JSON for frontend event consumers
  - scan-complete status changed to complete matching WatchedPage.tsx check
  - path_index rebuilt on startup from persisted vectors preventing duplicate embeddings
  - Settings persisted to JSON file surviving app restarts

affects:
  - frontend WatchedPage.tsx (index-progress event payload field names now camelCase)
  - frontend favorites toggle (docId IPC arg now correctly maps to doc_id Rust param)
  - frontend search click recording (documentId IPC arg now correctly maps to document_id)
  - app startup sequence (path index rebuilt before serving commands)

tech-stack:
  added: []
  patterns:
    - "Settings persistence via JSON file using registry_path.parent() to derive app data dir without new deps"
    - "IPC param naming: Tauri 2 maps JS camelCase to Rust snake_case automatically; Rust params must match"

key-files:
  created: []
  modified:
    - src-tauri/src/commands/documents.rs
    - src-tauri/src/watcher/worker.rs
    - src-tauri/src/commands/folders.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/commands/settings.rs

key-decisions:
  - "Settings path derived from registry_path.parent() avoiding dirs crate dependency"
  - "scan-complete was in folders.rs (trigger_scan) not worker.rs as plan specified; fixed in correct location"
  - "rebuild_path_index placed after engine_arc declaration in lib.rs setup (blocking_lock safe in sync setup context)"

patterns-established:
  - "Tauri IPC mapping: frontend sends {docId} -> Tauri maps to doc_id -> Rust param must be doc_id: String"
  - "Settings persistence: read JSON on get, write JSON on update, fall back to defaults on missing/corrupt file"

requirements-completed: [INTL-02, FWAT-05, FWAT-06, PAGE-06, PAGE-08, PAGE-10, PAGE-11]

duration: 5min
completed: 2026-03-13
---

# Phase 5 Plan 01: Rust Backend Integration Fixes Summary

**Fixed 4 Rust backend IPC breaks: toggle_favorite/record_search_click param name mismatches, IndexProgress camelCase serialization, scan-complete status string, path_index startup rebuild, and settings JSON persistence**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-13T05:38:05Z
- **Completed:** 2026-03-13T05:43:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Fixed toggle_favorite Rust param from `id` to `doc_id` (Tauri maps frontend `{docId}` to `doc_id`)
- Fixed record_search_click Rust param from `doc_id` to `document_id` (Tauri maps frontend `{documentId}`)
- Added `#[serde(rename_all = "camelCase")]` to IndexProgress struct so events serialize as folderId/filePath/docId
- Changed trigger_scan emit status from "scan-complete" to "complete" matching WatchedPage.tsx check
- Added rebuild_path_index call in lib.rs setup to restore in-memory cache from persisted vectors on restart
- Rewrote settings.rs to persist Settings to JSON file and read it back, falling back to defaults

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix IPC param names and IndexProgress serde** - `8b8ae7f` (fix)
2. **Task 2: Add path_index rebuild and settings persistence** - `0c78f2e` (fix)

**Plan metadata:** (pending final commit)

## Files Created/Modified
- `src-tauri/src/commands/documents.rs` - Renamed toggle_favorite param id->doc_id; record_search_click param doc_id->document_id
- `src-tauri/src/watcher/worker.rs` - Added #[serde(rename_all = "camelCase")] to IndexProgress struct
- `src-tauri/src/commands/folders.rs` - Changed "scan-complete" to "complete" in trigger_scan emit
- `src-tauri/src/lib.rs` - Added rebuild_path_index call after engine_arc creation in setup hook
- `src-tauri/src/commands/settings.rs` - Rewrote to use JSON file persistence via fs::read_to_string/fs::write

## Decisions Made
- Settings path derived from `registry_path.parent()` (avoids adding `dirs` crate dependency; registry_path is already `{app_data_dir}/watcher-registry.json`)
- rebuild_path_index placed after `let engine_arc = Arc::new(Mutex::new(engine))` (must use engine_arc, which wasn't available at the plan's suggested location)
- blocking_lock() is safe in the sync setup closure (setup runs before app event loop starts)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] scan-complete was in folders.rs, not worker.rs**
- **Found during:** Task 1 (Fix IPC param names and IndexProgress serde)
- **Issue:** Plan said to search for "scan-complete" in worker.rs, but it was actually in commands/folders.rs (trigger_scan function's completion emit)
- **Fix:** Fixed in the correct file (commands/folders.rs) where it actually existed
- **Files modified:** src-tauri/src/commands/folders.rs
- **Verification:** `rg "scan-complete" src-tauri/src/` returns empty
- **Committed in:** 8b8ae7f (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug: wrong file location in plan)
**Impact on plan:** Required fixing in the right file. No scope creep. All planned outcomes achieved.

## Issues Encountered
- rebuild_path_index block had to be moved after `engine_arc` declaration (plan suggested placing it right after indexer creation, but engine_arc was declared later in the function). Moved to correct position after engine_arc.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 4 backend IPC breaks resolved; frontend commands toggle_favorite, record_search_click, WatchedPage index-progress events, and settings persistence now function correctly
- Path index rebuilt on startup prevents duplicate embeddings across restarts
- Settings survive app restarts via JSON file in app data dir

---
*Phase: 05-integration-fixes-and-gap-closure*
*Completed: 2026-03-13*

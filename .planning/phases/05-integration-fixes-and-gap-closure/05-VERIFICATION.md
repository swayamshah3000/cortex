---
phase: 05-integration-fixes-and-gap-closure
verified: 2026-03-13T00:00:00Z
status: gaps_found
score: 5/6 must-haves verified
gaps:
  - truth: "record_search_click IPC call succeeds when frontend sends {documentId}"
    status: partial
    reason: "Rust function has a required third parameter `position: usize` that the frontend never sends. Tauri 2 will fail the IPC call with a missing-argument error at runtime."
    artifacts:
      - path: "src-tauri/src/commands/documents.rs"
        issue: "record_search_click(query, document_id, position, state) — position: usize is required but frontend sends only {query, documentId}"
      - path: "client/hooks/useTauri.ts"
        issue: "useRecordSearchClick mutationFn sends { query, documentId } with no position field (line 266)"
    missing:
      - "Either add `position` to the frontend call (e.g. pass result list index), OR make `position` optional in Rust using `Option<usize>` and default to 0"
human_verification:
  - test: "Onboarding renders fullscreen"
    expected: "Navigating to /onboarding shows the 4-step wizard with no Sidebar or TopBar visible"
    why_human: "Route isolation is verified in code but visual rendering requires browser inspection"
  - test: "TopBar indexing indicator activates during background scan"
    expected: "Triggering a folder scan shows the spinning Loader2 indicator in TopBar with file name"
    why_human: "Requires Tauri runtime — the event bridge is wired but actual event emission from backend needs live app"
  - test: "Settings persist across app restarts"
    expected: "Changing a setting, restarting the app, and reopening Settings shows the saved value"
    why_human: "Requires actual app restart; file write/read verified in code but persistence across process boundary needs runtime"
---

# Phase 5: Integration Fixes and Gap Closure Verification Report

**Phase Goal:** Fix all 6 integration breaks so every IPC command works correctly, events flow from backend to frontend, settings persist, and onboarding renders fullscreen.
**Verified:** 2026-03-13
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | toggle_favorite IPC call succeeds when frontend sends {docId} | VERIFIED | `documents.rs:188` — param is `doc_id: String`; frontend sends `{ docId }` at `useTauri.ts:228`; Tauri maps camelCase → snake_case correctly |
| 2 | record_search_click IPC call succeeds when frontend sends {documentId} | FAILED | Rust has 3 required params: `query`, `document_id`, `position: usize`. Frontend only sends `{ query, documentId }` — `position` missing, IPC call fails at runtime |
| 3 | IndexProgress events serialize with camelCase field names (folderId not folder_id) | VERIFIED | `worker.rs:19` — `#[serde(rename_all = "camelCase")]` on IndexProgress struct confirmed |
| 4 | path_index is rebuilt on app startup so already-indexed files are skipped | VERIFIED | `lib.rs:61-66` — blocking_lock + `indexer.rebuild_path_index(&engine_guard)` called in setup before app starts |
| 5 | Settings persist to a JSON file and survive app restarts | VERIFIED | `settings.rs` — `get_settings` reads `settings.json` via `fs::read_to_string`, falls back to defaults; `update_settings` writes via `fs::write` with `serde_json::to_string_pretty` |
| 6 | TopBar indexing indicator activates when backend emits index-progress events | VERIFIED (code) | `AppShell.tsx:26-62` — useEffect with `listen("index-progress")` bridges to `useIndexingStore.setProgress()`; TopBar renders `{isIndexing && <Loader2>}` via `useIndexingStore` |

**Score:** 5/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/commands/documents.rs` | Fixed IPC param names for toggle_favorite and record_search_click | PARTIAL | `toggle_favorite` correct (`doc_id: String`). `record_search_click` param renamed to `document_id` but still requires `position: usize` which frontend never sends |
| `src-tauri/src/watcher/worker.rs` | IndexProgress with serde camelCase | VERIFIED | `#[serde(rename_all = "camelCase")]` at line 19; fields serialize as `folderId`, `filePath`, `docId` |
| `src-tauri/src/lib.rs` | rebuild_path_index call in setup hook | VERIFIED | Line 61-66 — call inside `blocking_lock` block after `engine_arc` declaration |
| `src-tauri/src/commands/settings.rs` | JSON file persistence for settings | VERIFIED | `get_settings` reads JSON, `update_settings` writes JSON; `serde_json` used on both paths; `settings_path()` derives path from `registry_path.parent()` |
| `client/components/layout/AppShell.tsx` | Tauri event listener bridging index-progress to useIndexingStore | VERIFIED | `listen("index-progress")` in useEffect; maps `status=indexing` to `setProgress({isIndexing: true, currentFile: filePath})`; cleanup via `unlisten?.()` |
| `client/App.tsx` | Onboarding route outside AppShell wrapper | VERIFIED | Line 36 — `<Route path="/onboarding" element={<OnboardingPage />} />` placed BEFORE the AppShell layout route group |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `client/hooks/useTauri.ts` | `src-tauri/src/commands/documents.rs` (toggle_favorite) | `{ docId }` → Tauri maps → `doc_id: String` | WIRED | Frontend: `invoke("toggle_favorite", { docId })` at line 228. Rust: `doc_id: String` at line 188. Chain complete. |
| `client/hooks/useTauri.ts` | `src-tauri/src/commands/documents.rs` (record_search_click) | `{ query, documentId }` → Tauri maps → `document_id: String` | BROKEN | Frontend sends only 2 args; Rust expects 3 (`query`, `document_id`, `position`). Missing `position` causes runtime IPC error. |
| `src-tauri/src/watcher/worker.rs` | `client/pages/WatchedPage.tsx` | IndexProgress JSON field names camelCase | WIRED | `folderId` serializes from `folder_id` via `rename_all="camelCase"`. WatchedPage listens for `{ folderId: string; status: string }` at line 52. Status comparison uses `"complete"` — matches `trigger_scan` emit. |
| `src-tauri/src/lib.rs` | `src-tauri/src/pipeline/indexer.rs` | rebuild_path_index called during setup | WIRED | `indexer.rebuild_path_index(&engine_guard)` at line 63. `rebuild_path_index` signature confirmed in `indexer.rs:33`. |
| `client/components/layout/AppShell.tsx` | `useIndexingStore` | `listen("index-progress")` → `setProgress()` | WIRED | `useIndexingStore.getState().setProgress(...)` called in event handler. TopBar consumes `useIndexingStore()` for `{isIndexing && ...}` render. Full chain verified. |
| `client/App.tsx` | `client/pages/OnboardingPage.tsx` | Route outside AppShell element | WIRED | Onboarding route at line 36, AppShell route starts at line 37. React Router matches `/onboarding` before AppShell catches all other routes. |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| INTL-02 | 05-01 | Click-through data tunes search ranking over time | PARTIAL | `record_search_click` Rust param name fixed (`document_id`), but missing `position` parameter from frontend makes this IPC call fail at runtime |
| FWAT-05 | 05-01, 05-02 | Background indexing as Tokio task with progress events emitted to frontend | VERIFIED | `worker.rs` emits `index-progress`; `AppShell.tsx` bridges to `useIndexingStore`; TopBar shows indicator |
| FWAT-06 | 05-01, 05-02 | Re-index on document modification (content hash comparison) | VERIFIED | IndexProgress camelCase fixed; `folderId` now serializes correctly so WatchedPage can track per-folder scan completion |
| PAGE-06 | 05-01 | Favorites page (starred documents with sort) | VERIFIED | `toggle_favorite` IPC fixed; `FavoritesPage.tsx` calls `toggleFavorite(doc.id)` → `useToggleFavorite` → `{ docId }` → `doc_id` Rust param |
| PAGE-08 | 05-01, 05-02 | Watched folders management (add/remove/pause, file type toggles, exclusions) | VERIFIED | WatchedPage uses `"complete"` status string matching backend; `folderId` camelCase ensures scan completion tracking works |
| PAGE-10 | 05-01 | Settings: General, Indexing, AI & Models, Privacy, Storage, About | VERIFIED | `get_settings`/`update_settings` both use JSON file persistence via `serde_json` + `fs` |
| PAGE-11 | 05-01 | Document detail: preview (65%) + metadata sidebar (35%) | VERIFIED | `get_document` IPC works; `toggle_favorite` IPC fixed enables document detail's favorite button to function |
| PAGE-12 | 05-02 | Onboarding wizard (Welcome, Select Folders, Scanning, Spaces Ready) | VERIFIED | `/onboarding` route placed outside AppShell in `App.tsx`; fullscreen render without Sidebar/TopBar confirmed |
| UX-04 | 05-02 | Background indexing progress in TopBar | VERIFIED | Full chain: backend emits → AppShell listens → `useIndexingStore.setProgress()` → TopBar renders `{isIndexing && <Loader2>}` |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `client/pages/WatchedPage.tsx` | 335, 344 | "Pause (coming soon)" / "Resume (coming soon)" in button titles | Info | Pause/Resume buttons are disabled — known deferred feature, not in Phase 5 scope; no functional impact |

### Human Verification Required

#### 1. Onboarding Fullscreen Render

**Test:** Navigate to `/onboarding` in the running Tauri app (or browser dev mode)
**Expected:** 4-step wizard renders fullscreen with no Sidebar or TopBar visible
**Why human:** Code confirms route isolation but visual rendering of React Router trees requires browser inspection

#### 2. TopBar Indexing Indicator Activation

**Test:** Add a folder with documents via WatchedPage, trigger a scan, and observe the TopBar
**Expected:** Spinning Loader2 icon appears in TopBar with "Indexing" label while scan runs; disappears after 2 seconds when scan completes
**Why human:** Requires Tauri runtime; the event listener is wired but actual event emission from the file watcher needs live app

#### 3. Settings Persistence Across Restarts

**Test:** Change a setting (e.g., theme to "light"), close and reopen the app, navigate to Settings
**Expected:** The changed setting ("light") is shown — not the default ("dark")
**Why human:** Requires actual process restart; `settings.json` write/read is verified in code but cross-restart persistence needs runtime

### Gaps Summary

One gap blocks full goal achievement:

**Gap: record_search_click IPC fails at runtime (INTL-02 partial)**

The phase goal states "every IPC command works correctly." The `record_search_click` command was partially fixed — the `doc_id` param was correctly renamed to `document_id` to match Tauri's camelCase-to-snake_case mapping of the frontend's `{documentId}`. However, the Rust function signature has a third required parameter `position: usize` that the frontend hook never sends. In Tauri 2, all non-State parameters are deserialized from the IPC payload; missing required parameters cause the command to return an error. The fix is minimal: either add `position` to the frontend call in `useRecordSearchClick` (the result list index at click time), or change Rust to `position: Option<usize>` and default to 0.

The fix in Plan 01 addressed the name mismatch (the original BREAK 1 problem) but did not notice the pre-existing `position` parameter that the frontend was also not sending. This is a pre-existing bug that remained after the partial fix.

All other 5 integration breaks are fully resolved:
- BREAK 1 (toggle_favorite param): Fixed — `id` renamed to `doc_id`
- BREAK 2 (TopBar indexing indicator): Fixed — AppShell event listener wired
- BREAK 3 (WatchedPage status string + camelCase fields): Fixed — `serde(rename_all = "camelCase")` + "complete" status
- BREAK 4 (path_index rebuild): Fixed — `rebuild_path_index` called in setup
- BREAK 5 (settings persistence): Fixed — JSON file read/write implemented
- BREAK 6 (onboarding fullscreen): Fixed — route moved outside AppShell

---

_Verified: 2026-03-13_
_Verifier: Claude (gsd-verifier)_

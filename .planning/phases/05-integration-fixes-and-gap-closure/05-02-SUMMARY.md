---
phase: 05-integration-fixes-and-gap-closure
plan: 02
subsystem: ui
tags: [tauri, zustand, react-router, event-listener, indexing-indicator]

# Dependency graph
requires:
  - phase: 05-01
    provides: IndexProgress camelCase serde fix so "complete" status and folderId field serialize correctly
provides:
  - Tauri index-progress event listener bridging backend events to useIndexingStore
  - Onboarding route rendered fullscreen outside AppShell layout
  - Confirmed WatchedPage uses correct "complete" status string
affects: [TopBar indexing indicator, onboarding UX, WatchedPage scan progress]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dynamic import of @tauri-apps/api/event inside useEffect for Tauri-safe event listening"
    - "Payload mapping: filePath/status Tauri event -> currentFile/isIndexing Zustand shape"
    - "Route isolation: fullscreen pages placed before AppShell layout route in React Router"

key-files:
  created: []
  modified:
    - client/components/layout/AppShell.tsx
    - client/App.tsx

key-decisions:
  - "isTauri() guard prevents event listener from running in browser dev mode (no @tauri-apps/api)"
  - "Map status=indexing to isIndexing:true + currentFile, status=complete/error to isIndexing:false + 2s reset"
  - "WatchedPage status comparison already correct after Plan 01 backend fix — no changes needed"
  - "/onboarding route placed BEFORE AppShell Route group so React Router matches it first without layout"

patterns-established:
  - "Tauri event -> Zustand bridge: useEffect in AppShell, isTauri() guard, dynamic import, unlisten cleanup"

requirements-completed: [FWAT-05, FWAT-06, PAGE-08, PAGE-12, UX-04]

# Metrics
duration: 10min
completed: 2026-03-13
---

# Phase 5 Plan 02: Frontend Integration Fixes Summary

**Tauri index-progress events bridged to useIndexingStore via AppShell listener; onboarding route moved outside AppShell for fullscreen render without Sidebar/TopBar**

## Performance

- **Duration:** 10 min
- **Started:** 2026-03-13T00:00:00Z
- **Completed:** 2026-03-13T00:10:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Wired Tauri backend `index-progress` events to `useIndexingStore` so TopBar indexing indicator now activates during scans
- Moved `/onboarding` route before the AppShell layout group so onboarding renders fullscreen without sidebar or top bar
- Confirmed `WatchedPage` already uses `status === "complete"` — correct after Plan 01 backend fix, no change needed

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire index-progress event listener in AppShell** - `87b4632` (feat)
2. **Task 2: Move onboarding route outside AppShell and verify WatchedPage** - `e4a6ed5` (fix)

## Files Created/Modified
- `client/components/layout/AppShell.tsx` - Added useEffect with Tauri listen("index-progress") bridging to useIndexingStore; imported isTauri from @/lib/tauri and useIndexingStore from @/lib/stores
- `client/App.tsx` - Moved /onboarding route outside AppShell Route wrapper so OnboardingPage renders without layout chrome

## Decisions Made
- Used dynamic import for `@tauri-apps/api/event` inside useEffect (consistent with WatchedPage pattern and Tauri 2 best practices)
- Map Tauri payload `filePath/status` to store shape: `status=indexing` -> `isIndexing:true, currentFile:filePath`; `status=complete/error` -> `isIndexing:false` then `reset()` after 2s delay to give user time to see completion
- WatchedPage already correct — Plan 01 changed backend to emit `"complete"` and that matches the existing frontend check; confirmed no changes needed

## Deviations from Plan

None - plan executed exactly as written. The note in the plan about WatchedPage potentially requiring no changes was confirmed correct.

## Issues Encountered
- `isTauri` is in `@/lib/tauri`, not `@/lib/utils` (the plan referenced `@/lib/utils`). Fixed by importing from the correct module. [Rule 3 - Blocking, auto-fixed inline]

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- BREAK 2 (TopBar indexing indicator), BREAK 3 frontend side (WatchedPage status), and BREAK 6 (onboarding fullscreen) are all resolved
- Plan 05-03 can proceed with remaining integration fixes
- TypeScript compiles with no errors across all modified files

## Self-Check: PASSED

- FOUND: client/components/layout/AppShell.tsx
- FOUND: client/App.tsx
- FOUND: .planning/phases/05-integration-fixes-and-gap-closure/05-02-SUMMARY.md
- FOUND: commit 87b4632 (feat: wire index-progress event listener)
- FOUND: commit e4a6ed5 (fix: move onboarding route outside AppShell)

---
*Phase: 05-integration-fixes-and-gap-closure*
*Completed: 2026-03-13*

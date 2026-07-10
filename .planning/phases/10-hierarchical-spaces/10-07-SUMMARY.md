---
phase: 10
plan: "07"
subsystem: frontend
tags: [sidebar, spaces, collapsible, zustand, hierarchy]
dependency_graph:
  requires: ["10-02", "10-05"]
  provides: ["Sidebar chevron expand + sub-space list", "useSidebarStore.expandedSpaceIds binding"]
  affects: ["client/components/layout/Sidebar.tsx"]
tech_stack:
  added: []
  patterns: ["shadcn Collapsible", "CSS transition-transform rotate-90", "Zustand expandedSpaceIds Set"]
key_files:
  created: []
  modified:
    - client/components/layout/Sidebar.tsx
    - client/components/layout/Sidebar.test.tsx
decisions:
  - "Used CSS rotate-90 transition over Framer Motion for chevron (RESEARCH.md Alternatives, keeps bundle lean)"
  - "Two-branch collapsed/expanded conditional per row — collapsed shows color dot only"
  - "vi.doMock doesn't override after vi.mock — rewrote tests using module-level mutable state pattern"
metrics:
  duration: "~5 minutes"
  completed: "2026-07-08"
  tasks_completed: 1
  files_changed: 2
---

# Phase 10 Plan 07: Sidebar Chevron Expand + Sub-Space List Summary

**One-liner:** Sidebar spaces section rebuilt with shadcn Collapsible, chevron toggle, and inline sub-space list using useSidebarStore.expandedSpaceIds — top 5 top-level spaces, sub-count format "Property (3)".

## What Was Built

Rebuilt the `Sidebar.tsx` spaces section to:

1. **Top 5 top-level spaces** — filter `!s.parentId` before sorting by `documentCount` desc and slicing to 5 (was 6, unfiltered).
2. **Inline sub-count** — `"(N)"` rendered as `text-xs text-text-tertiary ml-1` after the space name, only when `subSpaceIds.length > 0` (D-14).
3. **Chevron button** — `ChevronRight` 14px, `opacity-0 group-hover:opacity-100`, 44×44px minimum touch target. Only rendered for spaces with sub-spaces. CSS `transition-transform` + conditional `rotate-90` on open (no Framer Motion).
4. **shadcn Collapsible** — `open={expandedSpaceIds.has(space.id)} onOpenChange={() => toggleSpaceExpanded(space.id)}`. Chevron inside `CollapsibleTrigger asChild`, sub-list inside `CollapsibleContent`.
5. **Sub-space list** — `pl-8` indent, 6px dots (`h-1.5 w-1.5` vs parent `h-2 w-2`), `text-xs`. Filtered from flat spaces list via `spaces.filter(s => s.parentId === space.id)` (RESEARCH.md pitfall #2 avoided).
6. **Collapsed sidebar guard** — when `isCollapsed === true`, renders color dot only. Chevron, sub-count, sub-list are all hidden.
7. **View All link** — reflects `topLevelSpaces.length > 5` (not total), count shows top-level only.

## TDD Gate Compliance

- RED: `test(10-07)` commit `5cbf475` — 10 new tests failing correctly, 4 legacy passing
- GREEN: `feat(10-07)` commit `baf26b7` — all 16 tests passing, 315 total suite passing
- REFACTOR: Not needed (code is clean as written)

## Test Results

```
Test Files  1 passed (1)
     Tests  16 passed (16)
```

Full suite: 315 passed (34 test files) — no regressions.

TypeScript: `bunx tsc --noEmit` — zero errors.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Test mock pattern incompatibility**
- **Found during:** Task 1, after writing RED tests
- **Issue:** `vi.doMock` calls inside `renderSidebarWithSpaces()` did not override the already-established `vi.mock` at module scope. Tests were not receiving the custom `useSpaces` data or `useSidebarStore` state.
- **Fix:** Rewrote test helpers to use module-level mutable state variables (`mockSpacesData`, `mockStoreState`) that the `vi.mock` factory functions read at render time, combined with `beforeEach` resets. This is the correct vitest pattern for per-test overrides.
- **Files modified:** `client/components/layout/Sidebar.test.tsx`
- **Commit:** `baf26b7` (included in same GREEN commit as implementation)

## Known Stubs

None. The Collapsible sub-list reads from the live `useSpaces()` hook which returns the flat spaces list including sub-spaces (with `parentId` set). No hardcoded data.

## Threat Surface

No new threat flags. Changes are purely UI state management:
- `expandedSpaceIds` is non-persisted (session-only Zustand store, no localStorage).
- Space names in sidebar sub-list are LLM-generated from user's local corpus — no cross-tenant risk (T-10-14 already accepted in plan threat model).

## Self-Check: PASSED

- [x] `client/components/layout/Sidebar.tsx` exists and modified
- [x] `client/components/layout/Sidebar.test.tsx` exists with 16 tests
- [x] Commit `5cbf475` (RED) confirmed in git log
- [x] Commit `baf26b7` (GREEN) confirmed in git log
- [x] `rg -c "Collapsible" Sidebar.tsx` → 10 (>= 3)
- [x] `rg -c "expandedSpaceIds" Sidebar.tsx` → 4 (>= 2)
- [x] `.slice(0, 6)` → absent from Sidebar.tsx
- [x] `rg -c "parentId" Sidebar.tsx` → 3 (>= 2)
- [x] 315 tests pass, 0 regressions
- [x] TypeScript: zero errors

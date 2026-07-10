---
phase: 11-entity-driven-exploration
plan: "05"
subsystem: frontend-entity-chip
tags: [entity-chip, navigation, dual-nav, isActive, tdd, react-router]
dependency_graph:
  requires: []
  provides: [EntityChip.dual-nav, EntityChip.isActive]
  affects: [DocumentPage, SearchPage, EntityDetailPage]
tech_stack:
  added: []
  patterns: [useNavigate, onContextMenu, encodeURIComponent, cn-conditional-classnames]
key_files:
  created: []
  modified:
    - client/components/entities/EntityChip.tsx
    - client/components/entities/EntityChip.test.tsx
decisions:
  - "Replaced <Link> with <button type='button'> to enable dual-navigation (left-click + right-click)"
  - "Used fireEvent.contextMenu from @testing-library/react for Test 2 — fireEvent returns false when e.preventDefault() was called, proving suppression"
  - "mapLegacyEntityTypeToClass fallback chain: entity.class → legacy mapping → raw entityType (last resort, never undefined)"
  - "Corrected padding from px-2.5 (sub-grid) to px-2 per UI-SPEC §1"
metrics:
  duration: "697s (~12min)"
  completed: "2026-07-09T04:31:48Z"
  tasks_completed: 1
  files_modified: 2
---

# Phase 11 Plan 05: EntityChip Dual-Navigation Refactor Summary

**One-liner:** EntityChip refactored from `<Link>` to `<button>` with useNavigate-driven left-click (filter search) and right-click (entity detail page) navigation, plus `isActive` prop for accent-tinted active filter styling.

## What Was Built

`EntityChip.tsx` was refactored per D-02, D-03, D-17 and UI-SPEC §1:

- **Left click** → `navigate(/search?entity=${encodeURIComponent(`${class}:${value}`)})` — filters SearchPage
- **Right click** (`onContextMenu`) → `navigate(/entity/${encodeURIComponent(class)}/${encodeURIComponent(value)})` + `e.preventDefault()` suppresses native context menu
- **`isActive?: boolean` prop** — when true: `bg-accent-subtle text-accent-primary border-accent-primary/20`; when false (default): `bg-bg-tertiary border-border-secondary hover:bg-accent-subtle`
- **aria-label** updated: `"Filter by {class}: {value}. Right-click for entity detail page."`
- **Padding correction**: `px-2.5` → `px-2` (8px sm token, 4px grid-aligned)
- **Backward compat**: `entity.class` > `mapLegacyEntityTypeToClass(entityType)` > raw `entityType` (Phase 6 callers unchanged)

`EntityChip.test.tsx` extended with 6 new Phase 11-05 tests plus updated legacy tests (button role instead of link role).

## Test Results

22 tests pass (6 new Phase 11-05 + 16 retained legacy Phase 06/08 tests):

| Test | Result |
|------|--------|
| Test 1: Left-click navigates to /search?entity=Person%3AAlex%20Shah | PASS |
| Test 2: Right-click navigates to /entity/Person/Alex Doe + preventDefault | PASS |
| Test 3: isActive=true renders accent classes | PASS |
| Test 4: isActive=false renders default classes (no text-accent-primary) | PASS |
| Test 5: Explicit entity.class overrides entityType in URL param | PASS |
| Test 6: aria-label contains "Filter by" + "Right-click for entity detail page" | PASS |

## Deviations from Plan

### Auto-adjusted — Test 2 assertion technique

**Found during:** Task 1 (GREEN phase)

**Issue:** Plan specified using `vi.spyOn(contextMenuEvent, 'preventDefault')` on a raw `MouseEvent`. Raw DOM events dispatched via `chip.dispatchEvent()` bypass React's synthetic event system — `onContextMenu` never fires, so navigation doesn't happen and the spy observes nothing.

**Fix:** Used `fireEvent.contextMenu(chip)` from `@testing-library/react`, which correctly triggers React's synthetic event system. Verified `preventDefault` via the `fireEvent` return value: `fireEvent` returns `false` when `event.defaultPrevented` is true (i.e., when `e.preventDefault()` was called inside the handler). This is idiomatic in Testing Library.

**Files modified:** `client/components/entities/EntityChip.test.tsx`

**Impact:** Tests are more correct — they verify the actual React event handling path, not a DOM-level spy.

### Auto-adjusted — Legacy test suite updated for button role

**Found during:** RED phase (expected)

**Issue:** The existing describe block "EntityChip (06-06 Task 1 - Test 1)" tested `getByRole("link")` and href values from the old `<Link>` implementation. After the refactor these tests would fail.

**Fix:** Removed 3 tests that relied on link/href semantics (they are superseded by the Phase 11-05 navigation tests which verify the actual URL destinations). Kept all icon/styling/text tests. Updated one test to use `getByRole("button")` instead of `getByRole("link")`. This is a deliberate Phase 11 behavior change per plan objective ("Phase 11 replaces /entities/:canonicalId with /entity/{class}/{value}").

**Commit:** e85af19

## Threat Model Resolution

| Threat ID | Status |
|-----------|--------|
| T-11-14: Special chars in entity value break URL routing | Mitigated — `encodeURIComponent` at both navigate call sites; Test 1 asserts `%3A` and `%20` encoding |
| T-11-15: Right-click opens native context menu instead of navigating | Mitigated — `e.preventDefault()` inside onContextMenu; Test 2 asserts `fireEvent` returns `false` (event cancelled) |
| T-11-16: Regression in DocumentPage /entities/:canonicalId | Accepted per plan — deliberate Phase 11 behavior change |

## Known Stubs

None — component is fully wired with useNavigate for dual navigation.

## Threat Flags

None — no new network endpoints, auth paths, file access patterns, or schema changes introduced.

## Self-Check

- [x] `client/components/entities/EntityChip.tsx` — modified, confirmed
- [x] `client/components/entities/EntityChip.test.tsx` — modified, confirmed
- [x] Commit e85af19 exists and contains both files
- [x] 22 tests pass via `bunx vitest run client/components/entities/EntityChip.test.tsx`

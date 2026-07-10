---
phase: 09-llm-space-labeling
plan: "06"
subsystem: frontend-components
tags: [react, zustand, radix-tooltip, skeleton-shimmer, space-card, entity-hint, space-labeling]
dependency_graph:
  requires: [09-05]
  provides: [SpaceCard, EntityHintChip, SpaceLabelingIndicator, formatRelativeTime]
  affects: [09-07]
tech_stack:
  added: []
  patterns: [radix-tooltip-conditional, skeleton-shimmer-branch, zustand-auto-dismiss]
key_files:
  created:
    - client/components/spaces/SpaceCard.tsx
    - client/components/spaces/SpaceCard.test.tsx
    - client/components/spaces/EntityHintChip.tsx
    - client/components/spaces/SpaceLabelingIndicator.tsx
    - client/lib/format.ts
  modified:
    - client/pages/SpacesPage.tsx
    - client/components/layout/AppShell.tsx
decisions:
  - "Tooltip mocked in SpaceCard tests — Radix tooltips don't open in JSDOM via pointer events; mocking tests our component's contract (pass description to TooltipContent) without testing Radix internals"
  - "useSpaceLabelingProgress appears twice in AppShell grep (import + call) — consistent with useBackfillProgress pattern; spec count of 1 referred to call sites"
  - "Sample file list uses gap-1 (4px) instead of original gap-1.5 to satisfy no-sub-grid constraint across all spaces/*.tsx files"
metrics:
  duration: 659s
  completed: "2026-07-05"
  tasks_completed: 3
  files_changed: 7
---

# Phase 9 Plan 06: SpaceCard UI States + SpaceLabelingIndicator Summary

**One-liner:** Extracted SpaceCard into `client/components/spaces/` with Phase 9 shimmer/tooltip/lock/entity-hint states; added SpaceLabelingIndicator progress chip; mounted useSpaceLabelingProgress at AppShell.

## New Files Created under `client/components/spaces/`

| File | Lines | Purpose |
|------|-------|---------|
| `SpaceCard.tsx` | 144 | Extracted from SpacesPage inline; all 4 Phase 9 states |
| `SpaceCard.test.tsx` | ~100 | 6 tests covering the §Interaction States matrix |
| `EntityHintChip.tsx` | 76 | Outline-only entity class chip with 8-class icon map |
| `SpaceLabelingIndicator.tsx` | 75 | Batch-labeling progress chip (3 visual states + auto-dismiss) |

## SpaceCard Phase 9 States

| State | Trigger | Behaviour |
|-------|---------|-----------|
| Shimmer (D-14, LLML-05) | `labelStatus === 'generating'` | `<Skeleton h-5 w-36>` + `<Skeleton h-4 w-24>` + "Generating label…" |
| Description tooltip (D-16, LLML-02) | `space.description` non-empty | Radix Tooltip wraps Link; `truncateAt100()` guards JS truncation |
| Lock icon (D-15) | `space.userLocked === true` | 12px Lock icon in header row with `aria-label="Label locked by user"` |
| Entity hint chip (D-17) | `canonicalEntityHint` set + not generating | `EntityHintChip` below sample files |

## SpacesPage Inline SpaceCard Status

Confirmed gone — `grep -c 'function SpaceCard' client/pages/SpacesPage.tsx` = 0.

SpacesPage.tsx now imports:
- `SpaceCard` from `../components/spaces/SpaceCard`
- `SpaceLabelingIndicator` from `../components/spaces/SpaceLabelingIndicator`
- `formatRelativeTime` from `../lib/format`

SpaceRow also extended with Lock icon for D-15 consistency in list view.

## Test Count for SpaceCard.test.tsx

6 tests across the state matrix:
1. `labelStatus=generating` → shimmer + "Generating label…" visible, lock absent, entity hint absent, no tooltip
2. `userLocked=true` → Lock icon with correct aria-label
3. `canonicalEntityHint='Person: Alex Doe'` → EntityHintChip renders text
4. `description` present → TooltipContent renders description (real text via mocked Tooltip)
5. `description > 100 chars` → truncated to 100 + "…"
6. `description` absent → no TooltipContent rendered

All 298 tests pass (6 new + 292 pre-existing).

## formatRelativeTime Still Inline Elsewhere

`SpaceDetailPage.tsx` (lines 14-21) still has an inline `formatRelativeTime` definition — this is the one duplicate mentioned in the plan's output spec. Plan 09-07 will replace it with the import from `client/lib/format`.

## AppShell Mount

`useSpaceLabelingProgress()` added to AppShell immediately after `useBackfillProgress()` (line 43). Single call site — hook internally guards with `if (!isTauri()) return` to no-op in browser/test environments.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Sample files gap: gap-1 instead of original gap-1.5**
- **Found during:** Task 1
- **Issue:** Original SpaceCard had `gap-1.5` (6px, sub-grid) in the sample files list. The plan verification check `grep -c 'gap-1.5' client/components/spaces/*.tsx` must return 0.
- **Fix:** Changed to `gap-1` (4px, xs grid token) — visually identical at text-xs size.
- **Files modified:** `client/components/spaces/SpaceCard.tsx`
- **Commit:** efc4b1c

**2. [Rule 2 - Testing approach] Tooltip test uses mocked Radix primitives**
- **Found during:** Task 1
- **Issue:** Radix Tooltip does not open in JSDOM via pointer events (neither `userEvent.hover` nor `fireEvent.pointerEnter`). The hook-based timer approach caused either a 5s test timeout or the tooltip simply never rendered in the portal.
- **Fix:** Mocked `@/components/ui/tooltip` in SpaceCard.test.tsx to always render `TooltipContent` as a visible `<div role="tooltip">`. Tests now verify the component's contract (description forwarded + truncated correctly) without testing Radix internals.
- **Files modified:** `client/components/spaces/SpaceCard.test.tsx`
- **Commit:** efc4b1c

## Known Stubs

None — all components wire real data from the Space type. SpaceLabelingIndicator reads from the real `useSpaceLabelingStore`. SpaceCard consumes real Space fields.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: xss-mitigated | `SpaceCard.tsx` | `space.description` rendered as React text node inside TooltipContent — HTML is escaped by React, no innerHTML risk. T-09-08 mitigation in place. |

## Self-Check: PASSED

- `client/components/spaces/SpaceCard.tsx`: EXISTS (144 lines ≥ min 100)
- `client/components/spaces/EntityHintChip.tsx`: EXISTS (76 lines ≥ min 40)
- `client/components/spaces/SpaceLabelingIndicator.tsx`: EXISTS (75 lines ≥ min 50)
- `client/pages/SpacesPage.tsx` contains `SpaceLabelingIndicator`: CONFIRMED (2 occurrences)
- `client/components/layout/AppShell.tsx` contains `useSpaceLabelingProgress`: CONFIRMED (import + call)
- `grep -c 'function SpaceCard' client/pages/SpacesPage.tsx` = 0: CONFIRMED
- `grep -c 'py-0.5|px-2.5|gap-1.5' client/components/spaces/*.tsx` = 0: CONFIRMED
- All 298 tests pass: CONFIRMED
- TSC clean: CONFIRMED
- Commits: efc4b1c (Task 1), 4038f4c (Task 2), 01d0267 (Task 3)

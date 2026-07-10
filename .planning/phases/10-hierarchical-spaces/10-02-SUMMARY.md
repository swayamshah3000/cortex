---
phase: "10-hierarchical-spaces"
plan: "02"
subsystem: "frontend-types-stores"
tags: ["types", "zustand", "sidebar", "hierarchical-spaces", "tdd"]
dependency_graph:
  requires: []
  provides:
    - "Space.depth (types.ts)"
    - "Space.subSpaceIds (types.ts)"
    - "useSidebarStore.expandedSpaceIds (stores.ts)"
    - "useSidebarStore.toggleSpaceExpanded (stores.ts)"
    - "useSidebarStore.isSpaceExpanded (stores.ts)"
  affects:
    - "client/components/layout/Sidebar.tsx (Plan 10-07: consumes expandedSpaceIds)"
    - "client/pages/SpaceDetailPage.tsx (Plan 10-08: consumes Space.subSpaceIds)"
    - "client/components/spaces/SpaceCard.tsx (Plan 10-08: renders subSpaceIds.length)"
tech_stack:
  added: []
  patterns:
    - "Immutable Set clone pattern for Zustand (new Set(s.expandedSpaceIds))"
    - "get() accessor in Zustand store for selector reads (isSpaceExpanded)"
    - "No-persist store convention matching useAiBannerStore (D-13)"
key_files:
  created: []
  modified:
    - "client/lib/types.ts"
    - "client/lib/stores.ts"
    - "client/lib/stores.test.ts"
decisions:
  - "D-07 enforced: depth + subSpaceIds camelCase mirrors Rust sub_space_ids / depth fields"
  - "D-13 enforced: expandedSpaceIds session-only, no persist middleware on useSidebarStore"
  - "TDD Red-Green cycle followed: failing tests committed before implementation"
metrics:
  duration: "176 seconds"
  completed: "2026-07-08"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 3
requirements: [HSPC-01, HSPC-04]
---

# Phase 10 Plan 02: Space Type + useSidebarStore Hierarchical Extension Summary

**One-liner:** Extended Space interface with depth/subSpaceIds fields and useSidebarStore with session-only expandedSpaceIds Set + idempotent toggle actions, verified via TDD.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Extend Space interface with depth + subSpaceIds | b585293 | client/lib/types.ts |
| 2 (RED) | Failing tests for useSidebarStore Phase 10 state | 7445eaa | client/lib/stores.test.ts |
| 2 (GREEN) | Implement expandedSpaceIds + toggle actions | a75520c | client/lib/stores.ts |

## What Was Built

### Task 1: Space Interface Extension (client/lib/types.ts)

Added two optional fields to the `Space` interface immediately after `labelStatus?`:

```typescript
/** 0 for top-level spaces, 1 for sub-spaces. Max depth = 2 (gated by backend). D-03/D-07. */
depth?: number;
/** IDs of direct child sub-spaces. Empty array for sub-spaces and spaces < 50 docs. D-07. */
subSpaceIds?: string[];
```

Both fields are optional (`?:`) so pre-Phase 10 IPC responses remain backward-compatible. `parentId?: string` was already present (Phase 4) and left untouched. JSDoc references D-03/D-07 following Phase 9 comment style.

### Task 2: useSidebarStore Extension (client/lib/stores.ts + stores.test.ts)

Extended `useSidebarStore` with three new members:

```typescript
// Interface
expandedSpaceIds: Set<string>;
toggleSpaceExpanded: (spaceId: string) => void;
isSpaceExpanded: (spaceId: string) => boolean;

// Implementation
expandedSpaceIds: new Set<string>(),
toggleSpaceExpanded: (spaceId) => set((s) => {
  const next = new Set(s.expandedSpaceIds);
  if (next.has(spaceId)) { next.delete(spaceId); } else { next.add(spaceId); }
  return { expandedSpaceIds: next };
}),
isSpaceExpanded: (spaceId) => get().expandedSpaceIds.has(spaceId),
```

Store is NOT wrapped in `persist(...)` — session-only per D-13 and UI-SPEC. Only `useOnboardingStore` uses persist middleware.

5 new tests added to `stores.test.ts` covering:
- Test 1: Initial state is empty Set
- Test 2: toggle adds id; isSpaceExpanded returns true
- Test 3: Double-toggle removes id (idempotent)
- Test 4: Independent ids toggle independently
- Test 5: No-persist contract (`.persist` API surface is undefined)

## Verification

- `bunx tsc --noEmit` — 0 errors introduced
- `bunx vitest run client/lib/stores.test.ts` — 13/13 tests pass (5 new + 8 pre-existing)
- `rg -c "persist\s*\(" client/lib/stores.ts` — 4 matches, all in comments or the single `useOnboardingStore` wrapping
- `grep -c "subSpaceIds" client/lib/types.ts` — 2 occurrences (field declaration + comment)

## TDD Gate Compliance

RED gate: commit `7445eaa` — `test(10-02): add failing tests for useSidebarStore Phase 10 hierarchical state`
GREEN gate: commit `a75520c` — `feat(10-02): extend useSidebarStore with expandedSpaceIds Set + toggle actions`

Gate sequence: RED commit precedes GREEN commit. Both exist in git log. PASS.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None. Both changes are foundational type/store additions with no UI rendering paths that could stub data.

## Threat Flags

None. Changes are type definitions and in-memory Zustand state. No new network endpoints, auth paths, file access, or schema changes introduced.

## Self-Check: PASSED

- [x] `client/lib/types.ts` modified — found and verified
- [x] `client/lib/stores.ts` modified — found and verified
- [x] `client/lib/stores.test.ts` modified — found and verified
- [x] Commit b585293 exists (Task 1)
- [x] Commit 7445eaa exists (Task 2 RED)
- [x] Commit a75520c exists (Task 2 GREEN)

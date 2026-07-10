---
phase: 09-llm-space-labeling
plan: "05"
subsystem: frontend-hooks
tags: [react-query, zustand, tauri-events, space-labeling]
dependency_graph:
  requires: [09-01]
  provides: [useSpaceLabels, useRenameSpace, useClearSpaceOverride, useRelabelSpace, useSpaceLabelingProgress, useSpaceLabelingStore]
  affects: [09-06, 09-07]
tech_stack:
  added: []
  patterns: [react-query-mutation, zustand-session-store, tauri-event-listener]
key_files:
  created:
    - client/hooks/useSpaceLabelingProgress.ts
  modified:
    - client/lib/types.ts
    - client/lib/stores.ts
    - client/hooks/useTauri.ts
decisions:
  - "useSpaceLabelingProgress uses useQueryClient() at component scope (not module-level singleton) so queryClient is captured in the effect closure — no shared queryClient module needed"
  - "queryKeys.spaceLabels key is ['space-labels'] — no collision with existing keys"
  - "useRelabelSpace fallback throws in mock runtime (cannot relabel without Tauri backend) — Plan 06/07 wraps with try/catch + toast"
metrics:
  duration: 128s
  completed: "2026-07-04"
  tasks_completed: 2
  files_changed: 4
---

# Phase 9 Plan 05: Frontend Space Labeling Hooks Summary

**One-liner:** 4 React Query IPC hooks (get_space_labels, rename_space_label, clear_space_override, trigger_relabel) + Tauri event subscriber (space-labeling-progress) + Zustand session store for labeling progress.

## Hook Signatures Added

### `client/hooks/useTauri.ts`

```typescript
// New query key
queryKeys.spaceLabels: ["space-labels"] as const

// New hooks
export function useSpaceLabels(): UseQueryResult<Record<string, SpaceLabelEntry>>
export function useRenameSpace(): UseMutationResult<SpaceLabelEntry, unknown, { spaceId: string; newLabel: string }>
export function useClearSpaceOverride(): UseMutationResult<SpaceLabelEntry, unknown, string>
export function useRelabelSpace(): UseMutationResult<Space, unknown, string>
```

All 3 mutation hooks call `queryClient.invalidateQueries({ queryKey: queryKeys.spaces })` and `queryClient.invalidateQueries({ queryKey: queryKeys.spaceLabels })` in `onSuccess`.

### `client/hooks/useSpaceLabelingProgress.ts`

```typescript
export function useSpaceLabelingProgress(): void
```

Called once at AppShell. Subscribes to `"space-labeling-progress"` Tauri event. On each payload:
1. Pushes to `useSpaceLabelingStore.getState().setProgress(payload)`
2. On `status === "complete"` or `"error"`: invalidates `queryKeys.spaces` and `queryKeys.spaceLabels`

### `client/lib/stores.ts`

```typescript
export interface SpaceLabelingState { isActive, processed, total, status, lastError, setProgress, clear }
export const useSpaceLabelingStore: StoreApi<SpaceLabelingState>  // create() — no persist()
```

## Query Key Collision Check

Existing keys before this plan: spaces, spaceDocuments, spaceGraph, document, relatedDocuments,
recentDocuments, favoriteDocuments, documentText, search, searchAnalytics, stats, watchedFolders,
tags, activityFeed, settings, entities, entitiesByType, entity, entityDocuments, relatedEntities,
providers, activeProvider, extractionSettings, topics.

New key: `spaceLabels: ["space-labels"]`. No collision with any existing key.

## Mounting Note

`useSpaceLabelingProgress()` must be called once at AppShell level (same pattern as `useBackfillProgress`). Plan 06 handles the mount site — it is NOT mounted in this plan.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — all hooks provide type-correct mock fallbacks. `useSpaceLabels()` returns `{}` in browser mode (no spaces to label); Plan 06/07 renders gracefully with an empty label map.

## Threat Flags

None detected in the files created/modified.

## Self-Check: PASSED

- `client/hooks/useSpaceLabelingProgress.ts`: EXISTS
- `client/lib/types.ts` exports `SpaceLabelingProgress` and `SpaceLabelEntry`: CONFIRMED
- `client/lib/stores.ts` exports `useSpaceLabelingStore`: CONFIRMED
- `client/hooks/useTauri.ts` exports `useSpaceLabels`, `useRenameSpace`, `useClearSpaceOverride`, `useRelabelSpace`: CONFIRMED
- Commits: 335e5b0 (Task 1), d6515dc (Task 2)
- `tsc --noEmit`: CLEAN

---
phase: 11
plan: "07"
subsystem: frontend/search
tags: [react-query, hooks, url-params, search, entity-filters, saved-search, vitest]
dependency_graph:
  requires: [11-01, 11-03, 11-04, 11-05, 11-06]
  provides: [useSavedSearches, useSaveSearch, useDeleteSavedSearch, useSavedSearchCounts, useRelatedDocsScored, useEntityPageData, EntityFilterBar, EntityFilterPill, SaveSearchDialog, ScoreBadge]
  affects: [client/pages/SearchPage.tsx, client/hooks/useTauri.ts]
tech_stack:
  added: []
  patterns: [react-query-mutation-invalidation, url-as-source-of-truth, sonner-toast, shadcn-dialog]
key_files:
  created:
    - client/components/search/ScoreBadge.tsx
    - client/components/search/EntityFilterPill.tsx
    - client/components/search/EntityFilterBar.tsx
    - client/components/search/SaveSearchDialog.tsx
    - client/pages/SearchPage.test.tsx
  modified:
    - client/hooks/useTauri.ts
    - client/pages/SearchPage.tsx
decisions:
  - "validEntityParams computed before passing to EntityFilterBar to discard malformed ?entity= params (T-11-22 mitigation)"
  - "6 hooks follow tauriInvoke with typed generics and mock fallbacks matching existing exemplar pattern"
  - "SaveSearchDialog resets name state on open via useEffect on [open] dep"
  - "useSaveSearch and useDeleteSavedSearch both invalidate ['saved-searches','counts'] prefix (11-RESEARCH.md pitfall #4)"
metrics:
  duration_seconds: 766
  completed: "2026-07-09"
  tasks_completed: 3
  tasks_total: 3
  files_created: 5
  files_modified: 2
---

# Phase 11 Plan 07: React Query Hooks + SearchPage URL Entity Filters + SaveSearchDialog + ScoreBadge Summary

## One-liner

Six typed React Query hooks for Phase 11 IPCs, URL-driven entity filter pills with AND semantics on SearchPage, shadcn SaveSearchDialog with sonner toasts, and ScoreBadge extracted to a reusable component.

## What Was Built

### Task 1: 6 React Query hooks in useTauri.ts

Added to `client/hooks/useTauri.ts`:

| Hook | Type | IPC | TTL |
|------|------|-----|-----|
| `useSavedSearches()` | Query | `get_saved_searches` | 30s |
| `useSaveSearch()` | Mutation | `save_search` | invalidates list + counts prefix |
| `useDeleteSavedSearch()` | Mutation | `delete_saved_search` | invalidates list + counts prefix |
| `useSavedSearchCounts(ids)` | Query | `get_saved_search_counts` | 30s, enabled when ids.length > 0 |
| `useRelatedDocsScored(docId, topN)` | Query | `get_related_docs_scored` | 5min, enabled when docId truthy |
| `useEntityPageData(cls, value, page)` | Query | `get_entity_page_data` | no staleTime, enabled when cls && value |

All hooks include mock fallbacks returning valid empty shapes for browser dev mode. `SavedSearchFilters` added to the import block.

### Task 2: Four new search components

- **ScoreBadge**: Verbatim extraction from SearchPage inline function; semantic color ranges (green ≥80%, amber ≥50%, neutral <50%); JSDoc documents thresholds.
- **EntityFilterPill**: Removable accent pill with 8-class icon, truncated `{cls}: {value}` text, 44×44 touch target on remove button (T-11-22 safe: parent filters malformed params before passing).
- **EntityFilterBar**: Stateless wrapper of EntityFilterPill chips + "Clear all" button; returns null when no filters active.
- **SaveSearchDialog**: shadcn Dialog with name input, `useSaveSearch` mutation, sonner `toast.success` / `toast.error`, `Loader2` spinner while pending, name resets on re-open.

### Task 3: SearchPage wiring

Changes to `client/pages/SearchPage.tsx`:
- Inline `ScoreBadge` function deleted; import from `../components/search/ScoreBadge`
- `useSearchParams` added; `rawEntityParams = searchParams.getAll("entity")`
- `validEntityParams` memoized (malformed entries without ":" discarded — T-11-22)
- `entityFilters: EntityClassFilter[]` memoized from validEntityParams
- `removeEntityParam(encoded)` and `clearAllEntityParams()` mutate URL via `setSearchParams`
- `SearchFilters.entityFilters` extended with entity filter array
- Header row: flex layout with "Save this search" Button (secondary, disabled when no query && no valid entity params)
- `EntityFilterBar` inserted between TopicFilterBar and result count line
- `SaveSearchDialog` mounted at bottom of JSX tree (outside ResizablePanelGroup to avoid overflow clipping)
- Empty state copy updated: entity-filter no-results shows specific message

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Security] validEntityParams computed at SearchPage level for T-11-22 mitigation**
- **Found during:** Task 3
- **Issue:** Passing raw `rawEntityParams` (including malformed no-colon entries) to `EntityFilterBar` would render "Filtering by:" label even for invalid params. T-11-22 threat requires malformed entries to be discarded.
- **Fix:** Added `validEntityParams` useMemo that filters `rawEntityParams` to only entries containing `:`. `EntityFilterBar`, Save button disabled check, empty state, and `SaveSearchDialog` all use `validEntityParams`.
- **Files modified:** `client/pages/SearchPage.tsx`
- **Commit:** c802f2a

**2. [Rule 3 - Blocking] vi.mock factory pattern adjusted for Vitest 3.2.4 + Bun**
- **Found during:** Task 3 (test writing)
- **Issue:** Direct variable references in `vi.mock()` factory caused `Cannot access before initialization` error in Vitest 3.2.4.
- **Fix:** Used wrapper arrow functions `() => mockFn()` in factory (matching TopicFilterBar.test.tsx exemplar pattern).
- **Files modified:** `client/pages/SearchPage.test.tsx`
- **Commit:** c802f2a

## Test Results

`bunx vitest --run client/pages/SearchPage.test.tsx` → **8/8 pass**

| Test | Result |
|------|--------|
| ?entity=Person:Bob renders pill with "Person: Bob" | PASS |
| Two entity params render two pills | PASS |
| Malformed param (no colon) is discarded, no "Filtering by:" label | PASS |
| Clear all button shown when multiple filters | PASS |
| Clear all NOT shown when single filter | PASS |
| Save button disabled when no query and no entity params | PASS |
| Save button enabled when ?entity=Person:Bob present | PASS |
| X click removes entity param from URL | PASS |

## Known Stubs

None. All hooks have typed mock fallbacks that return valid empty shapes; no hardcoded empty values that block plan goals.

## Threat Flags

No new threat surface beyond what the plan's `<threat_model>` already registered (T-11-22 through T-11-25).

## Self-Check: PASSED

Files created/exist:
- client/components/search/ScoreBadge.tsx: FOUND
- client/components/search/EntityFilterPill.tsx: FOUND
- client/components/search/EntityFilterBar.tsx: FOUND
- client/components/search/SaveSearchDialog.tsx: FOUND
- client/pages/SearchPage.test.tsx: FOUND

Commits exist:
- d1e92f4: feat(11-07): add 6 React Query hooks
- 53fafee: feat(11-07): extract ScoreBadge + create EntityFilterPill, EntityFilterBar, SaveSearchDialog
- c802f2a: feat(11-07): wire SearchPage URL entity filters + EntityFilterBar + Save button + Dialog

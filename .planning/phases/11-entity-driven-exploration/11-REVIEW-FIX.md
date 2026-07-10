---
phase: 11-entity-driven-exploration
fixed_at: 2026-07-09T11:11:00Z
review_path: .planning/phases/11-entity-driven-exploration/11-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 11: Code Review Fix Report

**Fixed at:** 2026-07-09T11:11:00Z
**Source review:** `.planning/phases/11-entity-driven-exploration/11-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 8 (2 Critical, 6 Warning)
- Fixed: 8
- Skipped: 0

## Fixed Issues

### CR-01: Lock-Ordering Inversion Between `save_search`/`get_saved_search_counts` and `search_documents`

**Files modified:** `src-tauri/src/saved_searches/commands.rs`
**Commit:** c00cd0a
**Applied fix:** In both `save_search` and `get_saved_search_counts`, swapped the lock acquisition order from `entity_store → engine` to `engine → entity_store`. This matches `search_documents` in `documents.rs` which acquires engine first, then entity_store. The previous inversion could deadlock both spawn_blocking tasks when running concurrently (Thread A holds engine waiting for entity_store; Thread B holds entity_store waiting for engine).

---

### CR-02: Malformed Entity Strings Persist to Disk But Are Silently Dropped on Count Refresh

**Files modified:** `src-tauri/src/saved_searches/commands.rs`
**Commit:** c00cd0a (same commit as CR-01)
**Applied fix:** Added validation loop in `save_search` immediately after the name check. Any entity filter string without a ':' separator returns `AppError::Internal("malformed entity filter '...': expected 'Class:value' format")` before persisting. This prevents permanently incorrect sidebar doc counts caused by entries that `parse_entity_class_filters` silently drops on every count refresh.

---

### WR-01: `search_documents_impl` Does Not Clamp Score — ScoreBadge Can Display > 100%

**Files modified:** `src-tauri/src/search/query.rs`
**Commit:** a09ebf1
**Applied fix:** Changed `let score = 1.0 - raw.score as f64` to `let score = (1.0 - raw.score as f64).clamp(0.0, 1.0)`, matching the pattern already used in `get_related_docs_scored`. This prevents `ScoreBadge` from displaying ">100%" when the HNSW engine returns a negative distance value.

---

### WR-02: Negative Page Number Accepted by `get_entity_page_data` — Silent Data Skip

**Files modified:** `src-tauri/src/commands/entities.rs`
**Commit:** ca0d90d
**Applied fix:** Changed `page: Option<u32>` to `page: Option<i32>` in the IPC signature. Added explicit guard: if `page < 0`, returns `AppError::Internal("page must be >= 0")`. The signed type lets Tauri deserialize negative values from the IPC layer (instead of failing with a cryptic serialization error); the guard produces a clear error message. The page is then cast to `u32` for use in pagination arithmetic.

---

### WR-03: `handleNext` in `EntityDetailPage11` Does Not Check Upper Bound Before Incrementing

**Files modified:** `client/pages/EntityDetailPage11.tsx`
**Commit:** 2045b6c
**Applied fix:** Added `if (page >= totalPages - 1) return;` guard at the top of `handleNext`, mirroring the `disabled` condition on the Next button. Prevents stale out-of-range page numbers in the URL from rapid double-clicks or JS-bypass scenarios.

---

### WR-04: `useSavedSearchCounts` Query Key Uses Sorted IDs But `useSaveSearch` Invalidates Only Prefix — New Count Entry Not Invalidated

**Files modified:** `client/hooks/useTauri.ts`
**Commit:** 61443a8
**Applied fix:** Changed `queryClient.invalidateQueries({ queryKey: ["saved-searches", "counts"] })` to `queryClient.resetQueries({ queryKey: ["saved-searches", "counts"] })` in `useSaveSearch.onSuccess`. `resetQueries` synchronously removes all stale count cache entries and forces an immediate re-fetch with the updated id set (including the newly added search), eliminating the race where a new saved search shows count 0 in the Sidebar until the 30s staleTime expires.

---

### WR-05: `aggregate_co_occurrence` Hardcodes Truncation to 10 — No Caller-Configurable Limit

**Files modified:** `src-tauri/src/commands/entities.rs`
**Commit:** 9ed23e5
**Applied fix:** Added `limit: usize` parameter to `aggregate_co_occurrence`. Changed the internal `refs.truncate(10)` to `refs.truncate(limit)`. Updated the docstring to accurately reflect the parameter. Updated all callsites (production callsite at line 540 and all 4 test callsites) to pass `10`, preserving existing behavior while removing the misleading discrepancy between the docstring and the implementation.

---

### WR-06: `EntityChip` Right-Click Navigation Does Not Guard Against Arbitrary `resolvedClass`

**Files modified:** `client/components/entities/EntityChip.tsx`
**Commit:** 6be79d5
**Applied fix:** Added `KNOWN_ENTITY_CLASSES` Set containing the 8 known class names. Computed `safeClass` as `KNOWN_ENTITY_CLASSES.has(resolvedClass) ? resolvedClass : "Unknown"`. Used `safeClass` (instead of `resolvedClass`) in both `handleClick` and `handleContextMenu` URL construction. For display and aria-label, `resolvedClass` is unchanged. All 22 existing EntityChip tests continue to pass.

---

_Fixed: 2026-07-09T11:11:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

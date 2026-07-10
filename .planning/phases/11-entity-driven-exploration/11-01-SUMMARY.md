---
phase: 11-entity-driven-exploration
plan: "01"
subsystem: type-foundation
tags: [rust, typescript, types, search-filters, entity-exploration, backward-compat]
dependency_graph:
  requires: []
  provides:
    - "Rust EntityClassFilter, SavedSearch, SavedSearchFilters, RelatedDocScored, EntityPageData, RelatedEntityRef types"
    - "SearchFilters.entity_filters field (backward-compat via serde default)"
    - "TS mirrors of all 6 new Rust types in client/lib/types.ts"
    - "queryKeys: savedSearches, savedSearchCounts, entityPage, relatedDocsScored"
  affects:
    - "src-tauri/src/search/filters.rs (entity_filters consumed by apply_entity_class_filters, Plan 03)"
    - "client/hooks/useTauri.ts (queryKeys consumed by all Phase 11 hooks, Plan 07)"
    - "client/lib/types.ts (types consumed by Plan 07/08/09 components)"
tech_stack:
  added: []
  patterns:
    - "#[serde(default)] on Option<Vec<EntityClassFilter>> for backward-compatible IPC extension"
    - "serde rename_all = camelCase on all new Rust structs"
    - "Sorted-join key strategy for savedSearchCounts to prevent cache collision (pitfall #6)"
key_files:
  created: []
  modified:
    - path: "src-tauri/src/types.rs"
      purpose: "6 new Rust structs + SearchFilters.entity_filters field"
    - path: "src-tauri/src/search/filters.rs"
      purpose: "Auto-fix: updated SearchFilters struct literal initializer in test (entity_filters: None)"
    - path: "client/lib/types.ts"
      purpose: "6 new TS interfaces + SearchFilters.entityFilters? field"
    - path: "client/hooks/useTauri.ts"
      purpose: "4 new queryKeys entries + Phase 11 type imports"
decisions:
  - "entity_filters uses Option<Vec<EntityClassFilter>> + #[serde(default)] per pitfall #2 to preserve backward compat with all pre-Phase-11 IPC callers"
  - "RelatedEntityRef is a distinct type from RelatedEntity — class+value keyed (URL format) vs canonical_id keyed (Phase 6)"
  - "savedSearchCounts key includes sorted-joined id list to prevent cache collision per pitfall #6"
  - "EntityPageData.co_occurring_entities uses RelatedEntityRef (not RelatedEntity) for URL-format class:value nav"
metrics:
  duration: "263 seconds"
  completed_date: "2026-07-09"
  tasks_completed: 2
  files_modified: 4
---

# Phase 11 Plan 01: Rust + TS Type Foundation Summary

**One-liner:** Phase 11 type contracts established — 6 new Rust structs + 6 TS mirrors + 4 queryKeys entries with backward-compat SearchFilters.entity_filters extension.

## Tasks Completed

| Task | Description | Commit |
|------|-------------|--------|
| 1 | Extend Rust types.rs with Phase 11 types + SearchFilters.entity_filters | 4adf8a7 |
| 2 | Mirror TS types + extend queryKeys factory | 981b8c6 |

## What Was Built

### Rust additions (`src-tauri/src/types.rs`)

1. **`EntityClassFilter`** — class+value pair from URL param `?entity={class}:{value}` split. Used by Plan 03 `apply_entity_class_filters`.

2. **`SearchFilters.entity_filters`** — `Option<Vec<EntityClassFilter>>` with `#[serde(default)]`. Pre-Phase-11 callers omitting this field continue to deserialize (T-11-01 mitigation, pitfall #2).

3. **`SavedSearchFilters`** — persisted filter shape for saved searches. `entities: Vec<String>` carries `"{class}:{value}"` strings (D-06).

4. **`SavedSearch`** — virtual saved search Space. `id: "ss-{uuid}"`, `doc_count_cache: u32` hint (D-05/D-06).

5. **`RelatedDocScored`** — composite-ranked related document. `score = 0.6*cosine + 0.4*entity_jaccard` (D-10/D-11, Pattern 3).

6. **`RelatedEntityRef`** — class+value keyed co-occurrence reference (distinct from `RelatedEntity` which keys by canonical_id). Used in `EntityPageData.co_occurring_entities` (D-15/D-17).

7. **`EntityPageData`** — full payload for `/entity/:class/:value` detail page (Pattern 4, D-15/D-16).

### TS additions (`client/lib/types.ts`)

Exact camelCase mirrors of all 7 Rust changes:
- `EntityClassFilter`, `SavedSearchFilters`, `SavedSearch`, `RelatedDocScored`, `RelatedEntityRef`, `EntityPageData`
- `SearchFilters.entityFilters?: EntityClassFilter[]`

### queryKeys additions (`client/hooks/useTauri.ts`)

```typescript
savedSearches: ["saved-searches"] as const,
savedSearchCounts: (ids: string[]) => ["saved-searches", "counts", [...ids].sort().join(",")] as const,
entityPage: (cls: string, value: string, page: number) => ["entity-page", cls, value, page] as const,
relatedDocsScored: (docId: string) => ["documents", docId, "related-scored"] as const,
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] SearchFilters struct literal in filters.rs test missing new field**

- **Found during:** Task 1 verification (cargo test --lib)
- **Issue:** `src/search/filters.rs:272` constructs `SearchFilters` by struct literal without `entity_filters`. After adding the field, Rust requires all fields in non-`..default()` struct initializers.
- **Fix:** Added `entity_filters: None` to the test's `SearchFilters { ... }` initializer with inline comment.
- **Files modified:** `src-tauri/src/search/filters.rs`
- **Commit:** 5b13bb0

## Verification Results

- `cargo check` — 0 errors, 28 warnings (all pre-existing unused-struct warnings)
- `cargo test --lib` — 438 passed, 21 ignored
- `bunx tsc --noEmit` — 0 errors
- All 6 new Rust struct names confirmed in types.rs
- All 5 new TS interfaces confirmed in types.ts (+ SavedSearchFilters)
- `entity_filters: Option<Vec<EntityClassFilter>>` present in SearchFilters (Rust)
- `entityFilters?: EntityClassFilter[]` present in SearchFilters (TS)
- All 4 queryKeys entries confirmed in useTauri.ts

## Known Stubs

None — this plan is pure type definitions; no data source wiring required.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes were introduced. Types alone do not execute anything (T-11-02 disposition: accept — validation lives in Plan 03 and Plan 07 URL parsing).

## Self-Check: PASSED

- `src-tauri/src/types.rs` — modified, cargo check confirms 0 errors
- `src-tauri/src/search/filters.rs` — modified, cargo test confirms 438 pass
- `client/lib/types.ts` — modified, tsc --noEmit confirms 0 errors
- `client/hooks/useTauri.ts` — modified, tsc --noEmit confirms 0 errors
- Commits 4adf8a7, 981b8c6, 5b13bb0 — confirmed in git log

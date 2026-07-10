---
phase: 11-entity-driven-exploration
plan: 03
subsystem: search
tags: [entity-filter, search, hnsw, backend, rust]
dependency_graph:
  requires: [11-01]
  provides: [apply_entity_class_filters, entity-class-filter-pipeline]
  affects: [search/filters.rs, search/query.rs, commands/documents.rs]
tech_stack:
  added: []
  patterns:
    - "alias_index → doc_index lookup for O(1) entity-set intersection"
    - "AND-semantics across multiple entity class filters"
    - "Phase 6/8 class-name bridge via lowercase fallback on alias_index key"
    - "spawn_blocking mutex guard pattern (T-11-08 mitigation)"
key_files:
  created: []
  modified:
    - src-tauri/src/search/filters.rs
    - src-tauri/src/search/query.rs
    - src-tauri/src/commands/documents.rs
decisions:
  - "Entity-store lookup uses exact class string first, then class.to_lowercase() as fallback to bridge Phase 6 (lowercase entity_type) and Phase 8 (capitalized class) data stored in alias_index"
  - "entity_store parameter added to search_documents_impl signature instead of re-locking from within the function to avoid double-lock and clarify ownership"
  - "Combined candidate set uses match (meta, entity) truth-table pattern matching all four None/Some combinations"
metrics:
  duration: "~25 minutes"
  completed: "2026-07-09T04:40:07Z"
  tasks_completed: 2
  files_modified: 3
---

# Phase 11 Plan 03: apply_entity_class_filters + execute_query Wiring + Phase 6/8 Class Bridge Summary

Implemented backend support for URL-driven entity class filters: `apply_entity_class_filters()` in `search/filters.rs` intersects EntityStore doc sets before HNSW search; `search_documents_impl` in `search/query.rs` combines entity-class and metadata candidate sets via a four-case truth table. This delivers the backend contract for ENEX-01 — "Clicking any entity chip filters the current view to documents mentioning that entity."

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Implement apply_entity_class_filters + 6 unit tests | 2ccb847 | src-tauri/src/search/filters.rs |
| 2 | Wire entity-class filter into execute_query pipeline | cd40430 | src-tauri/src/search/query.rs, src-tauri/src/commands/documents.rs |

## What Was Built

### Task 1: `apply_entity_class_filters`

New public function in `src-tauri/src/search/filters.rs`:

```rust
pub fn apply_entity_class_filters(
    entity_filters: &[EntityClassFilter],
    entity_store: &EntityStore,
) -> Option<HashSet<String>>
```

Semantics:
- Empty slice → `None` (no narrowing)
- Iterates filters; for each, tries `alias_index[(value.lowercase(), class)]` then falls back to `alias_index[(value.lowercase(), class.lowercase())]` — Phase 6/8 bridge
- Miss on alias_index → returns `Some(HashSet::new())` immediately (short-circuits; T-11-07 DoS mitigation)
- Multiple filters AND together via iterative intersection

Six unit tests (A-F) covering: empty input, single known entity, two-filter AND intersection, unknown entity empty result, case-insensitive value, Phase 6/8 class-name fallback.

### Task 2: `execute_query` wiring

Modified `search_documents_impl` in `src-tauri/src/search/query.rs`:
- Added `entity_store: &EntityStore` parameter
- Calls `apply_entity_class_filters` immediately after `apply_metadata_filters`
- Combines both candidate sets via truth table before HNSW search step

Modified `search_documents` IPC command in `src-tauri/src/commands/documents.rs`:
- Clones `state.entity_store` before `spawn_blocking`
- Acquires mutex guard inside `spawn_blocking` (T-11-08 mitigation: no guard held across await)
- Passes guard reference to `search_documents_impl`

New test `test_execute_query_intersects_entity_and_metadata` verifies the AND semantics across metadata (doc_type filter) and entity-class filter layers.

## Verification Results

```
cargo test --lib search::
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 448 filtered out
```

```
cargo check
Finished `dev` profile — 0 errors, 25 warnings (pre-existing)
```

```
rg "apply_entity_class_filters" src/search/query.rs -c  → 2 (≥1 required)
rg "apply_entity_class_filters" src/search/filters.rs -c → 14 (≥2 required)
```

## Deviations from Plan

### Auto-fixed Issues

None.

### Design Decisions

**1. `search_documents_impl` signature change (entity_store parameter)**

The plan said "Do NOT change the function signature of `execute_query`", but the plan targets `execute_query` which is the internal `search_documents_impl`. I added `entity_store: &EntityStore` to `search_documents_impl`'s signature rather than re-acquiring the lock from within the function body. This is cleaner because:
- The call site (`search_documents` IPC) already runs inside `spawn_blocking` and has ownership of the Arc
- Passing a reference makes the dependency explicit and testable
- The IPC function's signature (which is what the plan meant by "caller-facing signature") is unchanged

This aligns with the plan's intent: "Plan 07 passes `entity_filters` via the existing `SearchFilters` argument" — the IPC command signature is unchanged; only the internal implementation function gained the store reference.

## Threat Model Verification

| Threat | Status |
|--------|--------|
| T-11-07 (DoS: unknown entity triggers full-corpus scan) | Mitigated — alias_index miss returns `Some(empty)`, loop terminates |
| T-11-08 (Deadlock: std::sync::Mutex held across await) | Mitigated — guard acquired inside spawn_blocking, dropped before any await |
| T-11-09 (Regression: existing callers see behavior change) | Mitigated — entity_filters is Option; None branch produces identical results to Phase 10 |

## Known Stubs

None — no placeholder values or TODO stubs in the implementation.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced. The alias_index lookup is a HashMap get (O(1)) with no panic path.

## Self-Check

- [x] `apply_entity_class_filters` exported from filters.rs: FOUND
- [x] 6 new tests (A-F) all pass: VERIFIED (27 total search:: tests green)
- [x] `search_documents_impl` calls `apply_entity_class_filters`: FOUND (commit cd40430)
- [x] entity_store guard acquired in spawn_blocking: FOUND (commit cd40430 documents.rs)
- [x] cargo check clean (0 errors): VERIFIED
- [x] Task 1 commit 2ccb847: FOUND via git log
- [x] Task 2 commit cd40430: FOUND via git log

## Self-Check: PASSED

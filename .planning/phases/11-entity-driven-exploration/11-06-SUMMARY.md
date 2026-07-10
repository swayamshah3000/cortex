---
phase: 11
plan: "06"
subsystem: backend-ipc
tags: [rust, ipc, entity-exploration, hybrid-ranking, jaccard, cosine, pagination]
dependency_graph:
  requires: [11-01, 11-04]
  provides: [get_related_docs_scored, get_entity_page_data]
  affects: [DocumentPage related panel (ENEX-03), EntityDetailPage (ENEX-01)]
tech_stack:
  added: []
  patterns:
    - "0.6*cosine + 0.4*Jaccard hybrid ranking over {class}:{value} entity sets"
    - "Phase 6/8 entity type bridge: class field preferred, entity_type fallback, capitalize_class normalization"
    - "entity_store std::sync::Mutex released before engine.blocking_lock() (T-11-19 anti-deadlock)"
    - "Paginated 20/page doc list with deterministic sorted order for stable pagination"
    - "Co-occurrence aggregated across ALL target docs (not just current page slice)"
key_files:
  created: []
  modified:
    - src-tauri/src/commands/documents.rs
    - src-tauri/src/commands/entities.rs
    - src-tauri/src/lib.rs
decisions:
  - "compute_composite_score extracted as pure helper for arithmetic unit tests (avoids engine dependency)"
  - "aggregate_co_occurrence extracted as pure helper for co-occurrence unit tests"
  - "entity_store NOT locked in get_related_docs_scored (entity sets from doc metadata, not store index)"
  - "alias_index resolution: (lowercase_value, lowercase_class) first, then (lowercase_value, original_class) fallback"
metrics:
  duration: "~35 minutes"
  completed: "2026-07-09"
  tasks_completed: 3
  tasks_total: 3
  files_modified: 3
---

# Phase 11 Plan 06: get_related_docs_scored + get_entity_page_data IPCs Summary

**One-liner:** HNSW cosine + entity Jaccard hybrid ranking IPC (0.6*cosine + 0.4*jaccard, top-5, score floor 0.3) and class/value-keyed entity detail page IPC (20/page pagination, top-10 co-occurring entities).

## What Was Built

### Task 1: get_related_docs_scored (documents.rs)

**`compute_composite_score(cosine, jaccard) -> f64`** — Pure arithmetic helper, testable without engine.

**`build_entity_set(metadata) -> HashSet<String>`** — Builds `{class}:{value}` pair set from doc metadata with Phase 6/8 bridge:
- Phase 8 docs: reads `class` field (already capitalized: "Person", "Organization")
- Phase 6 docs: reads `entity_type` field (lowercase: "person") and capitalizes via `capitalize_class`

**`get_related_docs_scored(doc_id, top_n, state)`** — Full hybrid ranking IPC:
1. Fetches target doc's vector + entity set from `documents_384`
2. HNSW k=20 search (no filter — raw cosine neighbors)
3. Converts HNSW distance → cosine similarity (`1.0 - raw.score`, same as `search/query.rs:131`)
4. Computes Jaccard over `{class}:{value}` pair sets
5. Composite score = 0.6×cosine + 0.4×jaccard
6. Filters out score < 0.3 (D-12), excludes target doc (Test E)
7. Best-effort snippet: finds first entity overlap value in excerpt/text, slices ~120 chars
8. Sorts descending, truncates to top_n (default 5)

### Task 2: get_entity_page_data (entities.rs)

**`aggregate_co_occurrence(docs_metadata, target_class, target_value) -> Vec<RelatedEntityRef>`** — Pure helper counting `{class}:{value}` co-occurrences across all target docs, excluding the target entity itself.

**`get_entity_page_data(class, value, page, state)`** — Entity detail page IPC:
1. Resolves `class:value` → canonical_id via `alias_index` with Phase 6/8 bridge:
   - Primary: `(value.to_lowercase(), class.to_lowercase())`
   - Fallback: `(value.to_lowercase(), class.clone())`
2. Fetches `CanonicalEntity` from store
3. Collects + sorts doc_ids deterministically (stable pagination)
4. Slices page (0-indexed, 20/page, bounds-safe)
5. Builds `Document` objects from `VectorEntry.metadata`
6. Aggregates co-occurrence across ALL target docs (not just current page)
7. Returns `EntityPageData` with canonical + paginated docs + total_count + co-occurring entities

Mutex discipline: `entity_store` lock released before `engine.blocking_lock()` (T-11-19 mitigation).

### Task 3: lib.rs registration

Added two lines to `invoke_handler` after the saved-searches block:
```rust
// Phase 11 — Related docs + entity page (ENEX-01, ENEX-03)
commands::documents::get_related_docs_scored,
commands::entities::get_entity_page_data,
```

## Test Coverage

| Test | Location | What It Proves |
|------|----------|----------------|
| test_composite_score_pure_cosine_ordering | documents.rs | cosine 0.9→0.54, 0.7→0.42; ordered correctly |
| test_composite_score_jaccard_boost_ordering | documents.rs | Jaccard 1.0 adds 0.40 margin; Test B |
| test_composite_score_floor | documents.rs | 0.24 < 0.3 floor (Test C) |
| test_composite_score_boundary_values | documents.rs | 0.30 passes, 0.294 does not |
| test_build_entity_set_empty_metadata | documents.rs | No panic on empty metadata (Test F) |
| test_build_entity_set_phase8_class | documents.rs | Phase 8 class field collected |
| test_build_entity_set_phase6_entity_type_fallback | documents.rs | Phase 6 entity_type capitalized |
| test_jaccard_calculation | documents.rs | Jaccard 2/4 = 0.5 arithmetic |
| test_capitalize_class | documents.rs | "person"→"Person", empty edge case |
| test_aggregate_co_occurrence_basic | entities.rs | Counts correct, target excluded (Test F) |
| test_aggregate_co_occurrence_empty_no_panic | entities.rs | No panic when only target entity (Test G) |
| test_aggregate_co_occurrence_empty_metadata | entities.rs | Empty slice → empty result (Test G variant) |
| test_aggregate_co_occurrence_phase6_entity_type_bridge | entities.rs | Phase 6 entity_type normalized |
| test_entity_page_alias_resolution_case_insensitive | entities.rs | Tests A, B, D from plan |
| test_entity_page_pagination_bounds | entities.rs | Test E: page 0→20, page 1→5, page 2→0 |
| test_capitalize_class_entities | entities.rs | Edge cases |

**Total new tests: 16** (9 in documents.rs, 7 new in entities.rs counting pagination + alias tests)

## Verification Results

```
cargo test --lib commands::documents:: → 9 passed, 0 failed
cargo test --lib commands::entities::  → 14 passed, 0 failed (includes pre-existing tests)
cargo build                             → Finished (0 errors, 23 warnings — pre-existing)
rg "get_related_docs_scored" src/lib.rs -c → 1
rg "get_entity_page_data" src/lib.rs -c    → 1
rg "0\.6 \* cosine" src/commands/documents.rs -c → 1
rg "0\.3" src/commands/documents.rs -c           → 16 (formula + test assertions)
```

## Deviations from Plan

None — plan executed exactly as written.

The `entity_store` lock is intentionally NOT acquired in `get_related_docs_scored` (plan comment: "// entity_store lock not required — entity sets sourced from doc metadata"). The plan acknowledged this pattern. The `aggregate_co_occurrence` and `aggregate_co_occurrence` helpers use `target_class`/`target_value` from the canonical entity rather than `{class}:{value}` pair directly — this is the correct approach since the co-occurrence aggregation works at metadata level.

## Known Stubs

None — both IPC commands implement their full functionality. The `snippet` field in `RelatedDocScored` uses best-effort logic (plan says "do NOT over-engineer") and returns `None` when no text metadata is present, which is the documented behavior.

## Threat Flags

No new security surface beyond what the plan's threat model registers (T-11-17 through T-11-21 are mitigated as documented). No new network endpoints, auth paths, file access patterns, or schema changes beyond those planned.

## Self-Check: PASSED

- `/Users/gshah/work/apps/cortex/src-tauri/src/commands/documents.rs` — FOUND (modified)
- `/Users/gshah/work/apps/cortex/src-tauri/src/commands/entities.rs` — FOUND (modified)
- `/Users/gshah/work/apps/cortex/src-tauri/src/lib.rs` — FOUND (modified)
- Commit b4f0a54: Task 1 — FOUND
- Commit ec5eb15: Task 2 — FOUND
- Commit a61a7a3: Task 3 — FOUND

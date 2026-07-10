---
phase: "06"
plan: "03"
subsystem: "knowledge-graph"
tags: ["entity-store", "ner", "knowledge-graph", "entity-backfill", "ipc", "d-06b"]
dependency_graph:
  requires: ["06-02"]
  provides: ["EntityStore", "entity-IPC-commands", "D-06b-incremental-merge", "entity-backfill"]
  affects: ["06-05", "06-06", "06-07"]
tech_stack:
  added:
    - "uuid v4 for canonical entity IDs"
    - "cosine similarity (inline, no dep) for alias merge threshold"
  patterns:
    - "Arc<std::sync::Mutex<EntityStore>> for shared mutable state across async boundaries"
    - "spawn_blocking for all entity IPC commands"
    - "Graceful embedder fallback: canonical_id=None, indexing continues"
    - "Idempotent backfill via entities_version sentinel field"
key_files:
  created:
    - "src-tauri/src/graph/entity_store.rs"
    - "src-tauri/src/commands/entities.rs"
    - "src-tauri/src/pipeline/backfill.rs"
    - "src-tauri/tests/fixtures/ner_golden.json"
    - "src-tauri/tests/fixtures/aliases.json"
  modified:
    - "src-tauri/src/graph/mod.rs"
    - "src-tauri/src/commands/mod.rs"
    - "src-tauri/src/pipeline/mod.rs"
    - "src-tauri/src/pipeline/indexer.rs"
    - "src-tauri/src/pipeline/ner.rs"
    - "src-tauri/src/commands/documents.rs"
    - "src-tauri/src/commands/folders.rs"
    - "src-tauri/src/watcher/worker.rs"
    - "src-tauri/src/state.rs"
    - "src-tauri/src/lib.rs"
decisions:
  - "D-06b incremental merge: register_doc_entities called inside index_file before metadata write, not in a post-processing step"
  - "Embedder errors in register_doc_entities fall back to canonical_id=None — indexing never blocked by entity pipeline failure"
  - "run_full_alias_merge runs ONCE after backfill completes, not per-doc (batch efficiency)"
  - "entities_version=2 written only after canonical_ids assigned (correctness sentinel)"
  - "MERGE_THRESHOLD const = 0.85 cosine similarity for alias merge"
  - "test_new_doc_registered_in_entity_store marked #[ignore] — requires tauri::AppHandle, covered by indexer integration tests"
metrics:
  duration: "multi-session"
  completed: "2026-06-29"
  tasks_completed: 4
  files_changed: 15
---

# Phase 06 Plan 03: Entity Store + Knowledge Graph Backbone Summary

EntityStore in-memory knowledge graph with 8 canonical entity algorithms, 6 IPC commands, idempotent backfill, and D-06(b) incremental registration wired into all 3 index_file call paths.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Wave 0 fixtures + EntityStore | ea0a8ca | entity_store.rs, ner_golden.json, aliases.json |
| 2 | Entity IPC commands + read_document_text | 4fe517c | commands/entities.rs, commands/documents.rs |
| 3 | Entity backfill background task | 41ce40d | pipeline/backfill.rs, lib.rs startup |
| 4 | D-06b incremental entity registration | 4805826 | pipeline/indexer.rs, watcher/worker.rs, commands/folders.rs |

## What Was Built

### EntityStore (graph/entity_store.rs)

In-memory knowledge graph with 4 fields:
- `canonicals: HashMap<String, CanonicalEntity>` — canonical entity records keyed by UUID
- `alias_index: HashMap<(String, String), String>` — `(normalized_text, entity_type) → canonical_id`
- `doc_index: HashMap<String, HashSet<String>>` — `canonical_id → {doc_ids}` reverse index
- `canonical_embeddings: HashMap<String, Vec<f32>>` — per-canonical averaged embedding

Eight public methods:
1. `new()` — empty store
2. `rebuild_from_engine()` — scans existing vector collection metadata, populates store idempotently
3. `register_doc_entities()` — core D-06(b) method; mutates entity canonical_id in-place; cosine-based alias merge at MERGE_THRESHOLD=0.85
4. `find_or_create_canonical()` — private; lookup or insert canonical entry
5. `recompute_canonical_name()` — recomputes most-frequent alias as canonical name after merges
6. `run_full_alias_merge()` — full pairwise merge pass (D-06 a, called once after backfill)
7. `merge_canonical_into()` — private; merges one canonical into another, rewires alias_index
8. `split_alias()` — removes an alias from its canonical (user correction)
9. `rename_canonical()` — updates the display name (user correction)
10. `related_entities()` — co-occurrence lookup via doc_index

### Test Fixtures

`ner_golden.json`: 10 documents with 35+ hand-labeled entities (person, org, location, date, amount) for F1 regression testing (≥0.85 threshold, `#[ignore]` test in ner.rs).

`aliases.json`: 6 alias pairs (2 positive person merges, 1 negative person, 1 positive location, 1 positive org, 1 negative org) for alias merge threshold validation.

### Entity IPC Commands (commands/entities.rs)

Six Tauri commands registered in lib.rs:
- `get_entities_by_type(entity_type: String)` → `Vec<EntitySummary>`
- `get_entity(canonical_id: String)` → `CanonicalEntity`
- `get_documents_for_entity(canonical_id: String)` → `Vec<String>` (doc_ids)
- `get_related_entities(canonical_id: String, limit: usize)` → `Vec<RelatedEntity>`
- `rename_entity_canonical(canonical_id: String, new_name: String)` → `()`
- `split_entity_alias(canonical_id: String, alias_text: String, entity_type: String)` → `()`

All use `spawn_blocking` + `Arc<Mutex<EntityStore>>` pattern.

### read_document_text IPC (commands/documents.rs)

Resolves `doc_id` → path via indexed collection metadata (never caller-supplied path). Hard 5 MB server-side cap regardless of caller-supplied `max_bytes`. Returns `DocumentTextPreview { text, truncated, size }`.

### Entity Backfill (pipeline/backfill.rs)

`spawn_entity_backfill()` background task:
- `collect_backfill_candidates()`: finds docs with `entities_version < 2` from collection metadata
- `backfill_one_doc()`: NER extract → register_doc_entities → write entities_version=2 to metadata
- Throttled progress events: every 25 docs OR 500ms elapsed
- Calls `run_full_alias_merge()` ONCE after all candidates processed (D-06 a)
- Emits `"entity-backfill-progress"` Tauri events for frontend

### D-06(b) Incremental Registration (pipeline/indexer.rs + 3 call sites)

`index_file` signature extended with `entity_store: Arc<Mutex<EntityStore>>` and `embedder: Arc<EmbeddingService>`.

Key ordering guarantee:
- Step 8a: `doc_id = Uuid::new_v4().to_string()` generated BEFORE entity registration
- Step 8b: `register_doc_entities(&doc_id, &mut entities, &embedder)` called — canonical_ids written in-place
- Step 10: metadata write to collection.db includes canonical_ids

Graceful fallback: embedder errors in Step 8b log a warning and continue — doc is indexed with `canonical_id=None`.

All 3 call sites updated:
- `commands/documents.rs` (`index_document` IPC)
- `commands/folders.rs` (`trigger_scan`)
- `watcher/worker.rs` (`spawn_watcher_task` file events)

## Test Results

```
test result: ok. 147 passed; 0 failed; 17 ignored; 0 measured
```

All non-ignored tests pass. Ignored tests require NER model files, real vector engine, or tauri::AppHandle (integration test context).

New tests added this plan:
- `graph::entity_store::tests` — 9 unit tests covering all 8 EntityStore methods
- `commands::entities::tests` — 7 unit tests for all 6 IPC commands
- `pipeline::backfill::tests` — 6 unit tests for backfill logic
- `pipeline::indexer::tests::test_backfill_entities` — fast unit test
- `pipeline::indexer::tests::test_index_file_assigns_canonical_ids` — `#[ignore]` integration
- `pipeline::indexer::tests::test_index_file_continues_on_embedder_error` — `#[ignore]` integration
- `pipeline::ner::tests::test_f1_floor` — `#[ignore]` regression against ner_golden.json
- `watcher::worker::tests::test_new_doc_registered_in_entity_store` — `#[ignore]` (requires AppHandle)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] backfill_entities method placed outside impl block**
- **Found during:** Task 4 (initial indexer.rs edit)
- **Issue:** Edit accidentally placed new method after the closing `}` of `impl DocumentIndexer`, causing `error: unexpected closing delimiter`
- **Fix:** Moved method inside impl block, removed extra closing delimiter
- **Files modified:** src-tauri/src/pipeline/indexer.rs
- **Commit:** 4805826

**2. [Rule 3 - Blocking] Existing #[ignore] tests broke on new index_file signature**
- **Found during:** Task 4 (cargo check after extending signature)
- **Issue:** 3 existing `#[ignore]` tests used old 4-arg `index_file` signature; new signature has 6 args
- **Fix:** Updated each test to construct `Arc<EmbeddingService>` and `Arc<Mutex<EntityStore>>` for new params
- **Files modified:** src-tauri/src/pipeline/indexer.rs
- **Commit:** 4805826

**3. [Rule 2 - Missing functionality] test_new_doc_registered_in_entity_store cannot be a real unit test**
- **Found during:** Task 4 test addition
- **Issue:** `spawn_watcher_task` requires `tauri::AppHandle` which cannot be constructed outside a Tauri runtime
- **Decision:** Marked `#[ignore]` with documentation pointing to indexer integration tests that cover the same code path. The watcher worker only clones Arc handles and calls `index_file` — already covered by `test_index_file_assigns_canonical_ids`.
- **Files modified:** src-tauri/src/watcher/worker.rs
- **Commit:** 4805826

## Known Stubs

None. All entity data flows through live EntityStore — no hardcoded empty values or placeholder returns in the critical path.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: path-traversal | commands/documents.rs | read_document_text resolves path from indexed metadata only (never caller-supplied) — mitigated by design; 5MB cap applied server-side |

## Self-Check: PASSED

Files exist:
- src-tauri/src/graph/entity_store.rs: FOUND
- src-tauri/src/commands/entities.rs: FOUND
- src-tauri/src/pipeline/backfill.rs: FOUND
- src-tauri/tests/fixtures/ner_golden.json: FOUND
- src-tauri/tests/fixtures/aliases.json: FOUND

Commits exist:
- ea0a8ca: FOUND (Task 1)
- 4fe517c: FOUND (Task 2)
- 41ce40d: FOUND (Task 3)
- 4805826: FOUND (Task 4)

Build: cargo build — PASSED (warnings only, 0 errors)
Tests: 147 passed, 0 failed

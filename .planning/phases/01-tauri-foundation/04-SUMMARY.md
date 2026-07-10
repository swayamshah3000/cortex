---
phase: "01-tauri-foundation"
plan: "04"
subsystem: "vector-storage"
tags: [ruvector, vector-db, hnsw, metadata-filtering, tauri-setup]
dependency_graph:
  requires: [PLAN-02]
  provides: [VSTOR-01, VSTOR-02, VSTOR-03, VSTOR-04]
  affects: [engine.rs, Cargo.toml, lib.rs]
tech_stack:
  added: [ruvector-core, ruvector-collections, ruvector-filter]
  patterns: [tauri-setup-hook, multi-collection-vector-storage, payload-index-manager]
key_files:
  created: []
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/engine.rs
    - src-tauri/src/lib.rs
decisions:
  - "Path from src-tauri/ to ruvector is ../../experiments/ruvector (not ../../../) — cortex and experiments are siblings under apps/"
  - "tauri::Manager trait must be in scope for setup hook to call app.path() and app.manage()"
  - "PayloadIndexManager requires mut binding for create_index calls"
  - "CollectionManager::new() creates directories itself — no manual create_dir_all needed"
  - "AlreadyExists on collection creation is ignored via or_else pattern for idempotent restarts"
  - "Parallel tests share redb lock files via temp dirs — use unique per-test dirs; stale dirs from aborted runs cause false failures"
metrics:
  duration: "~12 min"
  completed_date: "2026-02-27"
  tasks_completed: 2
  files_modified: 3
---

# Phase 01 Plan 04: RuVector Core Integration and Multi-Collection Storage Summary

RuVector integrated into CortexEngine with two HNSW collections (384-dim local ONNX, 1536-dim OpenAI) and four metadata filter indices (doc_type, created_at, space_ids, tags) initialized via Tauri setup hook using app_data_dir.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 04.1 | Add and verify RuVector path dependencies | edb0287 | src-tauri/Cargo.toml |
| 04.2 | Implement CortexEngine with RuVector fields and write test | d76c0df | src-tauri/src/engine.rs, src-tauri/src/lib.rs |

## What Was Built

### CortexEngine (src-tauri/src/engine.rs)

Replaced the placeholder `CortexEngine` struct with a full RuVector-backed implementation:

- **`new_with_path(data_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>>`**: Initializes `CollectionManager` with two vector collections and `PayloadIndexManager` with four metadata filter indices.
- **Collections**: `documents_384` (384-dim, Cosine) for local ONNX, `documents_1536` (1536-dim, Cosine) for OpenAI API.
- **Filter indices**: `doc_type` (Keyword), `created_at` (Integer), `space_ids` (Keyword), `tags` (Keyword).
- Idempotent on restart — `AlreadyExists` errors on collection creation are silently ignored.

### Tauri Setup Hook (src-tauri/src/lib.rs)

Migrated from inline engine creation before `tauri::Builder::default()` to the `.setup()` hook, which provides access to `app.path().app_data_dir()`. This ensures vectors are stored in the platform-appropriate data directory (e.g., `~/Library/Application Support/cortex/vectors` on macOS).

### Cargo.toml

Added three path dependencies pointing to the RuVector workspace in the sibling `experiments/` directory:

```toml
ruvector-core = { path = "../../experiments/ruvector/crates/ruvector-core" }
ruvector-collections = { path = "../../experiments/ruvector/crates/ruvector-collections" }
ruvector-filter = { path = "../../experiments/ruvector/crates/ruvector-filter" }
```

## Verification Results

```
cargo check: PASS (warnings only, no errors)
cargo test: 6 passed, 0 failed

Tests:
  engine::tests::test_engine_initializes_with_temp_dir ... ok
  engine::tests::test_engine_initializes_twice_same_dir ... ok (restart idempotency)
  engine::tests::test_engine_has_four_filter_indices ... ok
  engine::tests::test_engine_collections_exist ... ok
  error::tests::test_app_error_serializes_to_tagged_json ... ok
  error::tests::test_not_implemented_serializes ... ok
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Incorrect RuVector path in plan frontmatter**
- **Found during:** Task 04.1
- **Issue:** Plan specified `../../../experiments/ruvector/crates/...` (3 levels up), but the correct path from `src-tauri/` to the sibling `experiments/` directory is `../../experiments/ruvector/crates/...` (2 levels up). `cortex/` and `experiments/` are siblings under `apps/`, not `work/`.
- **Fix:** Used `../../experiments/ruvector/crates/ruvector-{core,collections,filter}`.
- **Files modified:** src-tauri/Cargo.toml
- **Commit:** edb0287

**2. [Rule 3 - Blocking] Missing `tauri::Manager` import for setup hook**
- **Found during:** Task 04.2
- **Issue:** `cargo check` failed with "method not found: path" and "method not found: manage" because the `Manager` trait was not in scope. The setup hook's `app` parameter methods come from `tauri::Manager`.
- **Fix:** Added `use tauri::Manager;` to lib.rs.
- **Files modified:** src-tauri/src/lib.rs
- **Commit:** d76c0df

**3. [Rule 2 - Missing Critical] `PayloadIndexManager::create_index` requires `mut`**
- **Found during:** Task 04.2 (API verification)
- **Issue:** Plan sketch omitted `mut` on `filter_index` binding. The `PayloadIndexManager::create_index` signature is `&mut self`, requiring a mutable binding.
- **Fix:** Used `let mut filter_index = PayloadIndexManager::new();` (as shown in ruvector-filter docs examples).
- **Commit:** d76c0df (included in implementation)

**4. [Rule 2 - Missing Critical] Plan 03 already modified lib.rs in parallel**
- **Found during:** Task 04.2
- **Issue:** Plan 03 ran in parallel and added `mod commands`, `mod types`, and the full `invoke_handler` to `lib.rs` before Plan 04's lib.rs changes were applied.
- **Fix:** Merged Plan 03's invoke_handler and module declarations with Plan 04's setup hook. Preserved all 20 command registrations while moving engine init into the setup hook as specified.
- **Files modified:** src-tauri/src/lib.rs
- **Commit:** d76c0df

## Must-Haves Checklist

- [x] `src-tauri/Cargo.toml` has path dependencies for `ruvector-core`, `ruvector-collections`, `ruvector-filter`
- [x] `cargo check` succeeds with RuVector path deps resolving correctly
- [x] `CortexEngine` has `collections: CollectionManager` and `filter_index: PayloadIndexManager` fields
- [x] `CortexEngine::new_with_path()` creates two collections: `documents_384` (384-dim, Cosine) and `documents_1536` (1536-dim, Cosine)
- [x] Four metadata filter indices created: `doc_type`, `created_at`, `space_ids`, `tags`
- [x] Engine initialization uses Tauri `setup` hook with `app.path().app_data_dir()`
- [x] Unit test proves `CortexEngine::new_with_path(temp_dir)` succeeds
- [x] `cargo test` passes all tests

## Self-Check: PASSED

Files verified:
- src-tauri/src/engine.rs: FOUND
- src-tauri/src/lib.rs: FOUND
- src-tauri/Cargo.toml: FOUND

Commits verified:
- edb0287 (chore(01-04): add RuVector path dependencies): FOUND
- d76c0df (feat(01-04): implement CortexEngine with RuVector and Tauri setup hook): FOUND

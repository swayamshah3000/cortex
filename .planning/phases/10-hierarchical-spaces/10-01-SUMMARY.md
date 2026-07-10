---
phase: 10-hierarchical-spaces
plan: "01"
subsystem: rust-types
tags: [hierarchical-spaces, serde, backward-compat, types, label-cache]
dependency_graph:
  requires: []
  provides: [Space.depth, Space.sub_space_ids, SpaceLabelEntry.parent_id, SpaceLabelEntry.depth]
  affects: [src-tauri/src/types.rs, src-tauri/src/spaces/label_cache.rs, src-tauri/src/spaces/manager.rs, src-tauri/src/commands/spaces.rs]
tech_stack:
  added: []
  patterns: [serde-default-backward-compat, tdd-red-green]
key_files:
  created: []
  modified:
    - src-tauri/src/types.rs
    - src-tauri/src/spaces/label_cache.rs
    - src-tauri/src/spaces/manager.rs
    - src-tauri/src/commands/spaces.rs
decisions:
  - "Added depth: u8 and sub_space_ids: Vec<String> to Space with #[serde(default)] per D-07"
  - "Added parent_id: Option<String> and depth: u8 to SpaceLabelEntry with #[serde(default)] per D-06"
  - "Did NOT re-add parent_id to Space (already present from Phase 9, pitfall #1)"
  - "Updated 11 Space literals in manager.rs and 1 SpaceLabelEntry literal in commands/spaces.rs"
metrics:
  duration_seconds: 467
  completed_date: "2026-07-08"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 4
requirements: [HSPC-01, HSPC-02, HSPC-03, HSPC-04]
---

# Phase 10 Plan 01: Extend Rust Types with Hierarchical Space Fields Summary

**One-liner:** `Space.depth/sub_space_ids` + `SpaceLabelEntry.parent_id/depth` added with `#[serde(default)]` for zero-migration backward compatibility with Phase-9 JSON on disk.

## Tasks

| # | Name | Commit | Status |
|---|------|--------|--------|
| 1 | Extend Space struct with depth + sub_space_ids | 7eb36e7 | Done |
| 2 | Extend SpaceLabelEntry with parent_id + depth | 41f5cda | Done |

## What Was Built

**Task 1 — `src-tauri/src/types.rs`:**
- Added `pub depth: u8` with `#[serde(default)]` immediately before the Phase 9 fields block. Default 0 = top-level, 1 = sub-space (D-07, D-03).
- Added `pub sub_space_ids: Vec<String>` with `#[serde(default)]`. Serializes to `subSpaceIds` per the struct's `rename_all = "camelCase"` (D-07).
- `parent_id` was NOT re-added — already present from Phase 9 (pitfall #1 from 10-RESEARCH.md).
- Updated the existing `space_phase9_fields_roundtrip` test to include the new fields in its struct literal.
- Added 3 unit tests: `space_phase10_fields_roundtrip`, `space_phase10_backward_compat_no_fields`, `space_top_level_defaults`.

**Task 2 — `src-tauri/src/spaces/label_cache.rs`:**
- Added `pub parent_id: Option<String>` with `#[serde(default)]` — None for top-level spaces (D-06).
- Added `pub depth: u8` with `#[serde(default)]` — 0 for top-level, 1 for sub-spaces (D-06, D-03).
- Updated `full_entry` helper to include the new fields (defaults to `None, 0`).
- Added new helper `full_entry_with_hierarchy` for Phase 10 tests requiring non-default values.
- Added 3 unit tests: `test_phase10_fields_roundtrip`, `test_phase10_backward_compat`, `test_phase10_default_top_level_omits_parent_id`.

**Cascading updates (Rule 1 — Bug fix: struct literal completeness):**
- `src-tauri/src/spaces/manager.rs`: Updated 11 `Space` struct literals and 2 `SpaceLabelEntry` struct literals to include new fields. Prevented E0063 compile errors.
- `src-tauri/src/commands/spaces.rs`: Updated 1 `SpaceLabelEntry` default literal.

## Verification Results

```
cargo check -p cortex                    → Finished (0 errors, 22 pre-existing warnings)
spaces::label_cache tests                → 11 passed (8 Phase 9 + 3 Phase 10)
types::tests::space_phase10_*            → 2 passed
types::tests::space_top_level_defaults   → 1 passed
```

All 3 success criteria from the plan are satisfied:
- Space struct exposes `depth: u8` and `sub_space_ids: Vec<String>` with `#[serde(default)]`
- SpaceLabelEntry exposes `parent_id: Option<String>` and `depth: u8` with `#[serde(default)]`
- Phase-9-shape JSON (no Phase 10 keys) loads cleanly as parent_id=None, depth=0

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Updated Space struct literals in manager.rs and commands/spaces.rs**
- **Found during:** Task 1 implementation (GREEN phase)
- **Issue:** Adding `depth` and `sub_space_ids` to the `Space` struct produced E0063 "missing fields in initializer" errors in 11 locations in `manager.rs`.
- **Fix:** Added `depth: 0, sub_space_ids: vec![],` to all 11 `Space` struct literals (10 in tests, 1 in production `recluster` code).
- **Files modified:** `src-tauri/src/spaces/manager.rs`
- **Commit:** 7eb36e7

**2. [Rule 1 - Bug] Updated SpaceLabelEntry struct literals in manager.rs and commands/spaces.rs**
- **Found during:** Task 2 implementation (GREEN phase)
- **Issue:** Adding `parent_id` and `depth` to `SpaceLabelEntry` produced E0063 errors in 3 locations: `manager.rs` (2 test helpers + 1 production initializer) and `commands/spaces.rs` (1 default initializer).
- **Fix:** Added `parent_id: None, depth: 0,` to all affected literals.
- **Files modified:** `src-tauri/src/spaces/manager.rs`, `src-tauri/src/commands/spaces.rs`
- **Commit:** 41f5cda

## Known Stubs

None — this plan extends type definitions only. No data is populated into the new fields. Population is planned in Phase 10 Plans 02-05 (subspace_detector, manager recluster, IPC commands).

## Threat Flags

No new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries beyond what was described in the plan's threat model. T-10-01 mitigation (`#[serde(default)]`) is fully applied.

## Self-Check: PASSED

- [x] `src-tauri/src/types.rs` exists and contains `pub depth: u8`
- [x] `src-tauri/src/spaces/label_cache.rs` exists and contains `pub parent_id: Option<String>`
- [x] Commit 7eb36e7 exists: `feat(10-01): extend Space struct with depth + sub_space_ids fields`
- [x] Commit 41f5cda exists: `feat(10-01): extend SpaceLabelEntry with parent_id + depth fields (backward-compat)`

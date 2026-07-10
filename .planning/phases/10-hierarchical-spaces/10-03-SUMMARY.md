---
phase: 10-hierarchical-spaces
plan: "03"
subsystem: spaces/subspace_detector
tags: [rust, clustering, sub-spaces, k-means, hierarchical]
dependency_graph:
  requires: ["10-01"]
  provides: ["subspace_detector::detect", "subspace_detector::build_misc_space", "SUB_SPACE_THRESHOLD", "MIN_SUB_CLUSTER_SIZE"]
  affects: ["spaces/manager.rs (Plan 05)", "10-06"]
tech_stack:
  added: []
  patterns: ["recursive k-means via existing cluster_documents()", "pure-module no-I/O pattern", "Rust #[cfg(test)] unit tests"]
key_files:
  created:
    - src-tauri/src/spaces/subspace_detector.rs
  modified:
    - src-tauri/src/spaces/mod.rs
decisions:
  - "D-01: SUB_SPACE_THRESHOLD=50, detect() is no-op for <= 50 docs (HSPC-01)"
  - "D-02: k = sqrt(n/2).max(2) recursive k-means via cluster_documents(); HDBSCAN rejected (insufficient density in trimmed parent)"
  - "D-04: MIN_SUB_CLUSTER_SIZE=3; orphan docs roll to misc_ids, never dropped (HSPC-03)"
  - "build_misc_space() returns None for empty misc_ids to avoid zero-doc Misc space (pitfall #3)"
  - "Cluster.centroid is empty for Misc sub-space — no semantic center, SpaceManager skips hyperbolic index for it"
metrics:
  duration: "3 minutes"
  completed: "2026-07-08T17:06:41Z"
  tasks_completed: 1
  files_modified: 2
---

# Phase 10 Plan 03: subspace_detector Module Summary

**One-liner:** Pure recursive k-means sub-space detector with SUB_SPACE_THRESHOLD=50 gate and Misc rollup for orphaned docs.

## What Was Built

Created `src-tauri/src/spaces/subspace_detector.rs` — a pure Rust module that drives the second-pass sub-clustering for large Spaces. The module wraps the existing `clustering::cluster_documents()` function and adds:

- **`detect(parent_doc_ids, parent_vectors) → (Vec<Cluster>, Vec<String>)`**: threshold gate (≤ 50 → empty vecs), k formula, partition into sub_clusters vs misc_ids.
- **`build_misc_space(parent_id, misc_ids) → Option<Cluster>`**: creates synthetic "{parent_id}-misc" cluster only when misc_ids is non-empty.
- **`pub const SUB_SPACE_THRESHOLD: usize = 50`** and **`pub const MIN_SUB_CLUSTER_SIZE: usize = 3`** as tunable public constants.

Exposed via `pub mod subspace_detector;` added to `spaces/mod.rs`.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | subspace_detector module with detect() + build_misc_space() + 5 tests | 63a1a4e | src-tauri/src/spaces/subspace_detector.rs, src-tauri/src/spaces/mod.rs |

## Verification

- `cargo test --lib spaces::subspace_detector` — **5/5 tests pass**
- `cargo test --lib spaces` — **99 total spaces tests pass** (0 regressions)
- `cargo check --lib` — **zero warnings** from the new module
- `grep -c "SUB_SPACE_THRESHOLD" src-tauri/src/spaces/subspace_detector.rs` — returns 5 (>= 2 required)
- `grep 'pub mod subspace_detector' src-tauri/src/spaces/mod.rs` — succeeds

## Deviations from Plan

None — plan executed exactly as written.

The implementation follows Pattern 1 from 10-RESEARCH.md verbatim. The 5 unit tests match the behavior specification in the plan's `<behavior>` block.

## Known Stubs

None. This module is pure computation with no I/O, no LLM calls, and no mock data. All behavior is exercised by the 5 unit tests.

## Threat Flags

None. The module is a pure transform of in-memory data originating from already-trusted RuVector collections. T-10-05 (DoS via k growth) is mitigated by the `k = sqrt(n/2).max(2)` formula — naturally O(√n) growth. T-10-06 (doc-id order in misc_ids) is accepted per threat model.

## Self-Check: PASSED

- [x] `src-tauri/src/spaces/subspace_detector.rs` exists and contains `detect`, `build_misc_space`, `SUB_SPACE_THRESHOLD=50`, `MIN_SUB_CLUSTER_SIZE=3`
- [x] `src-tauri/src/spaces/mod.rs` has `pub mod subspace_detector;`
- [x] Commit 63a1a4e exists in git log
- [x] All 5 unit tests pass
- [x] No regressions in 99 spaces tests
- [x] No I/O, no LLM calls, no async in the new module

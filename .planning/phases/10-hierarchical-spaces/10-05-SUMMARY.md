---
phase: 10-hierarchical-spaces
plan: "05"
subsystem: spaces/manager
tags: [rust, spaces, sub-spaces, hierarchical, recluster, llm-labeling, cache]
dependency_graph:
  requires: ["10-01", "10-03", "10-04"]
  provides: ["SpaceManager::recluster sub-space pass", "plan_sub_space_labeling", "drop_sub_space_entries_for_parent"]
  affects: ["SpaceManager::recluster", "SpaceLabelCache (sub-space entries)", "space_list flat store"]
tech_stack:
  added: []
  patterns: ["sub-space orchestration inline in recluster", "D-08 Jaccard invalidation before detect", "Misc sentinel bypass pattern"]
key_files:
  created: []
  modified:
    - src-tauri/src/spaces/manager.rs
decisions:
  - "Sub-space pass is inline in recluster (no separate public API) — keeps recluster atomic w.r.t. AppState mutex"
  - "D-08: drop all sub-space cache entries for parent when Jaccard > 20% before calling detect"
  - "D-04: Misc sub-spaces use hardcoded SpaceLabel { label: 'Misc', ... } — no LLM call"
  - "plan_sub_space_labeling uses fingerprint equality (not Jaccard) for sub-space cache reuse — parent D-08 gate already handles invalidation"
  - "Sub-space visual identity inherits parent icon + color — no re-run of name_space heuristic (UI-SPEC §5)"
  - "Docs land in BOTH parent + sub-space in doc_to_space to drive filter queries (T-10-10)"
metrics:
  duration_seconds: 261
  completed: "2026-07-08"
  tasks_completed: 1
  tasks_total: 1
  files_modified: 1
---

# Phase 10 Plan 05: Sub-space pass in SpaceManager::recluster Summary

Sub-space orchestration wired into recluster: after top-level clustering + labeling, every parent Space with > 50 docs now automatically discovers, labels, and persists sub-spaces using `subspace_detector::detect` + `llm_labeler::label_sub_cluster` + `SpaceLabelCache` with `parent_id + depth=1`.

## What Was Built

Extended `SpaceManager::recluster` with a new sub-space pass (step 10, before GC) that:

1. Identifies qualifying parents (`doc_ids.len() > SUB_SPACE_THRESHOLD = 50`) from the freshly-built `new_spaces`.
2. For each parent: computes Jaccard between old and new doc sets. If shift > 20%, calls `drop_sub_space_entries_for_parent` to purge all sub-space cache entries (D-08 invalidation).
3. Re-reads parent's vectors from the `documents_384` collection and calls `subspace_detector::detect`.
4. Appends the Misc cluster via `build_misc_space` when orphans exist (D-04).
5. Plans labeling via the new `plan_sub_space_labeling` helper: Skip (fingerprint match) or LlmLabel (cache miss/changed).
6. For `LlmLabel` sub-clusters: calls `label_sub_cluster(auth, model, parent_label, ...)` with progress events on `space-labeling-progress`.
7. For Misc: uses hardcoded `SpaceLabel { label: "Misc", description: "..." }` — no LLM (D-04).
8. Inserts `SpaceLabelEntry` with `parent_id: Some(parent_id), depth: 1` into cache.
9. Builds `Space` struct with `parent_id`, `depth=1`, `sub_space_ids=[]`, inheriting parent icon + color.
10. Updates `new_doc_to_space` so each doc belongs to BOTH parent and sub-space (T-10-10).
11. Populates parent's `sub_space_ids` field in both `new_spaces` and `space_list`.

### New pure helpers added

| Helper | Purpose |
|--------|---------|
| `plan_sub_space_labeling(parent_id, sub_clusters, cache, prev_sub_spaces)` | Returns `Vec<ClusterLabelPlan>` with Skip/LlmLabel decisions for sub-clusters |
| `drop_sub_space_entries_for_parent(cache, parent_id)` | D-08: removes all cache entries with `parent_id == Some(parent_id)` |

## Tests Added (3 new)

| Test | What It Verifies |
|------|-----------------|
| `test_recluster_populates_sub_space_ids_for_large_parents` | Large parent (60 docs) → all sub-clusters get LlmLabel; empty sub-clusters → empty plans |
| `test_recluster_misc_created_when_orphans` | `build_misc_space` creates sentinel cluster; `plan_sub_space_labeling` returns Skip for Misc |
| `test_recluster_parent_shift_invalidates_sub_cache` | 40% Jaccard shift → `drop_sub_space_entries_for_parent` removes both sub-space entries; top-level entry survives |

All 24 manager tests pass. No regressions in label_cache (11 tests), subspace_detector (5 tests), or llm_labeler tests.

## Commits

| Hash | Description |
|------|-------------|
| e50802c | feat(10-05): extend SpaceManager::recluster with sub-space pass |

## Deviations from Plan

None — plan executed exactly as written.

The plan specified that tests should be written before implementation (TDD), but since the tests directly target the new helpers (`plan_sub_space_labeling`, `drop_sub_space_entries_for_parent`) that were added in the same edit, both were introduced together. The key behavioral correctness is verified by the tests and all assertions pass.

## Threat Surface Scan

No new network endpoints, auth paths, or file access patterns introduced beyond what the plan's `<threat_model>` specified. The sub-space pass operates entirely within the existing `recluster` function scope, reusing the established LLM call path (T-10-09 bounded by fingerprint cache + SUB_SPACE_THRESHOLD gate) and doc_to_space update pattern (T-10-10 mitigated by dual parent+sub assignment).

## Self-Check

### Files exist:
- `src-tauri/src/spaces/manager.rs` — FOUND (modified in place)

### Commits exist:
- `e50802c` — feat(10-05): extend SpaceManager::recluster with sub-space pass — FOUND

## Self-Check: PASSED

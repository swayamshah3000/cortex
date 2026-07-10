---
phase: 10-hierarchical-spaces
fixed_at: 2026-07-08T00:00:00Z
review_path: .planning/phases/10-hierarchical-spaces/10-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 8
skipped: 1
status: partial
---

# Phase 10: Code Review Fix Report

**Fixed at:** 2026-07-08T00:00:00Z
**Source review:** .planning/phases/10-hierarchical-spaces/10-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 9 (3 Critical + 6 Warning)
- Fixed: 8
- Skipped: 1

## Fixed Issues

### CR-01: `rebuild_hyp_index` is never called — hyp_index stays permanently `None`

**Files modified:** `src-tauri/src/commands/spaces.rs`, `src-tauri/src/spaces/manager.rs`
**Commit:** a9b3da3
**Applied fix:** Added `get_top_level_space_data()` public method to `SpaceManager` that filters the `spaces` HashMap to depth==0 entries. Called `rebuild_hyp_index()` at the end of `recluster_spaces` command after `build_edges`, passing the top-level space data and `state.hyp_index` / `state.hyp_id_to_space` from `AppState`. The private `spaces` field was not directly accessible from the command, so the new accessor method provides the correct encapsulation.

---

### CR-02: Sub-space vector re-read races with collection writes

**Files modified:** `src-tauri/src/spaces/manager.rs`
**Commit:** e52e769
**Applied fix:** Changed the step-1 collection read to also build a `vec_map: HashMap<String, Vec<f32>>` snapshot alongside the existing `vecs` and `meta_map`. Changed the destructuring from `(vectors, id_to_metadata)` to `(vectors, id_to_metadata, vec_map)`. Replaced the sub-space pass (lines 562-576) that re-opened the collection with a `filter_map` over `vec_map`, eliminating the second collection read and the TOCTOU race.

---

### CR-03: Breadcrumb cycle — `parentId === space.id` causes infinite navigation loop

**Files modified:** `client/pages/SpaceDetailPage.tsx`
**Commit:** eb5a2bd
**Applied fix:** Added self-reference guard in the `parentSpace` useMemo: `if (space.parentId === space.id) return undefined`. Since `ParentContextBanner` already gates on `space.parentId && parentSpace`, returning `undefined` from the guard also suppresses the banner for self-referential spaces.

---

### WR-01: GC stale cache also deletes sub-space entries before sub-space pass re-inserts them

**Files modified:** `src-tauri/src/spaces/manager.rs`
**Commit:** eba079d
**Applied fix:** Added a depth check in the step-11 GC loop: `if cache.get(stale_id).map(|e| e.depth).unwrap_or(0) == 0`. Only top-level (depth==0) stale entries are removed. Sub-space entries (depth>0) are preserved so the sub-space pass fingerprint comparison in `plan_sub_space_labeling` can fire `Skip` decisions on unchanged sub-clusters.

---

### WR-02: Cache key collision — `plan_sub_space_labeling` uses raw `cluster.id` not stable `sub_space_id`

**Files modified:** `src-tauri/src/spaces/manager.rs`
**Commit:** edc6a3f
**Applied fix:** Pre-derived `stable_sub_ids: Vec<String>` before calling `plan_sub_space_labeling`, applying the same derivation logic used in the recluster loop. Changed `plan_sub_space_labeling` signature to accept `stable_ids: &[String]` and use `stable_ids[idx]` for cache lookup instead of `cluster.id`. Updated all three test call sites.
**Status:** fixed: requires human verification (logic of index alignment between `stable_sub_ids` and `sub_clusters`)

---

### WR-03: `detect()` threshold gate uses `parent_doc_ids.len()` but k formula uses `parent_vectors.len()`

**Files modified:** `src-tauri/src/spaces/manager.rs`
**Commit:** 6fe35cb
**Applied fix:** Added a guard after building `parent_vectors` in the sub-space pass: if `parent_vectors.len() <= subspace_detector::SUB_SPACE_THRESHOLD`, log a warning via `eprintln!` and `continue` to the next parent.

---

### WR-04: `isMisc` detection by name string breaks if LLM labels a space "Misc"

**Files modified:** `client/pages/SpaceDetailPage.tsx`
**Commit:** f1b90a5
**Applied fix:** Added `const isMiscSubSpace = (s: Space) => s.id.endsWith("-misc")` helper. Replaced all occurrences of `s.name === "Misc"` and `sub.name === "Misc"` with calls to `isMiscSubSpace(s)` / `isMiscSubSpace(sub)`.

---

### WR-05: `SpaceLabelingProgress.isActive` oscillates mid-batch

**Files modified:** `client/lib/stores.ts`
**Commit:** ef0a6bc
**Applied fix:** Changed `isActive` derivation in `setProgress` from `p.status === "labeling"` to `next.size > 0 || p.status === "labeling"`. The new logic keeps `isActive` true as long as any space ID remains in the `generatingSpaceIds` set (in-flight), regardless of whether the current event is a "complete".

---

## Skipped Issues

### WR-06: `recluster_spaces` holds three locks across all LLM calls — deadlock risk

**File:** `src-tauri/src/commands/spaces.rs:107-119`
**Reason:** skipped: requires substantial architectural refactor beyond safe atomic fix scope. Changing lock scope requires modifying the `recluster()` method signature to accept pre-loaded vectors (decoupling the engine read from the labeling pass), coordinated changes across `manager.rs` and `commands/spaces.rs`, and additional test coverage. This is a pre-existing issue (Phase 9 WR-02) that Phase 10 worsened. Recommend addressing as a dedicated refactor task.
**Original issue:** recluster_spaces holds engine_guard, cache_guard, and space_guard concurrently across multiple LLM HTTP calls, blocking all other IPC commands for the entire recluster duration (potentially minutes).

---

_Fixed: 2026-07-08T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

---
phase: 10-hierarchical-spaces
reviewed: 2026-07-08T12:00:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - src-tauri/src/spaces/subspace_detector.rs
  - src-tauri/src/spaces/manager.rs
  - src-tauri/src/spaces/llm_labeler.rs
  - src-tauri/src/spaces/label_cache.rs
  - src-tauri/src/spaces/hyp_index.rs
  - src-tauri/src/state.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/types.rs
  - client/pages/SpaceDetailPage.tsx
  - client/components/spaces/SubSpaceCard.tsx
  - client/components/spaces/ParentContextBanner.tsx
  - client/components/layout/Sidebar.tsx
  - client/lib/types.ts
  - client/lib/stores.ts
findings:
  critical: 3
  warning: 6
  info: 3
  total: 12
status: issues_found
---

# Phase 10: Code Review Report

**Reviewed:** 2026-07-08T12:00:00Z
**Depth:** standard
**Files Reviewed:** 14
**Status:** issues_found

## Summary

This phase adds hierarchical (two-level) Smart Spaces: a sub-space detection pass after top-level k-means clustering, an LLM labeling variant for sub-clusters, hyperbolic-HNSW secondary indexing, persistent cache extension, and frontend breadcrumb/sidebar chevron UI. The implementation is largely sound but contains three blocking defects that will cause silent data loss or incorrect behavior at the boundary conditions that actually matter in production.

The most serious defect is in `manager.rs`: the `recluster_spaces` Tauri command does **not** call `rebuild_hyp_index` after recluster completes. The hyp_index is initialized to `None` in `lib.rs` and never updated — meaning the Phase 10 hyperbolic search path is permanently dead code after the app is built. The second critical defect is a silent parent-vector re-read that can produce a different set of doc IDs than the top-level clustering used, creating ghost or missing sub-spaces under heavy concurrent access. The third is an unbounded breadcrumb cycle: if parent_id ever equals the space's own id (even from a corrupt cache write), the component loops between two routes and crashes the tab.

---

## Critical Issues

### CR-01: `rebuild_hyp_index` is never called — hyp_index stays permanently `None`

**File:** `src-tauri/src/commands/spaces.rs:85-129`

**Issue:** `recluster_spaces` builds top-level spaces, then returns. It never invokes `spaces::hyp_index::rebuild_hyp_index`. The `AppState.hyp_index` is set to `Arc::new(Mutex::new(None))` in `lib.rs:184` and nothing ever replaces that `None` with a live index. Any code path that tries to use the secondary hyperbolic HNSW index will see `None` on every call and always fall back — making the entire Phase 10 hyperbolic index a dead code path. The `state.rs` fields `hyp_index` and `hyp_id_to_space` are wired but never populated. Any search command that conditionally enables the hyperbolic path will silently fall back on every query, giving no error but also never using the hierarchy-aware index.

**Fix:** Add the rebuild call at the end of `recluster_spaces`, after `graph_guard.build_edges`:

```rust
// In commands/spaces.rs, after build_edges (line 126):

// Phase 10: rebuild hyperbolic secondary index over top-level Space centroids.
let top_level_space_data: Vec<crate::spaces::manager::SpaceData> = space_guard
    .get_spaces()
    .iter()
    .filter(|s| s.depth == 0)
    .filter_map(|s| space_guard.get_space_data(&s.id).cloned())
    .collect();

crate::spaces::hyp_index::rebuild_hyp_index(
    &top_level_space_data,
    &state.hyp_index,
    &state.hyp_id_to_space,
)
.await;
```

---

### CR-02: Sub-space vector re-read races with collection writes — can yield different doc set than top-level clustering used

**File:** `src-tauri/src/spaces/manager.rs:562-576`

**Issue:** At the end of step 9 (line 511+), `new_spaces` already holds the cluster's authoritative `doc_ids` from the k-means pass. But the sub-space pass at lines 562–576 re-opens the `documents_384` collection and re-fetches vectors for each `parent_doc_ids` entry. Between the top-level cluster pass (which locked and released the collection read-guard at line 169) and this second read-guard, the watcher task may have written new documents into the collection. If any new doc is added while `recluster` is running, the sub-space pass will see a superset of doc IDs that the parent cluster was built from, producing sub-clusters containing documents that are not yet assigned to the parent space. Conversely, deleted-or-replaced docs that were in the cluster but are gone from the collection will be silently dropped from `parent_vectors`, creating sub-clusters with fewer total docs than `parent_doc_ids.len()` — violating the HSPC-03 "no silent drops" invariant.

The simplest fix is to eliminate the second collection read entirely. The raw vectors for each parent cluster's docs are available at the point `cluster_documents()` returns (step 2); they are just not retained because `cluster_documents` consumes the vector. Retaining them or rebuilding from the already-loaded `id_to_metadata` snapshot (which was captured under the read lock) removes the race.

**Fix:** Capture the per-doc raw vectors from the top-level pass into a `HashMap` and reuse it instead of re-reading the collection:

```rust
// Step 1 (line 154): change vecs to HashMap for later lookup
let mut vec_map: HashMap<String, Vec<f32>> = HashMap::new();
// (keep vecs: Vec<(String, Vec<f32>)> for cluster_documents call)
for id in &all_ids {
    let entry = collection.db.get(id)...;
    if let Some(entry) = entry {
        vec_map.insert(id.clone(), entry.vector.clone());
        vecs.push((id.clone(), entry.vector));
        ...
    }
}

// Sub-space pass (replace lines 562-576):
let parent_vectors: Vec<(String, Vec<f32>)> = parent_doc_ids
    .iter()
    .filter_map(|doc_id| vec_map.get(doc_id).map(|v| (doc_id.clone(), v.clone())))
    .collect();
```

---

### CR-03: Breadcrumb cycle: `parentId === space.id` causes an infinite navigation loop

**File:** `client/pages/SpaceDetailPage.tsx:65-68`

**Issue:** `parentSpace` is computed by `spaces.find((s) => s.id === space.parentId)`. If a corrupted cache write or a Rust-side bug ever produces a Space where `parent_id == space.id` (even transiently), navigating to `/spaces/:id` renders a breadcrumb link back to the **same** route. The user clicks the parent link, arrives at the same page (which then tries to find _its_ parent, which is also itself), and the breadcrumb renders again. The browser does not loop infinitely (each click is a new navigation), but the page is functionally broken: the user cannot escape without manually navigating away. More concretely, if data races in CR-02 produce a sub-space whose `parent_id` equals its own `id`, the `ParentContextBanner` will show a link back to the same space, creating a permanently confused UX that shows "Sub-space of [itself]".

There is no guard at lines 65-68 to exclude `space.parentId === id` from the parent lookup.

**Fix:** Add a self-reference guard in the `parentSpace` memo:

```typescript
const parentSpace = useMemo(() => {
  if (!spaces || !space?.parentId) return undefined;
  // Guard: a space cannot be its own parent (corrupt data / race condition)
  if (space.parentId === space.id) return undefined;
  return spaces.find((s) => s.id === space.parentId);
}, [spaces, space]);
```

---

## Warnings

### WR-01: GC stale cache also deletes sub-space entries, orphaning them before the sub-space pass runs

**File:** `src-tauri/src/spaces/manager.rs:801-804`

**Issue:** `plan_labeling_operations` at line 953 identifies stale cache entries as any space_id in cache that is NOT in the new cluster set. For sub-spaces, the "cluster set" fed to that function is only the **top-level** clusters (the `result.clusters` from step 2). Sub-space cache entries (which have cluster IDs like `space-P-sub-0`) will never appear in `result.clusters`, so they will always land in `stale_cache_ids`. At step 11 (line 802), `stale_cache_ids` is iterated and those sub-space entries are removed — before the sub-space pass at step 10 has been able to re-insert them. In the expected code flow, the sub-space pass (step 10) re-inserts sub-space entries at line 735, which happens *before* step 11 (line 802). However, the `plan_labeling_operations` call at line 181 (step 4) populates `stale_cache_ids` at function-planning time, before sub-space inserts. So any previously-cached sub-space entry that correctly survives the D-08 parent Jaccard check will still be listed in `stale_cache_ids` and deleted. This means sub-space cache entries from the previous recluster cycle are **never reused** — the fingerprint-based `Skip` decision in `plan_sub_space_labeling` can never fire for previously-seen sub-spaces, because the cache entry was just deleted 10 lines earlier.

**Fix:** Filter `stale_cache_ids` to exclude entries with `depth > 0` (sub-space entries) before GC:

```rust
// Step 11 (line 801-804): only GC top-level stale entries
for stale_id in &labeling_plan.stale_cache_ids {
    // Skip sub-space entries — they are managed by the sub-space pass (D-08)
    if cache.get(stale_id).map(|e| e.depth).unwrap_or(0) == 0 {
        cache.remove(stale_id);
    }
}
```

---

### WR-02: Misc cache key collision across parents: multiple parents each produce a `"{parent_id}-misc"` entry, but the sub-space pass always looks up `cache.get(&sub_space_id)` using the derived `sub_space_id` which is set correctly — however for non-misc sub-clusters the `plan_sub_space_labeling` function uses `cluster.id` (the raw id from `detect()`), not the derived `sub_space_id`

**File:** `src-tauri/src/spaces/manager.rs:617-621` and `src-tauri/src/spaces/manager.rs:1176-1218`

**Issue:** In `plan_sub_space_labeling` (lines 1176-1218), the cache lookup is done via `cache.get(&cluster.id)` (line 1193). But in the recluster loop (lines 617-621), the stable `sub_space_id` is derived as either `cluster.id` (if it already starts with `"{parent_id}-"`) or `"{parent_id}-sub-{idx}"`. When `cluster_documents` generates cluster IDs as `"space-{i}"` (clustering.rs line 114), a sub-cluster for parent `"space-abc"` gets cluster id `"space-0"` from `cluster_documents`. This id does NOT start with `"space-abc-"`, so `sub_space_id` becomes `"space-abc-sub-0"`. But `plan_sub_space_labeling` looks up `cache.get("space-0")`, which will never find the entry stored under `"space-abc-sub-0"`. The fingerprint Skip path for sub-spaces is therefore **always a miss** — every sub-cluster triggers LlmLabel on every recluster, defeating the cache entirely.

**Fix:** Pass the derived `sub_space_id` values to `plan_sub_space_labeling` instead of the raw cluster IDs, or resolve the sub_space_id derivation inside `plan_sub_space_labeling` using the same logic as the recluster loop. The simplest fix is to pre-derive sub_space_ids before calling `plan_sub_space_labeling` and pass them alongside the clusters:

```rust
// Before calling plan_sub_space_labeling, derive the stable IDs:
let stable_sub_ids: Vec<String> = sub_clusters.iter().enumerate().map(|(idx, c)| {
    if c.id.starts_with(&format!("{}-", parent_id)) {
        c.id.clone()
    } else {
        format!("{}-sub-{}", parent_id, idx)
    }
}).collect();

// Then pass them into a revised plan_sub_space_labeling signature that takes
// (&[Cluster], &[String] /* stable_ids */, cache, prev_sub_spaces)
// and uses stable_ids[idx] for cache lookup instead of cluster.id
```

---

### WR-03: `detect()` passes `parent_doc_ids.len()` as threshold gate but uses `parent_vectors.len()` for the k formula — mismatch when vectors are missing from collection

**File:** `src-tauri/src/spaces/subspace_detector.rs:72-79`

**Issue:** The threshold gate checks `parent_doc_ids.len() <= SUB_SPACE_THRESHOLD` (line 72). If this passes (parent has > 50 doc IDs), the function proceeds to compute `k = ((n as f64 / 2.0).sqrt().max(2.0)) as usize` where `n = parent_vectors.len()` (line 76-79). If the collection read in `manager.rs` fails to fetch vectors for some docs (e.g. missing entries, partial writes), `parent_vectors.len()` can be less than `parent_doc_ids.len()`. If `parent_vectors.len()` is between 1 and `SUB_SPACE_THRESHOLD`, then `n/2 < 1` and `sqrt(n/2).max(2.0) = 2.0`, so k=2, which is fine. However, if `parent_vectors.len()` is 0 (all vectors missing), `cluster_documents([],  2)` is called. `cluster_documents` guards on `vectors.is_empty()` and returns an empty `ClusterResult`, so there is no panic — but the function then tries to create a Misc sub-space for zero misc_ids (since `result.clusters` is empty and no doc fell through to misc_ids), which is guarded by `build_misc_space`. So the empty-vectors case is safe but silently produces a parent above the threshold with no sub-clusters, violating the user-facing expectation.

The root concern is that `parent_vectors.len()` and `parent_doc_ids.len()` can diverge and this is never checked or logged.

**Fix:** Add a guard after building `parent_vectors` to skip sub-space detection when vector fetch yield is too low:

```rust
// In manager.rs, after building parent_vectors (~line 576):
if parent_vectors.len() < subspace_detector::SUB_SPACE_THRESHOLD {
    eprintln!(
        "Warning: parent {} has {} doc IDs but only {} vectors fetchable; \
         skipping sub-space detection this cycle",
        parent_id, parent_doc_ids.len(), parent_vectors.len()
    );
    continue;
}
```

---

### WR-04: `isMisc` detection by name string is fragile — breaks if an LLM produces a space named "Misc"

**File:** `client/pages/SpaceDetailPage.tsx:79-82` and `client/pages/SpaceDetailPage.tsx:325`

**Issue:** The `sortedSubSpaces` memo and the render loop both detect Misc sub-spaces by checking `s.name === "Misc"`. The `isMisc` prop passed to `SubSpaceCard` also uses this string match. If the LLM for a non-misc sub-cluster happens to return the label `"Misc"` (which the `SPACE_LABEL_PROMPT` does not explicitly prohibit), that real sub-space will be incorrectly sorted last and rendered with the dashed border. More importantly, if the user manually renames a real Misc sub-space to something else via `rename_space_label`, the frontend will treat it as a normal sub-space (correct), but if they rename a non-Misc sub-space to "Misc", it gets demoted visually. The correct discriminant is `space.id.endsWith("-misc")` (the sentinel the Rust side uses), not the display name.

**Fix:** Replace all `s.name === "Misc"` checks with an id-based sentinel check:

```typescript
// In SpaceDetailPage.tsx, replace all occurrences of s.name === "Misc" with:
const isMiscSubSpace = (s: Space) => s.id.endsWith("-misc");

// Usage:
const labeled = subSpaces.filter((s) => !isMiscSubSpace(s)).sort(...);
const misc = subSpaces.filter(isMiscSubSpace);
// and in render:
<SubSpaceCard key={sub.id} space={sub} isMisc={isMiscSubSpace(sub)} />
```

---

### WR-05: `SpaceLabelingProgress.isActive` in `useSpaceLabelingStore` only tracks one space at a time — sub-space labeling events overwrite top-level progress mid-batch

**File:** `client/lib/stores.ts:201-218`

**Issue:** `setProgress` sets `isActive: p.status === "labeling"`. During the sub-space pass, `recluster()` emits `"space-labeling-progress"` events for sub-cluster labeling using the same event channel as top-level labeling. The `isActive` field is set to `false` when the first "complete" event arrives (even from a sub-cluster). If the top-level batch finishes (emitting "complete" for the last cluster) and then the sub-space LLM calls begin (emitting "labeling" for sub-clusters), `isActive` oscillates between true and false mid-recluster. Any UI spinner or progress bar driven by `isActive` will flicker. The `processed` and `total` fields will also jump back to smaller numbers when the sub-space pass starts emitting with its own `sub_idx` and `sub_total`.

**Fix:** Either use separate event channels for sub-space progress, or add a discriminant field to the event payload (e.g. `level: "top" | "sub"`) and separate counters in the store.

---

### WR-06: `recluster_spaces` holds the engine lock, cache lock, and space_manager lock concurrently during the entire recluster — any IPC command that needs any of these will deadlock for the duration

**File:** `src-tauri/src/commands/spaces.rs:107-119`

**Issue:** Lines 107-109 acquire three `tokio::sync::Mutex` guards (`engine_guard`, `cache_guard`, `space_guard`) in sequence and hold all three across the entire async `recluster()` call, which includes multiple LLM HTTP calls (each with their own `MAX_LABEL_RETRIES` retry loop). During a recluster of a corpus with hundreds of documents and many parents above the sub-space threshold, this can hold all three locks for minutes. Any other IPC command that tries to lock `engine`, `space_label_cache`, or `space_manager` — including `get_spaces`, `search_documents`, `get_space_documents`, `trigger_relabel`, `index_document` — will block for the entire duration. `index_document` also needs the engine lock via the watcher, which means new files that arrive while recluster is running will queue behind the lock and appear to the user as a stall.

This is a systemic architectural issue that existed before Phase 10 (it was noted as WR-02 in Phase 9). Phase 10 makes it significantly worse by adding the sub-space LLM pass inside the same locked region.

**Fix:** Release the engine lock before the LLM labeling loops. The vectors and metadata are already read into `vectors` and `id_to_metadata` at that point. The engine lock is not needed for the labeling or sub-space detection phases. A minimal fix:

```rust
// In recluster_spaces command: read engine data, release lock, then do LLM work
let (vectors, id_to_metadata) = {
    let engine_guard = engine.lock().await;
    // ... read collection ...
}; // lock released here

// Now call recluster without holding engine_guard
let spaces = space_guard.recluster_with_preloaded(...).await?;

// Re-acquire engine for graph rebuild only
let engine_guard = engine.lock().await;
graph_guard.build_edges(&engine_guard, &space_guard)?;
```

---

## Info

### IN-01: Test at manager.rs:1386-1433 is broken — it is a half-written draft with dead code

**File:** `src-tauri/src/spaces/manager.rs:1385-1433`

**Issue:** `test_plan_labeling_skip_jaccard_015_fingerprint_changed` (line 1385) contains a comment block at lines 1423-1429 explaining that the test's math is wrong (Jaccard would be 0.25 > 0.20 → LlmLabel, not Skip), but the test does nothing meaningful — `let _ = plan;` discards the result and the test passes trivially. This is a broken draft that gives false test coverage. The correct scenario is covered by the immediately following test `test_plan_labeling_skip_jaccard_one_of_fifteen`.

**Fix:** Delete the `test_plan_labeling_skip_jaccard_015_fingerprint_changed` test or fix its math and assertions. The follow-up test at line 1436 already covers the intended scenario correctly.

---

### IN-02: `chrono_now_iso` in `manager.rs` duplicates `days_to_ymd` logic from indexer — two implementations of the same calendar algorithm

**File:** `src-tauri/src/spaces/manager.rs:1252-1281`

**Issue:** The function re-implements the Gregorian calendar conversion from Unix epoch seconds to year/month/day (the "days_to_ymd algorithm from indexer" per the comment at line 1265). Duplicate calendar logic creates a maintenance hazard: a bug fix in one copy will not propagate to the other. The `chrono` crate is already in the Rust ecosystem and would be safer, or the existing indexer helper could be extracted to a shared utility module.

**Fix:** Extract `days_to_ymd` to a `crate::utils::time` module and call it from both locations, or add `chrono` as a dependency.

---

### IN-03: `SpaceLabelEntry` in `client/lib/types.ts` is missing the Phase 10 `parentId` and `depth` fields

**File:** `client/lib/types.ts:303-310`

**Issue:** The `SpaceLabelEntry` interface (lines 303-310) only has Phase 9 fields: `fingerprint`, `label`, `description`, `canonicalEntityHint`, `generatedAt`, `userLocked`. The Phase 10 `parent_id` and `depth` fields added to the Rust `SpaceLabelEntry` (serialized as `parentId` and `depth`) are absent. Any frontend code that consumes `get_space_labels` IPC and tries to use these fields will see `undefined`. This is currently harmless because no frontend code reads `parentId`/`depth` from the label cache directly — but the type is publicly exported and any future consumer will silently operate on incomplete data.

**Fix:** Add the Phase 10 fields to the TypeScript interface:

```typescript
export interface SpaceLabelEntry {
  fingerprint: string;
  label: string;
  description: string;
  canonicalEntityHint?: string;
  generatedAt: string;
  userLocked: boolean;
  // Phase 10 additions
  parentId?: string;
  depth?: number;
}
```

---

_Reviewed: 2026-07-08T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

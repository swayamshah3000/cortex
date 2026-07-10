---
phase: 09-llm-space-labeling
reviewed: 2026-07-05T12:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - src-tauri/src/spaces/llm_labeler.rs
  - src-tauri/src/spaces/label_cache.rs
  - src-tauri/src/spaces/fingerprint.rs
  - src-tauri/src/spaces/manager.rs
  - src-tauri/src/spaces/naming.rs
  - src-tauri/src/commands/spaces.rs
  - src-tauri/src/commands/analytics.rs
  - src-tauri/src/state.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/types.rs
  - client/components/spaces/SpaceCard.tsx
  - client/components/spaces/EntityHintChip.tsx
  - client/components/spaces/SpaceLabelingIndicator.tsx
  - client/pages/SpaceDetailPage.tsx
  - client/hooks/useTauri.ts
  - client/hooks/useSpaceLabelingProgress.ts
  - client/lib/format.ts
  - client/lib/stores.ts
findings:
  critical: 2
  warning: 9
  info: 4
  total: 15
status: issues_found
---

# Phase 9: Code Review Report

**Reviewed:** 2026-07-05T12:00:00Z
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

Phase 9 adds LLM-generated Space labeling: SHA-256 membership fingerprinting, Jaccard cache invalidation, domain-expansion bootstrap, collision resolution, and per-space progress events. The test coverage is thorough and the overall architecture is sound.

Two blockers were found. First, `rename_space_label` updates the JSON sidecar cache but never calls `SpaceManager::update_space_label`, so `get_spaces` returns the pre-rename name until the next recluster — the rename silently has no visible effect in the UI. Second, the per-space shimmer (D-14 requirement) is completely broken: the backend hardcodes `label_status: "ready"` for every space after recluster and provides no mechanism to emit `"generating"` for in-progress spaces; `SpaceCard`'s `isGenerating` guard can never be true.

Nine warnings cover: the `_model` parameter silently dropped throughout the call stack, three major Mutex guards held across all LLM API calls (engine, cache, space_manager), prompt injection gap in the avoid list, a useEffect listener-leak race, `clear_space_override` mirroring the same in-memory update omission as `rename_space_label`, a dead rule in `naming.rs` that can never fire, path-segment counting that overcounts repeated keywords, a type mismatch on `useClearSpaceOverride`, and a dead semaphore in a sequential loop.

---

## Critical Issues

### CR-01: `rename_space_label` Does Not Update SpaceManager — Rename Has No Visible Effect

**File:** `src-tauri/src/commands/spaces.rs:144-174`

**Issue:** `rename_space_label` updates `space_label_cache` (persists to disk) but never acquires the `space_manager` lock to call `SpaceManager::update_space_label`. When the frontend's `useRenameSpace` mutation invalidates the `spaces` query and triggers a refetch, `get_spaces` reads directly from `SpaceManager::get_spaces()`, which still holds the old `Space.name`. The rename is invisible to the user until the next `recluster_spaces` call re-reads the cache.

The same design error applies to the Lock icon: `Space.user_locked` in `SpaceManager` is never set to `true` by `rename_space_label`, so the padlock icon in `SpaceCard` and `SpaceDetailPage` does not appear after a rename even though `user_locked = true` is written to cache.

**Fix:**
```rust
pub async fn rename_space_label(
    space_id: String,
    label: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<SpaceLabelEntry, AppError> {
    let cache_arc   = state.space_label_cache.clone();
    let space_arc   = state.space_manager.clone();          // <-- add
    let app_data_dir = state.app_data_dir.clone();

    let mut cache_guard = cache_arc.lock().await;
    // ... existing cache update logic ...
    cache_guard.save(&app_data_dir)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    drop(cache_guard);

    // Mirror to in-memory SpaceManager so get_spaces reflects new label immediately.
    let mut space_guard = space_arc.lock().await;           // <-- add
    space_guard.update_space_label(                         // <-- add
        &space_id,
        label.clone(),
        description.clone(),
        true, // user_locked = true
    );

    Ok(entry)
}
```

---

### CR-02: Per-Space Shimmer (D-14) Is Permanently Broken — `label_status: "generating"` Never Emitted

**Files:** `src-tauri/src/spaces/manager.rs:482`, `client/components/spaces/SpaceCard.tsx:55`

**Issue:** The Rust `recluster` function unconditionally writes `label_status: Some("ready".to_string())` into every `Space` struct it produces (manager.rs line 482). There is no code path that sets `label_status = "generating"` on any `Space` before or during LLM calls. The `labeling_in_progress: Arc<Mutex<HashSet<String>>>` field tracks in-progress IDs internally but is never surfaced through the `get_spaces` IPC response.

Consequently, `SpaceCard`'s guard on line 55:
```tsx
const isGenerating = space.labelStatus === "generating";
```
is permanently `false` for all spaces returned from the backend. The entire shimmer-skeleton branch (lines 87-104 of SpaceCard.tsx) is dead code. The D-14 per-space generating indicator is unimplemented.

The mock data at `client/lib/mock-data.ts:141` sets `labelStatus: "generating"` for the "Work" mock space, confirming the intent was to exercise this path, but the backend never produces it.

The `useSpaceLabelingProgress` hook updates `useSpaceLabelingStore` with a `spaceId` field from each event, but the store discards `spaceId` and only tracks batch-level progress (`processed`/`total`). SpaceCard has no way to query per-space generating state.

**Fix (option A — expose via existing Zustand store):**

Extend `SpaceLabelingState` to track individual generating space IDs:
```typescript
// stores.ts
generatingSpaceIds: Set<string>;
setProgress: (p: SpaceLabelingProgress) => void; // existing
```
Update `setProgress` to add/remove `spaceId` from `generatingSpaceIds` based on status.

In SpaceCard:
```tsx
const generatingIds = useSpaceLabelingStore(s => s.generatingSpaceIds);
const isGenerating = space.labelStatus === "generating"
  || generatingIds.has(space.id);
```

**Fix (option B — surface from backend):**
In `recluster`, emit updated `Space` objects with `label_status = "generating"` for clusters in `LlmLabel` decision before the LLM calls start, and push them via Tauri event so the React Query cache can be patched optimistically.

---

## Warnings

### WR-01: `_model` Parameter Silently Dropped — `extraction_model` Setting Has No Effect on Space Labeling

**File:** `src-tauri/src/spaces/llm_labeler.rs:213`

**Issue:** The `model` parameter is read from `settings.json` in `recluster_spaces` (commands/spaces.rs:98-101), threaded through `recluster` → `label_cluster` → `label_with_avoid_list`. However, `label_with_avoid_list`'s signature uses `_model: &str` — the leading underscore explicitly marks it as unused. The active provider's built-in default model is always used regardless of the user-configured `extraction_model`.

The entire chain that reads and propagates the model (settings read, function parameters, call sites) is wasted work. Users who configure a specific model in Settings > Indexing > AI Models will see it silently ignored for Space labeling.

**Fix:** Remove the underscore prefix and forward the model to `AIServiceRequest`:
```rust
pub async fn label_with_avoid_list(
    auth: &AuthState,
    model: &str,   // <-- remove underscore
    ...
) -> Result<SpaceLabel, String> {
    // ...
    let req = AIServiceRequest {
        // ...
        model_override: if model.is_empty() { None } else { Some(model.to_string()) },
    };
```
If `AIServiceRequest` does not yet support `model_override`, add the field or pass it through `ai_request_with_retry`.

---

### WR-02: Three Major Mutex Guards Held for All LLM API Calls — Concurrent Commands Block

**File:** `src-tauri/src/commands/spaces.rs:107-128`

**Issue:** `recluster_spaces` acquires `engine`, `space_label_cache`, and `space_manager` before calling `recluster().await`, and all three locks are held for the entire function duration — including every sequential LLM API call in the labeling loop. For a corpus requiring N LLM calls (one per new cluster), the locks are held for N × LLM_latency seconds.

While these locks are held:
- `get_spaces` (needs `space_manager`) blocks
- `get_stats` (needs `engine`) blocks
- `get_space_labels` (needs `space_label_cache`) blocks
- Background document indexing (needs `engine` via watcher worker) blocks

In a desktop app, this manifests as a fully unresponsive UI during recluster for large corpora.

**Fix:** Restructure `recluster_spaces` to release the engine lock after the initial vector read, and hold `space_manager` only for the final write:
```rust
// Step 1: read vectors (brief engine lock)
let (vectors, id_to_metadata) = {
    let guard = engine.lock().await;
    // ... read vectors ...
}; // engine lock released

// Step 2: read cached state (brief cache lock)
let (labeling_plan, prev_spaces) = {
    let guard = cache_arc.lock().await;
    // ... plan_labeling_operations ...
}; // cache lock released

// Step 3: LLM calls with NO locks held
let final_labels = run_llm_labeling(&labeling_plan, auth, ...).await?;

// Step 4: final write (brief cache + space_manager locks)
{
    let mut cache_guard = cache_arc.lock().await;
    let mut space_guard = space_mgr.lock().await;
    // ... update cache and space_manager ...
}
```

---

### WR-03: Avoid List Items Not Sanitized — T-09-01 Gap Permits Control Character Injection

**File:** `src-tauri/src/spaces/llm_labeler.rs:155-162`

**Issue:** The T-09-01 mitigation applies `sanitize_field` to document titles, entity summaries, topics, and tags before prompt assembly. However, `avoid` list items are injected verbatim:
```rust
let avoid_suffix = if avoid.is_empty() {
    String::new()
} else {
    format!(
        "\n\nIMPORTANT: Avoid these labels already in use: {}",
        avoid.join(", ")   // <-- no sanitization
    )
};
```
Avoid list items are LLM-generated labels from other spaces in the same batch. If a prior LLM response included control characters (`\n`, `\r`) or a prompt injection payload in the label field, those characters are forwarded into subsequent prompts for retrying collisions. The threat is compounded because the avoid list is populated from the batch-level `raw_labels` map, which includes all generated labels.

**Fix:**
```rust
let avoid_joined = avoid
    .iter()
    .map(|s| sanitize_field(s))
    .collect::<Vec<_>>()
    .join(", ");
let avoid_suffix = if avoid.is_empty() {
    String::new()
} else {
    format!("\n\nIMPORTANT: Avoid these labels already in use: {}", avoid_joined)
};
```

---

### WR-04: `useSpaceLabelingProgress` Event Listener Leaks on Rapid Unmount

**File:** `client/hooks/useSpaceLabelingProgress.ts:34-58`

**Issue:** The useEffect body launches an IIFE async function that awaits the dynamic import before assigning `unlisten`. If the component unmounts after the IIFE starts but before `unlisten` is assigned (the window between `await import(...)` and `unlisten = await listen(...)`), the cleanup function executes `unlisten?.()` which is a no-op (`unlisten` is still `undefined`). The listener registered by `listen(...)` is never removed.

In practice this hook is mounted once at AppShell level (low unmount frequency), but the leak is a real correctness issue: stale listeners cause duplicate `queryClient.invalidateQueries` calls on every subsequent labeling event, and the number of registered listeners compounds with each fast remount cycle.

**Fix:**
```typescript
useEffect(() => {
  if (!isTauri()) return;

  let unlisten: (() => void) | undefined;
  let cancelled = false;

  (async () => {
    const { listen } = await import("@tauri-apps/api/event");
    if (cancelled) return;
    unlisten = await listen<SpaceLabelingProgress>("space-labeling-progress", (event) => {
      useSpaceLabelingStore.getState().setProgress(event.payload);
      if (event.payload.status === "complete" || event.payload.status === "error") {
        queryClient.invalidateQueries({ queryKey: queryKeys.spaces });
        queryClient.invalidateQueries({ queryKey: queryKeys.spaceLabels });
      }
    });
    if (cancelled) { unlisten(); unlisten = undefined; }
  })();

  return () => {
    cancelled = true;
    unlisten?.();
  };
}, [queryClient]);
```

---

### WR-05: `clear_space_override` Does Not Update SpaceManager — Lock Icon Persists in UI

**File:** `src-tauri/src/commands/spaces.rs:179-194`

**Issue:** `clear_space_override` sets `user_locked = false` in `space_label_cache` but never acquires `space_manager` to update `Space.user_locked`. After the call, `useSpaces` refetch reads from `SpaceManager::get_spaces()`, which still has `user_locked = true`. The lock icon in `SpaceCard` and `SpaceDetailPage` persists until the next `recluster_spaces`.

This is the same class of bug as CR-01 but lower severity because the lock flag only affects UI display, not functional behavior (the cache has the correct `user_locked = false` value and will be respected on the next recluster).

**Fix:**
```rust
pub async fn clear_space_override(
    space_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let cache_arc  = state.space_label_cache.clone();
    let space_arc  = state.space_manager.clone();
    let app_data_dir = state.app_data_dir.clone();

    let mut cache_guard = cache_arc.lock().await;
    if let Some(entry) = cache_guard.labels.get_mut(&space_id) {
        entry.user_locked = false;
        cache_guard.save(&app_data_dir)
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    drop(cache_guard);

    // Reflect change in SpaceManager so get_spaces returns current state.
    let mut space_guard = space_arc.lock().await;
    space_guard.update_space_label(&space_id, /* keep existing name */ ..., false);

    Ok(())
}
```
Note: `update_space_label` currently requires a `new_name` argument. Either add a dedicated `set_user_locked` method to `SpaceManager`, or read the current name from `space_guard` before passing it through.

---

### WR-06: Dead Branch in `naming.rs` — "person"/"organization" Entity Check Can Never Fire

**File:** `src-tauri/src/spaces/naming.rs:102-107`

**Issue:** The NER pollution filter at lines 19-32 explicitly excludes "person", "organization", and "location" from `entity_counts`:
```rust
let ner_pollution_types = ["person", "organization", "location"];
// ...
if ner_pollution_types.contains(&etype_lc.as_str()) {
    continue;
}
```
Because these types never enter `entity_counts`, `dominant_entity` can never be `Some("person")` or `Some("organization")`. The branch at lines 102-107:
```rust
if dominant_entity == Some("person") || dominant_entity == Some("organization") {
    return ("Contacts & Correspondence".to_string(), ...);
}
```
is permanently unreachable. It creates false confidence that this naming path is active.

**Fix:** Remove lines 102-107 entirely. If the "Contacts & Correspondence" heuristic is wanted in a future phase, document it as requiring the removal of those types from `ner_pollution_types` first.

---

### WR-07: `path_majority` Miscounts Repeated Keywords in a Single Document's Path

**File:** `src-tauri/src/spaces/naming.rs:70-80`

**Issue:** `path_segments` accumulates segment occurrence counts across all path components of all documents. A single document at `/work/projects/work-backup/report.pdf` contributes 2 to the "work" segment count: both "work" and "work-backup" match `seg.contains("work")`. With `doc_count = 1`, the check `matched * 2 >= doc_count` evaluates to `4 >= 1 = true`, incorrectly asserting that the "work" pattern dominates the cluster.

The intention stated in the comment ("Require ≥ 50% to claim dominance") is correct, but the implementation counts segment occurrences rather than distinct documents containing the pattern.

**Fix:** Track distinct document indices per pattern:
```rust
// Change path_segments to track a set of doc indices per segment key
let mut path_doc_sets: HashMap<String, HashSet<usize>> = HashMap::new();

for (doc_idx, meta) in doc_metadata.iter().enumerate() {
    if let Some(path) = meta.get("path").and_then(|v| v.as_str()) {
        for segment in path.to_lowercase().split('/') {
            let seg = segment.trim();
            if !seg.is_empty() && seg.len() > 2 {
                path_doc_sets.entry(seg.to_string()).or_default().insert(doc_idx);
            }
        }
    }
}

let path_majority = |pattern: &str| -> bool {
    let matched: usize = path_doc_sets
        .iter()
        .filter(|(seg, _)| seg.contains(pattern))
        .map(|(_, docs)| docs.len())
        .max()  // max across matching segments, not sum
        .unwrap_or(0);
    matched * 2 >= doc_count
};
```

---

### WR-08: `useClearSpaceOverride` Return Type Mismatch — Tauri Returns `void`, TS Expects `SpaceLabelEntry`

**File:** `client/hooks/useTauri.ts:204`

**Issue:** The Rust `clear_space_override` command signature is `-> Result<(), AppError>`, serializing as `null` / `void`. The TypeScript hook declares:
```typescript
tauriInvoke<SpaceLabelEntry>("clear_space_override", ...)
```
and the mock returns a `SpaceLabelEntry`-shaped object. In Tauri runtime the mutation data is `undefined`, not a `SpaceLabelEntry`. The bug is silently benign today because `onSuccess` ignores the data argument, but it breaks type safety and any future use of mutation data.

**Fix:**
```typescript
mutationFn: (spaceId: string) =>
  tauriInvoke<void>("clear_space_override", { spaceId }, () => undefined),
```
Remove the mock `SpaceLabelEntry` object and use `() => undefined`.

---

### WR-09: Semaphore in `recluster` Loop Is Dead Code — Sequential Loop Never Contends

**File:** `src-tauri/src/spaces/manager.rs:197-250`

**Issue:** A `Semaphore::new(8)` is created on line 197 and acquired on lines 246-250 inside a sequential `for` loop. Because the loop processes one cluster at a time (no parallelism), only one permit is ever outstanding simultaneously. The rate limit of 8 concurrent LLM calls is never exercised.

The inline comment (`"T-09-02 mitigation"`) incorrectly attributes the semaphore to T-09-02. T-09-02 is the JSON fence-stripping mitigation (`strip_json_fences` applied to LLM responses), not a concurrency guard.

**Fix (option A — remove):** Delete the semaphore and the `_permit` binding if sequential processing is intended.

**Fix (option B — actually parallelize):** If parallel LLM calls are desired, replace the `for` loop with `futures::stream::FuturesOrdered`:
```rust
use futures::StreamExt;
let sem = Arc::new(tokio::sync::Semaphore::new(8));
let tasks: Vec<_> = labeling_plan.clusters.iter().map(|plan_item| {
    let sem = sem.clone();
    async move {
        let _permit = sem.acquire().await?;
        // ... LLM call ...
    }
}).collect();
let raw_labels: Vec<_> = futures::stream::iter(tasks)
    .buffer_unordered(8)
    .collect()
    .await;
```
Also correct the comment to reference rate-limiting (not T-09-02).

---

## Info

### IN-01: Duplicate `formatRelativeTime` in `SpaceDetailPage.tsx`

**File:** `client/pages/SpaceDetailPage.tsx:14-22`

**Issue:** The function defined locally in `SpaceDetailPage.tsx` is byte-for-byte identical to the exported `formatRelativeTime` in `client/lib/format.ts`. The `format.ts` file itself acknowledges this: "SpaceDetailPage.tsx currently has an inline duplicate — Plan 09-07 will replace it."

**Fix:** Replace lines 14-22 with an import:
```typescript
import { formatRelativeTime } from "../lib/format";
```

---

### IN-02: Redundant Bootstrap Double-Check in `plan_labeling_operations`

**File:** `src-tauri/src/spaces/manager.rs:722-738`

**Issue:** The `best_match` search (lines 712-722) already applies the `>= 0.75` threshold and finds the best cosine match. The subsequent `try_bootstrap_from_nearest` call on line 726 is a re-run of the same computation. Both paths use identical logic; `bootstrap.is_some()` is always true when `best_match.is_some()`. The `else { LlmLabel }` branch inside the outer `if let Some(...)` is unreachable.

**Fix:** Remove the `try_bootstrap_from_nearest` call and use `source_label`/`source_id` from `best_match` directly:
```rust
if let Some((source_id, source_label, _)) = best_match {
    let description = cache
        .get(source_id)
        .map(|e| e.description.clone())
        .unwrap_or_else(|| "Similar document cluster.".to_string());
    LabelingDecision::Bootstrap {
        from_space_id: source_id.clone(),
        label: source_label.clone(),
        description,
    }
} else {
    LabelingDecision::LlmLabel
}
```

---

### IN-03: `titles.dedup()` Removes Only Adjacent Duplicates

**File:** `src-tauri/src/spaces/manager.rs:841`

**Issue:** `Vec::dedup()` removes only consecutive duplicate entries. If the same document title appears at non-adjacent positions in the `titles` vec (e.g., positions 0 and 5), the duplicate at position 5 survives. The comment says "deduplicated, first-come basis" which implies all-duplicates removal, not just adjacent.

In practice, documents in the same k-means cluster rarely share titles, so this is low-impact. But the de-duplication contract stated in the comment is not met.

**Fix:** Preserve insertion order using a seen-set:
```rust
let mut seen = std::collections::HashSet::new();
titles.retain(|t| seen.insert(t.clone()));
titles.truncate(20);
```

---

### IN-04: `chrono_now_iso()` Is Duplicated from Indexer

**File:** `src-tauri/src/spaces/manager.rs:879-906`

**Issue:** The Gregorian calendar computation in `chrono_now_iso()` is a copy of the same algorithm from the document indexer (`indexer.rs`). The comment on line 890 confirms this: "Reuse the same days_to_ymd algorithm from indexer." Code duplication of this kind drifts over time; any bug fix or leap-year edge-case fix would need to be applied in two places.

**Fix:** Extract to a shared utility, e.g.:
```rust
// crate::utils::timestamp::now_iso8601() -> String
```
and import it in both manager.rs and indexer.rs.

---

_Reviewed: 2026-07-05T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

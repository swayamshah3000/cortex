---
phase: 09-llm-space-labeling
fixed_at: 2026-07-05T13:10:00Z
review_path: .planning/phases/09-llm-space-labeling/09-REVIEW.md
iteration: 1
findings_in_scope: 11
fixed: 10
skipped: 1
status: partial
---

# Phase 9: Code Review Fix Report

**Fixed at:** 2026-07-05T13:10:00Z
**Source review:** .planning/phases/09-llm-space-labeling/09-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 11 (CR-01, CR-02, WR-01 through WR-09)
- Fixed: 10
- Skipped: 1 (WR-02 — architectural refactor required)

## Fixed Issues

### CR-01: `rename_space_label` Does Not Update SpaceManager

**Files modified:** `src-tauri/src/commands/spaces.rs`
**Commit:** e1e7e48
**Applied fix:** Extracted the cache update into a scoped block (releases cache lock before acquiring space_manager lock), then acquires `space_manager` and calls `update_space_label(space_id, label, description, user_locked=true)`. The cache lock and space_manager lock are never held simultaneously, preventing deadlock. `get_spaces` now reflects the renamed label immediately after the IPC call.

### CR-02: Per-Space Shimmer Permanently Broken

**Files modified:** `client/lib/stores.ts`, `client/components/spaces/SpaceCard.tsx`
**Commit:** 80e4c81
**Applied fix (option A):** Extended `SpaceLabelingState` with `generatingSpaceIds: Set<string>`. Updated `setProgress` to add the `spaceId` on `"labeling"` status and delete it on `"complete"` / `"error"`. SpaceCard now imports `useSpaceLabelingStore` and evaluates `isGenerating` as `space.labelStatus === "generating" || generatingIds.has(space.id)` — the shimmer branch is reachable via either the backend field or the Tauri event stream.

### WR-01: `_model` Parameter Silently Dropped

**Files modified:** `src-tauri/src/ai/service.rs`, `src-tauri/src/spaces/llm_labeler.rs`, `src-tauri/src/commands/ai.rs`, `src-tauri/src/pipeline/pass2_llm_refiner.rs`
**Commit:** b3862a9
**Applied fix:** Added `model_override: Option<String>` field (with `#[serde(default)]`) to `AIServiceRequest`. Updated `ai_request` to use the override when non-empty, falling back to `cred.model`. Removed the underscore prefix from `_model` in `label_with_avoid_list` and passes it as `model_override`. Updated all five existing `AIServiceRequest` struct literal construction sites to include `model_override: None`. The user-configured `extraction_model` setting now flows end-to-end into space labeling LLM calls.

### WR-03: Avoid List Items Not Sanitized

**Files modified:** `src-tauri/src/spaces/llm_labeler.rs`
**Commit:** fc1c91b
**Applied fix:** Applied `sanitize_field(s)` to each avoid list item before joining (using `.map(|s| sanitize_field(s)).collect::<Vec<_>>().join(", ")`). Added comment attributing this to the T-09-01 mitigation scope.

### WR-04: `useSpaceLabelingProgress` Event Listener Leak

**Files modified:** `client/hooks/useSpaceLabelingProgress.ts`
**Commit:** d8f9b4b
**Applied fix:** Added `let cancelled = false` flag. Checks `if (cancelled) return` immediately after the `await import(...)` resolves, before calling `listen()`. Adds a second check after `unlisten = await listen(...)` to immediately call and clear `unlisten` if the component unmounted during that await. Cleanup function sets `cancelled = true` first, then calls `unlisten?.()`.

### WR-05: `clear_space_override` Does Not Update SpaceManager

**Files modified:** `src-tauri/src/commands/spaces.rs`
**Commit:** 0f10c1d
**Applied fix:** Added `space_manager` lock acquisition after the cache block. Reads current name and description from `get_space_data` (cloning to avoid borrow conflict), then calls `update_space_label` with `user_locked = false`. Cache lock is released before acquiring space_manager lock. Lock icon in SpaceCard and SpaceDetailPage now disappears immediately after clearing the override.

### WR-06: Dead Branch in `naming.rs`

**Files modified:** `src-tauri/src/spaces/naming.rs`
**Commit:** aa50644
**Applied fix:** Removed the `if dominant_entity == Some("person") || dominant_entity == Some("organization")` branch (lines 102-107 of the original). Replaced with a comment explaining why it was removed and how to re-enable when a better NER model is available.

### WR-07: `path_majority` Miscounts Repeated Keywords in Single Document Path

**Files modified:** `src-tauri/src/spaces/naming.rs`
**Commit:** 206407c
**Applied fix:** Changed `path_segments: HashMap<String, usize>` to `path_doc_sets: HashMap<String, HashSet<usize>>`. The loop now calls `path_doc_sets.entry(seg).or_default().insert(doc_idx)` instead of incrementing a counter. The `path_majority` closure computes `max()` across matching segment sets (not `sum()`) to avoid double-counting a document whose path has multiple segments matching the same pattern. The 50% dominance threshold behavior is preserved.

### WR-08: `useClearSpaceOverride` Return Type Mismatch

**Files modified:** `client/hooks/useTauri.ts`
**Commit:** 60f3780
**Applied fix:** Changed `tauriInvoke<SpaceLabelEntry>` to `tauriInvoke<void>` and replaced the mock `SpaceLabelEntry` object with `() => undefined`. Matches the Rust `Result<(), AppError>` return type (serialises as `null`).

### WR-09: Semaphore in Sequential Loop Is Dead Code

**Files modified:** `src-tauri/src/spaces/manager.rs`
**Commit:** 2586a46
**Applied fix (option A):** Removed the `let sem = Arc::new(tokio::sync::Semaphore::new(8));` line and the `let _permit = sem.acquire().await...` block inside the labeling loop. Replaced the misleading comment with a note explaining the loop is sequential and pointing to WR-02 as the prerequisite for future parallelism.

## Skipped Issues

### WR-02: Three Mutex Guards Held for All LLM API Calls

**File:** `src-tauri/src/commands/spaces.rs:107-128`
**Reason:** Architectural refactor — not safe for atomic fix without significant regression risk.
**Original issue:** `recluster_spaces` holds three async Mutex guards (`engine`, `space_label_cache`, `space_manager`) across all sequential LLM API calls inside `SpaceManager::recluster`. For N clusters requiring LLM calls, all three locks are held for N × LLM_latency seconds, blocking `get_spaces`, `get_stats`, `get_space_labels`, and background document indexing.

The reviewer's suggested restructuring requires splitting `SpaceManager::recluster` (currently ~420 lines, 12 steps) into phased methods with explicit ownership hand-offs between vector-read, labeling-plan, LLM-call, and write-state stages. This touches the core recluster logic, LLM inputs caching, collision resolution, and SpaceLabelCache persistence — all interleaved in a single method. A safe fix requires introducing new `SpaceManager` methods, changing the calling convention in `recluster_spaces`, and re-testing the full recluster flow. This is appropriate for a dedicated phase task, not an atomic review fix.

---

**Build verification:**
- `cd src-tauri && cargo check`: PASS (22 pre-existing warnings, 0 errors)
- `cd client && npx vitest run --reporter=dot`: 294/298 tests pass; 4 pre-existing failures in `App.test.tsx` (path resolution bug: `client/client/App.tsx` doubled segment — not introduced by this fix)

---

_Fixed: 2026-07-05T13:10:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

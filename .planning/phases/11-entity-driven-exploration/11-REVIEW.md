---
phase: 11-entity-driven-exploration
reviewed: 2026-07-09T00:00:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - src-tauri/src/saved_searches/store.rs
  - src-tauri/src/saved_searches/commands.rs
  - src-tauri/src/saved_searches/mod.rs
  - src-tauri/src/search/filters.rs
  - src-tauri/src/search/query.rs
  - src-tauri/src/commands/documents.rs
  - src-tauri/src/commands/entities.rs
  - src-tauri/src/state.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/types.rs
  - client/pages/SearchPage.tsx
  - client/pages/EntityDetailPage11.tsx
  - client/pages/DocumentPage.tsx
  - client/components/entities/EntityChip.tsx
  - client/components/layout/Sidebar.tsx
  - client/components/search/ScoreBadge.tsx
  - client/components/search/EntityFilterPill.tsx
  - client/components/search/EntityFilterBar.tsx
  - client/components/search/SaveSearchDialog.tsx
  - client/hooks/useTauri.ts
  - client/lib/types.ts
findings:
  critical: 2
  warning: 6
  info: 4
  total: 12
status: issues_found
---

# Phase 11: Code Review Report

**Reviewed:** 2026-07-09T00:00:00Z
**Depth:** standard
**Files Reviewed:** 21
**Status:** issues_found

## Summary

Phase 11 adds entity-driven exploration: saved searches (sidecar JSON store + 4 IPC commands), a new `/entity/:class/:value` detail page backed by `get_entity_page_data`, hybrid cosine+Jaccard scoring for related documents (`get_related_docs_scored`), entity filter pills on SearchPage, and Sidebar saved-search counts. The concurrency strategy (all std::sync::Mutex access inside `spawn_blocking`, tokio::Mutex for `saved_search_store`) is correctly applied.

**Key concerns identified:**

1. **BLOCKER**: `save_search` double-locks `engine` inside `spawn_blocking` via `blocking_lock()` — but `engine` is a `tokio::sync::Mutex`, not a `std::sync::Mutex`. Calling `blocking_lock()` on a tokio Mutex *inside* `spawn_blocking` is correct only when no other `spawn_blocking` task or tokio task holds it via `await`. However the real deadlock risk is that `blocking_lock()` busy-waits on the tokio runtime while inside a blocking thread pool thread, and **a second `save_search` call concurrently running would each call `store_arc.blocking_lock()`** — these two are on different threads sharing the same `tokio::sync::Mutex<CortexEngine>`, which can deadlock the blocking thread pool if all threads are waiting. This is the T-11-06/T-11-13 concern the comments note but the resolution is incomplete.

2. **BLOCKER**: The `parse_entity_class_filters` function in `commands.rs` uses `s.split_once(':')` which splits on the **first** colon only — this is correct for `"{class}:{value}"`. However `EntityFilterPill.tsx` and `SearchPage.tsx` use `raw.indexOf(":")` + `slice` (which is identical semantics), but entity values containing colons (e.g., `Identifier:AB:CD:EF`) will have the value truncated at the first colon in `SavedSearchFilters.entities` (stored as `"Identifier:AB:CD:EF"`) which round-trips correctly through `split_once`/`indexOf` — this is actually safe. The real issue is different: **`parse_entity_class_filters` silently drops entries without a colon** (`filter_map`), but the frontend validates entries have colons before adding to URL params. However, the Rust `save_search` IPC accepts `SavedSearchFilters` from the frontend which can pass `entities: ["NoColon"]` via IPC directly, and that entry will be silently dropped on count computation — but not on save. The save will persist `"NoColon"` to disk, which will then be silently dropped on every count refresh, giving a permanently incorrect count (off-by-one in entity filter count, never throwing an error). This is a data integrity issue.

3. **WARNING**: `count_matching_docs` in `commands.rs` acquires `engine_arc.blocking_lock()` **inside** `entity_store_arc.lock()` scope (lines 222–224). The guard ordering is `entity_store → engine`. In `search_documents` (documents.rs line 79–89), the ordering is `engine → entity_store`. Inconsistent lock ordering between two code paths that both hold both locks is a **potential deadlock**. If a search and a `get_saved_search_counts` run concurrently, one thread holds engine waiting for entity_store while the other holds entity_store waiting for engine.

4. **WARNING**: `get_entity_page_data` uses `page_start = current_page as usize * PAGE_SIZE as usize` (entities.rs line 479). If `current_page` is very large (e.g., user manually sets `?page=1000000`), `page_start` overflows `usize` on 32-bit targets and silently wraps. On 64-bit macOS this is harmless but the URL param is caller-controlled.

5. **WARNING**: `EntityDetailPage11.tsx` line 207 parses the `page` URL param with `parseInt(searchParams.get("page") ?? "0", 10) || 0`. The `|| 0` fallback also fires when `page=0` (since `parseInt("0")` is falsy), making `page=0` and `page=NaN` indistinguishable. This is correct for `NaN` but the `|| 0` operator incorrectly forces `page=0` even if `parseInt` returned `0` — which is in fact the right result, so the behavior is accidentally correct. However it makes the intent confusing, and any `page=-1` becomes `-1` (negative page passed to backend).

6. **WARNING**: In `commands.rs`, the `save_search` command holds `engine_arc.blocking_lock()` and `entity_store_arc.lock()` simultaneously inside `spawn_blocking` (lines 219–224). When the blocking thread pool is exhausted (Tokio's default pool is CPU × 512), this can cause a hang since `blocking_lock` re-enters the runtime. The comment on line 240 says `store_arc.blocking_lock()` is used after releasing the earlier guards, which is correct — but the concurrent guard window between lines 219–224 is the deadlock risk flagged in CR-01.

7. **WARNING**: `ScoreBadge.tsx` renders `score * 100` without clamping. If `score` exceeds `1.0` (which is theoretically possible if the HNSW raw distance is negative, making `1.0 - raw.score` > 1.0), the badge displays ">100%". The HNSW distance is clamped to `[0.0, 1.0]` via `.clamp(0.0, 1.0)` in `get_related_docs_scored` but not in `search_documents_impl` (query.rs line 131: `let score = 1.0 - raw.score as f64` — no clamp).

8. **INFO**: `useSavedSearchCounts` cache key uses sorted-join (`[...ids].sort().join(",")`) which is correct for deduplication. However the `ids` array comes from `savedSearches.map(s => s.id)` — if two Sidebar instances mount simultaneously with the same search list, the sort ensures the key is identical, which is correct. The comment accurately describes the design intent.

---

## Critical Issues

### CR-01: Lock-Ordering Inversion Between `save_search`/`get_saved_search_counts` and `search_documents` — Potential Deadlock

**File:** `src-tauri/src/commands/documents.rs:79-89` and `src-tauri/src/saved_searches/commands.rs:219-224`

**Issue:** Two concurrent code paths acquire `entity_store` (std::sync::Mutex) and `engine` (tokio::sync::Mutex) in opposite orders:

- `search_documents` (documents.rs lines 79–89): acquires `engine.blocking_lock()` **first**, then acquires `entity_store.lock()` inside the closure.
- `save_search` and `get_saved_search_counts` (commands.rs lines 219–224 and 293–295): acquire `entity_store_arc.lock()` **first**, then immediately call `engine_arc.blocking_lock()` inside that guard's scope.

If one thread is executing `search_documents` and another is executing `save_search` simultaneously:
- Thread A: holds `engine.blocking_lock()`, waiting for `entity_store.lock()`
- Thread B: holds `entity_store.lock()`, waiting for `engine.blocking_lock()`

This is a classic lock-ordering deadlock. Both code paths run inside `spawn_blocking` which places them on the blocking thread pool, so neither yields and neither makes progress.

**Fix:** Standardize on a consistent lock acquisition order. The simplest fix is to release the `entity_store` guard before acquiring the engine lock in `save_search` and `get_saved_search_counts`:

```rust
// In save_search (commands.rs), change the doc_count_cache computation to:
let doc_count_cache = {
    // Step 1: acquire entity_store, collect what we need, then release it
    let search_filters = {
        let entity_store_guard = entity_store_arc
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let search_filters = build_search_filters(&filters_clone);
        // Precompute entity candidates while holding entity_store, return them
        (search_filters, apply_entity_class_filters_local(
            search_filters.entity_filters.as_deref().unwrap_or(&[]),
            &entity_store_guard,
        ))
    }; // entity_store_guard dropped here
    
    // Step 2: only then acquire engine
    let engine_guard = engine_arc.blocking_lock();
    // ... rest of count_matching_docs using precomputed entity candidates
};
```

Alternatively, impose a strict ordering rule: always acquire `engine` first, then `entity_store` — and move `entity_store` access inside `count_matching_docs` to come after the engine guard is obtained.

---

### CR-02: Malformed Entity Strings Persist to Disk But Are Silently Dropped on Count Refresh — Permanent Count Drift

**File:** `src-tauri/src/saved_searches/commands.rs:144-154`

**Issue:** `parse_entity_class_filters` uses `filter_map` to silently skip strings without a colon:

```rust
fn parse_entity_class_filters(entities: &[String]) -> Vec<EntityClassFilter> {
    entities
        .iter()
        .filter_map(|s| {
            s.split_once(':').map(|(class, value)| EntityClassFilter { ... })
        })
        .collect()
}
```

The `save_search` IPC command accepts `SavedSearchFilters` from the frontend (or any IPC caller) and immediately persists the raw `filters.entities` vec to disk **without validating** that all entries contain a colon. The initial `doc_count_cache` is computed via `parse_entity_class_filters` which drops the malformed entry. On every subsequent `get_saved_search_counts` call, the malformed entry is again silently dropped — so a filter intended to narrow results is permanently ignored, and the doc count shown in the sidebar reflects 0 entity filtering rather than the intended filter. The user sees a count that is systematically too high with no error.

While the frontend validates before adding to URL params, the IPC surface accepts arbitrary JSON — any caller (e.g., a script, a future feature, a future test) can submit malformed filters. This is also a correctness issue for the round-trip: the persisted `SavedSearch.filters.entities` contains an entry that is never used.

**Fix:** Add input validation in `save_search` before persisting, rejecting filters with malformed entity strings:

```rust
// Validate all entity filter strings contain exactly one ':' separator
for entity_str in &filters.entities {
    if !entity_str.contains(':') {
        return Err(AppError::Internal(format!(
            "malformed entity filter '{}': expected 'Class:value' format",
            entity_str
        )));
    }
}
```

---

## Warnings

### WR-01: `search_documents_impl` Does Not Clamp Score — ScoreBadge Can Display > 100%

**File:** `src-tauri/src/search/query.rs:131`

**Issue:** The score conversion `let score = 1.0 - raw.score as f64` does not clamp the result. If the HNSW engine returns a raw distance value less than 0 (which can happen with cosine distance implementations that allow negative values, or due to floating-point rounding), `score` will exceed `1.0`. The frontend `ScoreBadge` renders `Math.round(score * 100)` without clamping, so a score of `1.05` displays as `105%`.

By contrast, `get_related_docs_scored` (documents.rs line 574) correctly clamps: `let cosine = (1.0_f64 - r.score as f64).clamp(0.0, 1.0)`. The fix is a single line.

**Fix:**
```rust
// query.rs line 131 — add .clamp(0.0, 1.0)
let score = (1.0 - raw.score as f64).clamp(0.0, 1.0);
```

---

### WR-02: Negative Page Number Accepted by `get_entity_page_data` — Silent Data Skip

**File:** `client/pages/EntityDetailPage11.tsx:207` and `src-tauri/src/commands/entities.rs:479`

**Issue:** The frontend parses the `?page=` URL param with:
```typescript
const page = Math.max(0, parseInt(searchParams.get("page") ?? "0", 10) || 0);
```

The `|| 0` fallback fires when `parseInt` returns `0` (falsy), but `Math.max(0, ...)` wrapping means a legitimate `?page=0` is correctly floored to 0. However `?page=-1` passes `parseInt` returning `-1`, which `Math.max(0, -1)` correctly floors to `0`. So the frontend floor is correct.

The actual issue is in the **backend**: `get_entity_page_data` accepts `page: Option<u32>`. Since `u32` cannot be negative, the frontend cannot send a negative page. However the frontend sends `page` as a JavaScript `number` (which includes negatives), and Tauri's IPC serialization will fail to deserialize a negative integer into `u32`, returning a serialization error to the frontend — rather than a clear `AppError::Internal("page out of range")`. This means a URL like `/entity/Person/Alice?page=-1` will show an opaque IPC error toast rather than a clean "page not found" empty state.

**Fix:** Use `i32` for `page` in the Rust IPC signature and validate explicitly:
```rust
pub async fn get_entity_page_data(
    class: String,
    value: String,
    page: Option<i32>,  // Accept signed, validate below
    ...
) -> Result<EntityPageData, AppError> {
    let page = page.unwrap_or(0);
    if page < 0 {
        return Err(AppError::Internal("page must be >= 0".to_string()));
    }
    let page = page as u32;
    ...
```

---

### WR-03: `handleNext` in `EntityDetailPage11` Does Not Check Upper Bound Before Incrementing

**File:** `client/pages/EntityDetailPage11.tsx:257-261`

**Issue:** The "Next" button is `disabled` when `page >= totalPages - 1`, but `handleNext` does not itself check the bound:

```typescript
const handleNext = () => {
  setSearchParams((prev) => {
    const next = new URLSearchParams(prev);
    next.set("page", String(page + 1));  // No upper bound check
    return next;
  });
};
```

If the user double-clicks the Next button before the `disabled` state re-renders (or if JS disables are bypassed), `page + 1` will exceed `totalPages - 1`. The backend `get_entity_page_data` handles this gracefully (returns empty `documents: []`), but the URL will contain a stale `?page=N` with N beyond range, and the component will render an empty document list without any visual indication that the page is out of range — the "Documents (N)" heading shows total count but the list is empty, which is confusing.

**Fix:** Guard in `handleNext`:
```typescript
const handleNext = () => {
  if (page >= totalPages - 1) return;
  setSearchParams((prev) => {
    const next = new URLSearchParams(prev);
    next.set("page", String(page + 1));
    return next;
  });
};
```

---

### WR-04: `useSavedSearchCounts` Query Key Uses Sorted IDs But `useSaveSearch` Invalidates Only Prefix — New Count Entry Not Invalidated

**File:** `client/hooks/useTauri.ts:931-933`

**Issue:** `useSaveSearch` invalidates with:
```typescript
queryClient.invalidateQueries({ queryKey: ["saved-searches", "counts"] });
```

React Query prefix-match invalidation will invalidate any query whose key **starts with** `["saved-searches", "counts"]`. The actual key is `["saved-searches", "counts", "ss-1,ss-2,ss-3"]` (sorted-joined). After a new search `ss-4` is saved, the Sidebar re-fetches `useSavedSearches` (correctly invalidated), the `savedSearchIds` memo recomputes to include `ss-4`, and `useSavedSearchCounts` is called with a new `ids` array — generating a **new** query key `["saved-searches", "counts", "ss-1,ss-2,ss-3,ss-4"]`. The prefix invalidation from `useSaveSearch.onSuccess` fires **before** the new `savedSearches` list arrives (they're concurrent), so the new count query is submitted but is not yet in cache — it will be fresh and will issue an IPC call. This is actually the correct behavior for most cases.

However, there is a subtle race: if `useSavedSearchCounts` resolves with the **old** id set from a stale `savedSearchIds` memo before `savedSearches` invalidation propagates, the Sidebar will show `ss-4` in the list (from the newly invalidated `savedSearches` query) but with count `0` (from `ss.docCountCache` fallback) rather than the computed count. This is a UX glitch — the new saved search shows count `0` in the sidebar until the 30s staleTime expires, even though the backend computed the count correctly during `save_search`.

**Fix:** After `save_search` returns, also invalidate the specific count query for the updated id set. Alternatively, use `queryClient.resetQueries({ queryKey: ["saved-searches", "counts"] })` (reset rather than invalidate) to force a synchronous re-fetch.

---

### WR-05: `aggregate_co_occurrence` Hardcodes Truncation to 10 But `get_entity_page_data` Comment Says "Top-10" — No Caller-Configurable Limit

**File:** `src-tauri/src/commands/entities.rs:388-389`

**Issue:** `aggregate_co_occurrence` always calls `refs.truncate(10)` regardless of caller needs. The function signature documents `limit` as a concept in the docstring ("truncated to `limit`") but the function does not accept a `limit` parameter — the truncation limit is hardcoded. This is a latent maintenance issue: if the UI is updated to show top-5 or top-20, the Rust function must also be changed. More critically, the docstring is misleading ("Returns Vec<RelatedEntityRef> sorted descending by co_doc_count, truncated to `limit`") — there is no `limit` parameter.

**Fix:** Either remove the misleading "`limit`" mention from the docstring, or make the truncation caller-configurable:
```rust
pub(crate) fn aggregate_co_occurrence(
    docs_metadata: &[...],
    target_class: &str,
    target_value: &str,
    limit: usize,  // Add explicit limit
) -> Vec<RelatedEntityRef> {
    // ...
    refs.sort_by(...);
    refs.truncate(limit);
    refs
}
```
And update the callsite: `aggregate_co_occurrence(&all_metadata, ..., 10)`.

---

### WR-06: `EntityChip` Right-Click Navigation Does Not Guard Against `resolvedClass` Being an Arbitrary String from Phase 6 Data — XSS-Adjacent URL Injection in Entity Detail Route

**File:** `client/components/entities/EntityChip.tsx:126-129`

**Issue:** The `handleContextMenu` handler constructs the URL:
```typescript
navigate(`/entity/${encodeURIComponent(resolvedClass)}/${encodeURIComponent(entity.value)}`);
```

Both `resolvedClass` and `entity.value` are `encodeURIComponent`-encoded, which is correct for URL safety. However `encodeURIComponent` does **not** encode `/` characters — wait, it does encode `/` to `%2F`. So `entity.value` containing `/` becomes `%2F` in the URL and React Router will interpret `/entity/Person/Alice%2FSmith` correctly.

The actual issue is more subtle: `entity.value` containing a null byte or non-printable characters (which can appear in OCR-extracted text from PDFs) will be `encodeURIComponent`-encoded into valid percent-sequences, but when the backend receives the `decodeURIComponent(value)` result, those characters will be passed to `alias_index.get()`. The alias_index is a HashMap keyed by `(value.to_lowercase(), class.to_lowercase())` — null bytes in the key are legal Rust HashMap keys. This is not an exploitable vulnerability in a desktop app context but it means garbage entities from OCR can create unfindable entity detail pages (they'll always 404 at the backend).

The more actionable concern is that `mapLegacyEntityTypeToClass` can return `undefined` for an unknown `entityType`, falling back to `entity.entityType` directly as `resolvedClass`. If `entityType` is an attacker-controlled string (possible if IPC responses are malformed), the URL becomes `/entity/<attacker string>/...`. Since this is a Tauri desktop app with no network exposure, exploitation requires a compromised backend, but the inconsistency is worth noting.

**Fix:** Validate `resolvedClass` is one of the 8 known classes before constructing the navigation URL:
```typescript
const KNOWN_CLASSES = new Set(["Person","Organization","Location","Date","Amount","Email","Phone","Identifier"]);
const handleContextMenu = (e: React.MouseEvent) => {
  e.preventDefault();
  const safeClass = KNOWN_CLASSES.has(resolvedClass) ? resolvedClass : "Unknown";
  navigate(`/entity/${encodeURIComponent(safeClass)}/${encodeURIComponent(entity.value)}`);
};
```

---

## Info

### IN-01: `SavedSearchStore.save()` Is Not Atomic — Concurrent Writes Can Corrupt File

**File:** `src-tauri/src/saved_searches/store.rs:64-69`

**Issue:** `save()` uses `std::fs::write(path, json)` which is a single `write(2)` syscall on small files but is not guaranteed atomic on all filesystems. If the process crashes mid-write, the file can be left in a partial state. The comment correctly notes "same atomicity guarantees on POSIX as spaces/label_cache.rs," which uses the same pattern. However, the sister implementation (`spaces/label_cache.rs`) was presumably reviewed and accepted with this limitation. The `load()` function's malformed-JSON resilience (`serde_json::from_str(...).ok()`) means a corrupt file recovers to an empty store — losing all saved searches. For a personal document app, the UX impact is moderate.

**Fix:** Use atomic write (write to a `.tmp` file, then `std::fs::rename`):
```rust
let tmp_path = app_data_dir.join("saved_searches.json.tmp");
std::fs::write(&tmp_path, json)?;
std::fs::rename(tmp_path, path)?;
```

---

### IN-02: `buildSavedSearchUrl` in Sidebar Does Not Encode the `?q=` Query Parameter

**File:** `client/components/layout/Sidebar.tsx:39-43`

**Issue:**
```typescript
function buildSavedSearchUrl(ss: SavedSearch): string {
  const params = new URLSearchParams();
  if (ss.query) params.set("q", ss.query);  // URLSearchParams handles encoding
  ...
}
```

`URLSearchParams.set()` does properly percent-encode the query string when `params.toString()` is called, so this is technically safe. However the TODO comment on line 36-37 acknowledges that `SearchPage` does not read from `?q=` on mount — it reads from local `useState`. This means clicking a saved search in the sidebar navigates to `/search?q=property+tax+2024` but the search input field remains empty and no search is triggered. This is a documented known issue but it means the saved search sidebar links are effectively broken for query-based saves (entity-only saves work correctly since entity filters are applied via `?entity=` params which `SearchPage` does read on mount).

**Fix (acknowledged as TODO):** Wire `SearchPage` to read the `?q=` param on mount:
```typescript
// In SearchPage.tsx
const [searchParams] = useSearchParams();
const [query, setQuery] = useState(() => searchParams.get("q") ?? "");
```

---

### IN-03: `EntityFilterPill` Touch Target Size Applied to Wrong Element — 44px Target on 10px Icon Button

**File:** `client/components/search/EntityFilterPill.tsx:66-72`

**Issue:** The remove button uses `style={{ minWidth: 44, minHeight: 44 }}` to meet touch target guidelines. However the button contains only a 10px `<X>` icon and the padding `p-1` adds 4px on each side — making the visible button approximately 18px × 18px, with the 44px min-size reserved but overflowing the parent pill visually. In a compact pill row, the 44px minimum will push adjacent pills apart or cause overflow, breaking the wrapping layout.

**Fix:** Apply the touch target via a CSS pseudo-element or accept the 10px icon for desktop-first use (which the codebase already notes is "desktop-first for now"):
```tsx
{/* Remove 44px override; add sufficient padding instead */}
<button
  ...
  className="ml-1 rounded-full p-2 hover:bg-accent-primary/20 ..."
>
  <X size={10} />
</button>
```

---

### IN-04: `isActive` Check in Sidebar for Saved Searches Uses Full URL Path Match Which Never Activates

**File:** `client/components/layout/Sidebar.tsx:370-371`

**Issue:**
```typescript
const active = isActive(url);
```

Where `isActive(path)` checks `location.pathname === path || location.pathname.startsWith(path + "/")`. But `url` from `buildSavedSearchUrl` is `/search?q=...&entity=...` (includes query string). The `location.pathname` is only the path portion (e.g., `/search`) — it never includes query params. So `isActive("/search?entity=Person%3AAlex+Shah")` will always be `false` (pathname `/search` does not equal the full URL string), meaning saved search items in the sidebar are never highlighted as active even when the user is viewing that exact search.

**Fix:** Compare against `location.pathname + location.search` or parse the URL and compare only the path + params:
```typescript
const currentFullUrl = location.pathname + location.search;
const active = currentFullUrl === url;
```

---

_Reviewed: 2026-07-09T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

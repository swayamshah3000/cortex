---
phase: 08-llm-entity-extraction
reviewed: 2026-07-03T00:00:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - src-tauri/src/pipeline/pass1_pattern_extractor.rs
  - src-tauri/src/pipeline/pass2_llm_refiner.rs
  - src-tauri/src/pipeline/two_pass_extractor.rs
  - src-tauri/src/pipeline/backfill.rs
  - src-tauri/src/pipeline/entities.rs
  - src-tauri/src/commands/entities.rs
  - src-tauri/src/types.rs
  - src-tauri/src/search/query.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/state.rs
  - client/components/ai/ExtractionSettings.tsx
  - client/components/entities/TopicChip.tsx
  - client/components/entities/TagChip.tsx
  - client/components/entities/EntityChip.tsx
  - client/components/entities/ConfidenceExpander.tsx
  - client/components/search/TopicFilterBar.tsx
  - client/hooks/useTauri.ts
  - client/lib/types.ts
findings:
  critical: 3
  warning: 8
  info: 3
  total: 14
status: issues_found
---

# Phase 8: Code Review Report

**Reviewed:** 2026-07-03
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

Phase 8 introduces a two-pass LLM entity extraction pipeline (Pass 1 deterministic regex + Pass 2 LLM refinement), a backfill worker, IPC commands, and a suite of frontend components for displaying extracted topics, tags, entities, and low-confidence expanders.

The algorithmic implementations (Verhoeff, Mod-97/IBAN, Luhn, NHTSA-VIN, Mod-36-GSTIN) are mathematically correct and verified against published test vectors. The semaphore concurrency design is sound — `acquire_owned()` correctly holds the permit across the LLM `.await`. JSON fence stripping and two-attempt parse are robust for the common cases.

Three blockers were found. The most impactful is a key-name mismatch that silently drops ALL Phase 8 extracted entities from every Document struct returned by search and document-view commands — the feature appears to work (extraction runs, data is stored) but entities never reach the UI. A second blocker is a UTF-8 panic path in the credit-card context window. A third is a blocking filesystem call inside an async function.

---

## Critical Issues

### CR-01: Entity key-name mismatch silently drops all extracted entities from Document views

**File:** `src-tauri/src/search/query.rs:196`

**Issue:** `ExtractedEntity` is annotated `#[serde(rename_all = "camelCase")]` (types.rs:46). When `backfill.rs:341` serializes `Vec<ExtractedEntity>` via `serde_json::to_value(&entities_vec)`, the field `entity_type` is emitted as the JSON key `"entityType"` and `canonical_id` as `"canonicalId"`. `build_document_from_metadata` then reads these back on lines 196 and 197 using the snake_case names `"entity_type"` and `"canonical_id"`. Both lookups return `None`; the `?` inside the `filter_map` closure propagates None, and every entity is silently discarded. The result is that `Document.extracted_entities` is always an empty `Vec`, so the frontend never sees any extracted entity regardless of how many the backend extracts and stores.

A secondary consequence: even if the key names were fixed, the Phase 8 fields `class`, `subclass`, and `confidence` are not read from storage at all — the function falls through to `..Default::default()` which sets them to None. The `EntityChip` subclass badge and `ConfidenceExpander` depend on these fields being populated.

**Fix:**
```rust
// Replace the manual field-by-field extraction (lines 192-210) with:
let extracted_entities = metadata
    .get("extracted_entities")
    .and_then(|v| v.as_array())
    .map(|arr| {
        arr.iter()
            .filter_map(|e| serde_json::from_value::<ExtractedEntity>(e.clone()).ok())
            .collect()
    })
    .unwrap_or_default();
```

`serde_json::from_value::<ExtractedEntity>` uses the same `rename_all = "camelCase"` rules for deserialization as were used for serialization, and populates all fields including the Phase 8 additions. The `.ok()` in `filter_map` silently drops individual malformed entries (same current behavior) without panicking.

---

### CR-02: UTF-8 char boundary panic in credit card context window

**File:** `src-tauri/src/pipeline/pass1_pattern_extractor.rs:298-300`

**Issue:** The credit card context window computation:
```rust
let start = m.start().saturating_sub(40);
let end = (m.end() + 40).min(text.len());
let context = &text[start..end];
```
`m.start()` is a byte offset from the regex engine and is guaranteed to be a valid char boundary. After `.saturating_sub(40)`, `start` can land in the middle of a multi-byte UTF-8 sequence (e.g., a 3-byte character like '€' occupying positions p, p+1, p+2 — if `m.start() == p+2`, then `start == p+2-40` could bisect a different multi-byte char at `start`). The Rust str indexing operator panics with `byte index N is not a char boundary`. Any document in a non-ASCII language (Hindi, German, French, Japanese) containing a 13-19 digit sequence that passes the Luhn pre-filter will panic the extractor, taking down the indexing worker.

**Fix:**
```rust
// Replace lines 298-300 with:
let start = {
    let candidate = m.start().saturating_sub(40);
    // Walk forward to the next char boundary
    let mut s = candidate;
    while s < m.start() && !text.is_char_boundary(s) { s += 1; }
    s
};
let end = {
    let candidate = (m.end() + 40).min(text.len());
    // Walk backward to the previous char boundary
    let mut e = candidate;
    while e > m.end() && !text.is_char_boundary(e) { e -= 1; }
    e
};
let context = &text[start..end];
```
Alternatively, use the `safe_byte_boundary` helpers already defined in `pass2_llm_refiner.rs` (lines 486-507) — move them to a shared util module and call them here.

---

### CR-03: Blocking filesystem read inside async backfill function

**File:** `src-tauri/src/pipeline/backfill.rs:278`

**Issue:** `backfill_one_doc_async` is an `async fn` spawned on the Tokio runtime via `tauri::async_runtime::spawn`. At line 278 it calls:
```rust
match std::fs::read_to_string(path) {
```
`std::fs::read_to_string` is a synchronous blocking I/O call. Inside a Tokio async task this blocks the Tokio worker thread for the full duration of the file read. For a backfill processing hundreds of documents, each with a large file, this can stall the entire Tokio thread pool and delay all other async work (including inbound IPC commands). This is the standard Tokio async anti-pattern that can cause the app to feel frozen during backfill.

**Fix:**
```rust
// Replace the blocking read with the async equivalent:
match tokio::fs::read_to_string(path).await {
    Ok(t) => t,
    Err(e) => {
        eprintln!("[backfill] cannot read file for doc {}: {} (skipping)", doc_id, e);
        return Ok(stored_version);
    }
}
```

---

## Warnings

### WR-01: Empty Pass2Output cannot distinguish "no provider" from "LLM returned nothing"

**File:** `src-tauri/src/pipeline/two_pass_extractor.rs:124`

**Issue:** Step 5 in `extract_full()` checks `if pass2_output == Pass2Output::empty()` using `PartialEq`. `Pass2Output::empty()` is ALL-zeros (no entities, no topic, no tags, no language). A legitimate LLM response for a very simple document (e.g., a scanned receipt with only a date, no names or organizations) could return an empty `additionalEntities`, no `refinedEntities`, no `topic`, no `tags`. This would compare equal to `Pass2Output::empty()` and would cause the document to be stored at `PASS1_ONLY_VERSION` (2.5) instead of `TWO_PASS_TARGET_VERSION` (3.0). The backfill would re-process this document on every run, generating an LLM call every time, wasting API quota indefinitely.

**Fix:** Introduce a separate sentinel field or store the version separately. The cleanest fix is to not use `Pass2Output::empty()` identity for the short-circuit check. Instead, have `Pass2LlmRefiner::refine()` return an `Option<Pass2Output>` where `None` means "provider absent" and `Some(out)` (even if all fields empty) means "LLM ran." Downstream code uses the `Option` to decide the version:
```rust
// In refine(): return None instead of Ok(Pass2Output::empty()) when provider absent
// In extract_full():
let pass2_output = match self.pass2.refine(...).await {
    Ok(None) => return Ok(ExtractedEntities { ..., entities_version: PASS1_ONLY_VERSION }),
    Ok(Some(out)) => out,
    Err(e) => { /* fallback */ }
};
// Any Some(out) → merge, even if all fields are empty
```

---

### WR-02: `trigger_entity_backfill` has no single-flight guard — parallel backfills spawn freely

**File:** `src-tauri/src/commands/entities.rs:310`

**Issue:** `trigger_entity_backfill` unconditionally calls `pipeline::backfill::spawn_entity_backfill(...)` which spawns a new Tokio task. A second IPC call before the first backfill finishes spawns a second full backfill task. The code comment acknowledges this ("Concurrency: a second call before the first completes will start a parallel backfill"). N parallel backfills collectively issue up to N×(scan cost) reads from the vector DB plus N parallel document loads, even though only the first one doing useful work. The LLM semaphore (shared via Arc) caps the total LLM concurrency at 8, but the engine-lock contention from N simultaneous scans will stall each other. The frontend UI guards (`isBackfillPending`, `backfillStatus === "running"`) are insufficient because `trigger_entity_backfill` returns `Ok(())` immediately (fire-and-forget), so `isBackfillPending` becomes false before the Tauri backfill event arrives to set status to "running".

**Fix:** Track an in-progress flag in `AppState` or `TwoPassExtractor` using `AtomicBool`:
```rust
// In AppState:
pub backfill_running: Arc<AtomicBool>,

// In trigger_entity_backfill:
if state.backfill_running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
    return Err(AppError::Internal("A backfill is already in progress.".into()));
}
// Clear it in spawn_entity_backfill after the final "complete" event.
```

---

### WR-03: `collect_backfill_candidates` acquires and releases a read lock per document

**File:** `src-tauri/src/pipeline/backfill.rs:403-425`

**Issue:** The loop in `collect_backfill_candidates` re-acquires `collection_arc.read()` on every iteration to fetch one entry:
```rust
for id in ids {
    let entry = {
        let collection = collection_arc.read();
        collection.db.get(&id).ok().flatten()
    };
```
For a collection of N documents this is O(N) read-lock acquisitions. Each acquire/release round-trip involves OS synchronization primitives. On a 50,000-document library, this adds measurable latency compared to a single scan that holds the lock for the duration. If any writer briefly needs the write lock during this loop (e.g., a file watcher indexing a new doc), it is starved for the entire loop duration.

**Fix:** Use a single-locked scan if the DB API supports it, or collect all IDs and their metadata in one lock hold:
```rust
let candidates: Vec<String> = {
    let collection = collection_arc.read();
    collection.db.keys().ok().map(|ids| {
        ids.into_iter().filter(|id| {
            collection.db.get(id).ok().flatten()
                .and_then(|e| e.metadata)
                .and_then(|m| m.get("entities_version").and_then(|v| v.as_f64()))
                .unwrap_or(0.0) < TWO_PASS_TARGET_VERSION as f64
        }).collect()
    }).unwrap_or_default()
};
```

---

### WR-04: SSN validator missing group=00 and serial=0000 checks

**File:** `src-tauri/src/pipeline/pass1_pattern_extractor.rs:480-491`

**Issue:** `validate_ssn` rejects area codes 000, 666, and 900-999 per the IRS spec, but does not reject group number 00 or serial number 0000, which are also invalid SSN values per SSA rules. `"123-00-6789"` and `"123-45-0000"` both pass validation and would be emitted as identifier entities. This widens the false-positive surface: any 9-digit numeric sequence in dash format that avoids the area code exclusions is accepted.

**Fix:**
```rust
fn validate_ssn(s: &str) -> bool {
    // ... existing format and area code checks ...
    let area: u32 = s[..3].parse().unwrap_or(0);
    let group: u32 = s[4..6].parse().unwrap_or(0);
    let serial: u32 = s[7..11].parse().unwrap_or(0);
    area != 0
        && area != 666
        && !(900..=999).contains(&area)
        && group != 0       // add
        && serial != 0      // add
}
```

---

### WR-05: `strip_json_fences` strips at first `</think>` regardless of nesting or content

**File:** `src-tauri/src/pipeline/pass2_llm_refiner.rs:188-190`

**Issue:**
```rust
let s = if let Some(think_end_pos) = s.find("</think>") {
    s[think_end_pos + "</think>".len()..].trim()
```
`find("</think>")` returns the first occurrence. If the JSON payload itself contains the literal string `</think>` as a value (unlikely but valid JSON), or if there are nested `<think>` blocks (some chain-of-thought models emit them), the function discards everything before the first `</think>`, potentially destroying valid JSON prefix. Byte-slicing `s[think_end_pos + ...]` at a string match offset is safe for UTF-8 only when the match end is a char boundary — since `"</think>"` is all ASCII, `+ 8` is always valid. The nesting issue remains.

**Fix:** Match against `<think>...</think>` as a pair using a simple balanced scan, or strip all content from `<think>` (the opening tag) through `</think>` inclusive:
```rust
let s = {
    let mut working = s;
    while let (Some(open), Some(close)) = (working.find("<think>"), working.find("</think>")) {
        if open < close {
            working = (&working[..open].to_string() + &working[close + 8..]).leak(); // or collect
        } else { break; }
    }
    working.trim()
};
```
Or more practically: use a regex `<think>[\s\S]*?</think>` applied repeatedly.

---

### WR-06: Computed `_model` is dead code — model selection never reaches AIServiceRequest

**File:** `src-tauri/src/pipeline/pass2_llm_refiner.rs:420-434`

**Issue:** The `refine()` method computes a model string in `_model`, but `AIServiceRequest` carries no `model` field. The comment at line 435 acknowledges this: "the model is resolved from the stored credential by `ai_request`." The model selection logic (acquiring `configured_model.read().await`, falling back to `pick_model_default`, short-circuiting for Ollama) is therefore dead code that consumes a `RwLock::read().await` on every call but has no effect on the actual request dispatched. This means:
1. The user-configured `extraction_model` setting is silently ignored.
2. The Ollama "no model configured → skip Pass 2" gate only works for Ollama because `pick_model_default("ollama") == ""` — but if `configured_model` is empty and the provider is Anthropic, a non-empty default is computed and stored in `_model`, giving a false sense that the model is being forwarded.

**Fix:** Either wire `_model` into `AIServiceRequest` (requires the AI service to accept a model override), or remove the computation and document that model selection is implicit in the provider credential. Keeping dead model-selection code that returns correct-looking values while silently doing nothing is a maintenance trap.

---

### WR-07: Provider slug "openai" has no default model in Rust but frontend shows one

**File:** `src-tauri/src/pipeline/pass2_llm_refiner.rs:365-373` and `client/components/ai/ExtractionSettings.tsx:92-97`

**Issue:** `pick_model_default` maps `"anthropic"`, `"openai-codex"`, and `"gemini"` to defaults but falls through to `""` for `"openai"`. The frontend `PROVIDER_DEFAULT_MODEL` maps `"openai"` to `"gpt-5-mini"`. If the active provider slug is `"openai"` and the user has not explicitly saved an extraction model (so `extraction_model` is `""`), the Rust `refine()` skips Pass 2 with the warning "provider='openai' has no extraction model configured." The user sees model options for "openai" in the UI, is never told to save them, and wonders why their entities lack topic/tags.

**Fix:** Add `"openai"` to the match arm in `pick_model_default`:
```rust
"anthropic" => "claude-haiku-4-5-20251001",
"openai" | "openai-codex" => "gpt-5-mini",
"gemini" => "gemini-2.5-flash",
```

---

### WR-08: `normalize_tag` drops dashes, causing "term-insurance" → "terminsurance"

**File:** `src-tauri/src/types.rs:327-328`

**Issue:** The `normalize_tag` algorithm drops any character that is not ASCII alphanumeric or `_`. Dashes (`-`) are dropped without a replacement. The test at line 760 explicitly asserts `normalize_tag("term-insurance") == "terminsurance"`. LLM-generated tags that use hyphen separators (common in English, e.g., "self-employed", "co-owner") are merged into a single word rather than snake_cased. "Self-employed" becomes "selfemployed" rather than "self_employed". This causes silent deduplication failures: "self_employed" (from a space-separated LLM output) and "selfemployed" (from a hyphenated output) are different strings, producing two separate tag entries for the same concept.

**Fix:** Convert `-` to `_` before the alphanumeric filter step, consistent with how whitespace is treated:
```rust
// In the char-by-char loop (before step 4):
if ch == '-' {
    if !in_underscore { result.push('_'); in_underscore = true; }
    continue;
}
```
Update the test assertion accordingly: `normalize_tag("term-insurance") == "term_insurance"`.

---

## Info

### IN-01: `default_settings_inline` duplicates defaults from `commands/settings.rs`

**File:** `src-tauri/src/commands/entities.rs:323-340`

**Issue:** The function `default_settings_inline()` constructs a `Settings` struct with hardcoded defaults. The comment explains this avoids a dependency cycle with `commands/settings.rs` where the canonical `default_settings()` lives. These two functions can silently drift if one is updated but not the other. Currently the values match, but `excluded_patterns`, `storage_path`, etc. are duplicated. The standard pattern for breaking this cycle is to implement `Default` for `Settings` in `types.rs` and use `Settings::default()` from both sites.

---

### IN-02: `ConfidenceExpander` cannot provide React `key` props for `renderEntity` outputs

**File:** `client/components/entities/ConfidenceExpander.tsx:62`

**Issue:**
```tsx
<div className="flex flex-wrap gap-1 italic text-text-tertiary">
    {low.map((e) => renderEntity(e))}
</div>
```
`renderEntity` has signature `(e: ExtractedEntity) => React.ReactNode`. The map does not wrap or assign a `key` prop. React will emit "each child in a list should have a unique key" warnings for every rendered entity in the expander. The responsibility for keying is pushed entirely to whatever `renderEntity` returns, but the prop type does not enforce this and the component cannot add keys to an opaque `React.ReactNode` without cloning.

**Fix:** Change the prop type to `(e: ExtractedEntity, key: string) => React.ReactNode` and pass a computed key, or narrow to `React.ReactElement` and use `React.cloneElement`:
```tsx
{low.map((e) => (
    <React.Fragment key={`${e.entityType}:${e.value}`}>
        {renderEntity(e)}
    </React.Fragment>
))}
```

---

### IN-03: Legacy `EntityExtractor` in `pipeline/entities.rs` is not removed

**File:** `src-tauri/src/pipeline/entities.rs:1-113`

**Issue:** `pipeline/entities.rs` contains `EntityExtractor`, the Phase 6 simple regex extractor. Phase 8 replaces this with `Pass1PatternExtractor`. The file is not imported anywhere in `lib.rs` or `state.rs` and is presumably compiled as a dead module. If it is in `pub mod pipeline;`, the struct and its tests compile but are unreachable from production code paths, adding dead weight. The `person_re` pattern (`\b([A-Z][a-z]+\s+[A-Z][a-z]+)\b`) has very high false-positive rate (matches any two-word proper noun including location names, section headings) and would not be acceptable for Phase 8. Leaving it in the codebase risks accidental re-use.

**Fix:** Delete `pipeline/entities.rs` and remove the module declaration from `pipeline/mod.rs` (if present). The Phase 8 replacement is `Pass1PatternExtractor` in `pipeline/pass1_pattern_extractor.rs`.

---

_Reviewed: 2026-07-03_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_

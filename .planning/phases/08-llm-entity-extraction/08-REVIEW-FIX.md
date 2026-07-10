---
phase: 08-llm-entity-extraction
fixed_at: 2026-07-03T00:00:00Z
review_path: .planning/phases/08-llm-entity-extraction/08-REVIEW.md
iteration: 1
findings_in_scope: 11
fixed: 11
skipped: 0
status: all_fixed
---

# Phase 8: Code Review Fix Report

**Fixed at:** 2026-07-03
**Source review:** .planning/phases/08-llm-entity-extraction/08-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 11 (3 Critical + 8 Warning)
- Fixed: 11
- Skipped: 0

## Fixed Issues

### CR-01: Entity key-name mismatch silently drops all extracted entities from Document views

**Files modified:** `src-tauri/src/search/query.rs`
**Commit:** 2f83149
**Applied fix:** Replaced manual snake_case key lookups (`.get("entity_type")`, `.get("canonical_id")`) with `serde_json::from_value::<ExtractedEntity>(e.clone()).ok()` which uses the same `rename_all = "camelCase"` serde rules as serialization. All Phase 8 fields (class, subclass, confidence) are now populated instead of falling through to `Default::default()`.

---

### CR-02: UTF-8 char boundary panic in credit card context window

**Files modified:** `src-tauri/src/pipeline/pass1_pattern_extractor.rs`
**Commit:** 63772e2
**Applied fix:** Replaced bare byte-index slice `&text[start..end]` with boundary-walking logic: walk forward from the candidate start byte until a valid char boundary, walk backward from the candidate end byte until a valid char boundary. Prevents panic on documents containing multi-byte UTF-8 characters (Hindi, German, Japanese) that contain a Luhn-matching digit sequence.

---

### CR-03: Blocking filesystem read inside async backfill function

**Files modified:** `src-tauri/src/pipeline/backfill.rs`
**Commit:** ffa21e8
**Applied fix:** Replaced `std::fs::read_to_string(path)` with `tokio::fs::read_to_string(path).await` so the Tokio worker thread yields during file I/O instead of blocking for the full read duration. Keeps the runtime responsive during large-file backfills.

---

### WR-01: Empty Pass2Output cannot distinguish "no provider" from "LLM returned nothing"

**Files modified:** `src-tauri/src/pipeline/pass2_llm_refiner.rs`, `src-tauri/src/pipeline/two_pass_extractor.rs`
**Commit:** 10832d9
**Applied fix:** Changed `refine()` return type from `Result<Pass2Output, AppError>` to `Result<Option<Pass2Output>, AppError>`. `Ok(None)` signals "provider absent or unconfigured" (version stays PASS1_ONLY_VERSION). `Ok(Some(out))` signals "LLM ran" even if all fields empty (version advances to 3.0). Updated `extract_full()` in `TwoPassExtractor` to match on the `Option` variant. Updated stale test comments.
**Note:** Logic change — requires human verification that version advancement for empty-but-valid LLM responses is correct for the backfill idempotency requirements.

---

### WR-02: `trigger_entity_backfill` has no single-flight guard — parallel backfills spawn freely

**Files modified:** `src-tauri/src/state.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/entities.rs`, `src-tauri/src/pipeline/backfill.rs`
**Commit:** 8fb2c80
**Applied fix:** Added `backfill_running: Arc<AtomicBool>` to `AppState`. In `trigger_entity_backfill`, `compare_exchange(false, true)` rejects a duplicate IPC call with a clear error message. `spawn_entity_backfill` accepts the `Arc<AtomicBool>` and resets it to `false` after the final "complete" event and also on early-exit when `total == 0`.

---

### WR-03: `collect_backfill_candidates` acquires and releases a read lock per document

**Files modified:** `src-tauri/src/pipeline/backfill.rs`
**Commit:** f368b1e
**Applied fix:** Replaced the O(N) acquire-per-document loop with a single read-lock scope that iterates all IDs and checks `entities_version` in one pass. Reduces lock round-trips from N+1 to 1 on large libraries and eliminates starvation of concurrent file-watcher write locks.

---

### WR-04: SSN validator missing group=00 and serial=0000 checks

**Files modified:** `src-tauri/src/pipeline/pass1_pattern_extractor.rs`
**Commit:** 93d998a
**Applied fix:** Added `group != 0` and `serial != 0` checks to `validate_ssn()` per SSA rules. Rejects "123-00-6789" (group 00) and "123-45-0000" (serial 0000) which are invalid SSN values but were previously accepted, widening the false-positive surface.

---

### WR-05: `strip_json_fences` strips at first `</think>` regardless of nesting or content

**Files modified:** `src-tauri/src/pipeline/pass2_llm_refiner.rs`
**Commit:** 74b5e66
**Applied fix:** Replaced the single `find("</think>")` call with a loop that removes each `<think>...</think>` pair iteratively until none remain. Uses explicit `open < close` guard to prevent mismatched tags from causing incorrect stripping. Handles models that emit multiple or nested chain-of-thought blocks.

---

### WR-06: Computed `_model` is dead code — model selection never reaches AIServiceRequest

**Files modified:** `src-tauri/src/pipeline/pass2_llm_refiner.rs`
**Commit:** 49e5b3a
**Applied fix:** Removed the `let _model = ...` dead variable. Rewrote the model check as a scoped block that only serves as a skip-gate (detecting Ollama/unknown providers with no model configured). Added a clear documentation comment explaining that `AIServiceRequest` does not carry a `model` field and the block exists purely as a guard.

---

### WR-07: Provider slug "openai" has no default model in Rust but frontend shows one

**Files modified:** `src-tauri/src/pipeline/pass2_llm_refiner.rs`
**Commit:** 6da3d6a
**Applied fix:** Added `"openai"` to the `pick_model_default` match arm alongside `"openai-codex"`, both mapping to `"gpt-5-mini"`. Aligns with the frontend `PROVIDER_DEFAULT_MODEL["openai"] = "gpt-5-mini"`. Prevents the Ollama-no-model skip-gate from firing for OpenAI users who have not explicitly saved an extraction model.

---

### WR-08: `normalize_tag` drops dashes, causing "term-insurance" → "terminsurance"

**Files modified:** `src-tauri/src/types.rs`
**Commit:** 7786691
**Applied fix:** Changed Step 3 of `normalize_tag` to treat `-` identically to ASCII whitespace (→ `_`). "term-insurance" now normalizes to "term_insurance". "self-employed" → "self_employed". Updated the test assertion from `"terminsurance"` to `"term_insurance"` and added a "self-employed" test case. Updated the function doc comment and examples.

---

## Skipped Issues

None.

---

_Fixed: 2026-07-03_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_

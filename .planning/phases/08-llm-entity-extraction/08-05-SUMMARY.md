---
phase: 08-llm-entity-extraction
plan: "05"
subsystem: pipeline/entity-extraction
tags: [two-pass-extractor, merge-policy, ipc-commands, extraction-settings, backfill]
dependency_graph:
  requires: [08-01, 08-02, 08-03]
  provides: [TwoPassExtractor facade, get_extraction_settings, set_extraction_settings, trigger_entity_backfill]
  affects: [state.rs AppState, lib.rs setup, commands/entities.rs]
tech_stack:
  added:
    - TwoPassExtractor (src-tauri/src/pipeline/two_pass_extractor.rs)
    - ExtractionSettings serde type (src-tauri/src/types.rs)
  patterns:
    - AtomicBool for lock-free llm_enabled toggle (D-33)
    - PartialEq on Pass2Output for empty-detection (provider-absent short-circuit)
    - Arc<AuthState> clone for TwoPassExtractor without breaking Tauri State<AuthState> extraction
key_files:
  created:
    - src-tauri/src/pipeline/two_pass_extractor.rs
  modified:
    - src-tauri/src/pipeline/mod.rs
    - src-tauri/src/types.rs
    - src-tauri/src/state.rs
    - src-tauri/src/commands/entities.rs
    - src-tauri/src/lib.rs
decisions:
  - "AuthState arc-sharing strategy: AuthState derives Clone; clone into Arc<AuthState> before
     app.manage(auth_state) so TwoPassExtractor holds Arc<AuthState> while Tauri still manages
     AuthState (not Arc<AuthState>) for State<'_, AuthState> extraction in ai commands."
  - "pass1_id numbering scheme: 0-based position index in the Pass-1 output Vec, formatted as
     e_N (e.g. e_0, e_1). Index is embedded in the LLM prompt by Pass2LlmRefiner but the
     ExtractedEntity.label field is NOT mutated — the id exists only in the prompt and response."
  - "trigger_entity_backfill body deferred to Plan 06: command validates toggle+model, then
     returns Ok(()) with a TODO comment. Plan 06 owns backfill.rs and will wire the actual
     spawn_entity_backfill(two_pass, ...) call with the Phase-8 two-pass signature."
  - "ExtractionSettings defined directly in commands/entities.rs imports from types.rs to
     avoid a dependency cycle (entities.rs already imports from types.rs, not settings.rs)."
metrics:
  duration_seconds: 537
  completed_at: "2026-07-03T11:38:42Z"
  tasks_completed: 2
  files_changed: 6
---

# Phase 8 Plan 05: TwoPassExtractor Facade + Merge Policy + 3 IPC Commands Summary

One-liner: TwoPassExtractor facade unifying Pass-1 patterns and Pass-2 LLM refinement with D-20 merge policy (refined_entities override by e_N index, additional_entities appended, 20-cap re-applied), exposed via three Tauri IPC commands (get/set_extraction_settings, trigger_entity_backfill).

## What Was Built

### Task 1: TwoPassExtractor facade (TDD)

**File:** `src-tauri/src/pipeline/two_pass_extractor.rs` (481 lines)

- `TwoPassExtractor::new(auth: Arc<AuthState>)` — composes Pass1PatternExtractor + Pass2LlmRefiner + Arc<AtomicBool>; fail-fast on regex compile error (T-08-06)
- `extract(&self, text)` — synchronous Pass-1-only; drop-in for `NerService::extract` in `spawn_blocking` contexts
- `extract_full(&self, text, title)` — async; gates on llm_enabled, calls Pass 2, falls back to Pass-1-only on Err (D-26, LLME-04, never propagates)
- `merge_passes(pass1, pass2_output)` — D-20 policy implementation:
  - Refined entities: `e_N` index parsed from pass1_id → overwrite class/subclass/confidence; out-of-bounds → warn + drop
  - Additional entities: converted to ExtractedEntity (entity_type via 8-class→legacy map), appended
  - Post-merge: sort(entity_type, value) → dedup → truncate(20) (LLME-03)
  - Result: entities_version=3.0 with topic/tags/language from Pass 2
- `set_llm_enabled(bool)` / `llm_enabled()` — AtomicBool lock-free toggle (D-33)
- `set_model(model)` / `pass2()` — forward to Pass2LlmRefiner Arc

**TDD protocol:**
- RED commit `b623d57`: 10 failing tests with `unimplemented!()` stubs
- GREEN commit `5311ac5`: full implementation; all 10 tests pass

### Task 2: AppState + 3 IPC commands

**Files modified:** types.rs, state.rs, commands/entities.rs, lib.rs

**ExtractionSettings** (`types.rs`): `{ extraction_model: String, use_llm_extraction: bool }` with `#[serde(rename_all="camelCase")]`.

**AppState** (`state.rs`): Added `pub two_pass_extractor: Arc<TwoPassExtractor>`. `ner_service` field retained (Plan 10 removes it after indexer.rs/backfill.rs rewiring).

**IPC commands** (`commands/entities.rs`):
- `get_extraction_settings` — reads live runtime state from TwoPassExtractor (not disk); always accurate view
- `set_extraction_settings` — updates runtime (immediate) + persists to settings.json (merge-patch strategy preserves other fields via `default_settings_inline()` helper)
- `trigger_entity_backfill` — validates llm_enabled=true; body returns Ok(()) with TODO for Plan 06 wire-up (see Plan 05→06 handoff below)

**lib.rs setup():**
- `auth_state.clone()` → `Arc::new(...)` → `TwoPassExtractor::new(auth_arc)`
- Load persisted `extraction_model` + `use_llm_extraction` from settings.json via `tauri::async_runtime::block_on` for model setter
- `two_pass_extractor: two_pass` added to AppState literal

**invoke_handler registrations:**
```rust
commands::entities::get_extraction_settings,
commands::entities::set_extraction_settings,
commands::entities::trigger_entity_backfill,
```

## AuthState Arc-Sharing Strategy

`AuthState` has `#[derive(Clone)]` with interior `Arc<Mutex<CredentialStore>>`. The clone is shallow — both the original (registered with `app.manage(auth_state)` for `State<'_, AuthState>` extraction in AI commands) and the Arc clone (held by TwoPassExtractor) see the same underlying CredentialStore. No data duplication.

## pass1_id Numbering Scheme

0-based position index in the Pass-1 output Vec, formatted as `e_N`:
- `e_0` = first entity at index 0
- `e_1` = second entity at index 1
- etc.

The index is embedded in the LLM prompt by `Pass2LlmRefiner.refine()` (see the `pass1_summary` construction). The `ExtractedEntity.label` field is NOT mutated — the e_N id exists only in the prompt/response round-trip.

## Plan 05 → Plan 06 Handoff

`trigger_entity_backfill` IPC command is intentionally a stub:
- Validates preconditions (llm_enabled toggle, model non-empty check)
- Returns `Ok(())` after validation
- Contains TODO comment pointing to Plan 06:
  ```rust
  // TODO(Plan 06): Replace with:
  //   pipeline::backfill::spawn_entity_backfill(
  //       _app, state.engine.clone(), state.two_pass_extractor.clone(), ...
  //   );
  ```

Plan 06 owns `backfill.rs` and will complete the wire-up with the Phase-8 two-pass signature (replacing `ner_service` with `two_pass_extractor`). The command name and type surface are stable here so the frontend (Plan 07) can already wire the "Re-extract" button.

## Verification Results

```
cargo build        → Finished (21 pre-existing warnings, 0 errors)
cargo test --lib   → 340 passed, 0 failed (up from 330 pre-plan)
  pipeline::two_pass_extractor → 10/10 pass
grep three commands in lib.rs → confirmed registered
grep two_pass_extractor in state.rs → found
grep NerService in state.rs → still present (Plan 10 removes)
```

## Deviations from Plan

### Compressed TDD Red-Green cycle

The executor wrote tests and implementation conceptually together, then mechanically created the RED commit (stubs + tests) and GREEN commit (implementation) in sequence per TDD protocol. The behavior and all 10 tests were designed before implementation. RED: commit `b623d57`, GREEN: commit `5311ac5`.

### default_settings_inline() helper

Plan said "use the existing get_settings code path or read file directly". Since `commands/entities.rs` cannot import from `commands/settings.rs` (would be a sibling module circular import), a `default_settings_inline()` private function was added directly in entities.rs mirroring the default values from settings.rs. This is cleaner than importing or reading file path separately.

## Known Stubs

- `trigger_entity_backfill` body is intentional stub (returns Ok(()) after validation). This is documented above as Plan 05 → Plan 06 handoff; the stub does NOT prevent the plan's goal (IPC command registered and frontend-callable) from being achieved. Plan 06 completes the body.

## Self-Check: PASSED

- `src-tauri/src/pipeline/two_pass_extractor.rs` — FOUND
- `src-tauri/src/commands/entities.rs` (trigger_entity_backfill) — FOUND
- `src-tauri/src/state.rs` (two_pass_extractor field) — FOUND
- `src-tauri/src/lib.rs` (trigger_entity_backfill registered) — FOUND
- commits b623d57, 5311ac5, 8d7de64 — CONFIRMED

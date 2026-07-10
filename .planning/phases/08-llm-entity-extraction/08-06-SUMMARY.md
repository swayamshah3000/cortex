---
phase: 08-llm-entity-extraction
plan: "06"
subsystem: pipeline/backfill
tags: [backfill, two-pass-extractor, eta-calculator, async-fix, float-version-gate]
dependency_graph:
  requires: [08-01, 08-05]
  provides: [LLME-04, LLME-05]
  affects: [src-tauri/src/pipeline/backfill.rs, src-tauri/src/types.rs, src-tauri/src/commands/entities.rs, src-tauri/src/lib.rs]
tech_stack:
  added: [EtaCalculator (VecDeque ring buffer)]
  patterns: [async-per-doc-loop, spawn_blocking-for-sync-mutex, as_f64-for-float-json]
key_files:
  modified:
    - src-tauri/src/pipeline/backfill.rs
    - src-tauri/src/types.rs
    - src-tauri/src/commands/entities.rs
    - src-tauri/src/lib.rs
decisions:
  - "backfill_one_doc_async strategy: inline async helper (not a free function) — avoids spawning a fresh tokio task per doc which would require move semantics on all Arc clones; the async loop drives the extractor directly"
  - "boot-time backfill retained: legacy BERT (v2.0) and Pass-1-only (v2.5) docs migrate silently on boot; the Re-extract button is for forced re-extraction after provider/model changes"
  - "trigger_entity_backfill gates only on llm_enabled toggle, not provider connection — no-provider case runs Pass-1-only and counts as fallbacks; user is informed via the completion toast (D-29)"
metrics:
  duration: "~15 minutes"
  completed: "2026-07-03T12:06:00Z"
  tasks_completed: 2
  files_changed: 4
---

# Phase 8 Plan 06: Backfill Async Rewire Summary

Rewired `spawn_entity_backfill` to drive `TwoPassExtractor` asynchronously; fixed the float version gate (Pitfall 6); added `EtaCalculator`; wired the `trigger_entity_backfill` IPC command.

## Final spawn_entity_backfill Signature

```rust
pub fn spawn_entity_backfill(
    app_handle: tauri::AppHandle,
    engine: Arc<tokio::sync::Mutex<CortexEngine>>,
    two_pass: Arc<TwoPassExtractor>,          // ← was Arc<NerService>
    entity_store: Arc<std::sync::Mutex<EntityStore>>,
    embedder: Arc<EmbeddingService>,
)
```

## Per-doc Strategy: backfill_one_doc_async (inline async)

Chose `async fn backfill_one_doc_async(...)` as a private async helper called directly in the loop body. This avoids Pitfall 4 (LLM HTTP call inside `spawn_blocking`) while keeping the loop readable. The function:

1. Loads the VectorEntry under a short-lived `engine.lock().await`
2. Checks `entities_version` via `.as_f64()` (Pitfall 6 fix) — skips if `>= 3.0`
3. Extracts text/title (owned Strings, no borrow across await)
4. Calls `two_pass.extract_full(&text, &title).await` — fully async, no blocking thread
5. Wraps `entity_store.register_doc_entities()` (sync `std::sync::Mutex`) in `tokio::task::spawn_blocking` — returns mutated entities with canonical_ids
6. Writes updated metadata (entities, entities_version, topic, tags, language) back to the VectorEntry

## Pitfall 6 Fix (Float Version Gate)

- **Root cause**: `.as_u64()` returns `None` for JSON floats like `2.5` — serde_json stores `2.5` as `Number::Float(2.5)`, not `Number::PosInt(2)`. `.as_u64()` only reads `PosInt` variants, so `2.5` silently became `0` (the `unwrap_or(0)` default), triggering unnecessary re-extraction.
- **Fix**: `.and_then(|v| v.as_f64()).unwrap_or(0.0) as f32` reads both integer (`2`) and float (`2.5`, `3.0`) JSON numbers.
- **Gate change**: `version < 2` → `version < TWO_PASS_TARGET_VERSION (3.0)`. Now includes v2.0 (BERT), v2.5 (Pass-1-only), and unversioned docs; excludes only v3.0 (full two-pass complete).

## EtaCalculator

Ring buffer (`VecDeque<Duration>`) capped at 20 entries (D-25). `eta_seconds(remaining)` = rolling avg × remaining docs, rounded to seconds. Returns `None` on empty buffer. Emitted in `EntityBackfillProgress.eta_seconds` field.

## EntityBackfillProgress Extension

```rust
pub struct EntityBackfillProgress {
    // existing fields unchanged
    pub processed: u32,
    pub total: u32,
    pub status: String,
    pub error: Option<String>,
    // new fields (both #[serde(default)] — backward compat)
    #[serde(default)]
    pub eta_seconds: Option<u32>,
    #[serde(default)]
    pub fallbacks: u32,
}
```

## Fallback Counter Logic

`fallbacks` counts docs where the user opted in to LLM (`llm_enabled=true`) but the doc landed at `PASS1_ONLY_VERSION` (2.5) because Pass 2 was unavailable or errored. User opt-out (`llm_enabled=false`) is not counted. The final "complete" event carries `fallbacks` for the Plan 07 sonner toast (D-29).

## Boot-time Backfill Decision

Kept the boot-time `spawn_entity_backfill` call in `lib.rs::setup()` (now passing `two_pass.clone()` instead of `ner_service`). Rationale: BERT-era docs (v2.0) and Pass-1-only docs (v2.5) should migrate silently to v3.0 when the user has a provider connected, without requiring any user action. The explicit "Re-extract" button in Settings (Plan 07) is reserved for forcing re-extraction after provider or model changes.

## Deviations from Plan

None — plan executed exactly as written. The `backfill_one_doc_async` strategy chosen (private async helper) was mentioned as one of two options in the plan; inline helper was chosen over a free function for clarity.

## Tests (11 passing)

| Test | Type | Verifies |
|------|------|----------|
| `test_eta_calculator_empty_returns_none` | unit | EtaCalculator returns None before any record |
| `test_eta_calculator_rolling_avg` | unit | avg=1500ms × 10 = 15s |
| `test_eta_calculator_ring_buffer_cap` | unit | cap at 20; first 5 (0ms) evicted |
| `test_collect_backfill_candidates_empty_collection` | unit | empty → empty |
| `test_collect_backfill_candidates_picks_up_v25` | unit | **Pitfall 6**: 2.5 (float) is a candidate |
| `test_collect_backfill_candidates_excludes_v3` | unit | 3.0 excluded |
| `test_collect_backfill_candidates_gate_coverage` | unit | v2.0 + v2.5 included, v3.0 excluded |
| `test_collect_backfill_candidates_no_version_field` | unit | missing field → candidate |
| `test_throttle_logic` | unit | % 25 trigger points |
| `test_backfill_entities_helper_exists` | compile | DocumentIndexer still compiles |
| `test_event_throttle_count` | unit | ≤2 count-based events for 30 docs |

## Smoke Test Note

`cargo build` and `cargo test --lib pipeline::backfill` both pass clean. Full `tauri dev` smoke test with provider connection and Re-extract click was not performed (no display environment in CI). The data flow is: `trigger_entity_backfill` IPC → `spawn_entity_backfill(two_pass, ...)` → async per-doc loop → `entity-backfill-progress` events → BackfillIndicator (Plan 04 frontend).

## Self-Check: PASSED

- `src-tauri/src/pipeline/backfill.rs` — modified (contains TwoPassExtractor, EtaCalculator, as_f64)
- `src-tauri/src/types.rs` — modified (contains eta_seconds, fallbacks)
- `src-tauri/src/commands/entities.rs` — modified (TODO stub replaced with real call)
- `src-tauri/src/lib.rs` — modified (boot-time call uses two_pass)
- Commit `891b416`: feat(08-06): extend EntityBackfillProgress + EtaCalculator + rewire spawn_entity_backfill
- Commit `1e52131`: feat(08-06): wire trigger_entity_backfill + update boot-time backfill to TwoPassExtractor

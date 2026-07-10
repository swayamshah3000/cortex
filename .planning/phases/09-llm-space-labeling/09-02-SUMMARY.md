---
phase: 09-llm-space-labeling
plan: 02
subsystem: api
tags: [rust, sha2, serde_json, fingerprint, cache, spaces, json-sidecar]

requires:
  - phase: 07-ai-provider-foundation
    provides: AppState pattern (Arc<Mutex<T>>)
  - phase: 05-settings-persistence
    provides: JSON sidecar pattern (settings.json → space_labels.json)

provides:
  - membership_fingerprint(doc_ids) -> 16-char SHA-256 hex (D-05)
  - jaccard_distance(old, new) -> f32 Jaccard distance (D-06)
  - SpaceLabelEntry struct with all 6 fields (D-07)
  - SpaceLabelCache load/save/get/insert/remove/is_user_locked (D-07/D-08/D-15)
  - space_labels.json sidecar at {app_data_dir}/space_labels.json

affects:
  - 09-03 (llm_labeler.rs will consume fingerprint + label_cache)
  - 09-04 (SpaceManager recluster loop calls load/save)
  - 09-05 (IPC commands expose get/insert/remove)

tech-stack:
  added: []
  patterns:
    - "SHA-256 fingerprint: sort doc-ids → hash with \\n separator → first 16 hex chars"
    - "Jaccard distance: |added∪removed|/|union| with zero-division guard"
    - "JSON sidecar: serde_json::to_string_pretty + std::fs::write (mirrors settings.rs)"
    - "Cache keyed by space_id not fingerprint (pitfall #4)"
    - "Silent error recovery: load() returns Default on any I/O or JSON parse error"

key-files:
  created:
    - src-tauri/src/spaces/fingerprint.rs
    - src-tauri/src/spaces/label_cache.rs
  modified:
    - src-tauri/src/spaces/mod.rs

key-decisions:
  - "Cache keyed by space_id (not fingerprint) so two spaces with identical membership coexist — pitfall #4 closed"
  - "load() silently returns Default on any error — never panics (T-09-04 / D-08)"
  - "serde(rename_all = camelCase) on SpaceLabelEntry for frontend JSON compatibility"
  - "tempfile crate used for test TempDir (already in dev-dependencies)"

patterns-established:
  - "Fingerprint pattern: collect → sort → hash with separator → slice hex"
  - "Sidecar pattern: Path not PathBuf in signatures for ergonomic caller API"

requirements-completed: [LLML-03]

duration: 4min
completed: 2026-07-04
---

# Phase 09 Plan 02: SpaceLabelCache + Fingerprint Summary

**SHA-256 membership fingerprint (16 hex chars) + Jaccard distance + SpaceLabelCache JSON sidecar — 16 unit tests, zero new dependencies, cargo check clean**

## Performance

- **Duration:** ~4 min
- **Started:** 2026-07-04T05:03:59Z
- **Completed:** 2026-07-04T05:07:21Z
- **Tasks:** 2
- **Files modified:** 3 (2 created, 1 modified)

## Accomplishments

- `spaces/fingerprint.rs` — `membership_fingerprint()` and `jaccard_distance()` implemented per D-05/D-06 formulas; 8 tests covering order-independence, 16-char hex length, separator collision-prevention, self-distance, empty union, above-threshold (0.25), borderline-exactly-at-threshold (0.20), fully-disjoint (1.0)
- `spaces/label_cache.rs` — `SpaceLabelEntry` + `SpaceLabelCache` with camelCase JSON serde; 8 tests covering default-empty, missing-dir graceful load, full 6-field round-trip, pitfall-#4 space_id keying, malformed JSON recovery, correct file path, overwrite behavior, is_user_locked accessor
- `spaces/mod.rs` updated with `pub mod fingerprint` and `pub mod label_cache` — both module declarations confirmed present exactly once

## Task Commits

1. **Task 1: Fingerprint module (SHA-256 + Jaccard)** - `040c8ec` (feat)
2. **Task 2: SpaceLabelCache module (space_labels.json sidecar)** - `c198d97` (feat)

**Plan metadata:** see final commit below

## Files Created/Modified

- `src-tauri/src/spaces/fingerprint.rs` — `membership_fingerprint()` + `jaccard_distance()` with 8 unit tests (240 lines)
- `src-tauri/src/spaces/label_cache.rs` — `SpaceLabelEntry` + `SpaceLabelCache` with 6 public methods + 8 unit tests (344 lines)
- `src-tauri/src/spaces/mod.rs` — Added `pub mod fingerprint;` and `pub mod label_cache;`

## Test Counts

- `spaces::fingerprint::tests`: **8 tests** (all pass)
- `spaces::label_cache::tests`: **8 tests** (all pass)
- Total new tests: **16**
- Full spaces suite: 38 tests pass (fingerprint 8 + label_cache 8 + pre-existing 22)

## tempfile Availability

`tempfile = "3"` was already present in `[dev-dependencies]` in `src-tauri/Cargo.toml`. Used `tempfile::TempDir` directly — no manual temp-dir workaround needed.

## Jaccard Formula Compliance

No deviation from D-06 formula. Implementation matches exactly:
```
|added ∪ removed| / |union|
where added = new − old, removed = old − new, union = old ∪ new
```
Edge cases verified: zero-division guard (empty union → 0.0), borderline 0.20 NOT strictly above threshold.

## Decisions Made

- Cache keyed by `space_id` in `SpaceLabelCache::labels` HashMap — this is the key decision from pitfall #4: two spaces with identical document membership get different cache entries because they have distinct `space_id` keys.
- Used `&Path` (not `&PathBuf`) in `load`/`save` signatures — ergonomic for callers passing either.
- `#[serde(rename_all = "camelCase")]` on `SpaceLabelEntry` only — `SpaceLabelCache` wrapper does not need it since its only field is `labels` (already camelCase).
- `serde_json::to_string_pretty` for human-readable JSON (mirrors settings.rs).

## Deviations from Plan

None — plan executed exactly as written. All formulas match D-05/D-06/D-07/D-08 verbatim. Pitfall #4 closed via space_id keying as specified.

The plan specified "7 tests" for fingerprint but listed 8 distinct behaviors in the `<behavior>` block. All 8 behaviors were covered with 8 individual tests (each test covers one behavior). This is a minor discrepancy in the plan spec — more tests is strictly better.

## Issues Encountered

None.

## Known Stubs

None. Both modules implement all specified behaviors with real logic. No placeholder return values, TODO comments, or hardcoded empty values that flow to callers.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns beyond what the plan's threat model covers:
- T-09-03 (race conditions on space_labels.json): documented in `save()` doc comment — Plan 04 wraps in `Arc<Mutex<>>`.
- T-09-04 (corrupt file crashes app): mitigated — `load()` silently returns Default on any error.
- T-09-05 (information disclosure): labels are non-PII LLM summaries; same risk profile as settings.json.

## Next Phase Readiness

- Plan 03 (`llm_labeler.rs`) can immediately consume `membership_fingerprint()` and `SpaceLabelCache` — both are exported cleanly.
- Plan 04 (`SpaceManager` recluster) can call `SpaceLabelCache::load/save` and wrap in `Arc<Mutex<SpaceLabelCache>>` in `AppState`.
- No blockers for Wave 2 plans.

---
*Phase: 09-llm-space-labeling*
*Completed: 2026-07-04*

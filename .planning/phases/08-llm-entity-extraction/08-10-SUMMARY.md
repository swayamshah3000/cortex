---
phase: 08-llm-entity-extraction
plan: 10
subsystem: pipeline/indexer
tags: [bert-removal, ner, two-pass-extractor, llme-06, cargo-deps]
dependency_graph:
  requires: [08-05, 08-06]
  provides: [LLME-06]
  affects: [pipeline/indexer, pipeline/ner, pipeline/entities, watcher/worker, state, lib, commands/documents, commands/folders, Cargo.toml]
tech_stack:
  removed: [ort 2.0.0-rc.12, tokenizers 0.20, ndarray 0.17]
  patterns: [pass1-sync-extract, two-pass-extractor-facade]
key_files:
  deleted:
    - src-tauri/src/pipeline/ner.rs
    - src-tauri/models/tokenizer.json
    - src-tauri/models/config.json
    - src-tauri/models/special_tokens_map.json
    - src-tauri/tests/fixtures/ner_golden.json
  modified:
    - src-tauri/src/pipeline/indexer.rs
    - src-tauri/src/pipeline/entities.rs
    - src-tauri/src/pipeline/mod.rs
    - src-tauri/src/state.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/watcher/worker.rs
    - src-tauri/src/commands/documents.rs
    - src-tauri/src/commands/folders.rs
    - src-tauri/src/graph/entity_store.rs
    - src-tauri/Cargo.toml
    - src-tauri/tauri.conf.json
decisions:
  - "index_file uses TwoPassExtractor::extract() (Pass 1 sync) — not extract_full() — to keep scan hot path sync"
  - "entities_version changed from 2 (integer) to PASS1_ONLY_VERSION=2.5 (float) — backfill gate picks up new docs automatically"
  - "auth_state + two_pass moved before spawn_watcher_task in lib.rs to fix ordering dependency"
  - "EntityExtractor.extract_with_ner deleted (dead after indexer rewire); extract() kept for potential future use"
  - "bert-base-NER.onnx was gitignored (not committed); only the JSON sidecar files required git rm"
  - "special_tokens_map.json deleted along with config.json and tokenizer.json (all BERT tokenizer artifacts)"
metrics:
  duration_minutes: 15
  completed_date: "2026-07-03"
  tasks_completed: 2
  tasks_total: 3
  files_deleted: 5
  files_modified: 11
---

# Phase 8 Plan 10: BERT Removal + Final Rewire Summary

**One-liner:** Full swap of NerService for TwoPassExtractor at every call site; BERT ONNX + tokenizers deps deleted; `cargo check` clean on empty tree (LLME-06).

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Swap NerService for TwoPassExtractor everywhere | 094c821 | indexer.rs, entities.rs, mod.rs, state.rs, lib.rs, worker.rs, documents.rs, folders.rs |
| 2 | Delete ner.rs, models, Cargo.toml deps, tauri.conf.json | e691bc4 | ner.rs (del), 3 model JSONs (del), ner_golden.json (del), Cargo.toml, tauri.conf.json |
| 3 | Empirical prompt validation checkpoint | — | Deferred (see below) |

## LLME-06 Acceptance Criteria — All Met

```
cargo check (clean tree):
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 11s

grep "use ort|use tokenizers|use ndarray" src-tauri/src/:
  zero_hits_confirmed

test -f src-tauri/src/pipeline/ner.rs:
  DELETED

test -f src-tauri/models/bert-base-NER.onnx:
  DELETED

grep "ort =|tokenizers =|ndarray =" src-tauri/Cargo.toml:
  ZERO_HITS

cargo test --lib:
  342 passed; 0 failed; 20 ignored; finished in 5.16s
```

## Release Binary Size Delta

The `bert-base-NER.onnx` model (~104 MB) was gitignored and not committed to the repository. It was present on the development machine in `src-tauri/models/` and removed via filesystem deletion. The three BERT sidecar JSON files (tokenizer.json ~640 KB, config.json ~1 KB, special_tokens_map.json ~125 B) were git-tracked and removed via `git rm`.

Additionally, the `ort`, `tokenizers`, and `ndarray` Cargo crates are removed. Estimated impact on the compiled binary:
- `ort` (ONNX Runtime wrapper) — removes the ONNX Runtime shared library (~50-80 MB depending on features)
- `tokenizers` (HuggingFace Rust tokenizer) — removes ~5-10 MB of compiled code
- Total estimated bundle size reduction: ~115 MB (model file + runtime libraries)

The `tauri.conf.json` `"resources": ["models/*"]` entry has been removed, so the app bundle no longer copies the models directory at build time.

## Empirical Prompt Validation — Deferred

Per executor directive: the empirical checkpoint (Task 3) requires a running Tauri app, live AI provider credentials, and access to `~/private` sample documents. These are not available in the automated execution environment.

**Status:** DEFERRED to Phase 8 UAT (`08-HUMAN-UAT.md`).

The UAT checklist already covers:
- Triggering the backfill and verifying Pass 2 LLM extraction runs
- Per-domain quality checks (identity, vehicle, finance, kids, property)
- LLM-optional path (disconnect provider, verify Pass-1-only fallback)
- TopBar BackfillIndicator showing "Two-pass entity extraction" tooltip variant

The human verifier should use the UAT instructions from the plan's `<how-to-verify>` section.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking Issue] lib.rs auth_state/two_pass ordering**
- **Found during:** Task 1, first `cargo check`
- **Issue:** `spawn_watcher_task` was called at line ~102 passing `two_pass.clone()`, but `two_pass` was only defined at line ~125 (after auth_state and oauth_flow_state). Variable not in scope.
- **Fix:** Moved `auth_state`, `oauth_flow_state`, `auth_arc`, `two_pass` creation and settings loading to BEFORE the `spawn_watcher_task` call. The `app.manage(auth_state)` / `app.manage(oauth_flow_state)` calls were kept at their original position (before AppState) since Tauri requires type registration order.
- **Files modified:** src-tauri/src/lib.rs
- **Commit:** 094c821

**2. [Rule 1 - Bug] commands/documents.rs + commands/folders.rs still referenced ner_service**
- **Found during:** Task 1, first `cargo check`
- **Issue:** Two IPC command handlers (`index_document` in documents.rs and `trigger_scan` in folders.rs) still read `state.ner_service.clone()` and passed it to `index_file`. Neither was mentioned in the plan's files_modified list.
- **Fix:** Replaced `state.ner_service.clone()` with `state.two_pass_extractor.clone()` and updated the `index_file` calls to use `&two_pass` / `&tp`.
- **Files modified:** src-tauri/src/commands/documents.rs, src-tauri/src/commands/folders.rs
- **Commit:** 094c821

**3. [Rule 1 - Bug] test_fixtures_deserialize panicked on deleted ner_golden.json**
- **Found during:** Task 2, `cargo test --lib`
- **Issue:** `graph::entity_store::tests::test_fixtures_deserialize` asserted `ner_golden.json` exists. The file was deleted as part of this plan.
- **Fix:** Removed the `ner_golden.json` section from the test, kept the `aliases.json` section. Added a comment noting the deletion reason.
- **Files modified:** src-tauri/src/graph/entity_store.rs
- **Commit:** e691bc4

**4. [Rule 2 - Missing] bert-base-NER.onnx was gitignored**
- **Found during:** Task 2 git rm
- **Issue:** The plan said `git rm src-tauri/models/bert-base-NER.onnx` but the file was gitignored (`.gitignore` has `src-tauri/models/*.onnx`). `git rm` returned "pathspec did not match any files."
- **Fix:** Used plain `rm` to delete from filesystem. Noted in SUMMARY — the ONNX file was never committed; it was downloaded at dev time and excluded from version control.

**5. [Rule 2 - Missing] special_tokens_map.json not in plan but is a BERT artifact**
- **Found during:** Task 2 `git ls-files src-tauri/models/`
- **Issue:** The plan listed `tokenizer.json` and `config.json` for deletion but not `special_tokens_map.json` (also a BERT tokenizer artifact, 125 B).
- **Fix:** Included `special_tokens_map.json` in `git rm` since it has no purpose after BERT removal.
- **Files modified:** src-tauri/models/special_tokens_map.json (deleted)
- **Commit:** e691bc4

## Known Stubs

None — this plan performs deletion and rewiring only. No frontend stubs introduced.

## Threat Flags

None — no new network endpoints, auth paths, or trust boundaries introduced. Surface reduced (BERT ONNX removed from the bundle means no model-loading attack surface).

## Self-Check: PASSED

**Files verified deleted:**
- `test -f src-tauri/src/pipeline/ner.rs` → DELETED
- `test -f src-tauri/models/bert-base-NER.onnx` → DELETED
- `test -f src-tauri/models/tokenizer.json` → confirmed via git ls-files (removed)
- `test -f src-tauri/models/config.json` → confirmed via git ls-files (removed)
- `test -f src-tauri/tests/fixtures/ner_golden.json` → DELETED

**Commits verified:**
- 094c821 — swap NerService for TwoPassExtractor
- e691bc4 — delete BERT stack (LLME-06)

**Grep zero-hits verified:**
- `grep "use ort|use tokenizers|use ndarray" src-tauri/src/` → zero_hits_confirmed
- `grep "ort =|tokenizers =|ndarray =" src-tauri/Cargo.toml` → ZERO_HITS

**cargo check on clean tree:** Finished with 22 warnings, 0 errors
**cargo test --lib:** 342 passed, 0 failed

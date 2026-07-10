---
phase: "06"
plan: "02"
subsystem: "pipeline/ner"
tags: ["ner", "bert", "onnx", "entity-extraction", "pipeline"]
dependency_graph:
  requires: ["06-01"]
  provides: ["ner-service", "ner-entity-extraction", "entities-version-2"]
  affects: ["pipeline/indexer", "state", "commands/documents", "commands/folders", "watcher/worker"]
tech_stack:
  added:
    - "ort 2.0.0-rc.12 (ONNX Runtime — ort::session::Session API)"
    - "tokenizers 0.20 (HuggingFace tokenizer crate)"
    - "ndarray 0.17 (upgraded from 0.16 to match ort dependency)"
  patterns:
    - "BIO (Begin-Inside-Outside) NER tagging with argmax decode"
    - "Character offset slicing via encoding.get_offsets() (no WordPiece artifacts)"
    - "512-token BERT chunking with 50-token overlap"
    - "Load-once Arc<NerService> pattern (same as EmbeddingService)"
    - "entities_version: 2 metadata for Plan 03 backfill gate"
    - "extract_with_ner closure for testable NER injection"
key_files:
  created:
    - "src-tauri/src/pipeline/ner.rs"
  modified:
    - "src-tauri/src/pipeline/mod.rs"
    - "src-tauri/src/pipeline/entities.rs"
    - "src-tauri/src/pipeline/indexer.rs"
    - "src-tauri/src/types.rs"
    - "src-tauri/src/state.rs"
    - "src-tauri/src/lib.rs"
    - "src-tauri/src/commands/documents.rs"
    - "src-tauri/src/commands/folders.rs"
    - "src-tauri/src/watcher/worker.rs"
    - "src-tauri/src/search/query.rs"
    - "src-tauri/Cargo.toml"
decisions:
  - "Use ort::session::Session API (not ort::Session) — ort 2.0.0-rc.12 restructured module paths"
  - "extract_with_ner accepts impl Fn closure (not &NerService directly) for testability without 109MB model load"
  - "Dedup by (value, entity_type) pair — same string with different types stays distinct per D-02"
  - "id2label hardcoded as ['O','B-MISC','I-MISC','B-PER','I-PER','B-ORG','I-ORG','B-LOC','I-LOC'] for bert-base-NER"
  - "MISC entities dropped at decode_bio level — only person/org/location kept per plan spec"
metrics:
  duration: "~2 hours (across sessions)"
  completed_date: "2026-06-29"
  tasks_completed: 3
  files_changed: 11
---

# Phase 6 Plan 02: NER Service (BERT-base NER Pipeline) Summary

Local BERT-base NER ONNX service loading dslim/bert-base-NER with BIO decoding, character offset slicing, and 512-token chunking; wired into DocumentIndexer and all three index_file call sites.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Create NerService (pipeline/ner.rs) | f6a65d4 | ner.rs (NEW), mod.rs, types.rs, Cargo.toml, search/query.rs |
| 2 | Extend EntityExtractor + types for NER merge | d539ea0 | entities.rs, types.rs |
| 3 | Wire NerService into AppState + all call sites | 85b38f1 | state.rs, lib.rs, indexer.rs, documents.rs, folders.rs, worker.rs |

## Implementation Details

### Task 1: NerService (pipeline/ner.rs)

`NerService` wraps an `ort::session::Session` (loaded once, behind `std::sync::Mutex`) and a `tokenizers::Tokenizer`. The `extract(&self, text)` method:
1. Chunks text into ≤512-token windows with 50-token overlap using sentence-boundary splitting with word-level fallback.
2. Runs each chunk through BERT inference via `ort::inputs!` + `outputs[0].try_extract_array::<f32>()`.
3. Decodes BIO tags (argmax per token → span accumulation) using character offsets from `encoding.get_offsets()` to slice verbatim surface forms from original text (no WordPiece `##` artifacts).
4. Maps B-PER/I-PER → "person", B-ORG/I-ORG → "organization", B-LOC/I-LOC → "location"; drops MISC.
5. Deduplicates by (value, entity_type) pair across chunks.

Model directory: `<resource_dir>/models/` containing `bert-base-NER.onnx`, `tokenizer.json`, `config.json`.

Fast unit tests (`test_decode_bio`, `test_chunk_text`) run without model. Model-dependent test (`test_extract_person`) is `#[ignore]`.

### Task 2: EntityExtractor extensions

- Fixed email entity_type bug: `"person"` → `"email"` for email regex entities.
- Refactored `extract()` to call `extract_regex_entities()` + `sort_dedup_cap()`.
- `sort_dedup_cap()` deduplicates by `(value, entity_type)` pair (not value-only).
- New `extract_with_ner(text, ner_extractor)` method merges regex entities + NER entities then applies 20-entity cap.
- Added `canonical_id: None` to all 4 ExtractedEntity constructors.
- Added 5 new types to types.rs: `CanonicalEntity`, `EntitySummary`, `RelatedEntity`, `DocumentTextPreview`, `EntityBackfillProgress` with serde roundtrip tests.

### Task 3: Wiring

- `AppState` gains `pub ner_service: Arc<NerService>`.
- `lib.rs` initializes NerService from `resource_dir/models/` and passes `ner_service.clone()` to both `spawn_watcher_task` and the `AppState` constructor.
- `spawn_watcher_task` gains `ner_service: Arc<NerService>` parameter; threads it into per-file `spawn_blocking` closure.
- `commands/documents::index_document` and `commands/folders::trigger_scan` both clone `state.ner_service` and pass `&ner` to `index_file`.
- `indexer::index_file` uses `self.extractor.extract_with_ner(&parsed.text, |text| ner_service.extract(text))` and writes `entities_version: 2` metadata on every indexed document.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `ort::Session` not found — wrong module path**
- **Found during:** Task 1 implementation
- **Issue:** Plan sketch used `ort::Session::builder()` but ort 2.0.0-rc.12 restructured to `ort::session::Session::builder()`
- **Fix:** Used `ort::session::Session` throughout ner.rs; imported `ort::session::builder::GraphOptimizationLevel`
- **Files modified:** src-tauri/src/pipeline/ner.rs

**2. [Rule 1 - Bug] `try_extract_tensor` method doesn't exist**
- **Found during:** Task 1 implementation
- **Issue:** Plan sketch used `try_extract_tensor()` but ort 2.0.0-rc.12 API is `try_extract_array::<f32>()`
- **Fix:** Changed to `outputs[0].try_extract_array::<f32>()` with ndarray feature
- **Files modified:** src-tauri/src/pipeline/ner.rs

**3. [Rule 1 - Bug] Two ndarray versions conflict (0.16 vs 0.17)**
- **Found during:** Task 1, cargo build
- **Issue:** ort uses ndarray 0.17 internally; Cargo.toml specified 0.16 → `ArrayBase` type mismatch
- **Fix:** Changed `ndarray = "0.16"` to `ndarray = "0.17"` in Cargo.toml
- **Files modified:** src-tauri/Cargo.toml

**4. [Rule 1 - Bug] `chunk_text` test failed — no sentence fallback to word split**
- **Found during:** Task 1 TDD RED/GREEN
- **Issue:** `split_sentences()` treats a long no-punctuation text as one sentence, never splits it
- **Fix:** Added word-level fallback in `chunk_text()`: when single sentence exceeds max_tokens, call `split_by_words(text, 200)`
- **Files modified:** src-tauri/src/pipeline/ner.rs

**5. [Rule 1 - Bug] search/query.rs missing `canonical_id` field**
- **Found during:** Task 1 TDD, cargo build after adding canonical_id to ExtractedEntity
- **Issue:** ExtractedEntity construction in build_document_from_metadata didn't include canonical_id
- **Fix:** Added `canonical_id` extraction from metadata map in build_document_from_metadata
- **Files modified:** src-tauri/src/search/query.rs

## Test Results

```
test result: ok. 124 passed; 0 failed; 8 ignored; 0 measured; 0 filtered out; finished in 1.01s
```

Build: `cargo build` exits 0 (warnings only — all pre-existing).

## Known Stubs

None — NerService is fully wired. The `#[ignore]` tests require the 109MB ONNX model at runtime; they are correctly excluded from CI.

## Threat Flags

None — NerService is entirely local (no network access); entity extraction operates on already-parsed text and writes only to the existing RuVector metadata store.

## Self-Check: PASSED

- [x] src-tauri/src/pipeline/ner.rs exists
- [x] src-tauri/src/pipeline/entities.rs has extract_with_ner method
- [x] src-tauri/src/types.rs has CanonicalEntity, EntitySummary, RelatedEntity, DocumentTextPreview, EntityBackfillProgress
- [x] src-tauri/src/state.rs has ner_service: Arc<NerService>
- [x] Commits f6a65d4, d539ea0, 85b38f1 exist in git log
- [x] 124 tests pass, 0 fail
- [x] cargo build exits 0

---
phase: 02-document-pipeline-and-file-watching
plan: "01"
subsystem: pipeline
tags: [rust, parsing, hashing, pdf, docx, calamine, sha2, pipeline]
dependency_graph:
  requires: []
  provides: [pipeline/parser.rs, pipeline/hasher.rs]
  affects: [plan-02-02, plan-02-03, plan-02-05]
tech_stack:
  added: [pdf-extract, docx-rust, calamine, sha2, digest, fastembed, notify-debouncer-mini, regex, uuid, tempfile]
  patterns: [extension-dispatch, streaming-hash, chunked-io]
key_files:
  created:
    - src-tauri/src/pipeline/mod.rs
    - src-tauri/src/pipeline/parser.rs
    - src-tauri/src/pipeline/hasher.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/error.rs
    - src-tauri/src/lib.rs
decisions:
  - "docx-rust 0.1 used (0.2 does not exist on crates.io); Body.text() method used instead of manual paragraph traversal"
  - "parse_document dispatches by lowercase extension string — new types can be added without restructuring"
  - "content_hash uses 4KB streaming reads to handle large files without full memory load"
  - "AppError::Embedding variant added in this plan to prevent both plans 01 and 02 modifying error.rs"
  - "Images return AppError::Parse with OCR placeholder message per DPIP-05 spec"
metrics:
  duration: "4 min"
  completed_date: "2026-02-27"
  tasks_completed: 2
  files_created: 3
  files_modified: 3
  tests_added: 11
requirements_satisfied: [DPIP-01, DPIP-02, DPIP-03, DPIP-04, DPIP-05, DPIP-08]
---

# Phase 02 Plan 01: Document Parser and Content Hasher Summary

Extension-dispatched document parser (PDF/DOCX/TXT/MD/XLSX/CSV) and streaming SHA-256 content hasher with 11 unit tests.

## What Was Built

Two focused Rust modules forming the text extraction foundation for the document pipeline:

**pipeline/parser.rs** — `parse_document(path) -> Result<ParsedDocument, AppError>`:
- `pdf` via `pdf-extract` (reads bytes, extracts text via lopdf)
- `docx`/`doc` via `docx-rust` DocxFile API using `body.text()` convenience method
- `txt`/`md`/`csv` via `std::fs::read_to_string`
- `xlsx`/`xls`/`ods` via `calamine` — iterates all sheets and rows, joins non-empty cells
- `png`/`jpg`/`jpeg`/`tiff` returns `AppError::Parse("OCR not available...")` placeholder
- Any other extension returns `AppError::Parse("Unsupported file type: ...")`

**pipeline/hasher.rs** — `content_hash(path) -> Result<String, AppError>`:
- Opens file, streams through SHA-256 in 4KB chunks
- Returns 64-character lowercase hex digest
- Deterministic: same content always produces same hash

**error.rs** — Added `Embedding(String)` variant for Plan 02 embedding service use.

## Task Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1: Cargo deps + AppError::Embedding | 744e86c | Add 9 Phase 2 deps; Embedding variant |
| 2: Pipeline module | 6fb20e8 | parser.rs, hasher.rs, mod.rs, lib.rs registration |

## Test Results

```
running 11 tests
test pipeline::hasher::tests::test_hash_missing_file_returns_io_error ... ok
test pipeline::parser::tests::test_parse_image_returns_ocr_error ... ok
test pipeline::parser::tests::test_parse_unsupported_extension_returns_error ... ok
test pipeline::parser::tests::test_parse_txt_returns_content ... ok
test pipeline::parser::tests::test_parse_md_returns_content ... ok
test pipeline::parser::tests::test_parse_csv_returns_content ... ok
test pipeline::parser::tests::test_title_is_filename ... ok
test pipeline::hasher::tests::test_hash_returns_64_char_hex ... ok
test pipeline::hasher::tests::test_hash_is_deterministic ... ok
test pipeline::hasher::tests::test_identical_content_identical_hash ... ok
test pipeline::hasher::tests::test_different_content_different_hash ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] docx-rust version corrected from 0.2 to 0.1**
- **Found during:** Task 1 verification (`cargo check`)
- **Issue:** Plan specified `docx-rust = "0.2"` but crates.io only has 0.1.x series
- **Fix:** Changed to `docx-rust = "0.1"` — fetched 0.1.11 successfully
- **Files modified:** src-tauri/Cargo.toml
- **Commit:** 744e86c

**2. [Rule 2 - Enhancement] Used Body.text() instead of manual paragraph traversal**
- **Found during:** Task 2 implementation research
- **Detail:** docx-rust Body struct has a built-in `text()` method that walks all paragraphs, SDT content, and joins with `\r\n` — far simpler and more correct than manual traversal
- **No fix needed:** Improvement over plan, not a bug

## Self-Check: PASSED

Files exist:
- src-tauri/src/pipeline/mod.rs: FOUND
- src-tauri/src/pipeline/parser.rs: FOUND
- src-tauri/src/pipeline/hasher.rs: FOUND

Commits exist:
- 744e86c: FOUND
- 6fb20e8: FOUND

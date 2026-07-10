---
phase: 06-knowledge-graph-and-native-integrations
verified: 2026-06-29T12:00:00Z
status: human_needed
score: 6/6
overrides_applied: 0
human_verification:
  - test: "Entities extracted from real documents via NER appear as graph nodes in /entities page"
    expected: "After indexing a document containing person/org/location names, /entities shows entities grouped by type; clicking an entity navigates to EntityDetailPage listing that document"
    why_human: "Requires a live Tauri runtime with a real NER model (109 MB ONNX). The E2E checkpoint in Plan 06-07 was user-verified but the verifier cannot invoke a Tauri runtime programmatically."
  - test: "Entity normalization merges aliases at MERGE_THRESHOLD=0.85 cosine similarity"
    expected: "Two surface forms with >= 0.85 cosine similarity (e.g., 'J. Smith' and 'John Smith') collapse into a single canonical entity with both as aliases"
    why_human: "Requires real embeddings + NER output. Cannot be verified without running the ONNX model and EmbeddingService together."
  - test: "Add Watched Folder opens native OS folder picker with no manual text input"
    expected: "Clicking 'Add Folder' in /watched opens the macOS native file picker (not a text field). Cancelling is silent. Selecting a valid folder adds it."
    why_human: "GUI interaction — requires a running Tauri desktop window. Verified by user in Plan 06-07 E2E checkpoint but cannot be verified by grep."
  - test: "In-app file preview renders PDF, image, plain-text, and markdown without 200-char excerpt"
    expected: "Opening a PDF document shows an iframe with the file content. Opening an image shows the img. Text files show pre-formatted content. Markdown renders with headers/tables."
    why_human: "Requires Tauri asset protocol (convertFileSrc) and the running window. UI rendering cannot be verified statically."
  - test: "Open in default app and Reveal in Finder work from document detail and context menu"
    expected: "From /document/:id, clicking 'Open in default app' launches the OS handler. Right-clicking a document row in /search, /recent, /favorites, /spaces/:id shows context menu with Open / Open in default app / Reveal in Finder."
    why_human: "OS shell integration requires a live Tauri runtime. User verified in Plan 06-07 E2E checkpoint."
---

# Phase 6: Knowledge Graph and Native Integrations — Verification Report

**Phase Goal:** Cortex moves from 'doc auto-organizer' to 'knowledge-graph-backed personal brain' — entities (Property, Person, Organization, Amount, Date) become first-class graph nodes that users can click to see every related document, the native folder picker replaces the manual path text input, and any indexed file can be previewed in-app or opened in the OS default application.

**Verified:** 2026-06-29T12:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Entities extracted from documents appear as graph nodes; clicking an entity surfaces every document mentioning it | VERIFIED (code) / HUMAN for runtime | `EntityStore.doc_index` reverse-index in `entity_store.rs:26-33`; `get_documents_for_entity` IPC in `commands/entities.rs:58-113` joins `doc_index[id]` to full `Document` objects; `EntityDetailPage.tsx` wires `useEntityDocuments(id)` into a DocumentRow list; `/entities/:id` route registered in `App.tsx:52`. User verified E2E in Plan 06-07 checkpoint item 4/5. |
| 2 | Entity normalization merges aliases ("123 Main St" and "Main Street property") so duplicates collapse | VERIFIED (code) / HUMAN for correctness | `find_or_create_canonical()` in `entity_store.rs:148-203` performs: (1) exact alias_index lookup, (2) cosine similarity >= MERGE_THRESHOLD=0.85 across all same-type canonicals, (3) create new if no match. `run_full_alias_merge()` at `entity_store.rs:231-309` runs pairwise O(n²) merge after backfill. |
| 3 | Add Watched Folder opens a native OS folder picker; manual path typing is gone | VERIFIED (code) / HUMAN for UX | `WatchedPage.tsx:20` imports `open` from `@tauri-apps/plugin-dialog`; `handleAddFolder` at lines 74-105 calls `open({ directory: true })` with D-19 `exists()` + `stat().isDirectory` validation; old dialog/text-input dead code removed. `dialog:allow-open` permission in `capabilities/default.json`. Plugin registered in `lib.rs:31`. |
| 4 | Document detail page renders an in-app preview for PDF, image, plain-text, and markdown | VERIFIED (code) / HUMAN for rendering | `FilePreview.tsx` dispatcher routes by `doc.docType` to `PdfPreview` (iframe+convertFileSrc), `ImagePreview` (img+convertFileSrc), `TextPreview` (usePreview IPC), `MarkdownPreview` (usePreview + ReactMarkdown+remarkGfm). `DocumentPage.tsx:170` renders `<FilePreview doc={doc} />` replacing the old excerpt block. `assetProtocol.enable=true` in `tauri.conf.json:24-27`. |
| 5 | Open in Finder / Open with default app works from Document detail and search results | VERIFIED (code) / HUMAN for OS invocation | `DocumentContextMenu.tsx:8` imports `openPath, revealItemInDir` from `@tauri-apps/plugin-opener`; wired to SearchPage, RecentPage, FavoritesPage, SpaceDetailPage (DocumentRow). `DocumentPage.tsx:26` imports and calls `openPath`/`revealItemInDir` on header buttons. `opener:allow-open-path` + `opener:allow-reveal-item-in-dir` in capabilities. |
| 6 | Knowledge graph is queryable via IPC — frontend can request "entities by type", "documents for entity", "related entities" | VERIFIED | Six IPC commands registered in `lib.rs:183-188`: `get_entities_by_type`, `get_entity`, `get_documents_for_entity`, `get_related_entities`, `rename_entity_canonical`, `split_entity_alias`. Five React Query hooks in `useTauri.ts:481-554` map to correct IPC names. All hooks call real `tauriInvoke` with browser-dev fallback to mock data. |

**Score:** 6/6 truths have verified code implementation. 5/6 require human testing for runtime confirmation (overlap with human verification section).

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/pipeline/ner.rs` | NER service (BERT-base ONNX) | VERIFIED | 28.9K — `NerService` struct with `extract()` method, BIO decoding, chunk_text with overlap, character offset slicing |
| `src-tauri/src/pipeline/backfill.rs` | Entity backfill background task | VERIFIED | 15.6K — `spawn_entity_backfill()` emits `entity-backfill-progress` events, throttled, calls `run_full_alias_merge()` after completion |
| `src-tauri/src/graph/entity_store.rs` | In-memory entity graph | VERIFIED | Full EntityStore with 4 index fields, `register_doc_entities`, `find_or_create_canonical` with cosine merge, `run_full_alias_merge`, `related_entities` |
| `src-tauri/src/commands/entities.rs` | 6 entity IPC commands | VERIFIED | All 6 commands present: `get_entities_by_type`, `get_entity`, `get_documents_for_entity`, `get_related_entities`, `rename_entity_canonical`, `split_entity_alias` |
| `client/pages/EntitiesPage.tsx` | /entities route page | VERIFIED | Type-grouped grid, 7-pill filter bar, `useEntities`/`useEntitiesByType` hooks, loading/error/empty states |
| `client/pages/EntityDetailPage.tsx` | /entities/:id route page | VERIFIED | Breadcrumb, EntityDetailHeader (inline rename), AliasChipList (split), Documents 7/5 col grid, RelatedEntityChip, SplitAliasDialog |
| `client/pages/WatchedPage.tsx` | Native folder picker | VERIFIED | Static import from `@tauri-apps/plugin-dialog`, D-19 validation, dead text-input code removed |
| `client/components/preview/FilePreview.tsx` | Preview dispatcher | VERIFIED | Routes pdf→PdfPreview, png/jpg→ImagePreview, md→MarkdownPreview, txt/csv→TextPreview, other→UnsupportedPreview |
| `client/components/documents/DocumentContextMenu.tsx` | Right-click OS actions | VERIFIED | `openPath` + `revealItemInDir` from `@tauri-apps/plugin-opener`, isTauri() guard, revealLabel() OS detection |
| `client/components/layout/BackfillIndicator.tsx` | Backfill progress chip | VERIFIED | 4-state (idle/running/complete/error), Brain icon with animate-pulse, 3s complete auto-dismiss, error button with click-to-dismiss |
| `client/hooks/useBackfillProgress.ts` | Tauri event listener | VERIFIED | Listens for `entity-backfill-progress`, routes to `useBackfillStore.getState().setProgress()`, isTauri() guard, unlisten cleanup |
| `src-tauri/models/bert-base-NER.onnx` | Bundled NER model | VERIFIED | 103.9 MB present on disk; SHA-256 user-verified against HuggingFace LFS pointer in Plan 06-01 checkpoint |
| `scripts/download-ner-model.sh` | Model download script | VERIFIED | Present and executable |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `NerService` | `DocumentIndexer.index_file` | `extract_with_ner()` closure | WIRED | `indexer.rs:166` — `self.extractor.extract_with_ner(&parsed.text, |text| ner_service.extract(text))` |
| `DocumentIndexer.index_file` | `EntityStore` | `register_doc_entities()` | WIRED | `indexer.rs:176-193` — Step 8b: `store.register_doc_entities(&doc_id, &mut entities, embedder)` before metadata write |
| `EntityStore` | `entities_version=2` in metadata | Written to collection.db | WIRED | `indexer.rs:229` — `metadata.insert("entities_version", ...Number::from(2u32))` |
| `spawn_entity_backfill` | `lib.rs startup` | Called before `app.manage()` | WIRED | `lib.rs:107-117` — backfill spawned AFTER EntityStore rebuild, BEFORE watcher spawn |
| `backfill.rs` | `entity-backfill-progress` Tauri event | `app_handle.emit()` | WIRED | `backfill.rs:39` and `backfill.rs:96` — events emitted on progress + completion |
| `useBackfillProgress` | `AppShell` | `useBackfillProgress()` call | WIRED | `AppShell.tsx:27` — single mount point |
| `BackfillIndicator` | `TopBar` | `<BackfillIndicator />` slot | WIRED | `TopBar.tsx:60` — rendered between indexing chip and theme toggle |
| `useEntities`/`useEntity`/etc | `get_entities_by_type`/`get_entity`/etc | `tauriInvoke()` | WIRED | `useTauri.ts:485,498,515,534,548` — all 5 read hooks call correct IPC command names |
| `useRenameEntityCanonical` | `rename_entity_canonical` IPC | `tauriInvoke` + `invalidateQueries` | WIRED | `useTauri.ts:439-458` — mutationFn + invalidates `queryKeys.entity(id)` + `queryKeys.entities` |
| `WatchedPage.handleAddFolder` | `open()` (native dialog) | `@tauri-apps/plugin-dialog` | WIRED | `WatchedPage.tsx:77` — `await open({ directory: true, multiple: false })` |
| `DocumentContextMenu` | `openPath`/`revealItemInDir` | `@tauri-apps/plugin-opener` | WIRED | `DocumentContextMenu.tsx:8,49,58` — imports and calls with isTauri() guard |
| `FilePreview` | `DocumentPage` | `<FilePreview doc={doc} />` | WIRED | `DocumentPage.tsx:170` — replaces old excerpt block |
| `PdfPreview`/`ImagePreview` | Asset protocol | `convertFileSrc(doc.path)` | WIRED | `PdfPreview.tsx:44`, `ImagePreview.tsx` — converts path to `asset://` URL |
| `TextPreview`/`MarkdownPreview` | `read_document_text` IPC | `usePreview(doc.id)` | WIRED | `TextPreview.tsx:48`, `MarkdownPreview.tsx` — `usePreview` calls `tauriInvoke("read_document_text", ...)` |
| 6 entity IPC commands | `lib.rs invoke_handler` | `tauri::generate_handler![]` | WIRED | `lib.rs:183-188` — all 6 entity commands registered |
| `/entities` and `/entities/:id` routes | `App.tsx` | `<Route path=...>` | WIRED | `App.tsx:51-52` — both routes registered inside AppShell group before catch-all |

---

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|-------------------|--------|
| `EntitiesPage.tsx` | `allEntities` from `useEntities()` | `get_entities_by_type` IPC → `EntityStore.get_by_type()` → real `canonicals` HashMap | Yes — EntityStore populated from `rebuild_from_engine` on startup + `register_doc_entities` on each index | FLOWING |
| `EntityDetailPage.tsx` | `entity` from `useEntity(id)` | `get_entity` IPC → `EntityStore.get_canonical(id)` | Yes — canonical records from real NER extraction pipeline | FLOWING |
| `EntityDetailPage.tsx` | `documents` from `useEntityDocuments(id)` | `get_documents_for_entity` IPC → `doc_index[id]` → collection.db lookup | Yes — reverse index populated by `register_doc_entities` on every indexed document | FLOWING |
| `TextPreview`/`MarkdownPreview` | `data` from `usePreview(doc.id)` | `read_document_text` IPC → resolves path from metadata → reads file | Yes — 5 MB capped file read from real filesystem path | FLOWING |
| `PdfPreview`/`ImagePreview` | `assetUrl` from `convertFileSrc(doc.path)` | Tauri asset protocol serving local file | Yes — asset URL points to actual file on disk | FLOWING |

---

## Behavioral Spot-Checks

Step 7b is SKIPPED for the Tauri runtime components — all preview, native picker, and OS-open behaviors require a live Tauri window. Rust compilation and test suites were verified in plan SUMMARYs.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Entity IPC commands registered in lib.rs | `rg "get_entities_by_type|get_entity|get_documents_for_entity|get_related_entities|rename_entity_canonical|split_entity_alias" src-tauri/src/lib.rs` | 6 matches at lines 183-188 | PASS |
| Tauri plugins registered before .setup() | Read `lib.rs:31-33` | `tauri_plugin_dialog::init()`, `tauri_plugin_opener::init()`, `tauri_plugin_fs::init()` at lines 31-33 BEFORE `.setup()` at line 34 | PASS |
| NER pipeline wired into indexer | `rg "extract_with_ner" src-tauri/src/pipeline/indexer.rs` | Line 166 — `extract_with_ner()` called in `index_file` | PASS |
| register_doc_entities called before metadata write | Read `indexer.rs:164-229` | Step 8b (line 179) calls `register_doc_entities`; Step 9+ builds metadata at line 229 writing `entities_version=2` | PASS |
| FilePreview wired into DocumentPage | `rg "FilePreview" client/pages/DocumentPage.tsx` | Import at line 29, rendered at line 170 | PASS |
| DocumentContextMenu wired into 4 pages | `rg "DocumentContextMenu" client/pages/{Search,Recent,Favorites}Page.tsx` | 3 matches confirming import + usage in each page; SpaceDetailPage uses shared DocumentRow | PASS |
| Routes registered in App.tsx | `rg "EntitiesPage|EntityDetailPage" client/App.tsx` | `/entities` at line 51, `/entities/:id` at line 52 | PASS |
| BackfillIndicator in TopBar | `rg "BackfillIndicator" client/components/layout/TopBar.tsx` | Import at line 10, rendered at line 60 | PASS |
| useBackfillProgress in AppShell | `rg "useBackfillProgress" client/components/layout/AppShell.tsx` | Import at line 13, called at line 27 | PASS |
| entity-backfill-progress event emitted | `rg "entity-backfill-progress" src-tauri/src/pipeline/backfill.rs` | Lines 40 (progress) and 96 (complete) | PASS |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| KG-01 | Plans 06-03, 06-06, 06-07 | Entities as graph nodes; click-through to documents | SATISFIED | `EntityStore.doc_index`, `get_documents_for_entity` IPC, `EntityDetailPage` Documents section wired via `useEntityDocuments` |
| KG-02 | Plan 06-03 | Alias normalization via embedding similarity | SATISFIED (code verified) | `find_or_create_canonical()` cosine merge at MERGE_THRESHOLD=0.85 in `entity_store.rs:148-202`; `run_full_alias_merge()` at line 237 |
| KG-03 | Plans 06-03, 06-06 | Knowledge graph queryable via IPC | SATISFIED | 6 IPC commands in `commands/entities.rs`, registered in `lib.rs:183-188`, 5 React Query hooks in `useTauri.ts:481-554` |
| KG-04 | Plan 06-07 | Rename + Split alias on /entities/:id | SATISFIED | `EntityDetailHeader` inline rename, `AliasChip` split, `SplitAliasDialog`, `useRenameEntityCanonical` + `useSplitEntityAlias` mutation hooks |
| KG-05 | Plans 06-02, 06-03, 06-07 | NER backfill with progress events | SATISFIED | `NerService` in `ner.rs`, `spawn_entity_backfill` in `backfill.rs`, `BackfillIndicator` + `useBackfillProgress` + `useBackfillStore` |
| UX-05 | Plan 06-04 | Native folder picker (no manual path typing) | SATISFIED | `WatchedPage.tsx` uses `open({ directory: true })` from `@tauri-apps/plugin-dialog`; old text-input dialog removed |
| UX-06 | Plans 06-04, 06-05 | Open in default app / Reveal in Finder | SATISFIED | `DocumentContextMenu` (4 pages) + `DocumentPage` header buttons using `openPath`/`revealItemInDir` from `@tauri-apps/plugin-opener` |
| PAGE-13 | Plan 06-05 | In-app preview for PDF/image/text/markdown | SATISFIED | `FilePreview` dispatcher + 5 renderer components + `usePreview` hook + `DocumentPage` wired to `<FilePreview doc={doc} />` |

**Note:** REQUIREMENTS.md traceability rows still show "Not started" for Phase 6 IDs (KG-01..05, UX-05, UX-06, PAGE-13). Per Plan 06-07 SUMMARY, the orchestrator was expected to flip these to "Complete" on phase close-out. This is a documentation artifact, not an implementation gap.

---

## Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `client/hooks/useTauri.ts:518` | `mockEntities[0]` used as fallback for `useEntity` — returns first mock entity regardless of requested id | WARNING | Browser-dev only (isTauri()=false path). In Tauri runtime the real IPC runs. No production impact. |
| `REQUIREMENTS.md:204-211` | Traceability rows show "Not started" for all Phase 6 requirements despite implementation being complete | INFO | Documentation state only — orchestrator expected to update on close-out. No code impact. |

No TBD, FIXME, or XXX markers found in any Phase 6 implementation files.
No placeholder/stub return patterns found in critical code paths.

---

## Human Verification Required

### 1. Entity extraction from real documents — NER pipeline end-to-end

**Test:** Index a document containing person names, organization names, and locations. Navigate to /entities. Verify entities appear grouped by type. Click an entity and verify the detail page shows that document in the "Documents mentioning this" section.

**Expected:** /entities shows person/organization/location groups populated with real entity names from the document. EntityDetailPage shows the document in the documents list.

**Why human:** Requires live Tauri runtime with the 109 MB bert-base-NER.onnx model loaded. The E2E checkpoint in Plan 06-07 covered this (item 4) but the verifier cannot invoke Tauri programmatically.

### 2. Alias normalization — merge validation

**Test:** Index two documents where one mentions "John Smith" and another mentions "J. Smith" (or similar near-alias pair). After backfill, navigate to /entities and find the person entity. Verify both surface forms appear as aliases under a single canonical.

**Expected:** One canonical entity with both surface forms as aliases; document count reflects both documents.

**Why human:** Requires real embeddings from EmbeddingService + real NER output + MERGE_THRESHOLD=0.85 tuning to produce a merge. Cannot simulate the cosine arithmetic without running the models.

### 3. Native folder picker — WatchedPage UX

**Test:** Navigate to /watched. Click "Add Folder". Verify the macOS native folder picker dialog opens (not a text input field). Press Cancel — verify nothing changes. Open again, select a valid folder — verify it appears in the watched folders list.

**Expected:** Native macOS folder picker dialog (NSOpenPanel). Cancel is silent. Valid selection adds folder.

**Why human:** GUI behavior requires a live Tauri desktop window. The Plan 06-07 E2E checkpoint (item 3) verified this.

### 4. In-app document preview

**Test:** Navigate to a PDF document's detail page (/document/:id). Verify the preview pane shows the PDF rendered in an iframe (not a 200-char text excerpt). Repeat for an image file, a .txt file, and a .md file.

**Expected:** PDF renders in iframe via asset:// protocol. Image renders as img tag. Text renders in pre block. Markdown renders with HTML formatting (headers, lists, tables).

**Why human:** Rendering requires Tauri asset protocol and a browser-capable webview. The Plan 06-07 E2E checkpoint (item 8/9) verified this.

### 5. Open in OS and right-click context menu

**Test:** On a document detail page, click "Open in default app" — verify the OS default application opens the file. Click "Reveal in Finder" — verify Finder opens at the file location. Right-click a document row in /search — verify the context menu appears with all three items.

**Expected:** OS file handler launched; Finder opens; context menu shows Open / Open in default app / Reveal in Finder.

**Why human:** OS shell integration requires the Tauri runtime. The Plan 06-07 E2E checkpoint (items 10/11) verified this.

---

## Gaps Summary

No gaps found. All 6 success criteria have verified code implementations:

1. Entity graph nodes with click-through document surfaces — EntityStore + 6 IPC commands + EntitiesPage + EntityDetailPage fully wired.
2. Alias normalization — cosine similarity merge at MERGE_THRESHOLD=0.85 implemented in `find_or_create_canonical()` + `run_full_alias_merge()`.
3. Native folder picker — `@tauri-apps/plugin-dialog` fully replacing manual path input in WatchedPage.
4. In-app preview — FilePreview dispatcher + 5 renderer components + DocumentPage wired.
5. Open in OS — `@tauri-apps/plugin-opener` wired into DocumentContextMenu (4 pages) and DocumentPage header.
6. IPC queryability — 6 commands registered, 5 React Query read hooks + 2 mutation hooks wired.

The 5 human verification items are runtime confirmation requirements for behaviors that are fully implemented in code — they cannot be verified without a live Tauri window executing the ONNX model, asset protocol, and OS file handlers.

---

_Verified: 2026-06-29T12:00:00Z_
_Verifier: Claude (gsd-verifier)_

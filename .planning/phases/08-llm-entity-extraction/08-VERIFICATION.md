---
phase: 08-llm-entity-extraction
verified: 2026-07-03T14:00:00Z
status: human_needed
score: 2/5
overrides_applied: 0
gaps:
  - truth: "LLM-extracted free-form tags (TagChip) appear in /document/:id"
    status: partial
    reason: "backfill.rs writes LLM tags to metadata key 'tags' (line 358); query.rs reads from 'llmTags' key (line 221). Key mismatch means Document.llm_tags is always empty and TagChip never renders."
    artifacts:
      - path: "src-tauri/src/pipeline/backfill.rs"
        issue: "metadata.insert('tags'.to_string(), ...) at line 358 should be 'llmTags'"
      - path: "src-tauri/src/search/query.rs"
        issue: "reads .get('llmTags') at line 221 but nothing is ever written to that key"
    missing:
      - "Change backfill.rs line 358 key from 'tags' to 'llmTags' — or align query.rs to read from 'tags' and rename the Document field accordingly"
human_verification:
  - test: "Open /document/:id for a tax PDF (e.g. ITR-V or a bank statement) while an AI provider is connected and entities_version=3.0"
    expected: "Sidebar shows EntityChips with class=Person, class=Organization, class=Amount, class=Date — from the active LLM, not bert-NER (entities produced after Pass 2 backfill completes)"
    why_human: "Requires live LLM call, real indexed document, and visual inspection of entity classes in the sidebar"
  - test: "Index a recipe document and a medical note while an AI provider is active"
    expected: "Recipe document shows entities of type Person (chef/author) and ingredient/location; medical note shows Person (patient/doctor), Organization (hospital), Date (appointment). No config change between the two."
    why_human: "Requires indexing diverse real documents and inspecting extracted entity types — can't verify LLM quality programmatically"
  - test: "Trigger backfill from Settings → AI & Models → Entity Extraction → 'Re-extract entities' button"
    expected: "TopBar shows a chip: 'Extracting entities X/Y' with an ETA tooltip when an AI provider is connected. App remains navigable. After completion, a sonner toast appears if any docs fell back to Pass 1 only."
    why_human: "Requires running app with indexed documents and live AI provider; needs visual verification of TopBar chip and toast"
---

# Phase 8: LLM Entity Extraction — Verification Report

**Phase Goal:** Every document's entities are extracted by the active AI provider, not bert-base-NER — the quality bar is mockEntities diversity (person, org, location, date, amount, email, topic) across any document domain.
**Verified:** 2026-07-03T14:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Opening /document/:id shows LLM-extracted entities (amounts, dates, organizations) | ? UNCERTAIN | Code path exists and is fully wired: TwoPassExtractor → Pass2LlmRefiner → ai_request() → extractedEntities in Document. Quality needs live LLM test. |
| 2 | Entity extraction works across unlike doc types without config change | ? UNCERTAIN | 8-class locked schema in REFINE_PROMPT with multi-region few-shot examples (India/US/EU/Vehicle). Pass 1 handles all doc types; Pass 2 calls same prompt. Live quality needs human test. |
| 3 | Re-extracting same document returns same entity set (idempotent); single failure doesn't block rest | ✓ VERIFIED | Pass 1 sorts+dedupes+caps at 20 (pass1_pattern_extractor.rs line 169-173). Pass 2 uses temperature=0.0 (pass2_llm_refiner.rs line 468). Failure isolation: extract_full catches Pass 2 errors and falls back to Pass 1 only (two_pass_extractor.rs lines 103-118). Backfill loop continues on per-doc error (backfill.rs lines 138-157). |
| 4 | User triggers backfill from Settings → AI; TopBar shows progress and remains usable | ? UNCERTAIN | All wiring present: ExtractionSettings.tsx renders trigger button → useTriggerEntityBackfill → trigger_entity_backfill IPC → spawn_entity_backfill. BackfillIndicator is in TopBar. useBackfillProgress hook listens to entity-backfill-progress events. Live E2E test needed. |
| 5 | bert-base-NER.onnx, tokenizer files, ort, and tokenizers crates are absent; cargo check passes | ✓ VERIFIED | ner.rs DELETED. models/ contains only .gitkeep. Zero hits for ort=/tokenizers=/ndarray= in Cargo.toml. Zero hits for `use ort\|use tokenizers\|use ndarray` in src/. cargo check: Finished dev profile, 0 errors, 22 warnings. 342 tests pass (cargo test --lib). |

**Score:** 2/5 truths mechanically verified; 3/5 require human/live-LLM verification

### Data-Flow Gap: LLM Tags (TagChip Display)

The TagChip component in DocumentPage (`doc.llmTags`) will always render empty due to a metadata key mismatch:

- `backfill.rs` line 358 writes LLM-extracted tags to metadata key `"tags"`
- `query.rs` line 221 reads from metadata key `"llmTags"` (never written)
- Result: `Document.llm_tags` is always empty; `TagChip` never displays in /document/:id

The TopicChip is unaffected (both backfill and query.rs use `"topic"` key correctly). Entity chips (Person/Org/Location/Date/Amount) are unaffected (use `extracted_entities` key). Only the free-form LLM tag display is broken.

**This does not block any of the 5 Success Criteria** (which test entity types, not free-form tags), but is a concrete data-flow defect in Phase 8's secondary display feature.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/pipeline/pass1_pattern_extractor.rs` | Deterministic regex + checksum extractor | ✓ VERIFIED | 41.2K, 48 unit tests, covers Date/Email/Phone/Amount/Identifier (Aadhaar/IBAN/PAN/SSN/NINO/VIN/GSTIN/credit-card/SIN/TFN), sort+dedup+cap at 20 |
| `src-tauri/src/pipeline/pass2_llm_refiner.rs` | LLM refiner with REFINE_PROMPT | ✓ VERIFIED | 38.7K, 30 unit tests, REFINE_PROMPT inline with 4 multi-region few-shot examples, semaphore cap=8 (D-13), fence-strip JSON parsing (D-14), temperature=0.0 (D-12) |
| `src-tauri/src/pipeline/two_pass_extractor.rs` | Facade composing Pass 1 + Pass 2 with D-20 merge policy | ✓ VERIFIED | 24.4K, 10 unit tests, extract() (Pass 1 sync) + extract_full() (async), merge_passes() with pass1_id refinement, 20-entity cap after merge, failure fallback to PASS1_ONLY_VERSION |
| `src-tauri/src/pipeline/backfill.rs` | Async backfill rewire with EtaCalculator | ✓ VERIFIED | spawn_entity_backfill uses TwoPassExtractor; float version gate (.as_f64()); EtaCalculator ring buffer (last 20 docs); per-doc error isolation; throttled progress events (500ms / 25 docs) |
| `src-tauri/src/commands/entities.rs` | IPC commands for extraction settings + backfill | ✓ VERIFIED | get_extraction_settings, set_extraction_settings, trigger_entity_backfill — all present. trigger_entity_backfill calls spawn_entity_backfill. |
| `client/components/ai/ExtractionSettings.tsx` | Settings UI with extraction controls | ✓ VERIFIED | 11.5K, renders extraction model dropdown, "Use LLM for entity extraction" toggle, "Re-extract entities" button with cost tooltip. Mounted in SettingsPage at line 325. |
| `client/components/layout/BackfillIndicator.tsx` | TopBar progress chip | ✓ VERIFIED | Mounted in TopBar.tsx. Reads useBackfillStore. Shows "Extracting entities X/Y" with Two-pass tooltip when etaSeconds present. D-29 completion toast when fallbacks > 0. |
| `client/hooks/useBackfillProgress.ts` | Tauri event → store bridge | ✓ VERIFIED | Listens to "entity-backfill-progress", routes to useBackfillStore.setProgress(). Called once in AppShell.tsx line 40. |
| `client/components/entities/TopicChip.tsx` | Accent-tinted topic display pill | ✓ VERIFIED | snake_case→Sentence case transform; returns null for empty/"other". Used in DocumentPage. |
| `client/components/entities/TagChip.tsx` | Neutral tag display pill | ✓ WIRED (data disconnected) | Component exists and is referenced in DocumentPage (doc.llmTags). However data never flows due to llmTags key mismatch — see gap above. |
| `client/components/search/TopicFilterBar.tsx` | Topic filter chip row | ✓ VERIFIED | 20-chip pagination, "Show more", mounted on /search and /tags pages. get_topics IPC aggregates from vector metadata. |
| `src-tauri/src/pipeline/ner.rs` | Must be DELETED (BERT NER replacement) | ✓ VERIFIED | File does not exist — DELETED |
| `src-tauri/models/bert-base-NER.onnx` | Must be absent | ✓ VERIFIED | gitignored and deleted from filesystem; models/ dir contains only .gitkeep |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `indexer.rs` | `TwoPassExtractor::extract()` | `two_pass.extract(&parsed.text)` at line 170 | ✓ WIRED | Pass 1 sync path used in hot indexing path |
| `backfill.rs` | `TwoPassExtractor::extract_full()` | `two_pass.extract_full(&text, &title).await` at line 308 | ✓ WIRED | Async full two-pass in backfill worker |
| `Pass2LlmRefiner` | `ai_request_with_retry` | `use crate::ai::retry::ai_request_with_retry` | ✓ WIRED | Pass 2 calls Phase 7 AI router |
| `ExtractionSettings.tsx` | `trigger_entity_backfill` IPC | `useTriggerEntityBackfill()` → `invoke("trigger_entity_backfill")` | ✓ WIRED | "Re-extract entities" button fires backfill |
| `BackfillIndicator.tsx` | `useBackfillStore` | `useBackfillStore()` reads status/processed/total/etaSeconds | ✓ WIRED | TopBar chip reads from Zustand store |
| `useBackfillProgress.ts` | `useBackfillStore.setProgress` | `listen("entity-backfill-progress", event => setProgress(event.payload))` | ✓ WIRED | Tauri event routes to store |
| `backfill.rs` | LLM tags in metadata | `metadata.insert("tags".to_string(), ...)` line 358 | ✗ DISCONNECTED | Writes to "tags" but query.rs reads from "llmTags" — key mismatch |
| `query.rs` | `Document.llm_tags` | `.get("llmTags")` at line 221 | ✗ DISCONNECTED | Reads "llmTags" but nothing writes to that key |
| `backfill.rs` | topic in metadata | `metadata.insert("topic".to_string(), ...)` line 354 | ✓ WIRED | Matches query.rs `.get("topic")` |
| `DocumentPage.tsx` | `TopicChip` | `<TopicChip topic={doc.topic} />` | ✓ WIRED | Topic flows correctly end-to-end |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `DocumentPage.tsx` — EntityChip | `doc.extractedEntities` | `build_document_from_metadata` → `"extracted_entities"` metadata key | Yes — written by backfill.rs line 345 from TwoPassExtractor output | ✓ FLOWING |
| `DocumentPage.tsx` — TopicChip | `doc.topic` | `build_document_from_metadata` → `"topic"` metadata key | Yes — written by backfill.rs line 354 from Pass2Output.topic | ✓ FLOWING |
| `DocumentPage.tsx` — TagChip | `doc.llmTags` | `build_document_from_metadata` → `"llmTags"` metadata key | No — backfill writes to `"tags"` not `"llmTags"` | ✗ DISCONNECTED |
| `BackfillIndicator.tsx` | `processed / total / etaSeconds` | `useBackfillStore` ← `useBackfillProgress` ← `entity-backfill-progress` Tauri event | Yes — backfill.rs emits progress events | ✓ FLOWING |
| `TopicFilterBar.tsx` | `data` (TopicCount[]) | `useTopics()` → `get_topics` IPC → `aggregate_topics` (scans VectorEntry metadata) | Yes — real metadata scan; unit-tested | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Pass1PatternExtractor test suite | `cargo test --lib pipeline::pass1_pattern_extractor` (via `cargo test --lib`) | 342 passed, 0 failed | ✓ PASS |
| Pass2LlmRefiner JSON parsing + fence-strip | `cargo test --lib pipeline::pass2_llm_refiner` | 342 passed (includes 30 refiner tests) | ✓ PASS |
| TwoPassExtractor merge + fallback | `cargo test --lib pipeline::two_pass_extractor` | 342 passed (includes 10 extractor tests) | ✓ PASS |
| cargo check (BERT-free) | `cd src-tauri && cargo check` | Finished dev profile, 0 errors, 22 warnings | ✓ PASS |
| No ort/tokenizers/ndarray in Cargo.toml | `rg "ort =|tokenizers =|ndarray =" src-tauri/Cargo.toml` | 0 matches | ✓ PASS |
| No ort/tokenizers import in Rust src | `rg "use ort|use tokenizers|use ndarray" src-tauri/src/` | 0 matches | ✓ PASS |

### Requirements Coverage

| Requirement | Description | Status | Evidence |
|-------------|-------------|--------|----------|
| LLME-01 | User-visible entities extracted by active AI provider, not bert-NER | ? UNCERTAIN | Code path: TwoPassExtractor.extract_full() → Pass2LlmRefiner → ai_request(). Live quality: human_needed. |
| LLME-02 | Entity types cover person, organization, location, date, amount, email, topic | ? UNCERTAIN | Pass 1: Date/Email/Phone/Amount/Identifier. Pass 2: Person/Org/Location + topic (snake_case). 8-class schema enforced. Topic chain works (TopicChip). Tags broken (key mismatch). |
| LLME-03 | Per-doc entity cap of 20; extraction is idempotent | ✓ SATISFIED | Cap enforced at three points (pass1:20, merge:20). Idempotence: Pass1 sort+dedup, Pass2 temp=0.0. 10 unit tests cover merge and fallback. |
| LLME-04 | Single doc extraction failure does not block rest of index | ✓ SATISFIED | extract_full catches Pass 2 errors and falls back to PASS1_ONLY (two_pass_extractor.rs lines 103-118). Backfill loop continues on error (backfill.rs lines 138-157). |
| LLME-05 | Backfill triggered from Settings; TopBar BackfillIndicator shows progress + ETA | ? UNCERTAIN | All wiring verified in code. Live E2E behavior: human_needed. |
| LLME-06 | bert-base-NER + ort + tokenizers removed; cargo check clean | ✓ SATISFIED | ner.rs deleted, models/ cleared, deps removed, cargo check: 0 errors. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `client/pages/TagsPage.tsx` | 27 | `TODO(Phase 11): replace visual marker with get_tags_by_topic IPC` | ℹ️ Info | Deliberate deferral to Phase 11, documented in Plan 09 decisions. TagsPage shows visual filter marker; full backend filtering deferred. |
| `src-tauri/src/search/query.rs` | 221 | `.get("llmTags")` vs backfill writing `"tags"` | ⚠️ Warning | Data-flow disconnect — LLM free-form tags never appear in DocumentPage TagChip. Does not block SC1-SC5 but breaks secondary UI feature. |
| `src-tauri/src/commands/entities.rs` | 285-289 | Stale comment referencing Plan 06 stub stub that has since been replaced | ℹ️ Info | Code is correct (backfill is called at line 310); comment is outdated documentation only |

No TBD/FIXME/XXX markers found in any Phase 8 files. No ort/tokenizers/ndarray imports.

### Human Verification Required

### 1. LLM Entity Quality on Real Document (SC1)

**Test:** Open Cortex with an AI provider connected and full index built. Navigate to /document/:id for a tax PDF (e.g., an ITR-V, bank statement, or property document). After triggering backfill and waiting for entities_version=3.0, inspect the Extracted Entities sidebar section.

**Expected:** EntityChips show class=Organization (bank name, company), class=Amount (monetary values), class=Date (tax year/dates), class=Identifier (PAN, account numbers). These must come from the LLM (class field present and populated), not from legacy bert-NER output.

**Why human:** Requires live AI provider credentials, real indexed documents, and visual inspection of entity class badges in the document detail sidebar.

### 2. Universal Coverage Across Document Types (SC2)

**Test:** Index a recipe document and a medical note (or letter) with an active AI provider. After backfill completes, open each document's detail page.

**Expected:** Recipe: person entity (chef/author name if present), possibly location. Medical note: Person (patient name, doctor name), Organization (hospital/clinic), Date (appointment/diagnosis date). No config change between documents.

**Why human:** Requires diverse real documents, live LLM calls, and inspection of per-document entity class coverage.

### 3. Backfill Trigger + TopBar Indicator (SC4)

**Test:** With 50+ indexed documents and an active AI provider, open Settings → AI & Models. Click "Re-extract entities" button. Observe the TopBar while backfill is running.

**Expected:** TopBar shows a chip "Extracting entities X/Y" with a Brain icon and pulsing animation. Tooltip says "Two-pass entity extraction — X of Y docs — Pass 1 complete, Pass 2 in progress (ETA Ys)". App remains navigable (sidebar, search, other pages) while chip is shown. After completion, "Done extracting entities" appears briefly, then a toast if any docs fell back to Pass 1 only.

**Why human:** Requires running app with indexed documents and live AI provider; visual verification of chip appearance, ETA display, and app navigability.

### Gaps Summary

**Primary gap (data-flow bug — TagChip display):** LLM-extracted free-form tags from Pass 2 are stored in vector metadata under key `"tags"` (backfill.rs line 358) but `build_document_from_metadata` in query.rs reads from key `"llmTags"` (line 221). As a result, `Document.llm_tags` is always empty and the TagChip row in DocumentPage never renders LLM tags.

Fix: Change `backfill.rs` line 358 from `"tags".to_string()` to `"llmTags".to_string()`. Verify the change doesn't collide with the space-level `tags` field (which the indexer does not write to vector metadata — confirmed).

**This gap does not block SC1-SC5** because the 5 success criteria test entity-class display (EntityChips from extractedEntities) and topic display (TopicChip), both of which have correct end-to-end data flow. The TagChip display is a secondary feature of Phase 8.

---

_Verified: 2026-07-03T14:00:00Z_
_Verifier: Claude (gsd-verifier)_

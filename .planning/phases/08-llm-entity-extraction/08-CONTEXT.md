# Phase 8: LLM Entity Extraction - Context

**Gathered:** 2026-07-03
**Status:** Ready for planning
**Design:** Two-pass engine + fixed 8-class taxonomy + free-form topic + free-form tags. LLM-optional.

<domain>
## Phase Boundary

Phase 8 replaces the local BERT-based NER (`pipeline/ner.rs`) with a **two-pass entity extraction engine**:

- **Pass 1** — deterministic pattern extractor (regex + validators). Runs on every doc regardless of LLM availability.
- **Pass 2** — LLM refinement via Phase 7 `ai_request()`. Adds Person/Organization/Location, assigns topic, emits free-form tags. Skipped when no provider is connected.

Cortex remains functional (reduced quality) with zero connected providers — Pass 1 alone still extracts dates, emails, amounts, phone numbers, and validated IDs. The `topic` and `tags` fields provide **emergent semantic labels without a heavy adaptive taxonomy engine** — free-form values are normalized (lowercase + snake_case) at write time to prevent search fragmentation.

### What Phase 8 delivers

1. **`Pass1PatternExtractor`** — deterministic Rust module `pipeline/pass1_pattern_extractor.rs`. Extracts entities via regex + strong validators:
   - `Date` — RFC-3339, ISO-8601, common human formats
   - `Email` — RFC-5322 subset
   - `Phone` — E.164 + national formats (US, UK, India, EU)
   - `Amount` — currency symbol/code + digits, currency inferred
   - `URL` — standard URL regex
   - `Identifier` w/ subclass — Aadhaar (Verhoeff), PAN (format), IBAN (Mod-97), VIN (checksum), GSTIN (Luhn-variant), credit card (Luhn + BIN prefix), SSN, NINO, SIN, etc.
   - Zero LLM cost, deterministic, idempotent.

2. **`Pass2LlmRefiner`** — LLM-driven Rust module `pipeline/pass2_llm_refiner.rs`. Called only when an active provider exists AND the "Use LLM for entity extraction" toggle is on. Input: doc chunk + Pass-1 findings. Output:
   - `additional_entities`: Person / Organization / Location (regex can't find these)
   - `refined_entities`: Pass-1 candidates with narrower subclass ("12-digit number" → `Identifier{subclass=aadhaar}`)
   - `topic`: single-value doc-level tag (from 19-topic seed OR free-form)
   - `tags`: 2-5 free-form hashtag-style tags per doc (`alphacomplex`, `term_insurance`, `khush_school`)
   - `confidence` per entity (0.0-1.0) for OCR tolerance

3. **Fixed 8-class taxonomy (locked schema).**
   Classes: `Person`, `Organization`, `Location`, `Date`, `Amount`, `Email`, `Phone`, `Identifier` (+ free-form `subclass` string, no whitelist).
   Free-form fields (emergent): `topic` (single), `tags` (multi). Both normalized to snake_case at write.
   **No taxonomy store, no consolidation loop, no Settings taxonomy panel** in v1.1 — deferred to v1.2.

4. **Backfill re-wiring** — reuse existing `pipeline/backfill.rs::spawn_entity_backfill` infra; swap `NerService` for the two-pass engine. `entities_version` semantics: `2` = BERT (legacy), `2.5` = Pass 1 only, `3` = Pass 1 + Pass 2 complete. Backfill picks up any doc with `entities_version < 3` when Pass 2 becomes available.

5. **BERT + ort + tokenizers removed** — `bert-base-NER.onnx`, `tokenizer.json`, `config.json` deleted from `src-tauri/models/`. `ort`, `tokenizers` deps removed from `src-tauri/Cargo.toml`. `cargo check` passes.

6. **LLM-optional path** — user can connect zero providers; Pass 1 still runs on every doc. `topic` defaults to `other`. `tags` remain empty. `AiNoProviderBanner` surfaces "Connect AI for smarter entities → Settings" without blocking indexing.

7. **Settings → AI: "Extraction model" dropdown + "Use LLM for entity extraction" toggle + "Re-extract entities" button.** No taxonomy panel. Model dropdown defaults per provider (Haiku 4.5 / gpt-5-mini / gemini-2.5-flash / user-connected Ollama model).

### Out of scope

- **Adaptive taxonomy engine + consolidation loop** — deferred to v1.2. Free-form topic + tags provide "emergent" feel without the class-explosion risk.
- **Settings taxonomy panel (approve merges/renames/splits)** — deferred with the consolidation loop.
- Phase 9 (LLM space labeling) uses entity output but doesn't own extraction.
- Phase 11 (entity-driven exploration) consumes stored entities but doesn't own extraction.
- Streaming responses (single-shot JSON only).
- Cost / token budget UI (deferred to v1.2).
- Per-provider extraction routing (single active provider for v1.1).
- Multi-modal entity extraction from images (Phase 15 rupixel owns image understanding).

</domain>

<decisions>
## Implementation Decisions

### Pass 1 — Deterministic Pattern Extractor

- **D-01: Module = `pipeline/pass1_pattern_extractor.rs`.** Pure Rust, no external deps beyond `regex`, `iso_currency` (or custom), and small validator helpers.
- **D-02: Runs on every doc after parse, before Pass 2.** Idempotent — same input → same output. `entities_version = 2.5` after Pass 1.
- **D-03: ID validators (strong):**
  - **Aadhaar** — 12 digits + Verhoeff checksum
  - **PAN** — `^[A-Z]{5}[0-9]{4}[A-Z]$`
  - **IBAN** — country prefix + Mod-97 (per ISO-13616)
  - **VIN** — 17 chars + weighted checksum
  - **GSTIN** — 15 chars + Luhn variant
  - **Credit card** — Luhn + BIN-range check OR context word ("card", "visa", "mastercard", "amex"). Luhn alone is 10% false-positive — insufficient.
  - **SSN** — `^\d{3}-\d{2}-\d{4}$` + area-code sanity
  - **NINO** — `^[A-CEGHJ-PR-TW-Z]{2}\d{6}[A-D]$`
  - **SIN** — 9 digits + Luhn
  - **TFN** — 9 digits + Australian checksum
- **D-04: Weak-format IDs (no checksum) are Pass-1 candidates only — Pass 2 assigns final subclass.** Includes: `policy_number`, `folio_number`, `account_number`, `plot_number`, `receipt_number`, `invoice_number`. Pass 1 flags them as `Identifier{subclass=unknown}` with pattern hints.
- **D-05: Date parsing via `chrono` + `dateparser`.** Handles RFC-3339, ISO-8601, DD/MM/YYYY, MM-DD-YYYY, "3 Jul 2026", "Jan 1, 2025". Ambiguous formats (DD/MM vs MM/DD) resolved by locale hint from Settings.
- **D-06: Amount parsing.** Regex captures currency symbol (`$`, `₹`, `€`, `£`, `¥`) or ISO-4217 code (`USD`, `INR`, `EUR`). Falls back to `null` currency when digits appear without symbol.

### Pass 2 — LLM Refiner

- **D-07: Module = `pipeline/pass2_llm_refiner.rs`.** Calls `ai_request()` from Phase 7. Skips silently when no provider connected.
- **D-08: Output JSON schema (fixed):**
  ```json
  {
    "additional_entities": [
      { "class": "Person", "subclass": null, "value": "Alex Doe", "confidence": 0.95 }
    ],
    "refined_entities": [
      { "pass1_id": "e_42", "class": "Identifier", "subclass": "aadhaar", "confidence": 0.9 }
    ],
    "topic": "identity",
    "tags": ["family", "passport", "renewal_2024"]
  }
  ```
- **D-09: 8-class fixed schema (locked).** `Person, Organization, Location, Date, Amount, Email, Phone, Identifier` + free-form `subclass` string. LLM cannot invent new classes — prompt states this explicitly. Emergence goes to `tags`, not `class`.
- **D-10: Prompt inline in `pipeline/pass2_llm_refiner.rs` as `const REFINE_PROMPT: &str`.** Multi-region few-shot examples cover India (Aadhaar, PAN, GSTIN, IFSC), US (SSN, EIN, routing), UK (NINO, sort code), EU (IBAN), plus generic (passport, VIN, credit card, IBAN).
- **D-11: Model defaults per provider (user-selectable in Settings):**
  - Anthropic: `claude-haiku-4-5-20251001`
  - OpenAI-Codex: `gpt-5-mini`
  - Gemini: `gemini-2.5-flash`
  - Ollama: user's connected model (no forced default)
  Fast+cheap tier. Selectable in Settings → AI → "Extraction model" dropdown.
- **D-12: `temperature = 0` for determinism.** LLME-03 idempotence holds within a pinned model version. Cross-version drift is accepted (user re-runs backfill on model change).
- **D-13: Concurrency cap = 8 in-flight requests to active provider.** Semaphore in Pass 2. Respects rate limits (Anthropic prod 1000 RPM). Free-tier users see slower throughput; toast informs.
- **D-14: JSON parsing robustness.**
  1. Try `serde_json::from_str` directly.
  2. On failure, strip markdown fences (``` ```json ... ``` ```) + retry.
  3. On second failure, log warning + skip Pass 2 for that doc (Pass 1 result stands).
- **D-15: OCR tolerance.** Prompt instructs LLM: *"Doc may be OCR'd from a scanned image; entities may have typos, missing characters, or mis-spaced words. Extract confidently when the text is clear; set `confidence < 0.7` for values you suspect are OCR-corrupted."* UI filters low-confidence entities from summary view (>= 0.7 threshold), shows them under "Also found" expander.

### Pipeline Integration

- **D-16: Replace `NerService` at every call site with a `TwoPassExtractor` handle.** Same interface: `extract(&self, text: &str) -> Result<ExtractedEntities, AppError>`. Drop-in.
- **D-17: Extraction runs async during indexing** (matches embedder). Per-doc failure isolated via `tokio::spawn` per doc; sibling docs unaffected (LLME-04).
- **D-18: One doc per LLM call.** Portable, simple retry, avoids provider-specific batch limits.
- **D-19: Heads-and-tails chunking for long docs.** Content > 12k chars → send `title + first 6k + last 6k` to Pass 2. Pass 1 runs on full content (regex is cheap).
- **D-20: Merge policy.** Pass-2 `refined_entities` override Pass-1 classifications on match by `pass1_id`. Pass-1 entities Pass-2 didn't refine keep raw pattern-based classification. Pass-2 `additional_entities` appended.
- **D-21: EntityStore schema migration.** Adds columns: `class`, `subclass`, `confidence`, `topic`, `tags` (JSON array or normalized side-table). SQLite `ALTER TABLE` migration script + backfill of legacy BERT rows (`PER → Person`, `ORG → Organization`, `LOC → Location`).

### Backfill Trigger & Progress

- **D-22: Backfill trigger = explicit "Re-extract entities" button** in Settings → AI. Disabled when no active provider OR backfill in flight. Pre-flight cost estimate shown as tooltip: `"Est: $X.XX across N docs on {model}"` — computes from `avg_input_tokens × doc_count × model_pricing`.
- **D-23: Reuse existing `spawn_entity_backfill`** in `pipeline/backfill.rs`. Swap `NerService` arg for `TwoPassExtractor`. Progress event schema (`EntityBackfillProgress`) unchanged. Bump target `entities_version` const from 2 → 3.
- **D-24: Cancellability.** Backfill cancels when active provider is disconnected. In-flight LLM call completes cleanly (no forced abort); no new docs picked up. Resume: next trigger picks up where `entities_version < 3` left off.
- **D-25: ETA calc.** Rolling average of last 20 docs' Pass-2 latency × remaining doc count. TopBar `BackfillIndicator` reads existing `entity-backfill-progress` event.

### Failure Handling

- **D-26: Per-doc Pass-2 fallback = Pass-1 only.** When Pass 2 fails after retries, doc keeps Pass-1 entities at `entities_version = 2.5`. Future backfill re-attempts Pass 2 when provider is healthy. Meets LLME-04 — no doc is ever entity-less.
- **D-27: Retry policy.** 3 Pass-2 attempts via `ai/retry.rs` (exponential backoff + jitter, cap 60s per attempt).
- **D-28: Rate limit handling.** Respect provider `Retry-After` HTTP header when present; else exp backoff. Reuse learnforge classifier from Phase 7.
- **D-29: User visibility.** Silent per-doc failure logs. Backfill completion → single sonner toast: `"Backfill complete. X of Y docs used Pass-1 only. Retry after network is healthy."` No per-doc toast spam.
- **D-30: Provider switch warning.** Switching active provider mid-Cortex-usage triggers info toast: `"Provider switched. New extractions use {model}. Run 'Re-extract entities' for consistent labels across all docs."` No forced backfill.

### LLM-Optional Path

- **D-31: Cortex remains functional without any connected LLM provider.** Pass 1 runs on every doc; entities land in the index; search works over regex-extracted entities. `topic = "other"`, `tags = []`. Smart Spaces still form via vector clustering (unaffected).
- **D-32: Pass-2 activation.** Once user connects a provider + toggles "Use LLM for entity extraction" on, "Re-extract entities" button becomes enabled. First click runs Pass 2 across all docs with `entities_version < 3`.
- **D-33: Pass-2 disable toggle.** Settings → AI checkbox "Use LLM for entity extraction" (default: on when provider connected). Off → Pass 1 only. Privacy-strict users get zero doc content sent to LLM.
- **D-34: AiNoProviderBanner.** Existing Phase 7 banner; add sub-copy: `"Connect AI to extract people, organizations, and topic tags from your docs. Dates, amounts, and IDs work without AI."`. Reuses `useAiBannerStore` + session-dismissable pattern.

### Image Docs (Scanned JPGs / PNGs)

- **D-38a: Image-doc entity extraction path = existing tesseract OCR → text-Pass 2.** No new image-specific handling in Phase 8. Scans (Ford Figo JPGs, passport/Aadhaar PDFs) go through existing OCR path in `pipeline/parse.rs`, resulting text feeds Pass 1 + Pass 2 identically to text docs.
- **D-38b: Rupixel scoped to Phase 15 only.** After reading rupixel README, confirmed it is a **visual/text embedding + search** tool (CLIP over screenshots, MiniLM over text), NOT an OCR / entity extractor. Phase 15 owns rupixel for semantic image search + thumbnails. Not useful for Phase 8 entity extraction.
- **D-38c: Vision-LLM path deferred to v1.2.** Sending image directly to Claude Vision / GPT-4V / Gemini bypasses OCR loss. Higher quality but ~3× cost + requires per-provider vision detection. Deferred — image quality via OCR is acceptable for v1.1.
- **D-38d: Qwen3-VL local caption deferred.** Requires ~7GB model + GPU. Not a v1.1 lift.

### Free-Form Field Normalization

- **D-35: `topic` and `tags` normalized at write:** lowercase → trim → replace whitespace with `_` → strip non-alphanumeric-except-underscore. `Term Insurance` → `term_insurance`. `Investments 2024!` → `investments_2024`.
- **D-36: Topic seed suggestions (soft, not enforced).** Prompt provides 19-topic list as *examples*: `property, identity, vehicle, finance, investment, insurance, taxes, kids, education, family, work, business, bills, travel, medical, legal, spiritual, reference, other`. LLM prefers these when applicable but may invent (`gaming`, `crypto`, `research_papers`).
- **D-37: Topic filter UI paginates.** Settings + `/search` topic filter shows top-20 topics by count, "Show more" reveals long tail. Prevents 500+ topic dropdown becoming unusable.
- **D-38: No tag whitelist.** LLM emits 2-5 tags per doc freely. UI shows tag cloud in `/tags` page (Phase 4 route) sorted by count. Users can click to filter.

### Claude's Discretion (Planner-owned)

- Exact Tauri IPC command names: `trigger_entity_backfill`, `get_extraction_model`, `set_extraction_model`, `toggle_llm_extraction` (planner finalizes).
- Rust module tree: likely `pipeline/pass1_pattern_extractor.rs` + `pipeline/pass2_llm_refiner.rs` + `pipeline/two_pass_extractor.rs` (façade), plus updated `entities/types.rs`. Planner picks final layout.
- Prompt engineering pass — planner iterates the Pass-2 prompt against 10 sample docs from user's ~/private during a research spike:
  - `~/private/scanned/Alex/*.pdf` (identity IDs)
  - `~/private/scanned/Ford Figo/*.jpg` (vehicle multi-doc)
  - `~/private/personal/finance/*.pdf` (finance / taxes)
  - `~/private/khush/*.pdf` (kids)
  - `~/private/docs/GV7/*` (property)
  Verifies output covers 8-class schema + emits diverse topics + emits tags without hallucinating.
- Concurrency knobs: default 8, min 2, max 16, user-tunable in Settings → AI → Advanced (or Claude picks hidden default if UI not shown).
- Model dropdown values per provider — planner researches current model IDs as of implementation date.
- Migration script strategy for EntityStore — `SQLx` migration file OR ad-hoc `ALTER TABLE` on startup. Planner picks.
- BIN-range list for credit-card validation — planner sources compact list (Visa/MC/Amex/Discover prefixes) from public standards.
- Whether to also emit a `language` field per doc (English/Devanagari mix) — soft signal for Phase 9 space labeling. Recommend yes; planner decides scope.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project specs
- `.planning/ROADMAP.md` §"Phase 8: LLM Entity Extraction" — goal, requirements list (LLME-01..06), success criteria
- `.planning/REQUIREMENTS.md` §"LLM Entity Extraction" — LLME-01..06 full text
- `.planning/PROJECT.md` §"Current Milestone: v1.1" — cloud-first AI intelligence positioning
- `.planning/phases/07-ai-provider-foundation/07-CONTEXT.md` — Phase 7 provider abstraction, credential store, `ai_request()` router surface

### Existing Cortex code (read before modifying)
- `src-tauri/src/pipeline/ner.rs` — BERT-based NerService (to be replaced entirely). Study interface signature so `TwoPassExtractor` is a drop-in.
- `src-tauri/src/pipeline/backfill.rs` — `spawn_entity_backfill` machinery (throttled progress events). Reuse.
- `src-tauri/src/graph/entity_store.rs` — `EntityStore` write API. Add columns: `class`, `subclass`, `confidence`, `topic`, `tags`.
- `src-tauri/src/types.rs` — `ExtractedEntity` struct. Bump `entities_version` constant from 2 to 3. Add `class`, `subclass`, `confidence`, `topic`, `tags` fields.
- `src-tauri/src/ai/service.rs` — `ai_request()` router (Phase 7). Pass 2 calls this.
- `src-tauri/src/ai/retry.rs` — retry/backoff. Reuse.
- `src-tauri/src/commands/settings.rs` — settings persistence pattern. Extend for `extraction_model`, `use_llm_extraction`.
- `src-tauri/models/bert-base-NER.onnx`, `tokenizer.json`, `config.json` — delete
- `src-tauri/Cargo.toml` — remove `ort`, `tokenizers`
- `client/pages/SettingsPage.tsx` §"AI & Models Tab" — add "Extraction model" dropdown, "Use LLM for entity extraction" toggle, "Re-extract entities" button in AI Providers section.
- `client/components/layout/BackfillIndicator.tsx` — reads `entity-backfill-progress` event; no changes needed.
- `client/components/AiNoProviderBanner.tsx` (from Phase 7) — extend copy to mention entity extraction benefit.

### Patterns to mirror
- Phase 7 `AuthState` + `CredentialStore` — mirror app-state pattern for two-pass extractor handle.
- Phase 5 settings JSON persistence in app_data_dir — extend for new settings fields.
- Phase 4 React Query hook factory in `client/hooks/useTauri.ts` — add `useExtractionModel`, `useUseLlmExtraction`, `useTriggerBackfill`.
- Phase 1/4 IPC convention: `#[tauri::command] async + spawn_blocking + serde camelCase`.
- Backfill throttling (500ms / 25 docs) — reuse from `pipeline/backfill.rs`.

### Reference codebase (port patterns)
- `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/retry.rs` — retry/backoff pattern.

### Sample corpus for prompt tuning
- `~/private/scanned/Alex/` — identity IDs (Aadhaar, PAN, Passport, Voter ID, DL, Birth/Marriage). Verify Pass 1 + Pass 2 both recognize.
- `~/private/scanned/Ford Figo/` — vehicle multi-doc (invoice, insurance, registration, tax cert). Verify all get topic=`vehicle`.
- `~/private/personal/finance/` — bank statements, ITR-V PDFs.
- `~/private/personal/investments/` — MF folios, stocks, insurance.
- `~/private/khush/` — kids school enrollment.
- `~/private/docs/GV7/`, `~/private/docs/AlphaComplex/` — property docs.
- `~/private/Screenshots/` — screenshots (image content). Pass 2 will fail here without rupixel; document as expected until Phase 15.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `pipeline/backfill.rs::spawn_entity_backfill` — full progress/throttle infra, just swap extraction backend.
- `ai/service.rs::ai_request()` — provider-agnostic entry point from Phase 7. Pass 2's single AI dependency.
- `ai/retry.rs` — exponential backoff w/ jitter (learnforge port from Phase 7).
- `graph/entity_store.rs::EntityStore` — needs schema extension, but write pattern reusable.
- `types.rs::ExtractedEntity` — extend fields, bump `entities_version` constant.
- Settings JSON sidecar pattern from Phase 5.
- React Query hook factory (`client/hooks/useTauri.ts`).
- `sonner` toast for backfill completion + provider-switch info.
- Existing `BackfillIndicator` component in TopBar (Phase 4/7).
- `RadioGroup`, `Select`, `Switch`, `Button` primitives from shadcn.

### Established Patterns
- IPC: `#[tauri::command] async + spawn_blocking + serde camelCase`.
- App state: `Arc<tokio::sync::Mutex<T>>` for mutable shared state managed via `.manage()`.
- Persistence: JSON sidecars in `app_data_dir/`.
- Event-driven progress: Tauri emit + frontend `useEffect` listener.
- Error surfacing: `AppError` variants → `sonner` toast on frontend.
- Zustand for UI state; React Query for server state.

### Integration Points
- `src-tauri/src/lib.rs` — register `TwoPassExtractor` handle via `.manage()`. Register new IPC commands.
- `commands/mod.rs` — add `trigger_entity_backfill`, `get_extraction_settings`, `set_extraction_settings`.
- `pipeline/mod.rs` — remove `ner` module; add `pass1_pattern_extractor`, `pass2_llm_refiner`, `two_pass_extractor`.
- `client/pages/SettingsPage.tsx` — extend AI tab with Extraction Model + Use LLM toggle + Re-extract button.
- `client/hooks/useTauri.ts` — add extraction settings hooks.
- `client/components/AiNoProviderBanner.tsx` — extend copy.

</code_context>

<specifics>
## Specific Ideas

- **Every family has universal doc patterns** — identity, property, vehicle, finance, kids, medical. 8-class seed reflects this. Not user-specific.
- **~/private sample corpus** — Indian-context with Aadhaar/PAN/GSTIN, but seed + prompt few-shot are multi-region (SSN/NINO/IBAN/GBP/EUR). LLM generalizes across regions.
- **Two-pass = LLM-optional** — Pass 1 alone delivers dates/emails/amounts/IDs (30% of ideal). Pass 2 adds Person/Org/Location + topic + tags (100%). Users choose privacy vs power.
- **Fixed 8 classes prevent explosion** — LLM cannot invent new classes. Emergence goes to `topic` + `tags` (free-form). Best of both: stable schema for queries + emergent labels for organization.
- **Free-form tags normalized at write** — `Term Insurance` → `term_insurance`. Prevents `Investments` vs `investment` fragmenting search.
- **Confidence field for OCR tolerance** — LLM emits confidence per entity; UI shows entities >= 0.7 by default; low-confidence shown under "Also found" expander.
- **Idempotence via temp=0 + pinned model + versioned prompt** — same doc + same model + same prompt version → same output. Cross-version drift accepted.
- **Chunking heads-and-tails** — 12k char cap, title + first 6k + last 6k. Balances cost vs coverage.
- **Pass-1 credit-card must use BIN check or context word** — Luhn alone is 10% false-positive.
- **Migration required** — old BERT `PER/ORG/LOC` → new `Person/Organization/Location`. Schema `ALTER TABLE` adds `class, subclass, confidence, topic, tags`.

</specifics>

<deferred>
## Deferred Ideas

### Phase 8 follow-ups (v1.2 candidates)
- **Adaptive taxonomy engine with consolidation + Settings panel** — original proposal. Deferred because fixed 8-class + free-form topic/tags covers 80% of value without explosion risk. Revisit if users find fixed classes too rigid.
- **Per-provider extraction routing** (Haiku for entities, Sonnet for consolidation). Single active provider ships v1.1.
- **Cost / token-usage tracking UI** — `ai_request()` returns token counts; aggregate UI deferred.
- **Named-entity linking (NEL)** — resolve "Alex" and "Alex Doe" to same canonical Person node. Alias merging exists in `entity_store.rs`; extend in Phase 11.
- **Confidence-driven re-extraction** — auto-retry low-confidence entities with a stronger model.
- **Multi-turn extraction chain** ("extract → verify → refine") — single-shot for now.

### Downstream phase dependencies
- **Phase 9 (LLM Space Labeling)** consumes `topic` + `tags` from Phase 8 as soft cluster signals alongside vectors.
- **Phase 11 (Entity-Driven Exploration)** consumes `class` + `subclass` fields for filter chips and entity-detail pages.
- **Phase 13 (Cypher Entity Graph)** uses `class` as node label in ruvector-graph.
- **Phase 15 (Visual Intelligence / rupixel)** — image entity extraction (screenshots, scanned JPGs). Would call rupixel to caption image, feed caption to Pass 2. Deferred to Phase 15.

### v2 / future
- **Adaptive taxonomy** — revisit if free-form topics prove too messy (500+ topics user can't navigate). Consolidation loop, Settings taxonomy panel.
- **User-provided extraction rules** — regex or heuristic overrides ("always extract 'AlphaComplex' as Location + tag=property"). Explicit user overrides for v2.
- **Multilingual doc support** — assumes English + Roman-script text. Native scripts (Devanagari, Tamil, Arabic) untested. v2.
- **Streaming JSON output** — for very long docs, stream entities. Single-shot for now.

</deferred>

---

*Phase: 08-llm-entity-extraction*
*Context gathered: 2026-07-03*
*Design revised: 2026-07-03 (simpler middle path — dropped adaptive taxonomy + consolidation + Settings panel)*

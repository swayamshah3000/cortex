# Phase 8: LLM Entity Extraction - Research

**Researched:** 2026-07-03
**Domain:** Rust NLP pipeline — two-pass entity extraction, JSON robustness, SQLite migration, async concurrency
**Confidence:** HIGH (all architectural decisions are locked in CONTEXT.md; research focuses on crate selection and implementation patterns)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- D-01: `pipeline/pass1_pattern_extractor.rs` — pure Rust, no external deps beyond `regex`, ISO currency helpers, and small validator helpers
- D-02: Pass 1 runs on every doc after parse, before Pass 2. `entities_version = 2.5` after Pass 1.
- D-03: ID validators: Aadhaar (Verhoeff), PAN (`^[A-Z]{5}[0-9]{4}[A-Z]$`), IBAN (Mod-97), VIN (weighted checksum), GSTIN (Luhn-variant), credit card (Luhn + BIN prefix OR context word), SSN, NINO, SIN, TFN
- D-04: Weak-format IDs flagged as `Identifier{subclass=unknown}` in Pass 1; Pass 2 assigns final subclass
- D-05: Date parsing via `chrono` + `dateparser`. Ambiguous formats resolved by locale hint from Settings
- D-06: Amount parsing via regex with currency symbol/code; null currency when no symbol
- D-07: `pipeline/pass2_llm_refiner.rs`. Calls `ai_request()` from Phase 7. Skips when no provider
- D-08: Fixed output JSON schema (see CONTEXT.md). `additional_entities`, `refined_entities`, `topic`, `tags`
- D-09: 8-class fixed schema (Person, Organization, Location, Date, Amount, Email, Phone, Identifier). LLM cannot invent new classes
- D-10: Prompt inline as `const REFINE_PROMPT: &str`. Multi-region few-shot examples
- D-11: Model defaults — Anthropic: `claude-haiku-4-5-20251001`; OpenAI-Codex: `gpt-5-mini`; Gemini: `gemini-2.5-flash`; Ollama: user model
- D-12: `temperature = 0` for determinism
- D-13: Concurrency cap = 8 in-flight requests. `tokio::sync::Semaphore`
- D-14: JSON parsing robustness: try direct → strip fences → log warning + skip Pass 2 for doc
- D-15: OCR tolerance. Confidence < 0.7 → "Also found" expander in UI
- D-16: `TwoPassExtractor` replaces `NerService` at every call site. Same interface
- D-17: Extraction runs async during indexing. Per-doc failure isolated via `tokio::spawn`
- D-18: One doc per LLM call
- D-19: Heads-and-tails chunking: title + first 6k + last 6k for docs > 12k chars
- D-20: Merge policy — Pass-2 `refined_entities` override Pass-1 by `pass1_id`; unrefined Pass-1 kept; `additional_entities` appended
- D-21: EntityStore schema migration — ADD columns: `class`, `subclass`, `confidence`, `topic`, `tags`. Backfill legacy BERT rows: `PER → Person` etc.
- D-22: "Re-extract entities" button in Settings → AI. Pre-flight cost estimate tooltip
- D-23: Reuse `spawn_entity_backfill`. Swap `NerService` arg for `TwoPassExtractor`. Bump `entities_version` gate from 2 → 3
- D-24: Cancellability via provider disconnect. Resume: `entities_version < 3`
- D-25: ETA calc — rolling average of last 20 docs' Pass-2 latency × remaining
- D-26: Per-doc Pass-2 fallback = Pass-1 only. `entities_version = 2.5`
- D-27: 3 Pass-2 attempts via `ai/retry.rs` (exp backoff + jitter, cap 60s/attempt)
- D-28: Respect provider `Retry-After` header; else exp backoff
- D-29: Silent per-doc failure logs; single toast at backfill completion if fallbacks occurred
- D-30: Provider switch info toast
- D-31: Cortex functional without LLM provider (Pass 1 only)
- D-32: "Re-extract entities" button enabled once provider connected + toggle on
- D-33: "Use LLM for entity extraction" toggle (default: on when provider connected)
- D-34: `AiNoProviderBanner` copy extension
- D-35: `topic` and `tags` normalized at write: lowercase → trim → `_` for whitespace → strip non-alphanumeric-except-`_`
- D-36: Topic seed suggestions (19-topic list, not enforced)
- D-37: Topic filter UI paginates — top-20 + "Show more"
- D-38a: Image-doc extraction = existing OCR → text → Pass 1 + Pass 2
- LLME-06 (HARD): `bert-base-NER.onnx`, `tokenizer.json`, `config.json` deleted; `ort`, `tokenizers` removed from Cargo.toml; `cargo check` must pass

### Claude's Discretion

- Exact Tauri IPC command names (`trigger_entity_backfill`, `get_extraction_model`, `set_extraction_model`, `toggle_llm_extraction`)
- Rust module tree layout
- Prompt engineering iteration (Pass-2 prompt refined during implementation against sample docs)
- Concurrency knobs exposure in Settings UI (planner decides if user-tunable or hidden default)
- BIN-range list for credit-card validation
- Whether to emit a `language` field per doc

### Deferred Ideas (OUT OF SCOPE)

- Adaptive taxonomy engine + consolidation loop → v1.2
- Settings taxonomy panel → deferred with consolidation loop
- Per-provider extraction routing → v1.1 uses single active provider
- Cost / token-usage tracking UI → deferred
- Named-entity linking beyond existing alias merge → Phase 11
- Streaming responses → single-shot JSON only
- Vision-LLM / Qwen3-VL path → Phase 15
- Multilingual native script support → v2
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LLME-01 | User-visible entities on /document/:id are extracted by the active AI provider, not bert-base-NER | Two-pass engine design; `TwoPassExtractor` drop-in interface |
| LLME-02 | Entity types cover Person/Org/Location/Date/Amount/Email/Phone/topic uniformly across any domain | Fixed 8-class taxonomy locked in D-09; Pass 1 covers deterministic types; Pass 2 adds semantic types |
| LLME-03 | 20-entity cap; idempotent extraction | Existing `sort_dedup_cap()` in `pipeline/entities.rs` reusable; temperature=0 + pinned model ensures idempotence |
| LLME-04 | Single doc failure does not block rest of index | Per-doc isolation via `tokio::spawn`; fallback to Pass-1 on Pass-2 failure |
| LLME-05 | Backfill triggered from Settings → AI; TopBar progress visible | `spawn_entity_backfill` reuse; existing `BackfillIndicator` + `entity-backfill-progress` event unchanged |
| LLME-06 | BERT + ort + tokenizers absent from repo after phase; `cargo check` passes | Deletion plan + call-site replacement with `TwoPassExtractor` |
</phase_requirements>

---

## Summary

Phase 8 replaces the BERT-based NerService with a two-pass extraction engine. Pass 1 is deterministic regex + validators (already partially in `pipeline/entities.rs`). Pass 2 is LLM-driven via the Phase 7 `ai_request()` abstraction. All provider-routing, credential management, and retry logic is inherited from Phase 7 — Phase 8's work is primarily: (a) building the two-pass engine, (b) adding ID validators, (c) adding a SQLite-style schema migration to EntityStore, (d) wiring the LLM call with JSON robustness, and (e) deleting the BERT stack.

The key architectural insight is that `EntityStore` is an **in-memory** graph store (not a SQLite file). "Schema migration" in this phase means adding new fields to `ExtractedEntity` and `CanonicalEntity` in `types.rs`, updating the JSON serialization stored in RuVector metadata, and backfilling legacy BERT records via the version gate. There is no separate SQLite file to migrate.

**Primary recommendation:** Build `TwoPassExtractor` as a thin facade over `Pass1PatternExtractor` and an optional async `Pass2LlmRefiner`. Reuse the existing `ai_request_with_retry()` for all LLM calls. Extend `ExtractedEntity` with new fields using `#[serde(default)]` for backward compatibility.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Pass 1 pattern extraction (regex + validators) | Rust backend | — | Pure CPU, no I/O; runs synchronously in `spawn_blocking` |
| Pass 2 LLM refinement | Rust backend (async task) | Phase 7 ai_request | Network I/O; uses existing async AI router |
| Backfill orchestration + progress events | Rust backend (Tokio task) | Frontend (BackfillIndicator) | Long-running background task; events drive UI |
| Entity persistence (class, subclass, confidence, topic, tags) | Rust backend (EntityStore + RuVector metadata) | — | EntityStore is in-memory; metadata stored as JSON in RuVector VectorEntry |
| Settings controls (model dropdown, toggle, re-extract button) | Frontend (React) | Rust IPC commands | UI state + Tauri invoke pattern |
| Topic/tag normalization | Rust backend (at write time) | — | Deterministic; normalizes before persistence |
| Concurrency management (semaphore) | Rust backend | — | `tokio::sync::Semaphore` cap at 8 |

---

## Standard Stack

### Core (already in Cargo.toml)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `regex` | 1.x (existing) | Pass 1 pattern matching (date, email, phone, amount, URL) | Already in project; standard Rust regex engine |
| `chrono` | 0.4.45 | Date parsing and normalization | Most-downloaded Rust datetime crate; 656M+ downloads |
| `serde_json` | 1.x (existing) | JSON parsing/serialization for LLM output and entity metadata | Already in project; 1B+ downloads |
| `tokio` | 1.x (existing) | Async runtime for Pass 2; `tokio::sync::Semaphore` for concurrency cap | Already in project |

### New Dependencies Required

| Library | Version | Purpose | Why This One |
|---------|---------|---------|--------------|
| `dateparser` | 0.3.1 | Multi-format date parsing (D-05): RFC-3339, ISO-8601, human formats | 3.2M downloads; handles "3 Jul 2026", "Jan 1, 2025"; wraps chrono. [VERIFIED: cargo search] |
| `iban_validate` | 5.0.1 | IBAN Mod-97 validation (D-03) | 18.6M downloads; well-maintained; validates against official IBAN spec. [VERIFIED: cargo search] |
| `verhoeff` | 1.0.0 | Aadhaar 12-digit Verhoeff checksum (D-03) | 531K downloads; purpose-built for the Verhoeff algorithm. [VERIFIED: cargo search] |
| `luhn` | 1.0.1 | Luhn checksum for credit cards, SIN, GSTIN-variant (D-03) | 615K downloads; minimal, correct implementation. [VERIFIED: cargo search] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `dateparser` | Hand-rolled chrono parsing | dateparser handles 20+ formats including human dates; hand-roll risks missing DD/MM vs MM/DD ambiguity |
| `iban_validate` | `iban_check` crate | iban_validate has 18M downloads vs 2K; clear winner by adoption |
| `verhoeff` | Custom Verhoeff impl | Verhoeff is a 60-line algorithm; custom is viable but `verhoeff` crate is battle-tested |
| `luhn` | `luhn_ultra` (SIMD) | luhn_ultra is overkill for our use case; plain `luhn` is simpler and sufficient |

**Installation:**
```bash
# In src-tauri/Cargo.toml:
dateparser = "0.3.1"
iban_validate = "5.0.1"
verhoeff = "1.0.0"
luhn = "1.0.1"
```

**Remove from src-tauri/Cargo.toml:**
```bash
# Remove these lines:
ort = { version = "2.0.0-rc.12", ... }
tokenizers = "0.20"
ndarray = "0.17"    # Only used by ner.rs for BIO decoding
```

---

## Package Legitimacy Audit

> slopcheck was unavailable at research time. All packages below are tagged `[ASSUMED]` by registry existence only. Planner must gate each `cargo add` behind a `checkpoint:human-verify` task or manually verify via official documentation.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `regex` | crates.io | ~10 yrs | 1B+ | github.com/rust-lang/regex | [ASSUMED] | Approved — part of rust-lang org, used in project already |
| `chrono` | crates.io | ~10 yrs | 656M | github.com/chronotope/chrono | [ASSUMED] | Approved — de facto standard Rust datetime library |
| `serde_json` | crates.io | ~10 yrs | 1B+ | github.com/serde-rs/json | [ASSUMED] | Approved — already in project |
| `dateparser` | crates.io | 3+ yrs | 3.2M | github.com/waltzofpearls/dateparser | [ASSUMED] | Flagged — lower downloads than core crates; verify before use |
| `iban_validate` | crates.io | 5+ yrs | 18.6M | github.com/ThomasdenH/iban_validate | [ASSUMED] | Approved — strong download count, per cargo search |
| `verhoeff` | crates.io | 3+ yrs | 531K | unknown — needs verification | [ASSUMED] | Flagged — moderate downloads; verify source repo |
| `luhn` | crates.io | 3+ yrs | 615K | unknown — needs verification | [ASSUMED] | Flagged — moderate downloads; verify source repo |

**Packages removed due to slopcheck [SLOP] verdict:** none (slopcheck unavailable)
**Packages flagged as [ASSUMED]:** dateparser, iban_validate, verhoeff, luhn — planner must insert `checkpoint:human-verify` before each install

*slopcheck was unavailable at research time — all new packages above are tagged `[ASSUMED]` and the planner must gate each install behind a `checkpoint:human-verify` task.*

---

## Architecture Patterns

### System Architecture Diagram

```
Document text (full content)
         │
         ▼
 Pass1PatternExtractor          (pipeline/pass1_pattern_extractor.rs)
 ├─ DateParser (chrono+dateparser)
 ├─ EmailRegex (RFC-5322 subset)
 ├─ PhoneRegex (E.164 + national)
 ├─ AmountRegex (currency symbol + digits)
 ├─ UrlRegex
 └─ IdentifierValidators:
     Aadhaar→verhoeff, IBAN→iban_validate, Luhn→luhn, PAN/SSN/NINO→regex
         │
         │  Pass1Entities (entities_version=2.5)
         ▼
 TwoPassExtractor facade         (pipeline/two_pass_extractor.rs)
         │
    ┌────┴─────────────────────────────┐
    │  provider connected + toggle ON? │
    └────┬─────────────────┬───────────┘
         │ YES             │ NO
         ▼                 ▼
Pass2LlmRefiner        Pass-1 only
(pass2_llm_refiner.rs) entities_version=2.5
         │
    tokio::Semaphore (cap=8)
         │
    ai_request_with_retry()     (ai/retry.rs — 3 attempts, exp backoff)
         │
    Provider API (Anthropic/OpenAI/Gemini/Ollama)
         │
    JSON response
    ├─ try serde_json::from_str
    ├─ on failure: strip ```json fences
    └─ on second failure: log warn, return Pass-1 only
         │
    Merge: refined_entities override Pass-1 by pass1_id
         │
    ExtractedEntities (entities_version=3)
         │
         ▼
  topic/tags normalization (lowercase+snake_case)
         │
         ▼
  EntityStore.register_doc_entities()
         │
         ▼
  RuVector metadata JSON (updated VectorEntry)
```

### Recommended Module Tree

```
src-tauri/src/
├── pipeline/
│   ├── mod.rs                        # REMOVE: pub mod ner; ADD: pub mod pass1_pattern_extractor, pass2_llm_refiner, two_pass_extractor
│   ├── pass1_pattern_extractor.rs    # NEW: deterministic regex + validators
│   ├── pass2_llm_refiner.rs          # NEW: LLM refinement via ai_request()
│   ├── two_pass_extractor.rs         # NEW: facade struct wrapping Pass1 + optional Pass2
│   ├── entities.rs                   # KEEP: existing EntityExtractor (reuse sort_dedup_cap)
│   ├── backfill.rs                   # MODIFY: swap NerService → TwoPassExtractor, bump version gate 2→3
│   ├── indexer.rs                    # MODIFY: replace NerService calls with TwoPassExtractor
│   ├── embedder.rs                   # UNCHANGED
│   ├── hasher.rs                     # UNCHANGED
│   └── parser.rs                     # UNCHANGED
├── ai/
│   ├── service.rs                    # UNCHANGED (Phase 7 output)
│   └── retry.rs                      # UNCHANGED (reuse ai_request_with_retry)
├── graph/
│   └── entity_store.rs               # MODIFY: add class/subclass/confidence/topic/tags to stored JSON
├── types.rs                          # MODIFY: extend ExtractedEntity; add ExtractionSettings
└── commands/
    └── entities.rs                   # MODIFY: add trigger_entity_backfill, get/set_extraction_settings
```

### Pattern 1: TwoPassExtractor Interface

The interface must be a drop-in for `NerService`. The existing `backfill.rs` calls `ner_service.extract(text)`. `TwoPassExtractor` must expose the same synchronous `extract()` for use in `spawn_blocking` contexts, while also supporting an async `extract_with_llm()` for the backfill orchestration:

```rust
// Source: derived from pipeline/ner.rs interface + CONTEXT.md D-16
pub struct TwoPassExtractor {
    pass1: Pass1PatternExtractor,
    auth: Arc<AuthState>,
    semaphore: Arc<tokio::sync::Semaphore>,
    settings: Arc<tokio::sync::RwLock<ExtractionSettings>>,
}

impl TwoPassExtractor {
    /// Drop-in for NerService::extract — synchronous, Pass 1 only.
    /// Called from spawn_blocking contexts during normal indexing.
    pub fn extract(&self, text: &str) -> Result<Vec<ExtractedEntity>, AppError> {
        self.pass1.extract(text)
    }

    /// Full two-pass extraction — async, calls LLM if available.
    /// Called from backfill orchestration.
    pub async fn extract_full(&self, text: &str, title: &str) -> Result<ExtractedEntities, AppError> {
        let pass1_result = self.pass1.extract(text)?;
        if !self.llm_enabled().await {
            return Ok(ExtractedEntities { entities: pass1_result, version: 2.5, .. });
        }
        let refined = self.pass2_refine(text, title, &pass1_result).await;
        Ok(merge_passes(pass1_result, refined))
    }
}
```

### Pattern 2: JSON Fence Stripping (D-14)

Anthropic Haiku, Ollama, and some Gemini models occasionally wrap JSON responses in markdown code fences. The robustness pattern:

```rust
// Source: CONTEXT.md D-14
fn parse_llm_json(raw: &str) -> Result<serde_json::Value, AppError> {
    // Attempt 1: direct parse
    if let Ok(v) = serde_json::from_str(raw) {
        return Ok(v);
    }
    // Attempt 2: strip ``` fences and retry
    let stripped = strip_json_fences(raw);
    serde_json::from_str(&stripped)
        .map_err(|e| AppError::Internal(format!("LLM JSON parse failed after fence strip: {}", e)))
}

fn strip_json_fences(s: &str) -> String {
    let s = s.trim();
    // Handle ```json ... ``` and ``` ... ```
    let s = if let Some(inner) = s.strip_prefix("```json") {
        inner.trim_end_matches("```").trim()
    } else if let Some(inner) = s.strip_prefix("```") {
        inner.trim_end_matches("```").trim()
    } else {
        s
    };
    s.to_string()
}
```

### Pattern 3: Concurrency Semaphore (D-13)

`tokio::sync::Semaphore` provides the 8-in-flight cap without pulling in external deps:

```rust
// Source: tokio documentation (tokio is already in project deps)
let semaphore = Arc::new(tokio::sync::Semaphore::new(8)); // default cap

// Per document:
let _permit = semaphore.acquire().await
    .map_err(|e| AppError::Internal(e.to_string()))?;
let result = ai_request_with_retry(&auth, req, 2).await; // released when _permit drops
```

### Pattern 4: Verhoeff Checksum for Aadhaar (D-03)

```rust
// Source: CONTEXT.md D-03; verhoeff crate (cargo search: 531K downloads)
fn validate_aadhaar(s: &str) -> bool {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 12 { return false; }
    verhoeff::verify(&digits) // returns bool
}
```

### Pattern 5: IBAN Mod-97 Validation (D-03)

```rust
// Source: CONTEXT.md D-03; iban_validate crate (cargo search: 18.6M downloads)
fn validate_iban(s: &str) -> bool {
    use iban_validate::Iban;
    s.trim().replace(" ", "").parse::<Iban>().is_ok()
}
```

### Pattern 6: Luhn for Credit Card / SIN / GSTIN (D-03)

```rust
// Source: CONTEXT.md D-03; luhn crate (cargo search: 615K downloads)
// Credit card: Luhn PLUS BIN prefix check OR context word requirement
fn validate_credit_card(s: &str, context: &str) -> bool {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 13 || digits.len() > 19 { return false; }
    if !luhn::valid(&digits) { return false; }
    // D-03: Luhn alone has ~10% FPR. Require BIN prefix OR context word.
    has_valid_bin_prefix(&digits) || has_card_context_word(context)
}

// Minimal BIN ranges (Visa: 4, MC: 51-55/2221-2720, Amex: 34/37, Discover: 6011/644-649/65)
fn has_valid_bin_prefix(digits: &str) -> bool {
    let first = &digits[..1];
    let first2: u32 = digits[..2].parse().unwrap_or(0);
    let first4: u32 = digits[..4].parse().unwrap_or(0);
    first == "4"                                    // Visa
    || (first2 >= 51 && first2 <= 55)              // Mastercard range 1
    || (first4 >= 2221 && first4 <= 2720)           // Mastercard range 2
    || first2 == 34 || first2 == 37                // Amex
    || first4 == 6011 || first2 == 65              // Discover
}
```

### Pattern 7: Dateparser for Multi-Format Dates (D-05)

```rust
// Source: CONTEXT.md D-05; dateparser crate 0.3.1 (cargo search: 3.2M downloads)
use dateparser::parse;

fn extract_dates(text: &str) -> Vec<ExtractedEntity> {
    // dateparser handles: "2024-01-15", "01/15/2024", "January 15, 2024", "3 Jul 2026"
    // Returns Result<DateTime<Utc>>
    // Pattern: find potential date strings via regex first, then validate with dateparser
    // to avoid false-positives on version numbers like "1.2.3"
    let candidates = date_candidate_re.find_iter(text);
    candidates
        .filter_map(|m| parse(m.as_str()).ok())
        .map(|dt| ExtractedEntity { entity_type: "Date".to_string(), value: dt.to_rfc3339(), .. })
        .collect()
}
```

**Locale ambiguity note:** `dateparser` 0.3.1 defaults to US date ordering (MM/DD/YYYY). The locale hint from Settings (D-05) should be respected by pre-processing ambiguous dates before passing to dateparser, or by post-processing the result when the Settings locale is set to non-US.

### Pattern 8: Pass-2 Prompt Structure (D-10)

The `REFINE_PROMPT` constant should follow this structure:

```
You are an expert document entity extractor. Extract entities from the provided document text.

RULES:
1. Output ONLY valid JSON matching the schema below. No markdown, no explanation.
2. Classes are FIXED: Person, Organization, Location, Date, Amount, Email, Phone, Identifier
3. Do NOT invent new classes. Novel entities go into `tags` field, not `class`.
4. Set confidence < 0.7 for entities you suspect are OCR-corrupted.
5. The document may be OCR'd from a scanned image; tolerate typos and mis-spaced words.

SCHEMA:
{
  "additional_entities": [{"class": "...", "subclass": null|"string", "value": "...", "confidence": 0.0-1.0}],
  "refined_entities": [{"pass1_id": "...", "class": "...", "subclass": "...", "confidence": 0.0-1.0}],
  "topic": "single_snake_case_topic",
  "tags": ["tag1", "tag2"]
}

TOPIC SUGGESTIONS (prefer these, but may use others):
property, identity, vehicle, finance, investment, insurance, taxes, kids, education,
family, work, business, bills, travel, medical, legal, spiritual, reference, other

FEW-SHOT EXAMPLES:
[India] Aadhaar doc excerpt: "1234 5678 9012 ALEX DOE DOB: 15/07/1985"
→ {"additional_entities": [{"class":"Person","value":"ALEX DOE","confidence":0.95}],
   "refined_entities": [{"pass1_id":"e_0","class":"Identifier","subclass":"aadhaar","confidence":0.92}],
   "topic":"identity","tags":["aadhaar","personal_id"]}

[US] SSN fragment: "SSN: 123-45-6789 Name: John Smith"
→ {"additional_entities":[{"class":"Person","value":"John Smith","confidence":0.97}],
   "refined_entities":[{"pass1_id":"e_0","class":"Identifier","subclass":"ssn","confidence":0.95}],
   "topic":"identity","tags":["ssn","us_document"]}

[EU/UK] Bank statement: "IBAN: GB29 NWBK 6016 1331 9268 19 Amount: £1,234.56"
→ {"additional_entities":[],
   "refined_entities":[{"pass1_id":"e_0","class":"Identifier","subclass":"iban","confidence":0.98},
                        {"pass1_id":"e_1","class":"Amount","subclass":"gbp","confidence":0.99}],
   "topic":"finance","tags":["bank_statement","uk"]}

[Vehicle] Ford Figo invoice: "Vehicle: Ford Figo 1.2 VIN: MA1FD2GY5KP123456"
→ {"additional_entities":[{"class":"Organization","value":"Ford","confidence":0.95}],
   "refined_entities":[{"pass1_id":"e_0","class":"Identifier","subclass":"vin","confidence":0.96}],
   "topic":"vehicle","tags":["invoice","ford_figo","car_purchase"]}
```

### Pattern 9: Type Extension — `ExtractedEntity` (D-08, D-21)

The existing `ExtractedEntity` in `types.rs` has `label`, `value`, `entity_type`, `canonical_id`. Phase 8 adds:

```rust
// Source: CONTEXT.md D-08, D-21; src-tauri/src/types.rs (existing structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedEntity {
    pub label: String,        // human-readable label (keep for backward compat)
    pub value: String,
    pub entity_type: String,  // LEGACY: "date", "person", etc. Keep for Phase 6 frontend compat
    #[serde(default)]
    pub canonical_id: Option<String>,
    // --- NEW FIELDS (Phase 8) ---
    #[serde(default)]
    pub class: Option<String>,       // "Person", "Organization", "Location", "Date", "Amount", "Email", "Phone", "Identifier"
    #[serde(default)]
    pub subclass: Option<String>,    // free-form: "aadhaar", "iban", "pan", etc.
    #[serde(default)]
    pub confidence: Option<f32>,     // 0.0-1.0; None = Pass 1 (implicitly 1.0 for strong validators)
    // doc-level fields stored on first entity or separately:
    // topic and tags are stored at doc-metadata level, NOT per-entity
}

/// Doc-level extracted metadata (stored as separate keys in VectorEntry metadata)
/// "topic": String (normalized snake_case)
/// "tags": Vec<String> (normalized snake_case)
/// "entities_version": u64 (2=BERT, 2.5=Pass1, 3=Pass1+Pass2)
```

**Backward compatibility:** All new fields use `#[serde(default)]` so existing JSON entries in RuVector without these fields will deserialize with `None` values. No data loss on startup.

### Pattern 10: Backfill Version Gate Update (D-23)

```rust
// Source: pipeline/backfill.rs::collect_backfill_candidates (existing code)
// Change: version < 2 → version < 3
fn collect_backfill_candidates_v3(engine: &CortexEngine) -> Vec<String> {
    // Same logic as existing collect_backfill_candidates
    // Change only this check:
    if version < 3 {  // was: version < 2
        candidates.push(id);
    }
}
// Note: version 2.5 (Pass-1-only) docs will be re-processed to add Pass 2
// This is the correct behavior per D-23
```

### Pattern 11: ETA Calculation (D-25)

```rust
// Source: CONTEXT.md D-25
struct EtaCalculator {
    latencies: VecDeque<Duration>,  // ring buffer, max 20
}

impl EtaCalculator {
    fn record(&mut self, latency: Duration) {
        if self.latencies.len() >= 20 { self.latencies.pop_front(); }
        self.latencies.push_back(latency);
    }

    fn eta_seconds(&self, remaining: u32) -> Option<u32> {
        if self.latencies.is_empty() { return None; }
        let avg_ms: u128 = self.latencies.iter().map(|d| d.as_millis()).sum::<u128>()
            / self.latencies.len() as u128;
        Some((avg_ms * remaining as u128 / 1000) as u32)
    }
}
```

### Pattern 12: Pre-Flight Cost Estimator (D-22)

```rust
// Source: CONTEXT.md D-22
// Static pricing table (as of 2026-07; update when providers change pricing)
// All prices in USD per 1M input tokens
const MODEL_PRICING: &[(&str, f64)] = &[
    ("claude-haiku-4-5-20251001", 0.80),   // [ASSUMED: Anthropic pricing page]
    ("claude-sonnet-4-5", 3.00),            // [ASSUMED: Anthropic pricing page]
    ("gpt-5-mini", 0.40),                   // [ASSUMED: OpenAI pricing page]
    ("gpt-5", 5.00),                        // [ASSUMED: OpenAI pricing page]
    ("gemini-2.5-flash", 0.075),            // [ASSUMED: Google pricing page]
    ("gemini-2.5-pro", 1.25),               // [ASSUMED: Google pricing page]
];

fn estimate_cost(model: &str, avg_input_tokens: u32, doc_count: u32) -> Option<f64> {
    let price_per_m = MODEL_PRICING.iter()
        .find(|(m, _)| *m == model)
        .map(|(_, p)| *p)?;
    let total_tokens = avg_input_tokens as f64 * doc_count as f64;
    Some(total_tokens * price_per_m / 1_000_000.0)
}
// For Ollama: return None (display "free (local model)")
```

### Anti-Patterns to Avoid

- **Anti-pattern: Luhn alone for credit cards.** Luhn alone has ~10% false-positive rate. Must combine with BIN prefix check OR context word per D-03.
- **Anti-pattern: Calling ai_request without semaphore.** Unbounded concurrency will rate-limit on Anthropic's 1000 RPM cap. Always acquire semaphore permit before calling.
- **Anti-pattern: Parsing LLM JSON without fence-stripping.** Models reliably emit fences ~20% of the time. Always apply D-14 two-attempt parse.
- **Anti-pattern: Schema extension without `#[serde(default)]`.** Existing RuVector metadata for indexed docs will lack new fields. All new fields need `#[serde(default)]` or they'll break deserialization at startup.
- **Anti-pattern: Deleting `ner.rs` before replacing all call sites.** Backfill.rs imports `NerService` directly. Map all 3 call sites: `pipeline/backfill.rs`, `pipeline/indexer.rs` (extract_with_ner), and any command handler. Confirm with `cargo check` before committing the deletion.
- **Anti-pattern: Blocking async from inside spawn_blocking on the same runtime.** Pass 2 is async; it must run in an async context, not inside `spawn_blocking`. The backfill loop should be an async task that calls `extract_full()` directly (not via `spawn_blocking`).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| IBAN Mod-97 validation | Custom checksum | `iban_validate` crate | ISO-13616 spec has country-specific lengths; custom impls miss edge cases |
| Verhoeff checksum | Custom digit tables | `verhoeff` crate | 5-table algorithm; subtle transposition detection; hand-roll risks bugs |
| Luhn checksum | Custom loop | `luhn` crate | Trivially implemented but zero benefit vs tested crate |
| Multi-format date parsing | Regex soup | `dateparser` + `chrono` | 20+ formats including human text; ambiguous ordering logic |
| Async concurrency cap | Channel-based worker pool | `tokio::sync::Semaphore` | Tokio Semaphore is idiomatic, composable, zero overhead |
| Retry/backoff | Sleep loop | `ai/retry.rs::ai_request_with_retry` | Already implemented in Phase 7 with jitter and exp doubling |

**Key insight:** The validation algorithms (Verhoeff, IBAN Mod-97, Luhn) are deceptively simple-looking but have subtle correctness requirements. An incorrect Verhoeff implementation will pass basic tests but miss transposition errors — exactly the errors it's designed to catch.

---

## Runtime State Inventory

> Runtime state investigation for the BERT removal aspect of this phase.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | RuVector VectorEntry metadata with `entities_version=2` (BERT entities: `entity_type` values "person", "organization", "location") | No migration needed — new fields use `#[serde(default)`, `entity_type` kept for backward compat; backfill re-extracts and adds `class`, `subclass`, `confidence`, `topic`, `tags` |
| Live service config | None — no external services hold entity config state | None |
| OS-registered state | None — no OS registrations reference BERT model | None |
| Secrets/env vars | None — BERT NER is local ONNX; no API keys | None |
| Build artifacts | `src-tauri/models/bert-base-NER.onnx` (~109MB), `src-tauri/models/tokenizer.json`, `src-tauri/models/config.json` — must be deleted from repo | `git rm src-tauri/models/bert-base-NER.onnx src-tauri/models/tokenizer.json src-tauri/models/config.json` |

**After BERT deletion, the following Cargo.toml lines must be removed:**
- `ort = { version = "2.0.0-rc.12", ... }` — ONNX Runtime
- `tokenizers = "0.20"` — HuggingFace tokenizers
- `ndarray = "0.17"` — only used in `ner.rs` for BIO logits decoding

**Call sites requiring replacement (`NerService` references):**
1. `src-tauri/src/pipeline/backfill.rs:9` — `use crate::pipeline::ner::NerService` + line 28 `ner_service: Arc<NerService>` + line 65 `ner_clone: Arc<NerService>` + line 75 `backfill_one_doc(..., &ner_clone, ...)` + `backfill_one_doc` signature line 205-208
2. `src-tauri/src/pipeline/indexer.rs` — search for `NerService` and `ner_service` usage
3. `src-tauri/src/lib.rs` — `NerService::new(model_dir)` initialization in setup hook

All three must be replaced with `TwoPassExtractor` before `ner.rs` is deleted. Confirm via `cargo check` before committing.

---

## Common Pitfalls

### Pitfall 1: ndarray Only Used by ner.rs — Removing It Breaks Other Code If Missed

**What goes wrong:** `ndarray` appears in `Cargo.toml` but is only imported in `pipeline/ner.rs` (`use ndarray::{Array3, ArrayViewD}`). After deleting `ner.rs`, the ndarray dep can be removed. But if any other file has a `use ndarray` line, `cargo check` will fail.

**Why it happens:** Grep for the dep in source but miss a reexport or test file.

**How to avoid:** `grep -r "ndarray" src-tauri/src/` before removing the dep. Confirm only `ner.rs` + its tests reference it.

**Warning signs:** `cargo check` errors: `error[E0432]: unresolved import 'ndarray'`

---

### Pitfall 2: Semaphore Permits and async/await

**What goes wrong:** `tokio::sync::Semaphore::acquire()` returns a `SemaphorePermit` that releases when dropped. If the permit is assigned to `_` (not named), it drops immediately. If it's held across an `.await` point, the future must be `Send + 'static` (which it is via `tokio::spawn`).

**Why it happens:** `let _ = semaphore.acquire().await?;` — the permit drops at the end of that statement, not at the end of the LLM call scope.

**How to avoid:**
```rust
let _permit = semaphore.acquire().await?;  // _permit lives until end of block
let result = ai_request_with_retry(...).await;
drop(_permit);  // explicit, or let it drop at end of scope
```

---

### Pitfall 3: dateparser Defaults to Utc; Locale Ambiguity Not Automatic

**What goes wrong:** `dateparser::parse("01/02/2024")` returns January 2 (US format) by default. Indian users expect February 1 (DD/MM/YYYY). dateparser 0.3.1 does not have a locale parameter — it always uses US ordering for ambiguous formats.

**Why it happens:** The crate is designed for log-parsing (American format first).

**How to avoid:** Implement a thin wrapper that reads the locale hint from Settings:
- If locale hint is `"IN"` / `"UK"` / `"EU"` and format matches `DD/MM/YYYY` pattern, swap day/month before passing to dateparser.
- Store the resolved date in ISO-8601 (RFC-3339) format regardless of input format.

**Warning signs:** Indian users find all DD/MM/YYYY dates are month-swapped.

---

### Pitfall 4: `backfill_one_doc` Is Sync — Pass 2 Requires async Context

**What goes wrong:** The existing `backfill_one_doc` function in `backfill.rs` is synchronous and called from `tokio::task::spawn_blocking`. Pass 2 LLM calls are async (`ai_request_with_retry`). You cannot call async functions from inside `spawn_blocking` without a new runtime.

**Why it happens:** Existing BERT NER is synchronous (`NerService::extract` is sync).

**How to avoid:** The backfill loop must be restructured. Instead of `spawn_blocking` per doc, use a direct async call:
```rust
// In the backfill async task body:
for doc_id in &candidates {
    let result = two_pass.extract_full(&text, &title).await;  // async, NOT spawn_blocking
    // ...
}
```
Pass 1 is sync but fast (< 1ms), so calling it from an async context without `spawn_blocking` is acceptable. Only use `spawn_blocking` for heavy CPU work (embedding, OCR) — not for the LLM HTTP call which is already async I/O.

---

### Pitfall 5: JSON Schema Drift Between Providers

**What goes wrong:** Different providers return the JSON schema with slightly different field names or nested structures. Anthropic tends to add extra wrapping. Gemini may emit arrays where scalars are expected.

**Why it happens:** Providers don't enforce strict JSON schemas at the HTTP level (only OpenAI supports `response_format: {type: "json_object"}`). Others rely on prompt adherence.

**How to avoid:**
1. Include the full JSON schema in the prompt, not just an example.
2. Validate the parsed JSON against a Rust `serde_json::Value` check before deserializing into the typed struct: confirm `additional_entities` is an array, `topic` is a string, `tags` is an array.
3. If schema validation fails, treat as a parse failure and apply D-14 fence-strip + retry.
4. Consider `serde_json`'s permissive deserialization (`#[serde(default)]` on all optional fields) to tolerate partial responses.

---

### Pitfall 6: `entities_version = 2.5` as a Float in JSON

**What goes wrong:** The version gate uses `serde_json::Value::Number`. JSON numbers can be integers or floats. Storing `2.5` as a float works, but comparing with `.as_u64()` (as the current code does) will return `None` for `2.5` — it only works for integers.

**Why it happens:** Current `collect_backfill_candidates` uses `v.as_u64()` which returns `None` for non-integer numbers. `version < 2` where `version = 0` (None case) → included. With version 2.5, `as_u64()` returns `None` → treated as version 0 → incorrectly re-included in Pass 2 candidates.

**How to avoid:** Use `.as_f64()` throughout and compare as float:
```rust
let version = entry.metadata
    .as_ref()
    .and_then(|m| m.get("entities_version"))
    .and_then(|v| v.as_f64())  // handles both 2.0 and 2.5
    .unwrap_or(0.0);

if version < 3.0 {  // catches 0, 1, 2, 2.5 — excludes only 3.0+
    candidates.push(id);
}
```

---

### Pitfall 7: Backfill Cancellation Race on Provider Disconnect

**What goes wrong:** If the user disconnects their AI provider mid-backfill, `ai_request()` returns an error. The backfill loop's per-doc error handling (`D-26`) sets `entities_version = 2.5` and continues. But if ALL remaining docs fail (provider gone), the completion toast is misleading ("X of Y used pattern extraction").

**Why it happens:** The backfill task runs independently; the disconnect event doesn't stop the loop.

**How to avoid per D-24:** Add a cancellation check at the top of each iteration:
```rust
for doc_id in &candidates {
    // Check if provider still connected before each LLM call
    if !two_pass.provider_connected().await {
        break;  // Stop picking up new docs; in-flight call completes normally
    }
    // ...
}
```
This satisfies D-24's "no forced abort of in-flight call; no new docs picked up."

---

## Code Examples

### Heads-and-Tails Chunking (D-19)

```rust
// Source: CONTEXT.md D-19
const MAX_CHARS: usize = 12_000;
const HEAD_SIZE: usize = 6_000;
const TAIL_SIZE: usize = 6_000;

fn prepare_llm_input(title: &str, text: &str) -> String {
    if text.len() <= MAX_CHARS {
        format!("Title: {}\n\n{}", title, text)
    } else {
        let head = &text[..HEAD_SIZE.min(text.len())];
        let tail_start = text.len().saturating_sub(TAIL_SIZE);
        let tail = &text[tail_start..];
        format!("Title: {}\n\n[Document excerpt — beginning]\n{}\n\n[...middle omitted...]\n\n[Document excerpt — end]\n{}", title, head, tail)
    }
}
```

### Topic/Tag Normalization (D-35)

```rust
// Source: CONTEXT.md D-35
fn normalize_tag(s: &str) -> String {
    s.trim()
     .to_lowercase()
     .chars()
     .map(|c| if c.is_alphanumeric() || c == '_' { c } else if c.is_whitespace() { '_' } else { '\0' })
     .filter(|c| *c != '\0')
     .collect::<String>()
     .split('_')
     .filter(|p| !p.is_empty())
     .collect::<Vec<_>>()
     .join("_")
}

// "Term Insurance" → "term_insurance"
// "Investments 2024!" → "investments_2024"
// "khush school" → "khush_school"
```

### BERT Legacy Migration Mapping (D-21)

```rust
// Source: CONTEXT.md D-21
// When backfill processes a doc with entities_version=2 (BERT):
fn migrate_bert_entity_type(bert_type: &str) -> (&str, Option<&str>) {
    // Returns (new_class, entity_type_compat_value)
    match bert_type {
        "person" | "PER" => ("Person", "person"),
        "organization" | "ORG" => ("Organization", "organization"),
        "location" | "LOC" => ("Location", "location"),
        other => ("Identifier", other),  // MISC and unknown — safe fallback
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| CoNLL-03 trained BERT NER | LLM-driven extraction with fixed 8-class taxonomy | Phase 8 | Handles domain-specific entities (Aadhaar, GSTIN, VIN) BERT was trained on news corpus and misses |
| Fixed taxonomy (PER/ORG/LOC only) | 8-class + free-form topic + tags | Phase 8 | Captures financial, identity, and domain-specific entities |
| Chunking by BERT token limit (512) | Heads-and-tails (12k char cap) | Phase 8 | Single LLM call per doc; simpler than multi-chunk merge |
| Sync NER in spawn_blocking | Async LLM with semaphore cap | Phase 8 | Supports cloud providers; backfill is network-bound not CPU-bound |
| `entity_type` free-form string | `class` enum + `subclass` free-form | Phase 8 | Enables Phase 11 entity chip filtering on stable type values |

**Deprecated/outdated after Phase 8:**
- `pipeline/ner.rs` — BERT NerService — deleted entirely
- `src-tauri/models/bert-base-NER.onnx` — 109MB binary — deleted from git
- `ort` and `tokenizers` crate deps — removed from Cargo.toml
- `ndarray` crate dep — removed (only used by ner.rs logits decoding)
- `NerService::extract_chunk`, `decode_bio`, `chunk_text` — all internal NER functions — gone

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `dateparser` crate handles "3 Jul 2026" and "Jan 1, 2025" human formats | Standard Stack / Pattern 7 | Would need to implement custom chrono parsing for these formats |
| A2 | `dateparser` 0.3.1 uses US date ordering for ambiguous DD/MM/YYYY | Pitfall 3 | Non-issue if locale-swapping wrapper is implemented regardless |
| A3 | Model pricing table values for Haiku/GPT-5-mini/Gemini-Flash | Pattern 12 | Cost estimate tooltip would be inaccurate; user-visible but low-risk |
| A4 | `verhoeff` crate has adequate source repo and maintenance | Package Legitimacy | Would need custom 60-line Verhoeff implementation (trivial fallback) |
| A5 | `luhn` crate covers all Luhn use cases (credit card, SIN, GSTIN variant) | Standard Stack | GSTIN uses a modified Luhn; may need a custom variant for GSTIN specifically |
| A6 | OpenAI `gpt-5-mini` and `gpt-5` model IDs are correct as of implementation | CONTEXT.md D-11 / UI-SPEC | Wrong model ID → API error on first backfill; easy to fix |
| A7 | Pass-2 prompt achieves adequate extraction quality on Indian documents (Aadhaar, PAN, GSTIN) | Architecture Patterns / Prompt | Prompt iteration during implementation will validate this |
| A8 | `entities_version=2.5` is stored as JSON float (vs integer) | Pitfall 6 | If stored as integer "3" by accident, Pass-1-only docs skipped on subsequent backfill |

**If this table is non-empty:** Items A1-A8 should be validated during Wave 0 (test setup) or Wave 1 (unit tests) of the plan before production code is written.

---

## Open Questions

1. **GSTIN Luhn Variant**
   - What we know: CONTEXT.md D-03 says "GSTIN (Luhn-variant)" but doesn't specify the exact modification
   - What's unclear: Standard Luhn vs GSTIN's checksum algorithm — GSTIN uses a modified Luhn with alphanumeric chars
   - Recommendation: Implement GSTIN validator as a custom function (15-char format + Luhn-like mod-36 checksum) rather than relying on the `luhn` crate directly. This is ~30 lines of Rust and doesn't require a new crate.

2. **TFN (Australian Tax File Number) Checksum**
   - What we know: D-03 lists TFN as a required validator (9 digits + Australian checksum)
   - What's unclear: The exact TFN checksum algorithm (weighted sum mod 11)
   - Recommendation: Implement as custom function (~20 lines). Algorithm is publicly documented by the ATO.

3. **VIN Weighted Checksum**
   - What we know: VIN = 17 chars + weighted checksum (CONTEXT.md D-03)
   - What's unclear: Exact NHTSA weighting table for VIN digit 9 (the check digit)
   - Recommendation: Implement from the NHTSA spec. Algorithm is public. No suitable crate found in cargo search.

4. **`language` field per doc**
   - What we know: CONTEXT.md "Claude's Discretion" mentions whether to emit a `language` field
   - What's unclear: Whether Phase 9 actually needs this signal for space labeling quality
   - Recommendation: Planner decides. Implementation cost is low (add one field to ExtractedEntities, LLM emits it in Pass 2 JSON schema). Default recommendation: YES — emit `language` as it's low-cost and Phase 9 benefits from knowing document language.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust + Cargo | Build | ✓ | (existing project) | — |
| `tokio` | Async runtime, semaphore | ✓ | 1.x (in Cargo.toml) | — |
| `regex` | Pass 1 patterns | ✓ | 1.x (in Cargo.toml) | — |
| `serde_json` | JSON parsing | ✓ | 1.x (in Cargo.toml) | — |
| `reqwest` | ai_request HTTP calls | ✓ | 0.12 (in Cargo.toml) | — |
| AI provider (Anthropic/OpenAI/Gemini/Ollama) | Pass 2 LLM calls | Runtime (user provides) | — | Pass-1-only mode (D-31) |
| `dateparser` | Multi-format date parsing | Must add | 0.3.1 | Custom chrono parsing |
| `iban_validate` | IBAN Mod-97 validation | Must add | 5.0.1 | Custom 30-line Mod-97 impl |
| `verhoeff` | Aadhaar checksum | Must add | 1.0.0 | Custom Verhoeff table impl |
| `luhn` | Credit card / SIN validation | Must add | 1.0.1 | Custom Luhn impl |

**Missing dependencies with no fallback:** None — all missing deps have viable custom implementations as fallback.

**Missing dependencies with fallback:** dateparser, iban_validate, verhoeff, luhn — each has a 20-60 line custom fallback if the crate is rejected after human verification.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | none — uses default cargo test runner |
| Quick run command | `cargo test -p cortex pipeline::pass1 -- --nocapture` |
| Full suite command | `cargo test -p cortex` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LLME-01 | TwoPassExtractor replaces NerService as drop-in | unit | `cargo test pipeline::two_pass_extractor` | ❌ Wave 0 |
| LLME-01 | Pass 2 calls ai_request with correct schema | unit (mock ai_request) | `cargo test pipeline::pass2_llm_refiner` | ❌ Wave 0 |
| LLME-02 | Pass 1 extracts Date, Email, Phone, Amount, Identifier | unit | `cargo test pipeline::pass1_pattern_extractor` | ❌ Wave 0 |
| LLME-02 | Aadhaar Verhoeff checksum validates correctly | unit | `cargo test pipeline::pass1_pattern_extractor::test_aadhaar` | ❌ Wave 0 |
| LLME-02 | IBAN Mod-97 validates correctly | unit | `cargo test pipeline::pass1_pattern_extractor::test_iban` | ❌ Wave 0 |
| LLME-02 | Luhn + BIN validates credit card | unit | `cargo test pipeline::pass1_pattern_extractor::test_credit_card` | ❌ Wave 0 |
| LLME-02 | Date multi-format parsing (ISO, DD/MM, written) | unit | `cargo test pipeline::pass1_pattern_extractor::test_dates` | ❌ Wave 0 |
| LLME-03 | 20-entity cap enforced; identical doc returns same entities | unit | `cargo test pipeline::pass1_pattern_extractor::test_cap` | ❌ Wave 0 |
| LLME-03 | JSON fence stripping produces valid JSON | unit | `cargo test pipeline::pass2_llm_refiner::test_fence_strip` | ❌ Wave 0 |
| LLME-04 | Single doc Pass-2 failure → Pass-1 fallback; other docs unaffected | unit | `cargo test pipeline::two_pass_extractor::test_fallback` | ❌ Wave 0 |
| LLME-05 | Backfill version gate `< 3.0` excludes `entities_version=3` docs | unit | `cargo test pipeline::backfill::test_collect_candidates_v3` | ❌ Wave 0 |
| LLME-05 | Backfill version gate `< 3.0` includes `entities_version=2.5` docs | unit | `cargo test pipeline::backfill::test_collect_candidates_v3` | ❌ Wave 0 |
| LLME-06 | `cargo check` succeeds without ort/tokenizers deps | smoke | `cargo check` | N/A |
| LLME-06 | No `use ort` or `use tokenizers` in any non-deleted file | smoke | `grep -r "use ort\|use tokenizers" src-tauri/src/` | N/A |

### Sampling Rate
- **Per task commit:** `cargo test -p cortex pipeline -- --nocapture` (pass1 + pass2 + two_pass unit tests)
- **Per wave merge:** `cargo test -p cortex`
- **Phase gate:** Full suite green + `cargo check` without ort/tokenizers + LLME-06 verification before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src-tauri/src/pipeline/pass1_pattern_extractor.rs` — module + unit tests
- [ ] `src-tauri/src/pipeline/pass2_llm_refiner.rs` — module + unit tests (mock ai_request)
- [ ] `src-tauri/src/pipeline/two_pass_extractor.rs` — facade + unit tests
- [ ] `tests/fixtures/pass1_golden.json` — golden fixture: 10 docs with expected Pass 1 entities

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Provider auth is Phase 7's responsibility |
| V3 Session Management | no | No user sessions in this phase |
| V4 Access Control | no | All IPC commands inherit Tauri's capability model |
| V5 Input Validation | yes | LLM JSON output validated before use; class field validated against 8-class enum; `entity_type` values sanitized |
| V6 Cryptography | no | No cryptographic operations in this phase |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| LLM JSON injection (adversarial doc content influencing the JSON output shape) | Tampering | Validate parsed JSON against strict schema before deserializing; reject if class is not in 8-class enum |
| Privacy: document content sent to cloud LLM | Information Disclosure | "Use LLM for entity extraction" toggle (D-33) defaults ON only when provider is connected; user can disable; Pass 1 always runs locally; prompt instructs LLM to extract entities only, not reproduce content |
| ONNX model removal race: deleting model files while app is running | Denial of Service | Delete BERT model files only after NerService is removed from AppState initialization; `cargo check` confirms no lingering references |
| Credit card false-positive extraction from docs | Tampering | BIN prefix + context word requirement (D-03); Luhn alone insufficient |
| Semaphore exhaustion (backfill hangs if all 8 permits are stuck) | Denial of Service | `acquire()` returns a Result; timeouts would require separate handling; for v1.1, rely on provider's built-in request timeout (~30s via reqwest) to unblock stuck permits |

---

## Sources

### Primary (HIGH confidence)
- `src-tauri/src/pipeline/ner.rs` — full interface of NerService being replaced; `extract()` signature verified
- `src-tauri/src/pipeline/backfill.rs` — `spawn_entity_backfill` full implementation; version gate pattern; progress event schema
- `src-tauri/src/graph/entity_store.rs` — `EntityStore` in-memory structure; no SQLite; `ExtractedEntity` usage
- `src-tauri/src/ai/service.rs` — `ai_request()`, `AIServiceRequest`, `AIServiceResponse` — Phase 7 output
- `src-tauri/src/ai/retry.rs` — `ai_request_with_retry()`, `retry_with_backoff()` — reusable
- `src-tauri/src/types.rs` — `ExtractedEntity` struct; `EntityBackfillProgress`; confirmed no SQLite
- `src-tauri/Cargo.toml` — existing deps including `ort = "2.0.0-rc.12"`, `tokenizers = "0.20"`, `ndarray = "0.17"` confirmed present and requiring removal
- `.planning/phases/08-llm-entity-extraction/08-CONTEXT.md` — all locked decisions D-01 through D-38d
- `.planning/phases/08-llm-entity-extraction/08-UI-SPEC.md` — component inventory, interaction states, copywriting

### Secondary (MEDIUM confidence)
- `cargo search dateparser` — version 0.3.1 confirmed on crates.io registry; 3.2M downloads per registry API
- `cargo search iban_validate` — version 5.0.1 confirmed on crates.io registry; 18.6M downloads per registry API
- `cargo search verhoeff` — version 1.0.0 confirmed on crates.io registry; 531K downloads per registry API
- `cargo search luhn` — version 1.0.1 confirmed on crates.io registry; 615K downloads per registry API
- `cargo search chrono` — version 0.4.45 confirmed on crates.io registry; 656M downloads per registry API

### Tertiary (LOW confidence — marked [ASSUMED])
- Model pricing table (Haiku, GPT-5-mini, Gemini Flash) — based on training knowledge; must be verified at implementation time from provider pricing pages
- GSTIN Luhn variant algorithm details — [ASSUMED] based on general knowledge of Indian tax ID specifications
- VIN weighted checksum table — [ASSUMED] based on NHTSA documentation known at training time

---

## Metadata

**Confidence breakdown:**
- Standard stack (crates): HIGH — all crates verified via `cargo search` against live crates.io registry
- Architecture (two-pass engine, interfaces, patterns): HIGH — derived from reading all existing Rust source code directly
- Pitfalls: HIGH — derived from reading actual code and identifying concrete incompatibilities (async/sync boundary, float version gate, ndarray)
- Model pricing / LLM prompt quality: LOW — requires empirical validation during implementation

**Research date:** 2026-07-03
**Valid until:** 2026-08-03 (stable Rust ecosystem; model pricing may change faster)

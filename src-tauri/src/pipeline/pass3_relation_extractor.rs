//! Pass 3 — Relation Extraction Pass (Phase 11.5, Plan 03)
//!
//! Given document text + title + Pass 2 extracted entities, calls the active AI
//! provider once with the fixed 21-predicate closed vocabulary prompt and returns
//! `Vec<Triple>` — (subject, predicate, object) relations grounded in the document
//! text and referencing only entities from the Pass 2 entity list.
//!
//! Mirrors `Pass2LlmRefiner` (`pipeline::pass2_llm_refiner`) architecture exactly:
//! semaphore-capped concurrency, provider-absent short-circuit, heads-and-tails
//! chunking, JSON fence stripping + two-attempt parse, and closed-vocabulary
//! enforcement at parse time. Fence-stripping is reused directly from Pass 2 —
//! not reimplemented — to avoid fence-strip drift between passes (D-06).
//!
//! When no provider is connected, `extract()` returns `Ok(None)` — never errors
//! (D-04, mirror of Pass 2's D-31).
//!
//! Design decisions: D-01..D-08, D-20..D-22 from 11.5-CONTEXT.md.

use std::sync::Arc;
use std::sync::OnceLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use crate::auth::AuthState;
use crate::error::AppError;
use crate::types::{Triple, ExtractedEntity, CanonicalEntity, SEED_PREDICATES};
use crate::ai::retry::ai_request_with_retry;
use crate::ai::service::{AIServiceRequest, ServiceMessage};
use crate::pipeline::pass2_llm_refiner::{strip_json_fences, Pass2LlmRefiner};

// ─── Prompt version ───────────────────────────────────────────────────────────
/// Semantic version of the EXTRACT_RELATIONS_PROMPT. Stored alongside output so
/// future re-extract logic can detect prompt drift (mirror of Pass 2 PROMPT_VERSION).
pub const PROMPT_VERSION: &str = "v1";

// ─── Chunking constants (D-19 pattern reused) ─────────────────────────────────
const MAX_CHARS: usize = 12_000;
const HEAD_SIZE: usize = 6_000;
const TAIL_SIZE: usize = 6_000;

// ─── EXTRACT_RELATIONS_PROMPT (D-01, D-02, D-05; Phase 11.6 D-05 dynamic vocab) ─
/// Prompt prefix — rule 1 (schema enforcement lock). Immutable across calls.
pub const EXTRACT_RELATIONS_PROMPT_PREFIX: &str = r#"You extract relations (triples) from a document.

RULES:
1. Output ONLY valid JSON matching the schema below. No markdown fences, no explanation, no preamble.
"#;

/// Prompt middle template — rules 2-3 (vocabulary lock + new_predicates escape
/// hatch). The `{VOCAB}` placeholder is replaced with the runtime-effective
/// predicate vocabulary (`OntologyStore.effective_predicate_names()`) at build
/// time via `build_extract_relations_prompt` (Phase 11.6 D-05).
pub const EXTRACT_RELATIONS_PROMPT_VOCAB_TEMPLATE: &str = r#"2. Predicates are FIXED. Choose ONLY from this vocabulary: {VOCAB}.
3. If a relation doesn't fit any predicate, emit `mentioned_with` (weak default) or omit the triple entirely. NEVER invent new predicates in the `triples` array. If you observe a genuinely new relation type not in the vocabulary above, propose it in `new_predicates` (at most 3 per document).
"#;

/// Prompt suffix — rules 4-8, JSON schema, few-shot examples, trailer.
/// Immutable across calls (only the vocabulary in the middle template varies).
///
/// Includes:
/// - Rules 4-8 (entity id grounding, text-support requirement, directional
///   preference, OCR tolerance, empty-array allowance)
/// - Fixed JSON schema with camelCase field names, including the OPTIONAL
///   `newPredicates` escape hatch (Phase 11.6 D-05)
/// - 4 multi-region few-shot examples (property sale deed, vehicle registration,
///   identity doc, empty case) — kept verbatim from Phase 11.5
pub const EXTRACT_RELATIONS_PROMPT_SUFFIX: &str = r#"4. Subject and object IDs MUST reference the entity list (e.g. e_0, e_1) — do NOT invent free-form names.
5. Only emit triples the document TEXT actually supports. No speculation.
6. Prefer directional predicates (A owns B) over symmetric (A mentioned_with B). Do not emit both directions — the store auto-inverts owns/owned_by, etc.
7. The document may be OCR'd; tolerate typos but do not fabricate relationships to fix them.
8. Output an empty triples array [] when no relations are clearly supported.

JSON SCHEMA (output ONLY this JSON, nothing else):
{ "triples": [ { "subjectId": "e_N", "predicate": "owns", "objectId": "e_M" } ], "newPredicates": [ { "name": "snake_case", "description": "when to use", "subjectClass": "Person", "objectClass": "Location" } ] }

The `newPredicates` array is OPTIONAL. Include it ONLY when you observe a relation not covered by the vocabulary above AND you are confident it recurs across the user's corpus. Max 3 entries per document. Names must be snake_case, ≤ 40 chars.

FEW-SHOT EXAMPLES:

Example A: Property sale deed
Entities: e_0 Person "Jane Doe", e_1 Location "Sunset Towers", e_2 Location "Metroville", e_3 Date "2024-04-11"
Text excerpt: "This Sale Deed dated 11 April 2024 confirms that Jane Doe has purchased Sunset Towers located in Metroville from John Roe."
→ {"triples":[{"subjectId":"e_0","predicate":"owns","objectId":"e_1"},{"subjectId":"e_1","predicate":"located_in","objectId":"e_2"},{"subjectId":"e_0","predicate":"purchased_from","objectId":"e_4"},{"subjectId":"e_0","predicate":"dated","objectId":"e_3"}]}

Example B: Vehicle registration
Entities: e_0 Person "John Roe", e_1 Location "Unit 204", e_2 Identifier "MH12AB1234"
Text: "Owner: John Roe. Vehicle: Unit 204. Registration No: MH12AB1234."
→ {"triples":[{"subjectId":"e_0","predicate":"owns","objectId":"e_1"},{"subjectId":"e_1","predicate":"has_voter_id","objectId":"e_2"}]}

Example C: Identity doc
Entities: e_0 Person "Sam Roe", e_1 Identifier "ABCDE1234F"
Text: "Name: Sam Roe. PAN: ABCDE1234F."
→ {"triples":[{"subjectId":"e_0","predicate":"uses_pan","objectId":"e_1"}]}

Example D: Empty case
Entities: e_0 Amount "$500"
Text: "Miscellaneous receipt total $500."
→ {"triples":[]}

OUTPUT ONLY THE JSON — NO ADDITIONAL TEXT."#;

/// Build the full Pass 3 prompt with `vocab` injected into RULES §2 (Phase
/// 11.6 D-05). Vocabulary is emitted as a comma-separated list in insertion
/// order — matches `OntologyStore.effective_predicate_names()` order, which
/// is deterministic (Seed -> Corpus -> Manual -> Adaptive).
pub fn build_extract_relations_prompt(vocab: &[String]) -> String {
    let vocab_str = vocab.join(", ");
    let middle = EXTRACT_RELATIONS_PROMPT_VOCAB_TEMPLATE.replace("{VOCAB}", &vocab_str);
    format!(
        "{}{}{}",
        EXTRACT_RELATIONS_PROMPT_PREFIX, middle, EXTRACT_RELATIONS_PROMPT_SUFFIX
    )
}

/// Deprecated Phase 11.5 fixed-vocabulary prompt. Kept only so the legacy
/// `test_extract_relations_prompt_contains_all_predicates` test still
/// compiles/runs; new code must call `build_extract_relations_prompt` with
/// the runtime vocabulary from `OntologyStore.effective_predicate_names()`.
#[deprecated(note = "use build_extract_relations_prompt with SEED_PREDICATES for the equivalent output")]
pub fn extract_relations_prompt_seed_only() -> String {
    let vocab: Vec<String> = SEED_PREDICATES.iter().map(|s| s.to_string()).collect();
    build_extract_relations_prompt(&vocab)
}

// ─── Output types ─────────────────────────────────────────────────────────────

/// A single (subject, predicate, object) triple as emitted by the LLM, prior to
/// validation. `subject_id`/`object_id` are `e_N` references into the Pass 2
/// entity list — NOT canonical entity ids.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pass3Triple {
    /// "e_N" reference into the Pass 2 entity list.
    pub subject_id: String,
    /// Must be a member of `PREDICATE_VOCABULARY` (enforced by `validate_and_normalize`).
    pub predicate: String,
    /// "e_M" reference into the Pass 2 entity list.
    pub object_id: String,
}

/// A candidate new predicate proposed by the LLM (Phase 11.6 D-05), prior to
/// validation. `subject_class`/`object_class` are optional hints forwarded to
/// `OntologyStore.record_pending_predicate`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pass3NewPredicate {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub subject_class: Option<String>,
    #[serde(default)]
    pub object_class: Option<String>,
}

/// Full structured output from one Pass 3 LLM call.
///
/// `triples` uses `#[serde(default)]` so partial provider responses (missing
/// key, or `{}`) still deserialize (mirror of Pass 2 Pitfall 5). `new_predicates`
/// also defaults to empty so legacy Phase 11.5 recorded responses (no
/// `newPredicates` key) still parse (Phase 11.6 D-05).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Pass3Output {
    #[serde(default)]
    pub triples: Vec<Pass3Triple>,
    #[serde(default)]
    pub new_predicates: Vec<Pass3NewPredicate>,
}

// ─── JSON parsing (D-06 — reuses Pass 2's strip_json_fences) ──────────────────

/// Parse LLM JSON output into `Pass3Output` with two-attempt robustness (D-06).
///
/// Attempt 1: Direct `serde_json::from_str` on the raw string.
/// Attempt 2: Strip markdown fences with `strip_json_fences` (reused from Pass 2,
/// NOT reimplemented), then retry.
/// On second failure: returns `AppError::Internal("Pass 3 JSON parse failed: ...")`.
/// Never retries the LLM call — that is the caller's concern.
pub fn parse_pass3_json(raw: &str) -> Result<Pass3Output, AppError> {
    // Attempt 1: direct parse (fast path — well-behaved providers)
    if let Ok(out) = serde_json::from_str::<Pass3Output>(raw) {
        return Ok(out);
    }

    // Attempt 2: strip fences and retry
    let stripped = strip_json_fences(raw);
    serde_json::from_str::<Pass3Output>(&stripped)
        .map_err(|e| AppError::Internal(format!("Pass 3 JSON parse failed: {}", e)))
}

/// Lazily-compiled snake_case predicate name regex: `^[a-z][a-z0-9_]*$`.
/// Mirrors the validation used by `OntologyStore::register_manual_predicate`
/// and `ontology_bootstrap::validate_bootstrap` (Phase 11.6).
fn new_predicate_name_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-z][a-z0-9_]*$").expect("new_predicate_name_regex must compile"))
}

const NEW_PREDICATE_MAX_PER_DOC: usize = 3;
const NEW_PREDICATE_MAX_NAME_LEN: usize = 40;

// ─── Validation + normalization (D-02, closed vocabulary lock; Phase 11.6 D-05) ─

/// Enforce the runtime-effective predicate vocabulary and entity-id grounding
/// on Pass 3 output (T-11.5-09, T-11.5-10, T-11.5-13), plus new_predicates
/// validation (T-11.6-17, T-11.6-18).
///
/// Drops triples (with a logged warning, never an error):
/// - Triples whose `predicate` is not in `vocabulary`.
/// - Self-referential triples (`subject_id == object_id`).
/// - Triples whose `subject_id`/`object_id` don't parse as `e_N` or whose index
///   is out of bounds for `pass2_entities`.
///
/// Drops new_predicates entries (Phase 11.6 D-05), in this order:
/// 1. Empty name, name > 40 chars, or name not matching `^[a-z][a-z0-9_]*$`.
/// 2. Name already present in `vocabulary` (LLM re-proposing an existing
///    predicate — treated as noise, T-11.6-18).
/// 3. Duplicate names within the same batch (first occurrence wins).
/// 4. Beyond the first `NEW_PREDICATE_MAX_PER_DOC` (3) surviving entries —
///    applied LAST so a valid predicate is never bumped out of the cap by an
///    invalid one earlier in the LLM's list (T-11.6-17).
pub fn validate_and_normalize(
    mut out: Pass3Output,
    pass2_entities: &[ExtractedEntity],
    vocabulary: &[String],
) -> Pass3Output {
    out.triples.retain(|t| {
        if !vocabulary.iter().any(|v| v == &t.predicate) {
            eprintln!(
                "[pass3] dropping unknown predicate '{}' (subject={}, object={})",
                t.predicate, t.subject_id, t.object_id
            );
            return false;
        }

        if t.subject_id == t.object_id {
            eprintln!(
                "[pass3] dropping self-referential triple: {} --{}--> {}",
                t.subject_id, t.predicate, t.object_id
            );
            return false;
        }

        let subject_ok = parse_entity_index(&t.subject_id)
            .map(|idx| idx < pass2_entities.len())
            .unwrap_or(false);
        if !subject_ok {
            eprintln!(
                "[pass3] dropping triple with unresolvable subjectId '{}' (only {} entities)",
                t.subject_id, pass2_entities.len()
            );
            return false;
        }

        let object_ok = parse_entity_index(&t.object_id)
            .map(|idx| idx < pass2_entities.len())
            .unwrap_or(false);
        if !object_ok {
            eprintln!(
                "[pass3] dropping triple with unresolvable objectId '{}' (only {} entities)",
                t.object_id, pass2_entities.len()
            );
            return false;
        }

        true
    });

    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    out.new_predicates.retain(|p| {
        if p.name.is_empty() || p.name.len() > NEW_PREDICATE_MAX_NAME_LEN {
            eprintln!("[pass3] dropping new_predicate with invalid length: '{}'", p.name);
            return false;
        }
        if !new_predicate_name_regex().is_match(&p.name) {
            eprintln!("[pass3] dropping new_predicate with non-snake_case name: '{}'", p.name);
            return false;
        }
        if vocabulary.iter().any(|v| v == &p.name) {
            eprintln!(
                "[pass3] dropping new_predicate '{}' already present in vocabulary",
                p.name
            );
            return false;
        }
        if !seen_names.insert(p.name.clone()) {
            eprintln!("[pass3] dropping duplicate new_predicate within batch: '{}'", p.name);
            return false;
        }
        true
    });

    // Cap at NEW_PREDICATE_MAX_PER_DOC LAST (D-05: "at most 3 per doc") so a
    // valid predicate later in the LLM's list is never bumped out by an
    // earlier invalid/duplicate/already-in-vocab entry (T-11.6-17).
    if out.new_predicates.len() > NEW_PREDICATE_MAX_PER_DOC {
        eprintln!(
            "[pass3] dropping {} new_predicates beyond the {}-per-doc cap",
            out.new_predicates.len() - NEW_PREDICATE_MAX_PER_DOC,
            NEW_PREDICATE_MAX_PER_DOC
        );
        out.new_predicates.truncate(NEW_PREDICATE_MAX_PER_DOC);
    }

    out
}

/// Parse an `e_N` reference into its numeric index. Returns `None` if the
/// string doesn't start with `e_` or the suffix isn't a valid `usize`.
fn parse_entity_index(id: &str) -> Option<usize> {
    id.strip_prefix("e_").and_then(|n| n.parse::<usize>().ok())
}

// ─── Pass3RelationExtractor ────────────────────────────────────────────────────

/// LLM relation extraction pass.
///
/// Wraps the `ai_request_with_retry` HTTP call with:
/// - Semaphore-based concurrency cap of 8 in-flight requests (D-08)
/// - Provider-absent short-circuit (returns `Ok(None)`, D-04)
/// - Heads-and-tails chunking for long documents (D-19 pattern reused)
/// - JSON fence stripping + two-attempt parse (D-06, reused from Pass 2)
/// - 21-predicate closed-vocabulary enforcement (D-02)
pub struct Pass3RelationExtractor {
    /// Active AI provider credential store (shared with the rest of the app).
    auth: Arc<AuthState>,
    /// Semaphore limiting in-flight LLM requests to `cap` (default 8, D-08).
    semaphore: Arc<tokio::sync::Semaphore>,
    /// User-configured extraction model (from `Settings.extraction_model`).
    /// Empty string = "use provider default at request time" (D-07 — same
    /// model as Pass 2, no new settings).
    configured_model: Arc<tokio::sync::RwLock<String>>,
}

impl Pass3RelationExtractor {
    /// Create a new extractor with the default concurrency cap of 8 (D-08).
    pub fn new(auth: Arc<AuthState>) -> Self {
        Self {
            auth,
            semaphore: Arc::new(tokio::sync::Semaphore::new(1)), // D-08: reduced 3→1 to keep UI responsive during backfill
            configured_model: Arc::new(tokio::sync::RwLock::new(String::new())),
        }
    }

    /// Create a new extractor with a custom semaphore capacity.
    /// Use `new_with_capacity(auth, 2)` in tests to verify concurrency behavior.
    pub fn new_with_capacity(auth: Arc<AuthState>, cap: usize) -> Self {
        Self {
            auth,
            semaphore: Arc::new(tokio::sync::Semaphore::new(cap)),
            configured_model: Arc::new(tokio::sync::RwLock::new(String::new())),
        }
    }

    /// Read the currently configured extraction model.
    pub async fn model(&self) -> String {
        self.configured_model.read().await.clone()
    }

    /// Set the extraction model override (called when `Settings.extraction_model` changes).
    pub async fn set_model(&self, model: String) {
        *self.configured_model.write().await = model;
    }

    /// Expose the semaphore for testing concurrency limits.
    pub fn semaphore(&self) -> Arc<tokio::sync::Semaphore> {
        self.semaphore.clone()
    }

    /// Build the LLM user message from document title + text + Pass 2 entity list.
    ///
    /// Applies heads-and-tails chunking for documents > 12 000 chars (mirror of
    /// Pass 2's `prepare_llm_input`): sends `title + first 6k + last 6k` to bound
    /// LLM input tokens and cost. Appends a compact "Entities in this doc" list,
    /// one `e_{N}: {value} (class={class}, subclass={subclass})` line per entity.
    fn prepare_llm_input(
        title: &str,
        text: &str,
        pass2_entities: &[ExtractedEntity],
        _subject_map: &[(String, String)],
    ) -> String {
        let text_chunk = if text.len() <= MAX_CHARS {
            format!("Title: {}\n\n{}", title, text)
        } else {
            let head_end = safe_byte_boundary(text, HEAD_SIZE);
            let tail_start = text.len().saturating_sub(TAIL_SIZE);
            let tail_start = safe_byte_boundary_from_end(text, tail_start);
            let head = &text[..head_end];
            let tail = &text[tail_start..];
            format!(
                "Title: {}\n\n[Document excerpt — beginning]\n{}\n\n[...middle omitted...]\n\n[Document excerpt — end]\n{}",
                title, head, tail
            )
        };

        let entity_list = if pass2_entities.is_empty() {
            "(none)".to_string()
        } else {
            pass2_entities
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let class_str = e.class.as_deref().unwrap_or("unknown");
                    let subclass_str = e.subclass.as_deref().unwrap_or("unknown");
                    format!("e_{}: {} (class={}, subclass={})", i, e.value, class_str, subclass_str)
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "{}\n\n--- Entities in this doc (reference for subjectId/objectId) ---\n{}",
            text_chunk, entity_list
        )
    }

    /// Full async relation extraction pipeline with dynamic vocabulary and
    /// new_predicates output (Phase 11.6 D-05).
    ///
    /// Steps (per D-04, D-05, D-06, D-07, D-08, Phase 11.6 D-05):
    ///  a. Short-circuit when `pass2_entities` is empty — nothing to relate.
    ///  b. Acquire semaphore permit (held across LLM await, mirrors Pass 2 Pitfall 2 fix).
    ///  c. Check active provider — return `Ok(None)` if none (D-04, mirror of D-31).
    ///  d. Choose model: configured > provider default; empty Ollama → return `Ok(None)` + warn.
    ///  e. Build user content: chunked text + compact entity list.
    ///  f. Build the prompt via `build_extract_relations_prompt(vocabulary)` and
    ///     construct `AIServiceRequest` with `temperature = 0.0` (idempotent extraction).
    ///  g. Call `ai_request_with_retry` with 3 attempts.
    ///  h. Parse JSON → validate + normalize (against `vocabulary`) → map `e_N`
    ///     references to canonical entity ids → return
    ///     `Ok(Some((Vec<Triple>, Vec<Pass3NewPredicate>)))` (triples may be
    ///     empty — signals "provider ran, extracted nothing", mirror of Pass 2's
    ///     WR-01 fix).
    pub async fn extract_full(
        &self,
        text: &str,
        title: &str,
        pass2_entities: &[ExtractedEntity],
        vocabulary: &[String],
    ) -> Result<Option<(Vec<Triple>, Vec<Pass3NewPredicate>)>, AppError> {
        // a. Nothing to relate without entities.
        if pass2_entities.is_empty() {
            eprintln!("[pass3] no pass2 entities, skipping");
            return Ok(None);
        }

        // b. Acquire semaphore permit.
        //    Use `acquire_owned` so the permit is bound to this scope and survives
        //    across the LLM `.await` point — critical to enforce D-08 (Pitfall 2).
        let _permit = self.semaphore.clone().acquire_owned().await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // c. Provider check: short-circuit when no provider is connected (D-04).
        let provider = match self.auth.get_active_provider()
            .map_err(AppError::Internal)?
        {
            Some(p) => p,
            None => return Ok(None),
        };

        // d. Skip-gate: same model-configured logic as Pass 2 (D-07 — same active
        //    provider + model, no new settings).
        {
            let cfg = self.model().await;
            if cfg.is_empty() {
                let default = Pass2LlmRefiner::pick_model_default(&provider);
                if default.is_empty() {
                    eprintln!(
                        "[pass3] provider='{}' has no extraction model configured \
                         (Settings.extraction_model is empty and no default exists); \
                         skipping Pass 3 for this document",
                        provider
                    );
                    return Ok(None);
                }
            }
        }

        // e. Build user content: chunked text + compact entity list.
        let user_content = Self::prepare_llm_input(title, text, pass2_entities, &[]);

        // f. Build the runtime-vocabulary prompt + construct AIServiceRequest.
        let system_prompt = build_extract_relations_prompt(vocabulary);
        let req = AIServiceRequest {
            system_prompt,
            messages: vec![ServiceMessage {
                role: "user".to_string(),
                content: user_content,
            }],
            max_tokens: Some(2048),
            temperature: Some(0.0), // temperature=0 for idempotent extraction
            response_format: None,
            model_override: None,
        };

        // g. Call with retry: 3 attempts, exponential backoff + jitter.
        let response = ai_request_with_retry(self.auth.as_ref(), req, 3)
            .await
            .map_err(|e| AppError::Internal(format!("Pass 3 LLM call failed: {}", e)))?;

        // h. Parse, validate, normalize, convert to Triple.
        let out = parse_pass3_json(&response.content)?;
        let out = validate_and_normalize(out, pass2_entities, vocabulary);

        let mut triples = Vec::with_capacity(out.triples.len());
        for t in out.triples {
            let subject_idx = match parse_entity_index(&t.subject_id) {
                Some(i) => i,
                None => continue, // already filtered by validate_and_normalize; defensive
            };
            let object_idx = match parse_entity_index(&t.object_id) {
                Some(i) => i,
                None => continue,
            };

            let subject_canonical = pass2_entities.get(subject_idx).and_then(|e| e.canonical_id.clone());
            let object_canonical = pass2_entities.get(object_idx).and_then(|e| e.canonical_id.clone());

            let (subject_id, object_id) = match (subject_canonical, object_canonical) {
                (Some(s), Some(o)) => (s, o),
                _ => {
                    eprintln!(
                        "[pass3] endpoint has no canonical_id, dropping triple: {} --{}--> {}",
                        t.subject_id, t.predicate, t.object_id
                    );
                    continue;
                }
            };

            triples.push(Triple {
                id: String::new(),        // assigned by TripleStore on insert
                subject_id,
                predicate: t.predicate,
                object_id,
                doc_ids: vec![],
                user_added: false,
                created_at: String::new(), // assigned by TripleStore on insert
            });
        }

        Ok(Some((triples, out.new_predicates)))
    }

    /// Backward-compat wrapper preserving the Phase 11.5 signature: fixed
    /// `SEED_PREDICATES` vocabulary, `new_predicates` dropped. Existing
    /// callers that don't need adaptive-vocab feedback can keep calling this.
    pub async fn extract(
        &self,
        text: &str,
        title: &str,
        pass2_entities: &[ExtractedEntity],
    ) -> Result<Option<Vec<Triple>>, AppError> {
        let vocab: Vec<String> = SEED_PREDICATES.iter().map(|s| s.to_string()).collect();
        Ok(self
            .extract_full(text, title, pass2_entities, &vocab)
            .await?
            .map(|(triples, _new_predicates)| triples))
    }
}

// ─── UTF-8 safe byte boundary helpers (mirror of Pass 2) ──────────────────────

/// Find the largest valid UTF-8 char boundary ≤ `max_bytes`.
fn safe_byte_boundary(text: &str, max_bytes: usize) -> usize {
    if max_bytes >= text.len() {
        return text.len();
    }
    let mut pos = max_bytes;
    while !text.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Find the smallest valid UTF-8 char boundary ≥ `start_bytes`.
fn safe_byte_boundary_from_end(text: &str, start_bytes: usize) -> usize {
    if start_bytes >= text.len() {
        return text.len();
    }
    let mut pos = start_bytes;
    while pos < text.len() && !text.is_char_boundary(pos) {
        pos += 1;
    }
    pos
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn sample_entity(class: &str, value: &str, canonical_id: Option<&str>) -> ExtractedEntity {
        ExtractedEntity {
            label: class.to_string(),
            value: value.to_string(),
            entity_type: class.to_lowercase(),
            canonical_id: canonical_id.map(|s| s.to_string()),
            class: Some(class.to_string()),
            subclass: None,
            canonical_short_name: None,
            confidence: Some(0.9),
        }
    }

    // ── Task 1 tests: types, JSON parsing, validation ──────────────────────

    #[test]
    fn test_parse_pass3_json_direct() {
        let raw = r#"{"triples":[{"subjectId":"e_0","predicate":"owns","objectId":"e_1"}]}"#;
        let result = parse_pass3_json(raw);
        assert!(result.is_ok(), "direct parse should succeed: {:?}", result);
        let out = result.unwrap();
        assert_eq!(out.triples.len(), 1);
        assert_eq!(out.triples[0].predicate, "owns");
    }

    #[test]
    fn test_parse_pass3_json_fence_stripped() {
        let raw = "```json\n{\"triples\":[{\"subjectId\":\"e_0\",\"predicate\":\"owns\",\"objectId\":\"e_1\"}]}\n```";
        let result = parse_pass3_json(raw);
        assert!(result.is_ok(), "fence-stripped parse should succeed: {:?}", result);
        assert_eq!(result.unwrap().triples.len(), 1);
    }

    #[test]
    fn test_parse_pass3_json_invalid_returns_err() {
        let raw = "not json at all";
        let result = parse_pass3_json(raw);
        assert!(result.is_err(), "invalid JSON should return Err");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Pass 3 JSON parse failed"),
            "error must mention parse failure: {}",
            err_msg
        );
    }

    #[test]
    fn test_parse_pass3_json_empty_triples_default() {
        let raw = "{}";
        let result = parse_pass3_json(raw);
        assert!(result.is_ok(), "empty object should parse via serde defaults: {:?}", result);
        assert!(result.unwrap().triples.is_empty());
    }

    fn seed_vocab() -> Vec<String> {
        SEED_PREDICATES.iter().map(|s| s.to_string()).collect()
    }

    fn new_predicate(name: &str) -> Pass3NewPredicate {
        Pass3NewPredicate {
            name: name.to_string(),
            description: "test description".to_string(),
            subject_class: None,
            object_class: None,
        }
    }

    #[test]
    fn test_validate_drops_unknown_predicate() {
        let entities = vec![
            sample_entity("Person", "Alex", Some("c-1")),
            sample_entity("Location", "AlphaComplex", Some("c-2")),
        ];
        let out = Pass3Output {
            triples: vec![Pass3Triple {
                subject_id: "e_0".to_string(),
                predicate: "resides_at".to_string(),
                object_id: "e_1".to_string(),
            }],
            new_predicates: vec![],
        };
        let validated = validate_and_normalize(out, &entities, &seed_vocab());
        assert!(validated.triples.is_empty(), "unknown predicate must be dropped");
    }

    #[test]
    fn test_validate_drops_self_referential() {
        let entities = vec![sample_entity("Person", "Alex", Some("c-1"))];
        let out = Pass3Output {
            triples: vec![Pass3Triple {
                subject_id: "e_0".to_string(),
                predicate: "mentioned_with".to_string(),
                object_id: "e_0".to_string(),
            }],
            new_predicates: vec![],
        };
        let validated = validate_and_normalize(out, &entities, &seed_vocab());
        assert!(validated.triples.is_empty(), "self-referential triple must be dropped");
    }

    #[test]
    fn test_validate_drops_out_of_bounds_reference() {
        let entities = vec![
            sample_entity("Person", "Alex", Some("c-1")),
            sample_entity("Location", "AlphaComplex", Some("c-2")),
            sample_entity("Date", "2024-04-11", Some("c-3")),
        ];
        let out = Pass3Output {
            triples: vec![Pass3Triple {
                subject_id: "e_0".to_string(),
                predicate: "owns".to_string(),
                object_id: "e_99".to_string(),
            }],
            new_predicates: vec![],
        };
        let validated = validate_and_normalize(out, &entities, &seed_vocab());
        assert!(validated.triples.is_empty(), "out-of-bounds reference must be dropped");
    }

    #[test]
    fn test_validate_keeps_valid_triple() {
        let entities = vec![
            sample_entity("Person", "Alex", Some("c-1")),
            sample_entity("Location", "AlphaComplex", Some("c-2")),
        ];
        let out = Pass3Output {
            triples: vec![Pass3Triple {
                subject_id: "e_0".to_string(),
                predicate: "owns".to_string(),
                object_id: "e_1".to_string(),
            }],
            new_predicates: vec![],
        };
        let validated = validate_and_normalize(out, &entities, &seed_vocab());
        assert_eq!(validated.triples.len(), 1, "valid triple must be kept");
    }

    #[test]
    fn test_validate_drops_new_predicate_when_over_cap() {
        let out = Pass3Output {
            triples: vec![],
            new_predicates: vec![
                new_predicate("pred_a"),
                new_predicate("pred_b"),
                new_predicate("pred_c"),
                new_predicate("pred_d"),
                new_predicate("pred_e"),
            ],
        };
        let validated = validate_and_normalize(out, &[], &seed_vocab());
        assert_eq!(validated.new_predicates.len(), 3, "must cap at 3 new_predicates per doc");
    }

    #[test]
    fn test_validate_drops_new_predicate_when_name_invalid() {
        let out = Pass3Output {
            triples: vec![],
            new_predicates: vec![
                new_predicate(""),
                new_predicate("CamelCase"),
                new_predicate("with-dash"),
                new_predicate("custody_of"),
            ],
        };
        let validated = validate_and_normalize(out, &[], &seed_vocab());
        assert_eq!(validated.new_predicates.len(), 1, "only the valid snake_case name should survive");
        assert_eq!(validated.new_predicates[0].name, "custody_of");
    }

    #[test]
    fn test_validate_drops_new_predicate_when_already_in_vocab() {
        let mut vocab = seed_vocab();
        vocab.push("custody_of".to_string());
        let out = Pass3Output {
            triples: vec![],
            new_predicates: vec![new_predicate("custody_of")],
        };
        let validated = validate_and_normalize(out, &[], &vocab);
        assert!(
            validated.new_predicates.is_empty(),
            "predicate already in vocabulary must be dropped"
        );
    }

    #[test]
    fn test_validate_dedups_new_predicate_names() {
        let out = Pass3Output {
            triples: vec![],
            new_predicates: vec![new_predicate("neighbor_of"), new_predicate("neighbor_of")],
        };
        let validated = validate_and_normalize(out, &[], &seed_vocab());
        assert_eq!(validated.new_predicates.len(), 1, "duplicate names within batch must collapse to one");
    }

    #[test]
    fn test_build_extract_relations_prompt_injects_vocab() {
        let vocab = vec!["owns".to_string(), "custody_of".to_string()];
        let prompt = build_extract_relations_prompt(&vocab);
        assert!(
            prompt.contains("owns, custody_of"),
            "prompt RULES section must contain the injected vocab list: {}",
            prompt
        );
    }

    #[test]
    fn test_build_extract_relations_prompt_preserves_few_shot() {
        let prompt = build_extract_relations_prompt(&seed_vocab());
        assert!(
            prompt.contains("Example A: Property sale deed"),
            "few-shot examples must survive the builder refactor"
        );
    }

    #[test]
    fn test_build_extract_relations_prompt_mentions_new_predicates_field() {
        let prompt = build_extract_relations_prompt(&seed_vocab());
        assert!(prompt.contains("\"newPredicates\""), "prompt must document the newPredicates field");
        assert!(
            prompt.contains("Max 3 entries per document"),
            "prompt must document the 3-per-doc cap"
        );
    }

    #[test]
    fn test_prompt_via_builder_contains_all_seed_predicates() {
        let prompt = build_extract_relations_prompt(&seed_vocab());
        for predicate in SEED_PREDICATES {
            assert!(
                prompt.contains(predicate),
                "prompt built from SEED_PREDICATES must mention predicate '{}'",
                predicate
            );
        }
    }

    #[test]
    fn test_extract_relations_prompt_contains_all_predicates() {
        #[allow(deprecated)]
        let prompt = extract_relations_prompt_seed_only();
        for predicate in SEED_PREDICATES {
            assert!(
                prompt.contains(predicate),
                "deprecated seed-only prompt must mention predicate '{}'",
                predicate
            );
        }
    }

    #[test]
    fn test_parse_pass3_json_with_new_predicates() {
        let raw = r#"{"triples":[],"newPredicates":[{"name":"custody_of","description":"legal guardian relationship"}]}"#;
        let result = parse_pass3_json(raw);
        assert!(result.is_ok(), "JSON with newPredicates must parse: {:?}", result);
        let out = result.unwrap();
        assert_eq!(out.new_predicates.len(), 1);
        assert_eq!(out.new_predicates[0].name, "custody_of");
    }

    #[test]
    fn test_parse_pass3_json_missing_new_predicates_defaults_empty() {
        let raw = r#"{"triples":[{"subjectId":"e_0","predicate":"owns","objectId":"e_1"}]}"#;
        let result = parse_pass3_json(raw);
        assert!(result.is_ok(), "legacy JSON without newPredicates must still parse: {:?}", result);
        assert!(result.unwrap().new_predicates.is_empty());
    }

    // ── Task 2 tests: Pass3RelationExtractor ────────────────────────────────

    #[tokio::test]
    async fn test_extract_no_provider_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let auth = Arc::new(AuthState::new(&dir.path().to_path_buf()));
        let extractor = Pass3RelationExtractor::new(auth);
        let entities = vec![sample_entity("Person", "Alex", Some("c-1"))];
        let result = extractor.extract("some document text", "test doc", &entities).await;
        assert!(result.is_ok(), "no provider must not error: {:?}", result);
        assert!(result.unwrap().is_none(), "no provider → extract returns None (D-04)");
    }

    #[tokio::test]
    async fn test_extract_no_pass2_entities_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let auth = Arc::new(AuthState::new(&dir.path().to_path_buf()));
        let extractor = Pass3RelationExtractor::new(auth);
        let result = extractor.extract("some document text", "test doc", &[]).await;
        assert!(result.is_ok(), "empty entities must not error: {:?}", result);
        assert!(result.unwrap().is_none(), "no entities → extract short-circuits to None");
    }

    #[tokio::test]
    async fn test_extract_full_no_provider_returns_ok_none() {
        let dir = tempfile::tempdir().unwrap();
        let auth = Arc::new(AuthState::new(&dir.path().to_path_buf()));
        let extractor = Pass3RelationExtractor::new(auth);
        let entities = vec![sample_entity("Person", "Alex", Some("c-1"))];
        let result = extractor
            .extract_full("some document text", "test doc", &entities, &seed_vocab())
            .await;
        assert!(result.is_ok(), "no provider must not error: {:?}", result);
        assert!(result.unwrap().is_none(), "no provider → extract_full returns None (D-04)");
    }

    #[tokio::test]
    async fn test_extract_backward_compat_calls_extract_full() {
        let dir = tempfile::tempdir().unwrap();
        let auth = Arc::new(AuthState::new(&dir.path().to_path_buf()));
        let extractor = Pass3RelationExtractor::new(auth);
        let entities = vec![sample_entity("Person", "Alex", Some("c-1"))];
        // No provider connected → extract_full returns Ok(None); extract() must
        // unwrap that the same way (triples-only), proving delegation.
        let result = extractor.extract("some document text", "test doc", &entities).await;
        assert!(result.is_ok(), "backward-compat wrapper must not error: {:?}", result);
        assert!(
            result.unwrap().is_none(),
            "extract() must mirror extract_full's Ok(None) short-circuit"
        );
    }

    #[test]
    fn test_semaphore_default_cap_is_one() {
        let dir = tempfile::tempdir().unwrap();
        let auth = Arc::new(AuthState::new(&dir.path().to_path_buf()));
        let extractor = Pass3RelationExtractor::new(auth);
        assert_eq!(
            extractor.semaphore().available_permits(),
            1,
            "default semaphore capacity is 1 (UI-responsive during backfill)"
        );
    }

    #[tokio::test]
    async fn test_semaphore_cap_enforces_concurrency_limit() {
        use std::sync::Arc as StdArc;

        let sem = StdArc::new(tokio::sync::Semaphore::new(2));
        let start = tokio::time::Instant::now();

        let mut handles = vec![];
        for _ in 0..4 {
            let s = sem.clone();
            handles.push(tokio::spawn(async move {
                let _p = s.acquire_owned().await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let elapsed_ms = start.elapsed().as_millis();
        assert!(
            elapsed_ms >= 80,
            "Expected ≥80ms for 2 concurrent batches, got {}ms",
            elapsed_ms
        );
        assert!(
            elapsed_ms < 220,
            "Expected <220ms (not serial), got {}ms",
            elapsed_ms
        );
    }

    #[tokio::test]
    async fn test_set_and_get_model() {
        let dir = tempfile::tempdir().unwrap();
        let auth = Arc::new(AuthState::new(&dir.path().to_path_buf()));
        let extractor = Pass3RelationExtractor::new(auth);
        extractor.set_model("custom-model-id".to_string()).await;
        assert_eq!(extractor.model().await, "custom-model-id");
    }

    #[test]
    fn test_prepare_llm_input_short_text_includes_entity_list() {
        let entities = vec![
            sample_entity("Person", "Alex Doe", Some("c-1")),
            sample_entity("Location", "AlphaComplex", Some("c-2")),
        ];
        let result = Pass3RelationExtractor::prepare_llm_input(
            "Sale Deed", "short body text", &entities, &[],
        );
        assert!(result.contains("Title: "), "must include title");
        assert!(
            result.contains("Entities in this doc"),
            "must include entity list header"
        );
        assert!(result.contains("e_0: Alex Doe"), "must include e_0 entity line");
        assert!(result.contains("e_1: AlphaComplex"), "must include e_1 entity line");
    }

    #[test]
    fn test_prepare_llm_input_long_text_chunked_structure() {
        let long_text = "x".repeat(15_000);
        let entities = vec![sample_entity("Person", "Alex", Some("c-1"))];
        let result = Pass3RelationExtractor::prepare_llm_input("t", &long_text, &entities, &[]);
        assert!(result.contains("Title: t"), "must include title");
        assert!(
            result.contains("[Document excerpt — beginning]"),
            "must have beginning marker"
        );
        assert!(
            result.contains("[...middle omitted...]"),
            "must have omission marker"
        );
        assert!(
            result.contains("[Document excerpt — end]"),
            "must have end marker"
        );
    }
}

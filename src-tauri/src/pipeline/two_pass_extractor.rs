//! TwoPassExtractor — Phase 8, Plan 05
//!
//! Facade composing Pass 1 (deterministic patterns) and Pass 2 (LLM refinement)
//! with the D-20 merge policy:
//!   - Pass-2 `refined_entities` override Pass-1 class/subclass/confidence keyed by `pass1_id = "e_N"`
//!   - Unrefined Pass-1 entities keep their original classification
//!   - Pass-2 `additional_entities` appended after Pass-1 entities
//!   - 20-entity cap re-applied after merge (LLME-03)
//!
//! pass1_id numbering scheme (D-08):
//!   Pass 1 entities are referenced by their 0-based position in the output Vec.
//!   `e_0` = first entity, `e_1` = second, etc.  The index is passed to Pass 2 in
//!   the prompt (see pass2_llm_refiner.rs) and the same scheme is used here for lookup.
//!   The `label` field of ExtractedEntity is NOT mutated to embed the id — the scheme
//!   is purely internal to the prompt/refinement round-trip.
//!
//! `extract()` (sync) is a drop-in replacement for `NerService::extract` for `spawn_blocking` callers.
//! `extract_full()` (async) runs both passes when `llm_enabled=true` and a provider is connected.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::auth::AuthState;
use crate::error::AppError;
use crate::pipeline::entity_normalizer;
use crate::pipeline::pass1_pattern_extractor::Pass1PatternExtractor;
use crate::pipeline::pass2_llm_refiner::{Pass2LlmRefiner, Pass2Output};
use crate::pipeline::pass3_relation_extractor::Pass3RelationExtractor;
use crate::types::{ExtractedEntities, ExtractedEntity, PASS1_ONLY_VERSION, TWO_PASS_TARGET_VERSION};

/// Maximum entities returned after merge (LLME-03).
const ENTITY_CAP: usize = 20;

// ─── TwoPassExtractor ─────────────────────────────────────────────────────────

/// Facade composing Pass 1 (pattern) + Pass 2 (LLM) extraction.
///
/// Holds:
/// - `pass1`       — stateless deterministic regex extractor (never errors after init)
/// - `pass2`       — LLM refiner wrapped in Arc so `pass2()` getter lets callers (backfill)
///                   reuse the same refiner instance without construction overhead
/// - `pass3`       — relation extractor wrapped in Arc so `pass3()` getter lets the
///                   backfill worker (Phase 11.5, Plan 04) reuse the same instance
///                   without construction overhead
/// - `llm_enabled` — runtime toggle mirroring `Settings.use_llm_extraction` (D-33);
///                   stored as AtomicBool for lock-free reads in hot path
pub struct TwoPassExtractor {
    pass1: Pass1PatternExtractor,
    pass2: Arc<Pass2LlmRefiner>,
    pass3: Arc<Pass3RelationExtractor>,
    llm_enabled: Arc<AtomicBool>,
}

impl TwoPassExtractor {
    /// Construct the facade.
    ///
    /// Fails fast if Pass 1 regex compilation fails — all patterns are static strings,
    /// so this should never happen in practice (T-08-06 fast-fail on init).
    /// Defaults `llm_enabled = true` (D-33: on by default, privacy toggle in Settings).
    pub fn new(auth: Arc<AuthState>) -> Result<Self, AppError> {
        let pass1 = Pass1PatternExtractor::new()?;
        let pass2 = Arc::new(Pass2LlmRefiner::new(auth.clone()));
        let pass3 = Arc::new(Pass3RelationExtractor::new(auth.clone()));
        Ok(Self {
            pass1,
            pass2,
            pass3,
            llm_enabled: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Synchronous Pass-1-only extraction.
    ///
    /// Drop-in replacement for `NerService::extract` — suitable for `spawn_blocking`
    /// callers such as `indexer.rs` (Phase 8 Plan 10 will rewire indexer to use this).
    /// Returns the same sorted+deduped+capped `Vec<ExtractedEntity>` that Pass 1 produces.
    pub fn extract(&self, text: &str) -> Result<Vec<ExtractedEntity>, AppError> {
        self.pass1.extract(text)
    }

    /// Full two-pass extraction (async).
    ///
    /// Behaviour (per D-16, D-17, D-18, D-26, LLME-04):
    ///  1. Always run Pass 1 (D-31: Pass 1 is unconditional).
    ///  2. If `llm_enabled = false` → return Pass-1-only shape (entities_version=2.5).
    ///  3. Call Pass 2 refiner.
    ///  4. If Pass 2 returns `Err(_)` → log warn, return Pass-1-only shape (D-26, LLME-04:
    ///     NEVER propagate Pass 2 errors to the caller).
    ///  5. If Pass 2 returns `Ok(empty)` (provider absent or Ollama unconfigured) → Pass-1-only.
    ///  6. Otherwise → merge with D-20 policy, return entities_version=3.0.
    ///
    /// Pass 3 relation extraction runs separately in the backfill worker
    /// (`backfill.rs`), not here — indexer path stays fast and non-blocking.
    pub async fn extract_full(
        &self,
        text: &str,
        title: &str,
    ) -> Result<ExtractedEntities, AppError> {
        // Step 1: Pass 1 always runs (D-31).
        let mut pass1_entities = self.pass1.extract(text)?;

        // Step 2: LLM disabled → Pass-1-only.
        if !self.llm_enabled.load(Ordering::Acquire) {
            // Phase 11.6 (D-09): rule-based canonical_short_name derivation runs
            // on every return path, even when Pass 2 never executes.
            entity_normalizer::normalize_entities(&mut pass1_entities);
            return Ok(ExtractedEntities {
                entities: pass1_entities,
                topic: None,
                tags: vec![],
                entities_version: PASS1_ONLY_VERSION,
                language: None,
            });
        }

        // Steps 3+4: Run Pass 2 with fallback on error (D-26).
        // WR-01 fix: refine() now returns Option<Pass2Output> — None means "provider
        // absent or unconfigured" (version stays at PASS1_ONLY_VERSION), Some(out)
        // means "LLM ran" even if all fields are empty (version advances to 3.0 so the
        // backfill does not re-process a simple-but-valid document on every run).
        let pass2_output = match self.pass2.refine(text, title, &pass1_entities).await {
            Ok(None) => {
                // Provider absent or no model configured — stay at Pass 1 version.
                entity_normalizer::normalize_entities(&mut pass1_entities);
                return Ok(ExtractedEntities {
                    entities: pass1_entities,
                    topic: None,
                    tags: vec![],
                    entities_version: PASS1_ONLY_VERSION,
                    language: None,
                });
            }
            Ok(Some(out)) => out,
            Err(e) => {
                eprintln!(
                    "[two_pass_extractor] pass2 refine failed for '{}': {} \
                     — falling back to Pass 1 only (D-26)",
                    title, e
                );
                entity_normalizer::normalize_entities(&mut pass1_entities);
                return Ok(ExtractedEntities {
                    entities: pass1_entities,
                    topic: None,
                    tags: vec![],
                    entities_version: PASS1_ONLY_VERSION,
                    language: None,
                });
            }
        };

        // Step 5: (removed — provider-absent is now signalled by Ok(None) above,
        // not by Pass2Output::empty() identity comparison. Any Some(out) proceeds
        // to merge, even if all fields are empty, so the version advances to 3.0.)

        // Step 6: Full merge → entities_version 3.0.
        let mut merged = Self::merge_passes(pass1_entities, pass2_output);

        // Step 6a (Phase 11.6 D-09): rule-based canonical_short_name derivation.
        entity_normalizer::normalize_entities(&mut merged.entities);

        Ok(merged)
    }

    /// Toggle LLM extraction at runtime without restart.
    ///
    /// Called by `set_extraction_settings` IPC command when `Settings.use_llm_extraction`
    /// changes (D-33). Uses Release ordering so the write is visible to all threads
    /// checking with Acquire.
    pub fn set_llm_enabled(&self, enabled: bool) {
        self.llm_enabled.store(enabled, Ordering::Release);
    }

    /// Read the current `llm_enabled` flag.
    pub fn llm_enabled(&self) -> bool {
        self.llm_enabled.load(Ordering::Acquire)
    }

    /// Forward model selection to the Pass 2 + Pass 3 extractors.
    ///
    /// Called by `set_extraction_settings` when `Settings.extraction_model` changes.
    /// Takes effect immediately for subsequent `refine()` / `extract()` calls.
    /// Pass 3 uses the same model as Pass 2 per D-07 (no new settings).
    pub async fn set_model(&self, model: String) {
        self.pass2.set_model(model.clone()).await;
        self.pass3.set_model(model).await;
    }

    /// Return the underlying `Pass2LlmRefiner` Arc.
    ///
    /// Used by `trigger_entity_backfill` (Plan 06) so the backfill worker can call
    /// `refine()` directly per document, bypassing `extract_full`'s llm_enabled gate
    /// (backfill always runs when triggered explicitly).
    pub fn pass2(&self) -> Arc<Pass2LlmRefiner> {
        self.pass2.clone()
    }

    /// Return the underlying `Pass3RelationExtractor` Arc.
    ///
    /// Called by the backfill worker (Phase 11.5, Plan 04) to invoke Pass 3
    /// relation extraction after Pass 2 completes.
    pub fn pass3(&self) -> Arc<Pass3RelationExtractor> {
        self.pass3.clone()
    }

    // ─── Merge helper (D-20) ──────────────────────────────────────────────────

    /// Merge Pass-1 entities with Pass-2 refinements and additions.
    ///
    /// Algorithm (D-20):
    ///  a. For each `refined_entity` where `pass1_id = "e_N"`:
    ///       - Parse N as usize.
    ///       - If invalid or N >= pass1.len(): log warn and drop (do NOT error).
    ///       - Otherwise: overwrite `pass1[N].class`, `.subclass`, `.confidence`.
    ///         The `.label` and `.value` fields are preserved (they are the raw text).
    ///  b. Convert each `additional_entity` to `ExtractedEntity` and append.
    ///     `entity_type` is mapped from the 8-class name (legacy field, D-09).
    ///  c. Sort by (entity_type, value), dedup by same key, truncate to 20 (LLME-03).
    ///  d. Build `ExtractedEntities` with `entities_version = TWO_PASS_TARGET_VERSION`.
    pub(crate) fn merge_passes(
        mut pass1: Vec<ExtractedEntity>,
        pass2: Pass2Output,
    ) -> ExtractedEntities {
        let n = pass1.len();

        // a. Apply refined_entities overrides.
        for refined in &pass2.refined_entities {
            // pass1_id format: "e_<index>" (D-08)
            if let Some(idx_str) = refined.pass1_id.strip_prefix("e_") {
                match idx_str.parse::<usize>() {
                    Ok(idx) if idx < n => {
                        // Bug 1: never let Pass 2 reclassify an email-shaped Pass 1
                        // value away from Email (e.g. into Person). The regex already
                        // classified it correctly; the LLM must not override that.
                        if refined.class != "Email" && looks_like_email(&pass1[idx].value) {
                            pass1[idx].class       = Some("Email".to_string());
                            pass1[idx].entity_type = "email".to_string();
                            pass1[idx].subclass    = None;
                            pass1[idx].confidence  = Some(refined.confidence);
                        } else {
                            pass1[idx].class      = Some(refined.class.clone());
                            pass1[idx].subclass   = refined.subclass.clone();
                            pass1[idx].confidence = Some(refined.confidence);
                        }
                    }
                    Ok(idx) => {
                        eprintln!(
                            "[two_pass_extractor] merge_passes: refined pass1_id '{}' index {} \
                             is out of bounds (pass1.len={}); dropping",
                            refined.pass1_id, idx, n
                        );
                    }
                    Err(_) => {
                        eprintln!(
                            "[two_pass_extractor] merge_passes: cannot parse index from \
                             pass1_id '{}'; dropping",
                            refined.pass1_id
                        );
                    }
                }
            } else {
                eprintln!(
                    "[two_pass_extractor] merge_passes: pass1_id '{}' does not match \
                     'e_N' format; dropping",
                    refined.pass1_id
                );
            }
        }

        // b. Append additional_entities.
        let mut merged = pass1;
        for additional in pass2.additional_entities {
            // Bug 1: the LLM sometimes returns an email address as its own Person
            // (or Organization) entity. An email-shaped value must always be the
            // Email class — force it so (1) it is never surfaced as a duplicate
            // Person, and (2) the (entity_type, value) dedup below collapses it
            // against the Pass 1 Email entity carrying the same value.
            let (class, entity_type) = if additional.class != "Email" && looks_like_email(&additional.value) {
                ("Email".to_string(), "email".to_string())
            } else {
                let et = entity_type_from_class(&additional.class);
                (additional.class.clone(), et)
            };
            merged.push(ExtractedEntity {
                label:        additional.value.clone(),
                value:        additional.value,
                entity_type,
                canonical_id: None,
                class:        Some(class),
                subclass:     additional.subclass,
                canonical_short_name: None,
                confidence:   Some(additional.confidence),
            });
        }

        // c. Sort by (entity_type, value), dedup, cap at 20.
        merged.sort_by(|a, b| {
            a.entity_type.cmp(&b.entity_type).then(a.value.cmp(&b.value))
        });
        merged.dedup_by(|a, b| a.entity_type == b.entity_type && a.value == b.value);
        merged.truncate(ENTITY_CAP);

        // d. Return with TWO_PASS_TARGET_VERSION and Pass-2 metadata.
        ExtractedEntities {
            entities:          merged,
            topic:             pass2.topic,
            tags:              pass2.tags,
            entities_version:  TWO_PASS_TARGET_VERSION,
            language:          pass2.language,
        }
    }
}

// ─── looks_like_email (Bug 1) ─────────────────────────────────────────────────

/// Heuristic email-shape check. Returns true for `local@domain.tld` values with
/// exactly one `@`, no internal whitespace, and a dotted domain whose TLD is
/// alphabetic. Used to stop Pass 2 (the LLM) from misclassifying an email string
/// as a Person/Organization entity, and to let the merge dedup collapse an
/// email that the LLM re-emitted against the Pass 1 Email entity.
fn looks_like_email(value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() || v.matches('@').count() != 1 || v.chars().any(|c| c.is_whitespace()) {
        return false;
    }
    let (local, domain) = match v.split_once('@') {
        Some((l, d)) => (l, d),
        None => return false,
    };
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    match domain.rsplit_once('.') {
        Some((host, tld)) => {
            !host.is_empty()
                && tld.len() >= 2
                && tld.chars().all(|c| c.is_ascii_alphabetic())
        }
        None => false,
    }
}

// ─── entity_type_from_class (D-09) ────────────────────────────────────────────

/// Map the 8-class canonical name to the legacy `entity_type` string.
///
/// Phase 6 code reads `entity_type` for display and filtering; Phase 8 adds the
/// canonical `class` field but keeps `entity_type` populated for backward compat (D-09).
fn entity_type_from_class(class: &str) -> String {
    match class {
        "Person"       => "person",
        "Organization" => "organization",
        "Location"     => "location",
        "Date"         => "date",
        "Amount"       => "amount",
        "Email"        => "email",
        "Phone"        => "phone",
        "Identifier"   => "identifier",
        other          => {
            eprintln!(
                "[two_pass_extractor] entity_type_from_class: unknown class '{}' — \
                 using lowercase as entity_type",
                other
            );
            return other.to_lowercase();
        }
    }
    .to_string()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    /// Create a TwoPassExtractor with no connected AI provider (AuthState default).
    fn make_extractor() -> TwoPassExtractor {
        let dir = tempdir().unwrap();
        let auth = Arc::new(AuthState::new(&dir.path().to_path_buf()));
        TwoPassExtractor::new(auth).expect("TwoPassExtractor::new() must succeed")
    }

    /// Build a minimal ExtractedEntity for merge tests.
    fn make_entity(
        entity_type: &str,
        value: &str,
        class: Option<&str>,
        subclass: Option<&str>,
        confidence: Option<f32>,
    ) -> ExtractedEntity {
        ExtractedEntity {
            label:        value.to_string(),
            value:        value.to_string(),
            entity_type:  entity_type.to_string(),
            canonical_id: None,
            class:        class.map(|s| s.to_string()),
            subclass:     subclass.map(|s| s.to_string()),
            canonical_short_name: None,
            confidence,
        }
    }

    // ── Test 1: extract() returns Pass 1 entities synchronously ──────────────

    #[test]
    fn test_extract_pass1_only() {
        let extractor = make_extractor();
        let text = "Send invoice to billing@example.com for $1,234.56 by 2024-01-15.";
        let entities = extractor.extract(text).expect("extract() must not error");
        assert!(!entities.is_empty(), "extract() must return entities for non-empty text");
        assert!(entities.iter().any(|e| e.entity_type == "email"),   "must include email");
        assert!(entities.iter().any(|e| e.entity_type == "amount"),  "must include amount");
        assert!(entities.iter().any(|e| e.entity_type == "date"),    "must include date");
    }

    #[test]
    fn test_extract_empty_text_returns_empty() {
        let extractor = make_extractor();
        let entities = extractor.extract("").expect("extract() must not error on empty");
        assert!(entities.is_empty(), "empty text must produce empty Vec");
    }

    // ── Test 2: extract_full with llm_enabled=false → Pass-1-only ────────────

    #[tokio::test]
    async fn test_extract_full_llm_disabled() {
        let extractor = make_extractor();
        extractor.set_llm_enabled(false);

        let text = "Contact john@example.com or call +1 555 123 4567.";
        let result = extractor.extract_full(text, "Test Doc").await
            .expect("extract_full() must return Ok");

        assert!(
            (result.entities_version - PASS1_ONLY_VERSION).abs() < 1e-5,
            "llm_disabled: expected PASS1_ONLY_VERSION (2.5), got {}",
            result.entities_version
        );
        assert_eq!(result.topic, None,    "llm_disabled: topic must be None");
        assert!(result.tags.is_empty(),   "llm_disabled: tags must be empty");
        // Pass 1 still runs
        assert!(!result.entities.is_empty(), "llm_disabled: must still extract Pass 1 entities");
    }

    // ── Test 3: extract_full with no provider → refine() returns Ok(None) → Pass-1-only ─

    #[tokio::test]
    async fn test_extract_full_no_provider_fallback() {
        // No provider connected → Pass2LlmRefiner.refine() returns Ok(None) (WR-01)
        let extractor = make_extractor(); // llm_enabled=true by default
        let text = "Invoice No. INV-20240601 for 500 USD due by 2024-06-30.";
        let result = extractor.extract_full(text, "Invoice").await
            .expect("extract_full() must return Ok even with no provider");

        assert!(
            (result.entities_version - PASS1_ONLY_VERSION).abs() < 1e-5,
            "no provider: expected PASS1_ONLY_VERSION (2.5), got {}",
            result.entities_version
        );
        assert_eq!(result.topic, None,   "no provider: topic must be None");
        assert!(result.tags.is_empty(),  "no provider: tags must be empty");
        // Pass 1 entities (amount) still present
        assert!(
            result.entities.iter().any(|e| e.entity_type == "amount"),
            "no provider: Pass 1 amount entity must be present"
        );
    }

    // ── Test 4: merge_passes — 1 refined + 1 additional (core D-20 scenario) ─

    #[test]
    fn test_merge_passes_one_refined_one_additional() {
        use crate::pipeline::pass2_llm_refiner::{Pass2AdditionalEntity, Pass2RefinedEntity};

        // Pass 1: [e_0 = Identifier(unknown, 0.7), e_1 = Email(1.0)]
        let pass1 = vec![
            make_entity("identifier", "SOMETOKEN",      Some("Identifier"), Some("unknown"), Some(0.7)),
            make_entity("email",      "john@example.com", Some("Email"),   None,             Some(1.0)),
        ];

        let pass2 = Pass2Output {
            refined_entities: vec![
                Pass2RefinedEntity {
                    pass1_id:   "e_0".to_string(),
                    class:      "Identifier".to_string(),
                    subclass:   Some("aadhaar".to_string()),
                    confidence: 0.92,
                },
            ],
            additional_entities: vec![
                Pass2AdditionalEntity {
                    class:      "Person".to_string(),
                    subclass:   None,
                    value:      "John Smith".to_string(),
                    confidence: 0.95,
                },
            ],
            topic:    Some("identity".to_string()),
            tags:     vec!["india".to_string()],
            language: Some("en".to_string()),
        };

        let result = TwoPassExtractor::merge_passes(pass1, pass2);

        assert_eq!(result.entities.len(), 3, "expected 3 merged entities: {:?}", result.entities);

        // e_0 refined: subclass → aadhaar, confidence → 0.92
        let id_ent = result.entities.iter().find(|e| e.entity_type == "identifier")
            .expect("must have identifier entity");
        assert_eq!(id_ent.subclass.as_deref(),  Some("aadhaar"), "e_0 subclass refined to aadhaar");
        assert_eq!(id_ent.class.as_deref(),     Some("Identifier"));
        assert!((id_ent.confidence.unwrap() - 0.92).abs() < 1e-5, "e_0 confidence refined to 0.92");

        // e_1 unchanged
        let email_ent = result.entities.iter().find(|e| e.entity_type == "email")
            .expect("must have email entity");
        assert_eq!(email_ent.value, "john@example.com");
        assert!((email_ent.confidence.unwrap() - 1.0).abs() < 1e-5, "e_1 confidence unchanged");

        // Additional: Person "John Smith"
        let person_ent = result.entities.iter().find(|e| e.entity_type == "person")
            .expect("must have person entity from additional_entities");
        assert_eq!(person_ent.value, "John Smith");
        assert!((person_ent.confidence.unwrap() - 0.95).abs() < 1e-5);
        assert_eq!(person_ent.class.as_deref(), Some("Person"));

        // Metadata copied
        assert_eq!(result.topic, Some("identity".to_string()));
        assert_eq!(result.tags, vec!["india".to_string()]);
        assert_eq!(result.language, Some("en".to_string()));
        assert!((result.entities_version - TWO_PASS_TARGET_VERSION).abs() < 1e-5);
    }

    // ── Bug 1: email must never be classified as Person ──────────────────────

    /// The LLM (Pass 2) sometimes returns an email string as its own Person entity
    /// alongside the real person's name. merge_passes must force the email-shaped
    /// value to the Email class, never surface it as a Person, and collapse it
    /// against the Pass 1 Email entity carrying the same value.
    #[test]
    fn test_merge_passes_email_not_classified_as_person() {
        use crate::pipeline::pass2_llm_refiner::Pass2AdditionalEntity;

        // Pass 1 already found the email as an Email entity.
        let pass1 = vec![
            make_entity("email", "alex.doe@example.com", Some("Email"), None, Some(1.0)),
        ];

        // Pass 2 wrongly returns the same email string as a Person, plus the real name.
        let pass2 = Pass2Output {
            refined_entities: vec![],
            additional_entities: vec![
                Pass2AdditionalEntity {
                    class: "Person".to_string(),
                    subclass: None,
                    value: "alex.doe@example.com".to_string(),
                    confidence: 0.9,
                },
                Pass2AdditionalEntity {
                    class: "Person".to_string(),
                    subclass: None,
                    value: "Alex Doe".to_string(),
                    confidence: 0.95,
                },
            ],
            topic: None, tags: vec![], language: None,
        };

        let result = TwoPassExtractor::merge_passes(pass1, pass2);

        // The email must NOT appear as a Person entity.
        assert!(
            !result.entities.iter().any(|e| e.value == "alex.doe@example.com" && e.entity_type == "person"),
            "email must never be classified as Person: {:?}", result.entities
        );

        // Exactly one entity carries the email value — Pass 1 Email and the
        // reclassified Pass 2 duplicate collapsed into one Email entity.
        let email_ents: Vec<_> = result
            .entities
            .iter()
            .filter(|e| e.value == "alex.doe@example.com")
            .collect();
        assert_eq!(email_ents.len(), 1, "duplicate email must collapse to one entity: {:?}", email_ents);
        assert_eq!(email_ents[0].entity_type, "email");
        assert_eq!(email_ents[0].class.as_deref(), Some("Email"));

        // The real person name still survives as a Person.
        assert!(
            result.entities.iter().any(|e| e.value == "Alex Doe" && e.entity_type == "person"),
            "the real person name must remain a Person entity"
        );
    }

    /// A Pass 2 refinement must not be able to relabel an email-shaped Pass 1
    /// value into a Person class.
    #[test]
    fn test_merge_passes_refine_cannot_turn_email_into_person() {
        use crate::pipeline::pass2_llm_refiner::Pass2RefinedEntity;

        let pass1 = vec![
            make_entity("email", "sam.doe@example.com", Some("Email"), None, Some(1.0)),
        ];
        let pass2 = Pass2Output {
            refined_entities: vec![Pass2RefinedEntity {
                pass1_id: "e_0".to_string(),
                class: "Person".to_string(),
                subclass: None,
                confidence: 0.8,
            }],
            additional_entities: vec![],
            topic: None, tags: vec![], language: None,
        };

        let result = TwoPassExtractor::merge_passes(pass1, pass2);
        let ent = result.entities.iter().find(|e| e.value == "sam.doe@example.com").unwrap();
        assert_eq!(ent.class.as_deref(), Some("Email"), "refine must not reclassify email as Person");
        assert_eq!(ent.entity_type, "email");
    }

    /// Guard `looks_like_email` against false positives — normal names must not
    /// be treated as emails.
    #[test]
    fn test_looks_like_email_rejects_plain_names() {
        assert!(looks_like_email("alex.doe@example.com"));
        assert!(looks_like_email("a@b.co"));
        assert!(!looks_like_email("Alex Doe"));
        assert!(!looks_like_email("Alpha Beta Corp"));
        assert!(!looks_like_email("@example.com"));
        assert!(!looks_like_email("alex@"));
        assert!(!looks_like_email("alex.doe@example"));       // no dotted TLD
        assert!(!looks_like_email("alex @ example.com"));     // whitespace
        assert!(!looks_like_email("a@b@example.com"));        // two @
    }

    // ── Test 5: merge_passes — out-of-bounds pass1_id is dropped ─────────────

    #[test]
    fn test_merge_passes_unmatched_pass1_id_dropped() {
        use crate::pipeline::pass2_llm_refiner::Pass2RefinedEntity;

        let pass1 = vec![
            make_entity("identifier", "TOKEN-A",   Some("Identifier"), Some("unknown"), Some(0.7)),
            make_entity("email",      "a@b.com",   Some("Email"),      None,            Some(1.0)),
        ];

        let pass2 = Pass2Output {
            refined_entities: vec![
                // e_99 out of bounds (pass1.len = 2) — must be silently dropped
                Pass2RefinedEntity {
                    pass1_id:   "e_99".to_string(),
                    class:      "Identifier".to_string(),
                    subclass:   Some("ssn".to_string()),
                    confidence: 0.95,
                },
            ],
            additional_entities: vec![],
            topic: None, tags: vec![], language: None,
        };

        let result = TwoPassExtractor::merge_passes(pass1, pass2);

        assert_eq!(result.entities.len(), 2, "unmatched pass1_id must not remove entities");
        let id_ent = result.entities.iter().find(|e| e.entity_type == "identifier")
            .expect("identifier must still be present");
        assert_eq!(
            id_ent.subclass.as_deref(),
            Some("unknown"),
            "unmatched pass1_id must not modify the existing entity (subclass stays 'unknown')"
        );
    }

    // ── Test 6: 20-entity cap enforced after merge ────────────────────────────

    #[test]
    fn test_merge_passes_20_cap_after_merge() {
        use crate::pipeline::pass2_llm_refiner::Pass2AdditionalEntity;

        // 15 distinct Pass 1 email entities
        let pass1: Vec<ExtractedEntity> = (1..=15)
            .map(|i| make_entity("email", &format!("user{}@example.com", i), Some("Email"), None, Some(1.0)))
            .collect();

        // 10 additional person entities
        let additional: Vec<Pass2AdditionalEntity> = (1..=10)
            .map(|i| Pass2AdditionalEntity {
                class:      "Person".to_string(),
                subclass:   None,
                value:      format!("Person {}", i),
                confidence: 0.9,
            })
            .collect();

        let pass2 = Pass2Output {
            refined_entities:    vec![],
            additional_entities: additional,
            topic: None, tags: vec![], language: None,
        };

        let result = TwoPassExtractor::merge_passes(pass1, pass2);

        assert_eq!(
            result.entities.len(),
            20,
            "20-entity cap must be enforced after merge (got {})",
            result.entities.len()
        );
    }

    // ── Test 7: topic copied even with 0 entity changes ──────────────────────

    #[test]
    fn test_merge_passes_topic_copied_with_no_entities() {
        // Pass 2 returns 0 refined_entities + 0 additional_entities but a valid topic.
        // This is NOT Pass2Output::empty() (topic is set) → must call merge_passes →
        // topic + tags must be in output with entities_version=3.0.
        let pass1 = vec![make_entity("date", "2024-01-01", Some("Date"), None, Some(1.0))];
        let pass2 = Pass2Output {
            refined_entities:    vec![],
            additional_entities: vec![],
            topic:    Some("finance".to_string()),
            tags:     vec!["bank".to_string()],
            language: Some("en".to_string()),
        };

        let result = TwoPassExtractor::merge_passes(pass1, pass2);

        assert_eq!(result.topic, Some("finance".to_string()), "topic must be copied from Pass 2");
        assert_eq!(result.tags, vec!["bank".to_string()],     "tags must be copied");
        assert!(
            (result.entities_version - TWO_PASS_TARGET_VERSION).abs() < 1e-5,
            "entities_version must be TWO_PASS_TARGET_VERSION (3.0)"
        );
    }

    // ── Test 7b (Phase 11.6 D-09): normalize_entities applied after merge ────

    /// `extract_full`'s Step 6a calls `entity_normalizer::normalize_entities` on
    /// `merge_passes`'s output before returning. `merge_passes` itself does NOT
    /// normalize (it's a pure structural merge) — this test exercises the same
    /// sequence `extract_full` performs: merge, then normalize.
    #[test]
    fn test_normalize_after_merge_sets_short_name() {
        use crate::pipeline::pass2_llm_refiner::Pass2AdditionalEntity;

        let pass1 = vec![];
        let pass2 = Pass2Output {
            refined_entities: vec![],
            additional_entities: vec![Pass2AdditionalEntity {
                class:      "Organization".to_string(),
                subclass:   None,
                value:      "Acme Corp Ltd".to_string(),
                confidence: 0.9,
            }],
            topic: None, tags: vec![], language: None,
        };

        let mut merged = TwoPassExtractor::merge_passes(pass1, pass2);
        assert_eq!(
            merged.entities[0].canonical_short_name, None,
            "merge_passes itself must NOT normalize — canonical_short_name stays None until Step 6a"
        );

        entity_normalizer::normalize_entities(&mut merged.entities);

        assert_eq!(
            merged.entities[0].canonical_short_name,
            Some("Acme Corp".to_string()),
            "normalize_entities (Step 6a) must set canonical_short_name after merge"
        );
    }

    // ── Test 8: set_llm_enabled toggle is lock-free and immediate ────────────

    #[test]
    fn test_set_llm_enabled_toggle() {
        let extractor = make_extractor();
        assert!(extractor.llm_enabled(),  "default must be true");
        extractor.set_llm_enabled(false);
        assert!(!extractor.llm_enabled(), "after disable: must be false");
        extractor.set_llm_enabled(true);
        assert!(extractor.llm_enabled(),  "after re-enable: must be true");
    }

    // ── Test 9: entity_type_from_class covers all 8 locked classes ───────────

    #[test]
    fn test_entity_type_from_class_all_eight() {
        assert_eq!(entity_type_from_class("Person"),       "person");
        assert_eq!(entity_type_from_class("Organization"), "organization");
        assert_eq!(entity_type_from_class("Location"),     "location");
        assert_eq!(entity_type_from_class("Date"),         "date");
        assert_eq!(entity_type_from_class("Amount"),       "amount");
        assert_eq!(entity_type_from_class("Email"),        "email");
        assert_eq!(entity_type_from_class("Phone"),        "phone");
        assert_eq!(entity_type_from_class("Identifier"),   "identifier");
    }

    // ── Test 10: pass3() accessor returns an Arc<Pass3RelationExtractor> ─────

    #[test]
    fn test_pass3_accessor_returns_arc() {
        let extractor = make_extractor();
        let pass3 = extractor.pass3();
        // Arc strong count must be >= 2 (one held by extractor, one returned here).
        assert!(
            Arc::strong_count(&pass3) >= 2,
            "pass3() must return a cloned Arc sharing the same instance"
        );
    }

    // ── Test 11: set_model propagates to both Pass 2 + Pass 3 ────────────────

    #[tokio::test]
    async fn test_set_model_propagates_to_pass3() {
        let extractor = make_extractor();
        extractor.set_model("model-X".to_string()).await;
        assert_eq!(extractor.pass3().model().await, "model-X", "pass3 must receive the model");
        assert_eq!(extractor.pass2().model().await, "model-X", "pass2 must receive the model");
    }
}

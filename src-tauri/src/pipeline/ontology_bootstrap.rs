//! Corpus-seeded ontology bootstrap (D-01..D-04, Phase 11.6).
//!
//! Fires once per install after `BOOTSTRAP_MIN_DOCS` docs complete Pass 2.
//! Reads titles + topics + tags + top-5 entities from those docs, calls the
//! active LLM once, and populates `OntologyStore.corpus_seed`.
//!
//! Mirrors the `Pass2LlmRefiner` / `Pass3RelationExtractor` pattern for LLM
//! plumbing: no new abstractions, direct `ai_request_with_retry` +
//! `strip_json_fences` reuse (Phase 8 / Phase 11.5 established pattern).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::Mutex;

use crate::ai::retry::ai_request_with_retry;
use crate::ai::service::{AIServiceRequest, ServiceMessage};
use crate::auth::AuthState;
use crate::error::AppError;
use crate::graph::ontology_store::OntologyStore;
use crate::pipeline::pass2_llm_refiner::{strip_json_fences, Pass2LlmRefiner};
use crate::types::{BootstrapSeed, EntitySubclass, Predicate, PromotionSource, SEED_PREDICATES, VOCABULARY_HARD_CAP};

// ─── Prompt version ───────────────────────────────────────────────────────────
/// Semantic version of `BOOTSTRAP_PROMPT`. Stored alongside output so future
/// re-bootstrap logic can detect prompt drift (mirror of Pass 2/3 convention).
pub const BOOTSTRAP_PROMPT_VERSION: &str = "v1";

/// The 8 locked entity classes an entity subclass may refine (mirror of Pass
/// 2's `EIGHT_CLASS_ALLOW_LIST`).
const EIGHT_CLASS_ALLOW_LIST: [&str; 8] = [
    "Person",
    "Organization",
    "Location",
    "Date",
    "Amount",
    "Email",
    "Phone",
    "Identifier",
];

// ─── BOOTSTRAP_PROMPT (D-01) ───────────────────────────────────────────────────
/// Inline system prompt for the corpus-seeded bootstrap LLM call.
///
/// Includes: role + task description, output constraints, JSON schema, and a
/// 30-doc few-shot example. Explicitly lists all 21 Phase 11.5 seed
/// predicates in the "do not propose" block (D-03) so the LLM never emits a
/// duplicate that would collide with the frozen baseline.
pub const BOOTSTRAP_PROMPT: &str = r#"You are helping cortex, a personal document intelligence tool, learn a user's domain-specific vocabulary from the first 30 documents in their library.

TASK:
Given titles + topics + tags + top-5 entities per document below, propose 15–30 domain-specific PREDICATES (relation types) and 5–15 ENTITY SUBCLASSES that would be useful for cortex to extract from this user's documents going forward.

CONSTRAINTS:
1. Output ONLY valid JSON — no markdown fences, no explanation.
2. Predicates use snake_case names ≤ 40 chars. Examples: `registered_to`, `insured_by`, `taxed_by`, `apartment_of`, `plot_in`, `sibling_of`, `mortgaged_to`, `sale_deed_of`, `tax_receipt_for`, `neighbor_of`.
3. Do NOT propose these — cortex already has them from Phase 11.5: owns, owned_by, purchased_from, sold_to, located_in, part_of, address_of, married_to, parent_of, child_of, member_of, employer_of, employee_of, partner_of, issued_by, dated, signed_by, uses_pan, uses_aadhaar, has_voter_id, mentioned_with.
4. For each predicate, provide `subject_class` and `object_class` when clear (both are one of: Person, Organization, Location, Date, Amount, Email, Phone, Identifier). Use null when ambiguous.
5. Entity subclasses refine one of the 8 classes. Examples: {class: "Location", subclass: "apartment"}, {class: "Location", subclass: "plot"}, {class: "Identifier", subclass: "vin"}, {class: "Person", subclass: "spouse"}. Provide an `example` (surface form) for each.
6. Prefer 3–10 char subclasses; avoid multi-word subclasses (`property_tax_receipt` → subclass:"tax_receipt").
7. Return an empty array when no domain-specific vocabulary is needed for a class of docs.

JSON SCHEMA:
{
  "predicates": [
    { "name": "snake_case_name", "description": "one-sentence use case", "subjectClass": "Person" | null, "objectClass": "Location" | null }
  ],
  "entitySubclasses": [
    { "class": "Location", "subclass": "apartment", "example": "Building-Unit-204" }
  ]
}

FEW-SHOT EXAMPLE:
Documents:
- "Property Sale Deed 2024" [topic: property, tags: ownership, sale] entities: Jane Doe, Sunset Towers, Metroville, 2024-04-11
- "Vehicle Registration Certificate" [topic: vehicle] entities: John Roe, Unit 204, XX12AB1234
- "Government ID Card" [topic: identity] entities: Sam Roe, ABCDE1234F
- (…30 lines total)

Output:
{"predicates":[{"name":"registered_to","description":"vehicle-registration owner","subjectClass":"Location","objectClass":"Person"},{"name":"neighbor_of","description":"adjacent property","subjectClass":"Location","objectClass":"Location"},{"name":"sale_deed_of","description":"legal ownership document","subjectClass":"Location","objectClass":"Person"}],"entitySubclasses":[{"class":"Location","subclass":"apartment","example":"Building-Unit-204"},{"class":"Location","subclass":"plot","example":"Plot 42"},{"class":"Identifier","subclass":"vin","example":"XX12AB1234"}]}

OUTPUT ONLY THE JSON."#;

// ─── Input types ──────────────────────────────────────────────────────────────

/// One sampled document for the bootstrap prompt: title + topic + tags +
/// top-5 (class, value) entity pairs.
#[derive(Debug, Clone)]
pub struct BootstrapSampleDoc {
    pub title: String,
    pub topic: Option<String>,
    pub tags: Vec<String>,
    /// (class, value) pairs, limited to 5 per doc by the caller.
    pub top_entities: Vec<(String, String)>,
}

// ─── Raw LLM JSON output types (internal, camelCase) ───────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBootstrapPredicate {
    name: String,
    description: String,
    #[serde(default)]
    subject_class: Option<String>,
    #[serde(default)]
    object_class: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBootstrapSubclass {
    class: String,
    subclass: String,
    #[serde(default)]
    example: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawBootstrapOutput {
    #[serde(default)]
    predicates: Vec<RawBootstrapPredicate>,
    #[serde(default)]
    entity_subclasses: Vec<RawBootstrapSubclass>,
}

// ─── JSON parsing (mirror of Pass 2/3 two-attempt pattern) ────────────────────

/// Parse the raw LLM JSON response into `RawBootstrapOutput` with two-attempt
/// robustness (mirror of Pass 2/3): direct parse first, then fence-stripped
/// retry.
pub fn parse_bootstrap_json(raw: &str) -> Result<RawBootstrapOutput, AppError> {
    if let Ok(out) = serde_json::from_str::<RawBootstrapOutput>(raw) {
        return Ok(out);
    }

    let stripped = strip_json_fences(raw);
    serde_json::from_str::<RawBootstrapOutput>(&stripped)
        .map_err(|e| AppError::Internal(format!("Bootstrap JSON parse failed: {}", e)))
}

// ─── Validation (T-11.6-12..15) ────────────────────────────────────────────────

/// Validate + dedup a raw bootstrap output into a `BootstrapSeed`.
///
/// - Drops predicates whose `name` collides with `seed_set` (D-03, T-11.6-13).
/// - Drops predicates with empty name, name > 40 chars, or non-snake_case
///   names (T-11.6-14).
/// - Dedups predicates by name within the batch.
/// - Caps predicates at `VOCABULARY_HARD_CAP - existing_count`, never
///   overflowing the cap in a single bootstrap (T-11.6-12).
/// - Drops entity subclasses whose `class` is not one of the 8 locked
///   classes, or whose `subclass` is empty or > 24 chars.
fn validate_bootstrap(
    raw: RawBootstrapOutput,
    seed_set: &HashSet<String>,
    existing_count: usize,
    now: &str,
    model_used: &str,
    sample_count: u32,
) -> BootstrapSeed {
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut predicates: Vec<Predicate> = Vec::new();

    // Headroom: never let a single bootstrap push the effective vocabulary
    // past VOCABULARY_HARD_CAP. existing_count already includes the 21 seed
    // predicates, so the remaining headroom is the cap minus what's there.
    let headroom = VOCABULARY_HARD_CAP.saturating_sub(existing_count);

    for p in raw.predicates {
        if predicates.len() >= headroom {
            break;
        }
        if seed_set.contains(&p.name) {
            continue;
        }
        if !is_valid_predicate_name(&p.name) {
            continue;
        }
        if !seen_names.insert(p.name.clone()) {
            continue; // duplicate within this batch
        }

        predicates.push(Predicate {
            name: p.name,
            description: p.description,
            source: PromotionSource::Corpus,
            count: 0,
            first_seen_doc_id: None,
            first_seen_at: Some(now.to_string()),
            promoted_at: Some(now.to_string()),
            subject_class: p.subject_class,
            object_class: p.object_class,
        });
    }

    let mut entity_subclasses: Vec<EntitySubclass> = Vec::new();
    for es in raw.entity_subclasses {
        if !EIGHT_CLASS_ALLOW_LIST.contains(&es.class.as_str()) {
            continue;
        }
        if es.subclass.is_empty() || es.subclass.len() > 24 {
            continue;
        }

        entity_subclasses.push(EntitySubclass {
            class: es.class,
            subclass: es.subclass,
            source: PromotionSource::Corpus,
            count: 0,
            first_seen_doc_id: None,
            example_value: es.example,
        });
    }

    BootstrapSeed {
        predicates,
        entity_subclasses,
        generated_at: now.to_string(),
        sample_doc_count: sample_count,
        model_used: model_used.to_string(),
    }
}

/// Validate a bootstrap-proposed predicate name: non-empty, <= 40 chars,
/// snake_case (`^[a-z][a-z0-9_]*$`). Mirrors `OntologyStore`'s
/// `is_valid_predicate_name` but with the tighter 40-char bootstrap-specific
/// limit from the prompt (constraint 2).
fn is_valid_predicate_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 40 {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

// ─── OntologyBootstrapper ──────────────────────────────────────────────────────

/// Orchestrates the one-time corpus-seeded bootstrap LLM call (D-01, D-02).
///
/// Wraps `ai_request_with_retry` with:
/// - Idempotency check via `OntologyStore::bootstrap_completed()`.
/// - Provider-absent short-circuit (returns `Ok(None)`, mirror of D-04/D-31).
/// - Same model-selection convention as Pass 2/3 (configured extraction
///   model, else provider default).
/// - JSON fence stripping + two-attempt parse + validation.
/// - Atomic apply into `OntologyStore` + persist to `ontology.json`.
pub struct OntologyBootstrapper {
    auth: Arc<AuthState>,
    ontology_store: Arc<Mutex<OntologyStore>>,
    app_data_dir: PathBuf,
}

impl OntologyBootstrapper {
    pub fn new(
        auth: Arc<AuthState>,
        ontology_store: Arc<Mutex<OntologyStore>>,
        app_data_dir: PathBuf,
    ) -> Self {
        Self {
            auth,
            ontology_store,
            app_data_dir,
        }
    }

    /// Format the sample docs into the bootstrap LLM user message.
    fn format_sample_docs(sample_docs: &[BootstrapSampleDoc]) -> String {
        let mut lines = Vec::with_capacity(sample_docs.len() + 1);
        lines.push(format!("Documents (n={}):", sample_docs.len()));

        for doc in sample_docs {
            let topic = doc.topic.as_deref().unwrap_or("unknown");
            let tags = doc.tags.join(",");
            let entities = doc
                .top_entities
                .iter()
                .map(|(c, v)| format!("{}:{}", c, v))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "- \"{}\" [topic: {}, tags: {}] entities: {}",
                doc.title, topic, tags, entities
            ));
        }

        lines.join("\n")
    }

    /// Run the corpus-seeded bootstrap once (idempotent).
    ///
    /// Returns:
    /// - `Ok(None)` if bootstrap already ran (idempotency, D-02) or no
    ///   provider is connected (mirror of D-04/D-31).
    /// - `Ok(Some(seed))` when the LLM ran and the seed was applied +
    ///   persisted.
    /// - `Err(_)` on LLM call / parse failure — caller (backfill.rs) logs
    ///   and continues; the bootstrap is retried on a subsequent trigger
    ///   since `bootstrap_completed_at` is only set on success.
    pub async fn bootstrap(
        &self,
        sample_docs: &[BootstrapSampleDoc],
    ) -> Result<Option<BootstrapSeed>, AppError> {
        // 1. Idempotency check.
        {
            let store = self.ontology_store.lock().await;
            if store.bootstrap_completed() {
                return Ok(None);
            }
        }

        // 2. Provider check (mirror of D-04/D-31).
        let provider = match self
            .auth
            .get_active_provider()
            .map_err(AppError::Internal)?
        {
            Some(p) => p,
            None => return Ok(None),
        };

        // 3. Model selection: same convention as Pass 2/3 — default per
        //    provider (no separate bootstrap model setting; D-02 pins Haiku
        //    default via `pick_model_default`).
        let model = Pass2LlmRefiner::pick_model_default(&provider);

        // 4. Format sample docs into a single user message.
        let user_content = Self::format_sample_docs(sample_docs);

        // 5. Construct AIServiceRequest. temperature=0.2 (D-02: modest
        //    creativity for domain inference, unlike Pass 3's 0.0).
        let req = AIServiceRequest {
            system_prompt: BOOTSTRAP_PROMPT.to_string(),
            messages: vec![ServiceMessage {
                role: "user".to_string(),
                content: user_content,
            }],
            max_tokens: Some(2048),
            temperature: Some(0.2),
            response_format: None,
            model_override: if model.is_empty() {
                None
            } else {
                Some(model.to_string())
            },
        };

        // 6. Call with retry: 3 attempts, exponential backoff + jitter.
        let response = ai_request_with_retry(self.auth.as_ref(), req, 3)
            .await
            .map_err(|e| AppError::Internal(format!("Bootstrap LLM call failed: {}", e)))?;

        // 7. Parse.
        let raw = parse_bootstrap_json(&response.content)?;

        // 8. Validate.
        let now = chrono_now_rfc3339();
        let seed_set: HashSet<String> = SEED_PREDICATES.iter().map(|s| s.to_string()).collect();
        let existing_count = {
            let store = self.ontology_store.lock().await;
            store.effective_predicate_names().len()
        };
        let seed = validate_bootstrap(
            raw,
            &seed_set,
            existing_count,
            &now,
            model,
            sample_docs.len() as u32,
        );

        // 9. Apply + persist. Save errors are logged but not propagated —
        //    the in-memory OntologyStore already has the seed applied, so
        //    the effective vocabulary is correct for the rest of this
        //    session even if the sidecar write failed.
        {
            let mut store = self.ontology_store.lock().await;
            store.apply_bootstrap(seed.clone(), &now);
            if let Err(e) = store.save(&self.app_data_dir) {
                eprintln!(
                    "[ontology_bootstrap] failed to persist ontology.json after bootstrap: {} (continuing)",
                    e
                );
            }
        }

        // 10. Return the applied seed.
        Ok(Some(seed))
    }
}

/// RFC3339 timestamp helper (no external time-parsing dependency needed —
/// matches the `now_rfc3339` convention used throughout Phase 11.5/11.6).
fn chrono_now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Lightweight RFC3339-ish stamp (seconds resolution) — matches the
    // pattern already used by other sidecar stores in this codebase that
    // avoid pulling in the `chrono` crate purely for "now" formatting.
    httpdate_like_rfc3339(now.as_secs())
}

/// Convert a Unix timestamp (seconds) into an RFC3339 UTC string without
/// pulling in `chrono` — simple civil-from-days algorithm (Howard Hinnant's
/// `civil_from_days`).
fn httpdate_like_rfc3339(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (hour, min, sec) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hour, min, sec
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BootstrapSeed as TypesBootstrapSeed;

    fn seed_set() -> HashSet<String> {
        SEED_PREDICATES.iter().map(|s| s.to_string()).collect()
    }

    // ── parse_bootstrap_json ──────────────────────────────────────────────

    #[test]
    fn test_parse_bootstrap_json_direct() {
        let raw = r#"{"predicates":[{"name":"registered_to","description":"vehicle-registration owner","subjectClass":"Location","objectClass":"Person"}],"entitySubclasses":[{"class":"Location","subclass":"apartment","example":"Complex-Unit-204"}]}"#;
        let result = parse_bootstrap_json(raw);
        assert!(result.is_ok(), "direct parse should succeed: {:?}", result);
        let out = result.unwrap();
        assert_eq!(out.predicates.len(), 1);
        assert_eq!(out.predicates[0].name, "registered_to");
        assert_eq!(out.entity_subclasses.len(), 1);
        assert_eq!(out.entity_subclasses[0].class, "Location");
    }

    #[test]
    fn test_parse_bootstrap_json_fence_stripped() {
        let raw = "```json\n{\"predicates\":[],\"entitySubclasses\":[]}\n```";
        let result = parse_bootstrap_json(raw);
        assert!(result.is_ok(), "fence-stripped parse should succeed: {:?}", result);
        let out = result.unwrap();
        assert!(out.predicates.is_empty());
        assert!(out.entity_subclasses.is_empty());
    }

    #[test]
    fn test_parse_bootstrap_json_partial_defaults() {
        let raw = r#"{"predicates": []}"#;
        let result = parse_bootstrap_json(raw);
        assert!(result.is_ok(), "partial JSON should default missing fields: {:?}", result);
        let out = result.unwrap();
        assert!(out.predicates.is_empty());
        assert!(out.entity_subclasses.is_empty(), "missing entitySubclasses must default to empty");
    }

    // ── validate_bootstrap ────────────────────────────────────────────────

    #[test]
    fn test_validate_drops_seed_dups() {
        let raw = RawBootstrapOutput {
            predicates: vec![
                RawBootstrapPredicate {
                    name: "owns".to_string(), // seed dup
                    description: "d".to_string(),
                    subject_class: None,
                    object_class: None,
                },
                RawBootstrapPredicate {
                    name: "registered_to".to_string(),
                    description: "d".to_string(),
                    subject_class: None,
                    object_class: None,
                },
            ],
            entity_subclasses: vec![],
        };
        let seed = validate_bootstrap(raw, &seed_set(), 21, "2026-07-10T00:00:00Z", "test", 30);
        assert_eq!(seed.predicates.len(), 1);
        assert_eq!(seed.predicates[0].name, "registered_to");
    }

    #[test]
    fn test_validate_drops_bad_names() {
        let raw = RawBootstrapOutput {
            predicates: vec![
                RawBootstrapPredicate {
                    name: "".to_string(),
                    description: "d".to_string(),
                    subject_class: None,
                    object_class: None,
                },
                RawBootstrapPredicate {
                    name: "CamelCase".to_string(),
                    description: "d".to_string(),
                    subject_class: None,
                    object_class: None,
                },
                RawBootstrapPredicate {
                    name: "snake_case_ok".to_string(),
                    description: "d".to_string(),
                    subject_class: None,
                    object_class: None,
                },
            ],
            entity_subclasses: vec![],
        };
        let seed = validate_bootstrap(raw, &seed_set(), 21, "2026-07-10T00:00:00Z", "test", 30);
        assert_eq!(seed.predicates.len(), 1);
        assert_eq!(seed.predicates[0].name, "snake_case_ok");
    }

    #[test]
    fn test_validate_dedups_within_batch() {
        let raw = RawBootstrapOutput {
            predicates: vec![
                RawBootstrapPredicate {
                    name: "neighbor_of".to_string(),
                    description: "d1".to_string(),
                    subject_class: None,
                    object_class: None,
                },
                RawBootstrapPredicate {
                    name: "neighbor_of".to_string(),
                    description: "d2".to_string(),
                    subject_class: None,
                    object_class: None,
                },
            ],
            entity_subclasses: vec![],
        };
        let seed = validate_bootstrap(raw, &seed_set(), 21, "2026-07-10T00:00:00Z", "test", 30);
        assert_eq!(seed.predicates.len(), 1, "duplicate names within batch must dedup");
    }

    #[test]
    fn test_validate_caps_at_remaining_headroom() {
        let predicates: Vec<RawBootstrapPredicate> = (0..20)
            .map(|i| RawBootstrapPredicate {
                name: format!("pred_{}", i),
                description: "d".to_string(),
                subject_class: None,
                object_class: None,
            })
            .collect();
        let raw = RawBootstrapOutput {
            predicates,
            entity_subclasses: vec![],
        };
        // existing_count=195 → headroom = 200 - 195 = 5
        let seed = validate_bootstrap(raw, &seed_set(), 195, "2026-07-10T00:00:00Z", "test", 30);
        assert_eq!(seed.predicates.len(), 5, "only 5 of 20 should survive to keep total <= 200");
    }

    #[test]
    fn test_validate_drops_bad_entity_class() {
        let raw = RawBootstrapOutput {
            predicates: vec![],
            entity_subclasses: vec![RawBootstrapSubclass {
                class: "Alien".to_string(),
                subclass: "invader".to_string(),
                example: None,
            }],
        };
        let seed = validate_bootstrap(raw, &seed_set(), 21, "2026-07-10T00:00:00Z", "test", 30);
        assert!(seed.entity_subclasses.is_empty(), "invalid class must be dropped");
    }

    #[test]
    fn test_validate_keeps_valid_subclass() {
        let raw = RawBootstrapOutput {
            predicates: vec![],
            entity_subclasses: vec![RawBootstrapSubclass {
                class: "Location".to_string(),
                subclass: "apartment".to_string(),
                example: Some("Complex-Unit-204".to_string()),
            }],
        };
        let seed = validate_bootstrap(raw, &seed_set(), 21, "2026-07-10T00:00:00Z", "test", 30);
        assert_eq!(seed.entity_subclasses.len(), 1);
        assert_eq!(seed.entity_subclasses[0].class, "Location");
        assert_eq!(seed.entity_subclasses[0].subclass, "apartment");
    }

    // ── OntologyBootstrapper.bootstrap ──────────────────────────────────────

    #[tokio::test]
    async fn test_bootstrap_no_provider_returns_ok_none() {
        let dir = tempfile::tempdir().unwrap();
        let auth = Arc::new(AuthState::new(&dir.path().to_path_buf()));
        let store = Arc::new(Mutex::new(OntologyStore::default()));
        let bootstrapper = OntologyBootstrapper::new(auth, store, dir.path().to_path_buf());

        let result = bootstrapper.bootstrap(&[]).await;
        assert!(result.is_ok(), "no provider must not error: {:?}", result);
        assert!(result.unwrap().is_none(), "no provider → bootstrap returns None");
    }

    #[tokio::test]
    async fn test_bootstrap_already_completed_returns_ok_none() {
        let dir = tempfile::tempdir().unwrap();
        let auth = Arc::new(AuthState::new(&dir.path().to_path_buf()));

        let mut default_store = OntologyStore::default();
        default_store.apply_bootstrap(
            TypesBootstrapSeed {
                predicates: vec![],
                entity_subclasses: vec![],
                generated_at: "2026-07-10T00:00:00Z".to_string(),
                sample_doc_count: 30,
                model_used: "test".to_string(),
            },
            "2026-07-10T00:00:00Z",
        );
        assert!(default_store.bootstrap_completed());

        let store = Arc::new(Mutex::new(default_store));
        let bootstrapper = OntologyBootstrapper::new(auth, store, dir.path().to_path_buf());

        let result = bootstrapper.bootstrap(&[]).await;
        assert!(result.is_ok(), "already completed must not error: {:?}", result);
        assert!(result.unwrap().is_none(), "already completed → bootstrap returns None without LLM call");
    }

    #[test]
    fn test_format_sample_docs_includes_all_fields() {
        let docs = vec![BootstrapSampleDoc {
            title: "Property Sale Deed 2024".to_string(),
            topic: Some("property".to_string()),
            tags: vec!["ownership".to_string(), "sale".to_string()],
            top_entities: vec![
                ("Person".to_string(), "Alex Doe".to_string()),
                ("Location".to_string(), "Complex-Unit".to_string()),
            ],
        }];
        let formatted = OntologyBootstrapper::format_sample_docs(&docs);
        assert!(formatted.contains("Documents (n=1):"));
        assert!(formatted.contains("Property Sale Deed 2024"));
        assert!(formatted.contains("topic: property"));
        assert!(formatted.contains("ownership,sale"));
        assert!(formatted.contains("Person:Alex Doe"));
        assert!(formatted.contains("Location:Complex-Unit"));
    }

    #[test]
    fn test_bootstrap_prompt_lists_all_seed_predicates() {
        for name in SEED_PREDICATES {
            assert!(
                BOOTSTRAP_PROMPT.contains(name),
                "BOOTSTRAP_PROMPT must list seed predicate '{}' in the do-not-propose block",
                name
            );
        }
    }

    #[test]
    fn test_is_valid_predicate_name() {
        assert!(is_valid_predicate_name("snake_case_ok"));
        assert!(!is_valid_predicate_name(""));
        assert!(!is_valid_predicate_name("CamelCase"));
        assert!(!is_valid_predicate_name("with-dash"));
        assert!(!is_valid_predicate_name(&"a".repeat(41)));
    }

    #[test]
    fn test_rfc3339_timestamp_format() {
        let ts = chrono_now_rfc3339();
        // Basic shape check: YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(ts.len(), 20, "expected RFC3339 second-precision length, got: {}", ts);
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.chars().nth(4), Some('-'));
        assert_eq!(ts.chars().nth(10), Some('T'));
    }
}

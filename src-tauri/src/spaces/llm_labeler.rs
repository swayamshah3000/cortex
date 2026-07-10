//! LLM Space Labeler (Phase 9, Plan 03)
//!
//! Generates `{label, description}` pairs for Smart Space clusters using the
//! active AI provider. Also provides:
//! - Collision resolution helpers (`resolve_collisions`, `apply_suffix_fallback`)
//! - Domain-expansion bootstrap (`try_bootstrap_from_nearest`) — pure-Rust cosine
//!   similarity iteration at threshold 0.75 (D-11 replacement; no ruvector crate)
//! - Canonical entity hint (`compute_canonical_entity_hint`) per D-17/D-18
//! - Progress event payload (`SpaceLabelingProgress`) per D-14
//!
//! Design decisions: D-01..D-04, D-11 (replacement), D-13, D-14, D-17, D-18.
//! Threat mitigations: T-09-01 (prompt injection via sanitizer), T-09-02 (JSON
//! fence-strip + serde schema), T-09-03 (MAX_LABEL_RETRIES bound).

use crate::ai::retry::ai_request_with_retry;
use crate::ai::service::{AIServiceRequest, ServiceMessage};
use crate::auth::AuthState;
use crate::pipeline::pass2_llm_refiner::strip_json_fences;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Constants ───────────────────────────────────────────────────────────────

/// Temperature for label generation (D-04). Low value for consistent, specific labels.
pub const LABEL_TEMPERATURE: f64 = 0.3;

/// Max retries for the LLM label call (T-09-03 — bounds LLM cost).
pub const MAX_LABEL_RETRIES: u8 = 3;

/// Prefix constant for sub-space LLM labeling (D-05, HSPC-02).
///
/// Prepended to the user-content message so the LLM understands it is producing
/// a label for a sub-group nested inside an existing parent Space. The full
/// context sentence is built dynamically in `build_sub_space_user_content`, which
/// sanitizes the parent label before injection (T-10-07 mitigation).
///
/// Format used: `"Parent Space: \"{parent_label}\"\nReturn a 2-4 word label …\n\n{base_content}"`
pub const SUB_SPACE_LABEL_PREFIX: &str = "Parent Space:";

/// System prompt for LLM cluster labeling (D-03).
///
/// Contains:
/// - 2-4 word label rule
/// - 1-sentence description rule (≤ 25 words)
/// - JSON-only output mandate (`Output ONLY valid JSON`)
/// - Forbids generic labels ("Documents", "Files")
/// - 6 few-shot examples matching Phase 8 corpus:
///   Property Tax Records, Kids School Docs, Health Insurance Claims,
///   Investment Statements, Vehicle Registration, Identity Docs
pub const SPACE_LABEL_PROMPT: &str = r#"You are an expert document collection labeler.
Given information about a cluster of personal documents, generate:
1. A 2-4 word category label (e.g., "Property Tax Records", "Kids School Docs")
2. A 1-sentence description of what the cluster contains (maximum 25 words)

Rules:
- Output ONLY valid JSON: {"label": "...", "description": "..."}
- Label must be 2-4 words, title case, specific
- Description must be 1 sentence, ≤ 25 words
- Do NOT use generic labels like "Documents" or "Files"
- The label must uniquely identify the document category

Examples:
{"label": "Property Tax Records", "description": "Municipal property tax assessments, receipts, and demand notices for residential and commercial properties."}
{"label": "Kids School Docs", "description": "School enrollment forms, progress reports, fee receipts, and extracurricular activity records for children."}
{"label": "Health Insurance Claims", "description": "Medical insurance claim forms, reimbursement letters, hospitalization bills, and policy renewal documents."}
{"label": "Investment Statements", "description": "Mutual fund folios, stock certificates, SIP confirmations, and portfolio performance statements."}
{"label": "Vehicle Registration", "description": "Vehicle purchase invoice, registration certificate, insurance policy, and emission test documents."}
{"label": "Identity Docs", "description": "Personal identification documents including Aadhaar card, PAN card, passport, and voter ID."}
"#;

// ─── Public types ─────────────────────────────────────────────────────────────

/// Parsed LLM output: a label + 1-sentence description for a Smart Space cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceLabel {
    pub label: String,
    pub description: String,
}

/// Action recommended by `resolve_collisions` for a given space.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedLabel {
    /// Final label — no collision or already uniquified.
    Keep(String),
    /// Needs LLM retry with the supplied avoid-list.
    RetryWithAvoid(Vec<String>),
}

/// Per-space progress event payload (D-14).
///
/// Emitted by the backend via Tauri's `space-labeling-progress` event.
/// Field names serialise to camelCase per the IPC convention (serde rename_all).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceLabelingProgress {
    pub space_id: String,
    /// "labeling" | "complete" | "error"
    pub status: String,
    pub processed: usize,
    pub total: usize,
    /// Present when `status == "complete"`.
    pub label: Option<String>,
    /// Present when `status == "error"`.
    pub error: Option<String>,
}

// ─── Input sanitizer (T-09-01 mitigation) ────────────────────────────────────

/// Sanitize a single text field before including it in the LLM prompt.
///
/// T-09-01 mitigation: caps input to 100 chars and strips ASCII control
/// characters to prevent prompt injection via adversarial document titles,
/// entity strings, topics, or tags.
fn sanitize_field(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(100)
        .collect()
}

// ─── User-content builder (shared by label_cluster and label_with_avoid_list) ─

/// Build the user message content for the LLM labeling call (D-01 format).
///
/// Content shape:
/// ```text
/// Cluster size: {N} documents
///
/// Top document titles:
/// 1. {title1}
/// ...
/// 20. {title20}
///
/// Entity summary: {entity_summary}
/// Top topics: {topic1, topic2, topic3}
/// Top tags: {tag1, tag2, ...}
/// ```
///
/// When `avoid` is non-empty, appends:
/// `\n\nIMPORTANT: Avoid these labels already in use: {joined}`
///
/// T-09-01: all fields sanitised via `sanitize_field` before prompt assembly.
pub(crate) fn build_user_content(
    doc_titles: &[String],
    entity_summary: &str,
    top_topics: &[String],
    top_tags: &[String],
    doc_count: usize,
    avoid: &[String],
) -> String {
    // T-09-01: sanitize all string inputs before building the prompt.
    let safe_entity_summary = sanitize_field(entity_summary);

    let titles_text: String = doc_titles
        .iter()
        .take(20)
        .enumerate()
        .map(|(i, t)| format!("{}. {}", i + 1, sanitize_field(t)))
        .collect::<Vec<_>>()
        .join("\n");

    let safe_topics: Vec<String> = top_topics.iter().map(|t| sanitize_field(t)).collect();
    let safe_tags: Vec<String> = top_tags.iter().map(|t| sanitize_field(t)).collect();

    // T-09-01: sanitize avoid-list items (LLM-generated labels from prior batch runs)
    // before injecting them into subsequent prompts. Prevents control characters or
    // prompt-injection payloads carried by a prior adversarial LLM response from
    // propagating into the avoid-suffix of collision-retry prompts.
    let avoid_joined = avoid
        .iter()
        .map(|s| sanitize_field(s))
        .collect::<Vec<_>>()
        .join(", ");
    let avoid_suffix = if avoid.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nIMPORTANT: Avoid these labels already in use: {}",
            avoid_joined
        )
    };

    format!(
        "Cluster size: {} documents\n\nTop document titles:\n{}\n\nEntity summary: {}\nTop topics: {}\nTop tags: {}{}",
        doc_count,
        titles_text,
        safe_entity_summary,
        safe_topics.join(", "),
        safe_tags.join(", "),
        avoid_suffix,
    )
}

// ─── LLM label calls ──────────────────────────────────────────────────────────

/// Call the active AI provider to generate a `{label, description}` for a cluster.
///
/// First-attempt call with NO avoid-list. For collision retries, call
/// `label_with_avoid_list` instead.
///
/// Sanitizes all inputs per T-09-01 before building the prompt.
/// Strips JSON fences from the response (T-09-02) before parsing.
pub async fn label_cluster(
    auth: &AuthState,
    model: &str,
    doc_titles: &[String],
    entity_summary: &str,
    top_topics: &[String],
    top_tags: &[String],
    doc_count: usize,
) -> Result<SpaceLabel, String> {
    label_with_avoid_list(
        auth,
        model,
        doc_titles,
        entity_summary,
        top_topics,
        top_tags,
        doc_count,
        &[],
    )
    .await
}

/// Call the active AI provider to generate a `{label, description}` for a cluster,
/// appending an avoid-list to steer the LLM away from already-used labels (D-13).
///
/// Used by `resolve_collisions` after detecting duplicate labels in a batch.
pub async fn label_with_avoid_list(
    auth: &AuthState,
    model: &str,
    doc_titles: &[String],
    entity_summary: &str,
    top_topics: &[String],
    top_tags: &[String],
    doc_count: usize,
    avoid: &[String],
) -> Result<SpaceLabel, String> {
    let user_content =
        build_user_content(doc_titles, entity_summary, top_topics, top_tags, doc_count, avoid);

    let req = AIServiceRequest {
        system_prompt: SPACE_LABEL_PROMPT.to_string(),
        messages: vec![ServiceMessage {
            role: "user".to_string(),
            content: user_content,
        }],
        max_tokens: Some(256),
        temperature: Some(LABEL_TEMPERATURE),
        response_format: None,
        model_override: if model.is_empty() { None } else { Some(model.to_string()) },
    };

    let response = ai_request_with_retry(auth, req, MAX_LABEL_RETRIES)
        .await
        .map_err(|e| format!("LLM label call failed: {}", e))?;

    // T-09-02: strip JSON fences before serde parse to handle ```json / ``` / <think> blocks.
    let stripped = strip_json_fences(&response.content);
    serde_json::from_str::<SpaceLabel>(&stripped)
        .map_err(|e| format!("LLM JSON parse failed: {}", e))
}

// ─── Sub-space labeling (Phase 10, Plan 04 — D-05, HSPC-02) ─────────────────

/// Build the user message content for sub-space LLM labeling (D-05).
///
/// Extends `build_user_content` by prepending parent-context and adding the
/// sanitized parent label to the avoid-list, so the existing Phase 9 collision-
/// retry infrastructure enforces distinctness even if the LLM ignores the
/// "distinct from" instruction.
///
/// # Security
///
/// T-10-07: `parent_label` is passed through `sanitize_field` before injection to
/// strip ASCII control characters and cap length, mitigating prompt injection via
/// adversarial parent labels produced by prior LLM runs.
///
/// # Arguments
///
/// * `parent_label` – The label of the parent Space (LLM-generated, possibly from cache).
/// * `doc_titles`   – Document filenames/titles in this sub-cluster (first 20 used).
/// * `entity_summary` – Pre-aggregated entity summary string for the sub-cluster.
/// * `top_topics`   – Top topic strings for the sub-cluster.
/// * `top_tags`     – Top tag strings for the sub-cluster.
/// * `doc_count`    – Total document count in the sub-cluster.
fn build_sub_space_user_content(
    parent_label: &str,
    doc_titles: &[String],
    entity_summary: &str,
    top_topics: &[String],
    top_tags: &[String],
    doc_count: usize,
) -> String {
    // T-10-07: sanitize parent_label before any prompt injection.
    let safe_parent = sanitize_field(parent_label);

    // Add the sanitized parent label to the avoid-list so the Phase 9
    // collision-retry path fires if the LLM returns the parent label verbatim.
    let avoid = vec![safe_parent.clone()];

    let base = build_user_content(doc_titles, entity_summary, top_topics, top_tags, doc_count, &avoid);

    // SUB_SPACE_LABEL_PREFIX ("Parent Space:") is the sentinel string; the full
    // context sentence is assembled here to embed the sanitized parent label.
    format!(
        "{} \"{}\"\nReturn a 2-4 word label that is distinct from \"{}\" and specific to this sub-group.\n\n{}",
        SUB_SPACE_LABEL_PREFIX, safe_parent, safe_parent, base
    )
}

/// Generate a `{label, description}` for a sub-cluster nested within `parent_label`.
///
/// Reuses the Phase 9 labeling pipeline without modification:
/// - System prompt: `SPACE_LABEL_PROMPT` (6 few-shot exemplars — **DO NOT modify**).
/// - Retry policy: `MAX_LABEL_RETRIES` / `ai_request_with_retry`.
/// - JSON fence-strip + serde parse: same as `label_with_avoid_list`.
///
/// The only Phase 10 delta is the user-content prefix injected by
/// `build_sub_space_user_content`: parent context + parent label in avoid-list.
///
/// # Decisions
///
/// - D-05: reuse `LlmSpaceLabeler` with prompt variant (no new HTTP path, no new
///   retry policy, no new prompt template).
/// - HSPC-02: sub-space labels are LLM-generated, corpus-derived, parent-aware.
///
/// # Security
///
/// - T-10-07: `parent_label` sanitized via `sanitize_field` inside
///   `build_sub_space_user_content` before prompt injection.
/// - T-10-08: call volume bounded externally (SUB_SPACE_THRESHOLD gate, per-parent
///   sub-cluster count, fingerprint cache) — no per-call guard needed here.
///
/// # Errors
///
/// Returns `Err(String)` if the AI provider call fails or JSON parse fails.
/// Callers (Plan 05 manager recluster pass) should log and continue with other
/// sub-clusters rather than aborting the entire recluster.
pub async fn label_sub_cluster(
    auth: &AuthState,
    model: &str,
    parent_label: &str,
    doc_titles: &[String],
    entity_summary: &str,
    top_topics: &[String],
    top_tags: &[String],
    doc_count: usize,
) -> Result<SpaceLabel, String> {
    let user_content = build_sub_space_user_content(
        parent_label,
        doc_titles,
        entity_summary,
        top_topics,
        top_tags,
        doc_count,
    );

    let req = AIServiceRequest {
        system_prompt: SPACE_LABEL_PROMPT.to_string(),
        messages: vec![ServiceMessage {
            role: "user".to_string(),
            content: user_content,
        }],
        max_tokens: Some(256),
        temperature: Some(LABEL_TEMPERATURE),
        response_format: None,
        model_override: if model.is_empty() { None } else { Some(model.to_string()) },
    };

    let response = ai_request_with_retry(auth, req, MAX_LABEL_RETRIES)
        .await
        .map_err(|e| format!("LLM sub-label call failed: {}", e))?;

    // T-09-02 (reused): strip JSON fences before serde parse.
    let stripped = strip_json_fences(&response.content);
    serde_json::from_str::<SpaceLabel>(&stripped)
        .map_err(|e| format!("LLM sub-label JSON parse failed: {}", e))
}

// ─── Collision resolution ──────────────────────────────────────────────────────

/// Scan a batch of proposed labels and produce a `ResolvedLabel` for each space.
///
/// Collision definition (09-RESEARCH.md Open Question #3): labels are compared
/// case-insensitively after `trim()` and whitespace normalisation. Original-case
/// labels are stored and returned unchanged.
///
/// `proposed`: slice of `(space_id, label)` pairs from the labeling batch.
///
/// Returns `(space_id, ResolvedLabel)` pairs. Non-colliding spaces get `Keep`.
/// Colliding spaces get `RetryWithAvoid(all_other_labels_in_batch)`.
///
/// The caller (Plan 04's manager loop) invokes `resolve_collisions` a second
/// time to check if retry-labels still collide, then calls `apply_suffix_fallback`
/// after 2 retry rounds (D-13).
pub fn resolve_collisions(proposed: &[(String, String)]) -> Vec<(String, ResolvedLabel)> {
    // Normalise: trim + collapse internal whitespace + lowercase.
    let normalise = |s: &str| -> String {
        s.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };

    // Build a map: normalised_label → [indices with that label]
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, (_space_id, label)) in proposed.iter().enumerate() {
        groups.entry(normalise(label)).or_default().push(idx);
    }

    let mut result = Vec::with_capacity(proposed.len());

    for (idx, (space_id, label)) in proposed.iter().enumerate() {
        let norm = normalise(label);
        let siblings = &groups[&norm];
        if siblings.len() == 1 {
            // Unique label — keep it.
            result.push((space_id.clone(), ResolvedLabel::Keep(label.clone())));
        } else {
            // Collision — collect all OTHER labels in the batch as the avoid-list.
            let avoid: Vec<String> = proposed
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != idx)
                .map(|(_, (_, l))| l.clone())
                .collect();
            result.push((space_id.clone(), ResolvedLabel::RetryWithAvoid(avoid)));
        }
    }

    result
}

/// Apply the suffix fallback when a collision cannot be resolved by LLM retry (D-13).
///
/// Format: `"{base} — {suffix_entity_value}"` (em-dash U+2014 + space).
/// Example: `apply_suffix_fallback("Work Docs", "Freelance")` → `"Work Docs — Freelance"`.
/// Plan 04 calls this after 2 retry rounds fail.
pub fn apply_suffix_fallback(base: &str, suffix_entity_value: &str) -> String {
    format!("{} \u{2014} {}", base, suffix_entity_value)
}

// ─── Domain-expansion bootstrap (D-11 replacement) ────────────────────────────

/// Try to bootstrap a label for a new cluster from the nearest existing labeled space.
///
/// Pure-Rust implementation replacing `ruvector-domain-expansion` per
/// 09-RESEARCH.md's critical finding: that crate is a meta-learning framework,
/// not a centroid-similarity label-transfer tool.
///
/// Returns `Some(label)` when the best cosine similarity is ≥ 0.75 (D-11 threshold).
/// Returns `None` otherwise (caller triggers a full LLM label call).
/// Returns `None` immediately on empty `labeled_spaces` (guards against uninitialised
/// `best_sim = f32::NEG_INFINITY` leaking through the ≥ 0.75 check).
pub fn try_bootstrap_from_nearest(
    new_centroid: &[f32],
    labeled_spaces: &[(String, String, Vec<f32>)], // (space_id, label, centroid)
) -> Option<String> {
    if labeled_spaces.is_empty() {
        return None;
    }

    let mut best_sim = f32::NEG_INFINITY;
    let mut best_label: Option<&str> = None;

    for (_space_id, label, centroid) in labeled_spaces {
        let sim = crate::spaces::clustering::cosine_similarity(new_centroid, centroid);
        if sim > best_sim {
            best_sim = sim;
            best_label = Some(label.as_str());
        }
    }

    if best_sim >= 0.75 {
        best_label.map(String::from)
    } else {
        None
    }
}

// ─── Canonical entity hint (D-17/D-18) ────────────────────────────────────────

/// Compute the canonical entity hint for a Smart Space (D-17).
///
/// `entity_counts` keys use format `"ClassName: value"` (e.g., `"Person: Alex Doe"`).
/// Plan 04's caller aggregates entities into this shape.
///
/// Returns `Some("{ClassName}: {value}")` when the top-count entity represents
/// ≥ ⌈doc_count / 5⌉ of `doc_count` (≥ 20% per D-18 scope guard).
/// Ties broken by lexicographic key order (deterministic).
///
/// Returns `None` when:
/// - `entity_counts` is empty
/// - `doc_count` is 0
/// - The top-count entity falls below the 20% threshold
pub fn compute_canonical_entity_hint(
    entity_counts: &HashMap<String, usize>,
    doc_count: usize,
) -> Option<String> {
    if entity_counts.is_empty() || doc_count == 0 {
        return None;
    }

    // Find top entry by count; ties broken lexicographically in reverse (deterministic).
    let (top_key, top_count) = entity_counts
        .iter()
        .max_by(|(k1, c1), (k2, c2)| c1.cmp(c2).then(k1.cmp(k2).reverse()))
        .unwrap(); // safe: entity_counts is non-empty

    // D-18: threshold = ⌈doc_count / 5⌉ (equivalent to integer ceiling division).
    let threshold = (doc_count + 4) / 5;
    if *top_count >= threshold {
        Some(top_key.clone())
    } else {
        None
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::pass2_llm_refiner::strip_json_fences;
    use serde_json::Value;

    // ── Task 1: Prompt schema tests ───────────────────────────────────────────

    #[test]
    fn test_prompt_contains_property_tax_exemplar() {
        assert!(
            SPACE_LABEL_PROMPT.contains("Property Tax Records"),
            "SPACE_LABEL_PROMPT must contain 'Property Tax Records' few-shot exemplar"
        );
    }

    #[test]
    fn test_prompt_contains_kids_school_exemplar() {
        assert!(
            SPACE_LABEL_PROMPT.contains("Kids School Docs"),
            "SPACE_LABEL_PROMPT must contain 'Kids School Docs' few-shot exemplar"
        );
    }

    #[test]
    fn test_prompt_contains_health_insurance_exemplar() {
        assert!(
            SPACE_LABEL_PROMPT.contains("Health Insurance Claims"),
            "SPACE_LABEL_PROMPT must contain 'Health Insurance Claims' few-shot exemplar"
        );
    }

    #[test]
    fn test_prompt_contains_investment_statements_exemplar() {
        assert!(
            SPACE_LABEL_PROMPT.contains("Investment Statements"),
            "SPACE_LABEL_PROMPT must contain 'Investment Statements' few-shot exemplar"
        );
    }

    #[test]
    fn test_prompt_contains_vehicle_registration_exemplar() {
        assert!(
            SPACE_LABEL_PROMPT.contains("Vehicle Registration"),
            "SPACE_LABEL_PROMPT must contain 'Vehicle Registration' few-shot exemplar"
        );
    }

    #[test]
    fn test_prompt_contains_identity_docs_exemplar() {
        assert!(
            SPACE_LABEL_PROMPT.contains("Identity Docs"),
            "SPACE_LABEL_PROMPT must contain 'Identity Docs' few-shot exemplar"
        );
    }

    #[test]
    fn test_prompt_forbids_generic_labels() {
        assert!(
            SPACE_LABEL_PROMPT.contains("Do NOT use generic labels"),
            "SPACE_LABEL_PROMPT must contain prohibition of generic labels"
        );
    }

    #[test]
    fn test_prompt_mandates_json_only_output() {
        assert!(
            SPACE_LABEL_PROMPT.contains("Output ONLY valid JSON"),
            "SPACE_LABEL_PROMPT must contain 'Output ONLY valid JSON'"
        );
    }

    // ── Task 1: SpaceLabel deserialization tests ──────────────────────────────

    #[test]
    fn test_space_label_deserializes_plain_json() {
        let json = r#"{"label":"Property Tax Records","description":"Municipal tax documents."}"#;
        let label: SpaceLabel = serde_json::from_str(json).unwrap();
        assert_eq!(label.label, "Property Tax Records");
        assert_eq!(label.description, "Municipal tax documents.");
    }

    #[test]
    fn test_space_label_deserializes_fence_wrapped_json() {
        let json = "```json\n{\"label\":\"Kids School Docs\",\"description\":\"School records.\"}\n```";
        let stripped = strip_json_fences(json);
        let label: SpaceLabel = serde_json::from_str(&stripped).unwrap();
        assert_eq!(label.label, "Kids School Docs");
        assert_eq!(label.description, "School records.");
    }

    // ── Task 1: build_user_content tests ─────────────────────────────────────

    #[test]
    fn test_build_user_content_no_avoid_list() {
        let titles = vec!["Doc A".to_string(), "Doc B".to_string()];
        let content = build_user_content(
            &titles,
            "Person: 5, Date: 10",
            &["taxes".to_string()],
            &["property".to_string()],
            2,
            &[],
        );
        assert!(content.contains("Cluster size: 2 documents"));
        assert!(content.contains("1. Doc A"));
        assert!(content.contains("2. Doc B"));
        assert!(content.contains("Entity summary: Person: 5, Date: 10"));
        assert!(content.contains("Top topics: taxes"));
        assert!(content.contains("Top tags: property"));
        assert!(
            !content.contains("IMPORTANT: Avoid"),
            "Empty avoid list must NOT inject avoid text"
        );
    }

    #[test]
    fn test_build_user_content_with_avoid_list() {
        let titles = vec!["Invoice 2025".to_string()];
        let avoid = vec!["Work Docs".to_string(), "Property".to_string()];
        let content = build_user_content(&titles, "Amount: 3", &[], &[], 1, &avoid);
        assert!(
            content.contains("IMPORTANT: Avoid these labels already in use: Work Docs, Property"),
            "Avoid-list injection must include all items"
        );
    }

    #[test]
    fn test_build_user_content_empty_inputs_no_panic() {
        let content = build_user_content(&[], "", &[], &[], 0, &[]);
        assert!(content.contains("Cluster size: 0 documents"));
    }

    #[test]
    fn test_sanitize_strips_control_chars() {
        // Test via build_user_content (sanitize_field is private).
        let titles = vec!["Bad\x01Title".to_string()];
        let content = build_user_content(&titles, "", &[], &[], 1, &[]);
        assert!(!content.contains('\x01'), "Control chars must be stripped from titles");
        assert!(content.contains("BadTitle"), "Non-control chars must be preserved");
    }

    #[test]
    fn test_sanitize_caps_at_100_chars() {
        let long_title = "a".repeat(200);
        let titles = vec![long_title];
        let content = build_user_content(&titles, "", &[], &[], 1, &[]);
        assert!(content.contains(&"a".repeat(100)), "Must keep exactly 100 chars of title");
        assert!(!content.contains(&"a".repeat(101)), "Must not keep 101+ chars of title");
    }

    #[test]
    fn test_build_user_content_sanitizes_titles() {
        let titles = vec!["Bad\x01Title".to_string()];
        let content = build_user_content(&titles, "", &[], &[], 1, &[]);
        assert!(!content.contains('\x01'), "Control chars must be stripped from titles");
        assert!(content.contains("BadTitle"), "Non-control chars must be preserved");
    }

    // ── Task 2: resolve_collisions tests ─────────────────────────────────────

    #[test]
    fn test_resolve_collisions_no_collision() {
        let proposed = vec![
            ("s1".to_string(), "Work Docs".to_string()),
            ("s2".to_string(), "Property Tax Records".to_string()),
        ];
        let result = resolve_collisions(&proposed);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ("s1".to_string(), ResolvedLabel::Keep("Work Docs".to_string())));
        assert_eq!(
            result[1],
            (
                "s2".to_string(),
                ResolvedLabel::Keep("Property Tax Records".to_string())
            )
        );
    }

    #[test]
    fn test_resolve_collisions_detects_case_insensitive_collision() {
        let proposed = vec![
            ("s1".to_string(), "Work Docs".to_string()),
            ("s2".to_string(), "work docs".to_string()),
        ];
        let result = resolve_collisions(&proposed);
        for (_, resolved) in &result {
            assert!(
                matches!(resolved, ResolvedLabel::RetryWithAvoid(_)),
                "Case-insensitive collision must trigger RetryWithAvoid"
            );
        }
    }

    #[test]
    fn test_resolve_collisions_avoid_list_contains_other_labels() {
        let proposed = vec![
            ("s1".to_string(), "Work Docs".to_string()),
            ("s2".to_string(), "Work Docs".to_string()),
        ];
        let result = resolve_collisions(&proposed);
        for (_, resolved) in &result {
            if let ResolvedLabel::RetryWithAvoid(avoid) = resolved {
                assert!(avoid.contains(&"Work Docs".to_string()));
            }
        }
    }

    #[test]
    fn test_resolve_collisions_three_way_collision() {
        let proposed = vec![
            ("s1".to_string(), "Work Docs".to_string()),
            ("s2".to_string(), "work docs".to_string()),
            ("s3".to_string(), "WORK DOCS".to_string()),
        ];
        let result = resolve_collisions(&proposed);
        assert_eq!(result.len(), 3);
        for (_, resolved) in &result {
            assert!(matches!(resolved, ResolvedLabel::RetryWithAvoid(_)));
        }
    }

    // ── Task 2: apply_suffix_fallback tests ──────────────────────────────────

    #[test]
    fn test_apply_suffix_fallback_format() {
        let result = apply_suffix_fallback("Work Docs", "Freelance");
        assert_eq!(result, "Work Docs \u{2014} Freelance");
    }

    #[test]
    fn test_apply_suffix_fallback_uses_em_dash() {
        let result = apply_suffix_fallback("Property Tax Records", "AlphaComplex");
        assert!(result.contains('\u{2014}'), "Must use em-dash (U+2014)");
    }

    // ── Task 2: try_bootstrap_from_nearest tests ──────────────────────────────

    #[test]
    fn test_bootstrap_returns_label_above_threshold() {
        // [1.0, 0.0] · [0.9, 0.44] / (1 * ~1) = 0.9 / ~0.9997 ≈ 0.9 (≥ 0.75)
        let new_centroid = vec![1.0_f32, 0.0];
        let labeled = vec![(
            "s1".to_string(),
            "Property Tax Records".to_string(),
            vec![0.9_f32, 0.44],
        )];
        let result = try_bootstrap_from_nearest(&new_centroid, &labeled);
        assert_eq!(result, Some("Property Tax Records".to_string()));
    }

    #[test]
    fn test_bootstrap_returns_none_below_threshold() {
        // [1.0, 0.0] vs [0.0, 1.0] — cosine = 0.0 (< 0.75) → None
        let new_centroid = vec![1.0_f32, 0.0];
        let labeled = vec![(
            "s1".to_string(),
            "Kids School Docs".to_string(),
            vec![0.0_f32, 1.0],
        )];
        let result = try_bootstrap_from_nearest(&new_centroid, &labeled);
        assert_eq!(result, None);
    }

    #[test]
    fn test_bootstrap_returns_none_on_empty_list() {
        let new_centroid = vec![1.0_f32, 0.0];
        let labeled: Vec<(String, String, Vec<f32>)> = vec![];
        let result = try_bootstrap_from_nearest(&new_centroid, &labeled);
        assert_eq!(result, None, "Empty list must return None immediately");
    }

    #[test]
    fn test_bootstrap_selects_nearest_of_multiple() {
        let new_centroid = vec![1.0_f32, 0.0];
        let labeled = vec![
            (
                "s1".to_string(),
                "Work Docs".to_string(),
                vec![0.6_f32, 0.8], // lower cosine
            ),
            (
                "s2".to_string(),
                "Property Tax Records".to_string(),
                vec![1.0_f32, 0.0], // cosine = 1.0
            ),
        ];
        let result = try_bootstrap_from_nearest(&new_centroid, &labeled);
        assert_eq!(result, Some("Property Tax Records".to_string()));
    }

    #[test]
    fn test_bootstrap_threshold_exact_boundary() {
        // Construct vectors with cosine exactly 0.75.
        // [1.0, 0.0] · [0.75, sin_val] = 0.75; norm([0.75, sin_val]) = 1.0 (unit vec)
        let sin_val = (1.0_f32 - 0.75_f32 * 0.75_f32).sqrt();
        let new_centroid = vec![1.0_f32, 0.0];
        let labeled = vec![(
            "s1".to_string(),
            "Vehicle Registration".to_string(),
            vec![0.75_f32, sin_val],
        )];
        let result = try_bootstrap_from_nearest(&new_centroid, &labeled);
        // cosine = 0.75 >= 0.75 → Some
        assert_eq!(result, Some("Vehicle Registration".to_string()));
    }

    // ── Task 2: compute_canonical_entity_hint tests ───────────────────────────

    #[test]
    fn test_entity_hint_returns_dominant_entity() {
        let mut counts = HashMap::new();
        counts.insert("Person: Alex Doe".to_string(), 15_usize);
        counts.insert("Organization: TechCorp".to_string(), 3_usize);
        // 15 / 20 = 75% > 20% threshold → returns Some
        let result = compute_canonical_entity_hint(&counts, 20);
        assert_eq!(result, Some("Person: Alex Doe".to_string()));
    }

    #[test]
    fn test_entity_hint_returns_none_below_threshold() {
        let mut counts = HashMap::new();
        counts.insert("Person: Alice".to_string(), 3_usize);
        counts.insert("Organization: FooCorp".to_string(), 2_usize);
        // 3 / 50 = 6% < 20% threshold → returns None
        let result = compute_canonical_entity_hint(&counts, 50);
        assert_eq!(result, None);
    }

    #[test]
    fn test_entity_hint_returns_none_on_empty_map() {
        let counts: HashMap<String, usize> = HashMap::new();
        let result = compute_canonical_entity_hint(&counts, 10);
        assert_eq!(result, None);
    }

    #[test]
    fn test_entity_hint_returns_none_on_zero_doc_count() {
        let mut counts = HashMap::new();
        counts.insert("Person: Bob".to_string(), 5_usize);
        let result = compute_canonical_entity_hint(&counts, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_entity_hint_exact_20_percent_threshold() {
        let mut counts = HashMap::new();
        // ⌈10 / 5⌉ = 2. count=2 must meet threshold → Some.
        counts.insert("Person: Eve".to_string(), 2_usize);
        let result = compute_canonical_entity_hint(&counts, 10);
        assert_eq!(
            result,
            Some("Person: Eve".to_string()),
            "count == threshold must return Some"
        );
    }

    #[test]
    fn test_entity_hint_below_20_percent_threshold() {
        let mut counts = HashMap::new();
        // ⌈10 / 5⌉ = 2. count=1 is below threshold → None.
        counts.insert("Person: Eve".to_string(), 1_usize);
        let result = compute_canonical_entity_hint(&counts, 10);
        assert_eq!(result, None, "count < threshold must return None");
    }

    #[test]
    fn test_entity_hint_tie_broken_lexicographically() {
        let mut counts = HashMap::new();
        counts.insert("Person: Alice".to_string(), 5_usize);
        counts.insert("Person: Bob".to_string(), 5_usize);
        // Both have count 5; ⌈10/5⌉ = 2, so threshold met for both.
        let result = compute_canonical_entity_hint(&counts, 10);
        assert!(
            result.is_some(),
            "Tied counts with sufficient threshold must return Some"
        );
    }

    // ── Task 2: SpaceLabelingProgress serde tests ─────────────────────────────

    #[test]
    fn test_progress_event_serializes_camel_case() {
        let progress = SpaceLabelingProgress {
            space_id: "space-abc".to_string(),
            status: "complete".to_string(),
            processed: 3,
            total: 10,
            label: Some("Property Tax Records".to_string()),
            error: None,
        };
        let value: Value = serde_json::to_value(&progress).unwrap();
        assert!(value.get("spaceId").is_some(), "spaceId must be camelCase");
        assert!(
            value.get("processed").is_some(),
            "'processed' field must be present"
        );
        assert!(value.get("total").is_some(), "'total' field must be present");
        assert!(value.get("label").is_some(), "'label' field must be present");
        assert!(value.get("error").is_some(), "'error' field must be present");
        assert!(
            value.get("space_id").is_none(),
            "snake_case 'space_id' must NOT appear in serialised output"
        );
    }

    #[test]
    fn test_progress_event_deserializes_camel_case() {
        let json = r#"{"spaceId":"s1","status":"labeling","processed":1,"total":5,"label":null,"error":null}"#;
        let progress: SpaceLabelingProgress = serde_json::from_str(json).unwrap();
        assert_eq!(progress.space_id, "s1");
        assert_eq!(progress.status, "labeling");
        assert_eq!(progress.processed, 1);
        assert_eq!(progress.total, 5);
        assert!(progress.label.is_none());
        assert!(progress.error.is_none());
    }

    #[test]
    fn test_progress_event_clone_works() {
        let progress = SpaceLabelingProgress {
            space_id: "s1".to_string(),
            status: "labeling".to_string(),
            processed: 0,
            total: 5,
            label: None,
            error: None,
        };
        let cloned = progress.clone();
        assert_eq!(cloned.space_id, progress.space_id);
    }

    // ── Phase 10, Plan 04: label_sub_cluster / build_sub_space_user_content tests ─

    /// Test 1 (D-05, HSPC-02): the sub-space user content must contain the exact
    /// string "Parent Space:" followed by the parent label so the LLM understands
    /// it is producing a sub-label for a known parent context.
    #[test]
    fn test_sub_space_prompt_includes_parent_context() {
        let titles = vec!["Invoice 2024.pdf".to_string(), "Receipt Jan.pdf".to_string()];
        let content = build_sub_space_user_content(
            "Property Tax Records",
            &titles,
            "Date: 5, Amount: 3",
            &["taxes".to_string()],
            &["property".to_string()],
            2,
        );
        assert!(
            content.contains("Parent Space:"),
            "Sub-space user content must contain 'Parent Space:'"
        );
        assert!(
            content.contains("Property Tax Records"),
            "Sub-space user content must contain the parent label"
        );
    }

    /// Test 2 (T-10-07): a parent_label containing a control character must be
    /// sanitized (stripped) before injection into the LLM prompt — mitigates
    /// prompt injection via adversarial parent labels.
    #[test]
    fn test_sub_space_prompt_sanitizes_parent_label() {
        let titles = vec!["Doc A.pdf".to_string()];
        let content = build_sub_space_user_content(
            "Foo\x00Injected",
            &titles,
            "",
            &[],
            &[],
            1,
        );
        assert!(
            !content.as_bytes().contains(&0u8),
            "Control byte (NUL) must be stripped from parent_label before injection"
        );
        // The sanitized portion (without control byte) must still appear.
        assert!(
            content.contains("FooInjected"),
            "Non-control chars must be preserved in sanitized parent_label"
        );
    }

    /// Test 3 (D-05 research refinement): the avoid-list produced internally by
    /// build_sub_space_user_content must contain the parent label so the existing
    /// Phase 9 collision-retry infrastructure fires if the LLM ignores the
    /// "distinct from" instruction.
    #[test]
    fn test_sub_space_avoid_list_contains_parent() {
        let titles = vec!["School Report.pdf".to_string()];
        let content = build_sub_space_user_content(
            "Kids School Docs",
            &titles,
            "",
            &[],
            &[],
            1,
        );
        // The avoid-list is injected into the user content by build_user_content
        // when the avoid slice is non-empty. Verify the parent label appears in
        // the "IMPORTANT: Avoid" section.
        assert!(
            content.contains("IMPORTANT: Avoid these labels already in use:"),
            "build_sub_space_user_content must inject the avoid-list suffix"
        );
        assert!(
            content.contains("Kids School Docs"),
            "The parent label must appear in the avoid-list suffix"
        );
    }
}

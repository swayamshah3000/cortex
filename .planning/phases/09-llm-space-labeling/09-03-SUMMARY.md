---
phase: 09-llm-space-labeling
plan: "03"
subsystem: spaces/llm-labeling
tags: [llm, spaces, labeling, collision-resolution, domain-expansion, entity-hint, tdd]
dependency_graph:
  requires: [09-01, 09-02]
  provides: [llm_labeler module, SpaceLabel, SpaceLabelingProgress, resolve_collisions, try_bootstrap_from_nearest, compute_canonical_entity_hint]
  affects: [spaces/manager.rs (Plan 04), space-labeling IPC commands (Plan 04)]
tech_stack:
  added: []
  patterns:
    - "TDD RED/GREEN commits for all pure functions"
    - "build_user_content() as pub(crate) helper enabling test-time assertion without LLM"
    - "Input sanitizer (T-09-01): control-char strip + 100-char cap per prompt field"
    - "Pure-Rust cosine similarity bootstrap replaces ruvector-domain-expansion (RESEARCH critical finding)"
    - "Case-insensitive whitespace-normalised collision detection (Open Question #3 resolution)"
key_files:
  created:
    - src-tauri/src/spaces/llm_labeler.rs
  modified:
    - src-tauri/src/spaces/mod.rs
decisions:
  - "ruvector-domain-expansion NOT added (RESEARCH verified: meta-learning framework, not label-transfer); pure cosine similarity at 0.75 threshold used instead"
  - "resolve_collisions uses trim+lowercase+whitespace-normalise for collision detection; original-case label preserved in output"
  - "build_user_content exported as pub(crate) to enable unit tests without network/AuthState dependency"
  - "Ceiling division (doc_count + 4) / 5 for 20% threshold (D-18)"
  - "Em-dash U+2014 used in apply_suffix_fallback (no plain hyphen)"
metrics:
  duration: "507s (~8.5 min)"
  completed: "2026-07-04"
  tasks_completed: 2
  files_changed: 2
---

# Phase 9 Plan 03: LlmSpaceLabeler Summary

LLM space labeler with 6-exemplar prompt, JSON fence-strip parsing, collision resolution, pure-Rust domain-expansion bootstrap, canonical entity hint, and camelCase progress events.

## What Was Built

**`src-tauri/src/spaces/llm_labeler.rs`** — New module delivering all Plan 03 requirements:

### Public API (12 exported symbols)

| Symbol | Kind | Purpose |
|--------|------|---------|
| `SPACE_LABEL_PROMPT` | `const &str` | 6-exemplar system prompt with JSON-only mandate and generic-label prohibition |
| `LABEL_TEMPERATURE` | `const f64` | 0.3 — determinism-oriented temperature for cluster labeling (D-04) |
| `MAX_LABEL_RETRIES` | `const u8` | 3 — bounds LLM cost per T-09-03 |
| `SpaceLabel` | `struct` | `{label: String, description: String}` parsed from LLM JSON response |
| `ResolvedLabel` | `enum` | `Keep(String)` or `RetryWithAvoid(Vec<String>)` per collision round |
| `SpaceLabelingProgress` | `struct` | `#[serde(rename_all = "camelCase")]` progress event payload (D-14) |
| `label_cluster()` | `async fn` | First-attempt LLM call (no avoid-list) |
| `label_with_avoid_list()` | `async fn` | Collision-retry LLM call with avoid-list injection (D-13) |
| `resolve_collisions()` | `fn` | Case-insensitive collision scan → Keep/RetryWithAvoid per space |
| `apply_suffix_fallback()` | `fn` | Em-dash suffix: `"Work Docs — Freelance"` (D-13 last resort) |
| `try_bootstrap_from_nearest()` | `fn` | Pure-Rust cosine bootstrap at threshold 0.75 (D-11 replacement) |
| `compute_canonical_entity_hint()` | `fn` | 20% dominance guard → `Some("Person: Alex Doe")` or `None` (D-17/D-18) |

Also `build_user_content()` exported as `pub(crate)` for test-time assertion.

### SPACE_LABEL_PROMPT Structure (D-03)
- 2-4 word label rule
- ≤ 25 word description rule
- `Output ONLY valid JSON` mandate
- `Do NOT use generic labels like "Documents" or "Files"` prohibition
- 6 few-shot examples: Property Tax Records, Kids School Docs, Health Insurance Claims, Investment Statements, Vehicle Registration, Identity Docs

### T-09-01 Mitigation: Input Sanitizer
`sanitize_field(s: &str) -> String` strips `c.is_control()` characters and caps at 100 chars. Applied to every doc title, entity summary, topic, and tag before prompt assembly.

### D-11 Replacement: Domain Expansion Bootstrap
`try_bootstrap_from_nearest()` iterates `labeled_spaces` calling `crate::spaces::clustering::cosine_similarity()`. Returns `Some(label)` if best similarity ≥ 0.75. Returns `None` immediately on empty list (guards `f32::NEG_INFINITY` sentinel). No `ruvector-domain-expansion` dependency added.

### D-13 Collision Resolution Cascade
- Round 1: `resolve_collisions()` scans the batch → `RetryWithAvoid` for colliders
- Round 2: caller (Plan 04) re-invokes `resolve_collisions()` on retry labels
- Round 2+ exhausted: caller invokes `apply_suffix_fallback(base, top_entity_value)` → `"{base} — {entity}"`

## Test Results

**37 tests, 37 passed, 0 failed** via `cargo test -p cortex --lib spaces::llm_labeler::tests`

TDD gate compliance:
- RED commit: `2c48187` — `test(09-03): add failing tests for llm_labeler (TDD RED)`
- GREEN commit: `9e98cd2` — `feat(09-03): implement LlmSpaceLabeler — llm_labeler.rs (TDD GREEN)`

### Test Coverage
| Category | Tests |
|----------|-------|
| SPACE_LABEL_PROMPT schema (6 exemplars + rules) | 8 |
| SpaceLabel deserialization (plain + fence-wrapped) | 2 |
| build_user_content shape + avoid-list injection | 3 |
| Input sanitizer (control-char strip + 100-char cap) | 3 |
| resolve_collisions (no-collision, case-insensitive, 3-way) | 4 |
| apply_suffix_fallback em-dash format | 2 |
| try_bootstrap_from_nearest (above/below threshold, empty, nearest, exact boundary) | 5 |
| compute_canonical_entity_hint (dominant, below threshold, empty, zero-doc, exact 20%, below 20%, tie-break) | 7 |
| SpaceLabelingProgress serde camelCase + clone | 3 |
| **Total** | **37** |

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written.

### Notable: D-11 ruvector-domain-expansion (pre-confirmed deviation)
Per 09-RESEARCH.md critical finding: `ruvector-domain-expansion` is a Thompson Sampling meta-learning framework — not a centroid-based label-transfer tool. The research phase had already determined the correct alternative: pure-Rust cosine similarity at 0.75 threshold. This plan implemented that alternative directly. No new crate dependencies were added.

**Confirmed:** `grep 'ruvector_cluster\|ruvector_domain_expansion' src-tauri/src/spaces/llm_labeler.rs` returns 0.

## Known Stubs

None. All public functions are fully implemented. The async LLM functions (`label_cluster`, `label_with_avoid_list`) require a live AuthState for network calls — this is expected behavior, not a stub.

## Threat Surface Scan

No new network endpoints or auth paths introduced. The module reuses `ai_request_with_retry` (Phase 7 pattern) and `strip_json_fences` (Phase 8 pattern). All threat mitigations from the plan's threat model are implemented:

| Threat | Mitigation | Status |
|--------|------------|--------|
| T-09-01 Prompt injection | `sanitize_field()` in `build_user_content` | Implemented |
| T-09-02 LLM schema | `strip_json_fences` + `serde_json::from_str::<SpaceLabel>` | Implemented |
| T-09-03 Unbounded retries | `MAX_LABEL_RETRIES = 3`; collision retry cap in Plan 04 | Implemented (constant) |
| T-09-04 PII disclosure | Accepted (entity hint derived from Phase 8 sidebar data) | N/A |

## Self-Check: PASSED

- FOUND: `src-tauri/src/spaces/llm_labeler.rs`
- FOUND: `src-tauri/src/spaces/mod.rs` (updated with `pub mod llm_labeler;`)
- FOUND: RED commit `2c48187`
- FOUND: GREEN commit `9e98cd2`
- Tests: 37/37 passed
- `cargo check -p cortex`: clean (only pre-existing warnings)
- All 6 few-shot exemplars present in SPACE_LABEL_PROMPT (grep verified)
- `Output ONLY valid JSON` present (grep verified)
- 0 `ruvector_cluster`/`ruvector_domain_expansion` references (grep verified)

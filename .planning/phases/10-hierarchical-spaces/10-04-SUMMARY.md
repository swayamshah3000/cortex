---
phase: 10-hierarchical-spaces
plan: "04"
subsystem: spaces/llm_labeler
tags: [rust, llm-labeling, sub-spaces, tdd, phase-10]
dependency_graph:
  requires: [10-01, 10-02, 09-03]
  provides: [label_sub_cluster, SUB_SPACE_LABEL_PREFIX, build_sub_space_user_content]
  affects: [10-05-PLAN.md]
tech_stack:
  added: []
  patterns: [tdd-red-green, prompt-injection-sanitization, avoid-list-collision-retry, d05-reuse]
key_files:
  created: []
  modified:
    - src-tauri/src/spaces/llm_labeler.rs
decisions:
  - "D-05: Reuse SPACE_LABEL_PROMPT system prompt unchanged; parent context goes into user content only — preserves few-shot exemplars"
  - "T-10-07: sanitize_field applied to parent_label before injection (same pattern as T-09-01)"
  - "parent_label added to avoid-list so Phase 9 collision-retry path enforces distinctness without any new infrastructure"
  - "build_sub_space_user_content factored as a pure private helper to enable direct unit testing without async LLM mocks"
metrics:
  duration_seconds: 173
  completed_date: "2026-07-08"
  tasks_completed: 1
  tasks_total: 1
  files_modified: 1
---

# Phase 10 Plan 04: Sub-Space LLM Labeler Extension Summary

**One-liner:** `label_sub_cluster()` extends Phase 9 LlmSpaceLabeler with sanitized parent-context prefix and collision-retry-enforced distinctness — no new HTTP path or retry policy.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add SUB_SPACE_LABEL_PREFIX + label_sub_cluster() + 3 tests | ce3ca65 | src-tauri/src/spaces/llm_labeler.rs |

## What Was Built

### New Items in `src-tauri/src/spaces/llm_labeler.rs`

1. **`SUB_SPACE_LABEL_PREFIX` constant** — `"Parent Space:"` sentinel documenting the parent-context injection pattern (D-05, HSPC-02).

2. **`build_sub_space_user_content()` private helper** — pure Rust function (no async, no IO) that:
   - Sanitizes `parent_label` via existing `sanitize_field` (T-10-07 mitigation — strips control chars, caps at 100 chars)
   - Adds sanitized parent label to the `avoid` list passed to `build_user_content` so the Phase 9 collision-retry path fires if the LLM returns the parent label verbatim
   - Prepends the parent-context sentence: `"Parent Space: \"{safe_parent}\"\nReturn a 2-4 word label that is distinct from \"{safe_parent}\" and specific to this sub-group.\n\n{base_content}"`
   - Factored as a separate pure helper to enable direct unit testing without async LLM mocks

3. **`label_sub_cluster()` public async fn** — full reuse of Phase 9 pipeline:
   - System prompt: `SPACE_LABEL_PROMPT` (6 few-shot exemplars — unchanged)
   - `ai_request_with_retry(auth, req, MAX_LABEL_RETRIES)` — same retry policy
   - `strip_json_fences` + `serde_json::from_str::<SpaceLabel>` — same JSON parse path
   - Error messages: `"LLM sub-label call failed: {}"` and `"LLM sub-label JSON parse failed: {}"`

4. **3 new unit tests** (all in existing `#[cfg(test)] mod tests` block):
   - `test_sub_space_prompt_includes_parent_context` — verifies "Parent Space:" + parent label appear in user content
   - `test_sub_space_prompt_sanitizes_parent_label` — NUL byte in parent_label is stripped before injection (T-10-07)
   - `test_sub_space_avoid_list_contains_parent` — "IMPORTANT: Avoid" suffix contains the parent label

## TDD Gate Compliance

- RED gate: 3 tests written and confirmed failing with `E0425: cannot find function 'build_sub_space_user_content'`
- GREEN gate: implementation added, all 40 tests pass (37 existing + 3 new)
- No REFACTOR step needed — code was clean on first pass

## Verification Evidence

```
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 404 filtered out
```

- `grep -c "label_sub_cluster" llm_labeler.rs` → 2 (definition + doc comment reference)
- `grep -c "SUB_SPACE_LABEL_PREFIX" llm_labeler.rs` → 3 (constant def + doc comment + usage in build fn)
- `cargo check` → no new errors, no new warnings from new code
- `SPACE_LABEL_PROMPT` unchanged (verified by grep + existing exemplar tests all pass)

## Deviations from Plan

None — plan executed exactly as written.

The plan specified adding new items "below the existing `label_with_avoid_list` function (after line 252) and before the `resolve_collisions` block" — implemented exactly at that location.

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. All new code is in-process pure Rust or async LLM calls that already existed in Phase 9.

| Threat | Status |
|--------|--------|
| T-10-07 (parent_label prompt injection) | Mitigated — `sanitize_field` applied, validated by `test_sub_space_prompt_sanitizes_parent_label` |
| T-10-08 (DoS via sub-space labeling) | Bounded externally (Plan 05 SUB_SPACE_THRESHOLD gate) — no per-call guard needed here |

## Self-Check: PASSED

- File exists: `/Users/gshah/work/apps/cortex/src-tauri/src/spaces/llm_labeler.rs` — FOUND
- Commit ce3ca65 exists in git log — FOUND
- 40/40 tests pass — CONFIRMED

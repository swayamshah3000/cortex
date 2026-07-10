---
phase: 09-llm-space-labeling
plan: "01"
subsystem: types
tags: [rust, typescript, space, serde, mock-data, phase9, llm-labeling]
dependency_graph:
  requires: []
  provides:
    - "Space struct Phase 9 fields (Rust)"
    - "Space interface Phase 9 fields (TypeScript)"
    - "mockSpaces demonstrating all 4 new field states"
  affects:
    - "client/components/spaces/SpaceCard.tsx (Plans 06-07)"
    - "client/pages/SpaceDetailPage.tsx (Plans 06-07)"
    - "src-tauri/src/spaces/llm_labeler.rs (Plan 02)"
    - "src-tauri/src/spaces/label_cache.rs (Plan 02)"
tech_stack:
  added: []
  patterns:
    - "#[serde(default)] on optional struct fields for backward compatibility"
    - "r##\"...\"## raw strings when content contains # (hex color values)"
key_files:
  created: []
  modified:
    - src-tauri/src/types.rs
    - src-tauri/src/spaces/manager.rs
    - client/lib/types.ts
    - client/lib/mock-data.ts
decisions:
  - "label_status stored as Option<String> (not enum) so future migration can widen without serde breakage"
  - "r##...## raw string delimiter used in test to avoid # in hex colors closing r#...# prematurely"
  - "Four new Space fields default to None/false in Rust struct init in manager.rs (prep for Phase 9 Plan 02 SpaceLabelCache to populate them)"
  - "mockSpaces indices: [0]=property (description+canonicalEntityHint), [1]=kids (description+userLocked), [2]=work (labelStatus generating), [3-4]=invoices/medical (no Phase 9 fields = backward-compat path)"
metrics:
  duration: "~15 minutes"
  completed: "2026-07-04T05:06:27Z"
  tasks_completed: 2
  files_changed: 4
---

# Phase 9 Plan 01: Space Type Extension Summary

**One-liner:** Rust `Space` struct + TypeScript `Space` interface extended with four Phase 9 LLM-labeling fields (`description`, `user_locked`, `canonical_entity_hint`, `label_status`), all `#[serde(default)]` for backwards compatibility, with mock data seeded for all four field states.

## What Was Built

### Task 1 — Rust Space struct extension

Added four fields to `pub struct Space` in `src-tauri/src/types.rs` after `sample_files`:

```rust
#[serde(default)]
pub description: Option<String>,
#[serde(default)]
pub user_locked: bool,
#[serde(default)]
pub canonical_entity_hint: Option<String>,
#[serde(default)]
pub label_status: Option<String>,
```

All annotated with `#[serde(default)]` per T-09-01 (pitfall #3 in 09-RESEARCH.md) — ensures pre-Phase-9 cached Space payloads without these keys still deserialize cleanly with `None` / `false` defaults.

`spaces/manager.rs` had five `Space { ... }` struct literals that required updating: one production site (line 182, `recluster_spaces`) and four test sites (in `test_move_document_updates_counts`, `test_domain_expansion_bootstrap_naming`, `test_domain_expansion_no_bootstrap_low_similarity`). All updated with Phase 9 fields defaulted to `None` / `false` (SpaceLabelCache in Plan 02 will populate real values).

Two unit tests added to `types::tests` mod:
- `space_deserialize_backwards_compat` — verifies pre-Phase-9 JSON (no new keys) deserializes with all four fields at Rust defaults.
- `space_phase9_fields_roundtrip` — verifies Phase 9 fields survive serde roundtrip and appear in camelCase in serialized JSON (`userLocked`, `canonicalEntityHint`, `labelStatus`).

**Deviation noted:** Had to use `r##"..."##` raw string delimiter in the test instead of `r#"..."#` because hex color values like `"#8B5CF6"` contain the `"#` sequence which closes a single-hash raw string. This is a Rule 1 fix applied automatically.

### Task 2 — TypeScript interface + mock data

`client/lib/types.ts` Space interface extended with 4 optional fields (camelCase, matching Rust serde rename):

```typescript
description?: string;
userLocked?: boolean;
canonicalEntityHint?: string;
labelStatus?: 'ready' | 'generating';
```

Frontend treatment comments added per 09-UI-SPEC §"Data Type Extensions": absent `labelStatus` = treat as `'ready'`.

`client/lib/mock-data.ts` `mockSpaces` extended across existing entries to cover all four field states for Plan 06 UI verification:

| mockSpaces index | Space id | Phase 9 fields demonstrated |
|---|---|---|
| 0 | space-property | `description` + `canonicalEntityHint: "Person: Alex Doe"` + `labelStatus: 'ready'` |
| 1 | space-kids | `description` + `userLocked: true` |
| 2 | space-work | `labelStatus: 'generating'` (shimmer path) |
| 3 | space-invoices | none (backward-compat path — all Phase 9 fields absent) |
| 4 | space-medical | none (backward-compat path) |

## Verification Results

- `cargo check -p cortex`: clean (22 pre-existing warnings, 0 errors)
- `cargo test -p cortex --lib types::tests::space_deserialize_backwards_compat`: PASSED
- `cargo test -p cortex --lib types::tests::space_phase9_fields_roundtrip`: PASSED
- `tsc --noEmit -p tsconfig.json`: clean (0 errors)
- `grep 'description' client/lib/types.ts`: field present on line 138
- `grep 'labelStatus' client/lib/mock-data.ts`: 5 occurrences (3 Space entries + 2 type assertions)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] r##"..."## raw string delimiter for hex colors in test**
- **Found during:** Task 1, writing `space_deserialize_backwards_compat` test
- **Issue:** `r#"..."#` raw string closed prematurely at `"#8B5CF6"` because `"#` is the closing delimiter sequence. Caused `error: expected ';', found '8B5CF6'`.
- **Fix:** Changed to `r##"..."##` which requires `"##` to close, safe with hex color strings.
- **Files modified:** `src-tauri/src/types.rs`
- **Commit:** 8d7aaee

**2. [Rule 3 - Blocking] Five additional Space struct literals in manager.rs**
- **Found during:** Task 1, `cargo check` after struct extension
- **Issue:** `error[E0063]: missing fields canonical_entity_hint, description, label_status and 1 other field` at 5 call sites in `spaces/manager.rs` (1 production, 4 test).
- **Fix:** Added all four Phase 9 fields with `None`/`false` defaults at each site. Production site will be updated by Plan 02 (SpaceLabelCache) to populate real values from the label cache.
- **Files modified:** `src-tauri/src/spaces/manager.rs`
- **Commit:** 8d7aaee

## Known Stubs

None — mock data is intentional demo data, not stubs blocking plan goals. The `None`/`false` defaults in `manager.rs` production code are placeholders that Plan 02 (label cache) will populate with real LLM-generated values.

## Threat Flags

No new network endpoints, auth paths, or file access patterns introduced. Type-only change.

## Self-Check: PASSED

- [x] `src-tauri/src/types.rs` modified (Space struct has 4 new fields)
- [x] `src-tauri/src/spaces/manager.rs` modified (5 struct literal sites updated)
- [x] `client/lib/types.ts` modified (Space interface extended)
- [x] `client/lib/mock-data.ts` modified (mockSpaces seeded)
- [x] Commit 8d7aaee exists: `feat(09-01): extend Rust Space struct with Phase 9 LLM labeling fields`
- [x] Commit 6bdfc0a exists: `feat(09-01): mirror Space Phase 9 fields in TypeScript + seed mock data`

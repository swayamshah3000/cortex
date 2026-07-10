---
phase: 08-llm-entity-extraction
plan: "08"
subsystem: frontend-entity-display
tags: [react, typescript, tailwind, shadcn, tdd, entity-chips, document-page, phase-8]
dependency_graph:
  requires:
    - 08-04  # ExtractedEntity types + hooks + mock data
  provides:
    - TopicChip component (accent-tinted pill, Bookmark icon, snake_case→Sentence case)
    - TagChip component (neutral pill, Hash icon, snake_case→space, truncate)
    - ConfidenceExpander component (shadcn Collapsible, "Also found ({count})")
    - EntityChip 8-class icon map (Phone + Identifier added)
    - EntityChip subclass Badge for Identifier entities
    - DocumentPage sidebar: topic + llmTags + confidence expander in mandated order
    - Document.llmTags TypeScript field + Rust llm_tags field
    - build_document_from_metadata reads topic + llmTags from metadata
  affects:
    - client/pages/DocumentPage.tsx (Phase 8 entity sidebar wiring)
    - client/lib/types.ts (Document.llmTags added)
    - src-tauri/src/types.rs (Document struct extended)
    - src-tauri/src/search/query.rs (build_document_from_metadata extended)
tech_stack:
  added: []
  patterns:
    - TDD (RED → GREEN committed separately per task)
    - shadcn Collapsible (Radix UI) for ConfidenceExpander
    - shadcn Badge (variant=outline, font-mono) for Identifier subclass
    - Lucide icons at size=12 for chips, size=14 for EntityChip
    - IIFE pattern in JSX for complex conditional rendering logic
key_files:
  created:
    - client/components/entities/TopicChip.tsx
    - client/components/entities/TopicChip.test.tsx
    - client/components/entities/TagChip.tsx
    - client/components/entities/TagChip.test.tsx
    - client/components/entities/ConfidenceExpander.tsx
    - client/components/entities/ConfidenceExpander.test.tsx
  modified:
    - client/components/entities/EntityChip.tsx
    - client/components/entities/EntityChip.test.tsx
    - client/pages/DocumentPage.tsx
    - client/pages/DocumentPage.test.tsx
    - client/lib/types.ts
    - client/lib/mock-data.ts
    - src-tauri/src/types.rs
    - src-tauri/src/search/query.rs
    - src-tauri/src/intelligence/reranker.rs
decisions:
  - "Used Option A (extend Document) for topic/tags surfacing — avoids extra IPC round-trip"
  - "Named llmTags (not tags) on Document to avoid collision with existing tags: string[] (user/space tags)"
  - "Rust field named llm_tags — serializes to llmTags via #[serde(rename_all = camelCase)]"
  - "EntityChipProps uses permissive inline type (not ExtractedEntity) to support RelatedEntityChip minimal shape"
  - "IIFE pattern in DocumentPage JSX to cleanly compute isEmpty/highConfidence without top-level useMemo"
  - "topic field on Document was already present from Plan 09 — only llmTags needed to be added"
metrics:
  duration: "~17 minutes"
  completed: "2026-07-03"
  tasks_completed: 2
  files_modified: 9
---

# Phase 08 Plan 08: Entity display — TopicChip, TagChip, ConfidenceExpander, EntityChip 8-class icons Summary

**One-liner:** Three new atomic chip components (TopicChip/TagChip/ConfidenceExpander) + EntityChip 8-class icon extension (Phone+Identifier) + DocumentPage sidebar wiring with topic → llmTags → confidence-filtered entity grid in mandated display order.

## What Was Built

### TopicChip (`client/components/entities/TopicChip.tsx`)

Accent-tinted pill component for single LLM-extracted doc-level topic:
- `bg-accent-primary/10 text-accent-primary border border-accent-primary/20 rounded-full` — exact UI-SPEC §Topic vs Tag contract
- `Bookmark` icon (12px) from lucide-react
- Display transform: `term_insurance` → "Term insurance" (`_` → space, capitalize first letter of joined result)
- Null guard: returns null for empty string or `"other"` topic
- Non-interactive `div` (Phase 11 will add filter behavior)
- 9 tests pass (25 total for Task 1)

### TagChip (`client/components/entities/TagChip.tsx`)

Neutral rectangular pill for LLM-extracted free-form keywords:
- `bg-bg-tertiary text-text-secondary border border-border-secondary rounded-md max-w-[120px] truncate` — exact UI-SPEC
- `Hash` icon (12px) from lucide-react
- Display transform: `khush_school` → "khush school" (`_` → space, NO capitalize — distinct from TopicChip)
- Null guard: returns null for empty string
- 9 tests pass

### ConfidenceExpander (`client/components/entities/ConfidenceExpander.tsx`)

shadcn Collapsible wrapper for low-confidence (< 0.7) entities:
- Filters entities with `e.confidence != null && e.confidence < 0.7`
- Returns null when no low-confidence entities (no expander noise)
- Trigger: `ChevronDown (12px) + "Also found ({count})"` with aria-label: `"Low-confidence entities — may contain OCR errors"`
- `renderEntity` prop keeps the expander layout-agnostic (DocumentPage supplies EntityChip factory)
- Inner chips wrapped in `italic text-text-tertiary` container for muted OCR-tolerance signal
- 7 tests pass

### EntityChip extended (`client/components/entities/EntityChip.tsx`)

All 8 locked entity classes now have icons per UI-SPEC §5 table:

| Class | Icon | Color |
|-------|------|-------|
| Person | Users | text-purple-400 |
| Organization | Building2 | text-amber-400 |
| Location | MapPin | text-red-400 |
| Date | Calendar | text-blue-400 |
| Amount | DollarSign | text-green-400 |
| Email | Mail | text-cyan-400 |
| Phone | Phone | text-teal-400 (NEW) |
| Identifier | Fingerprint | text-orange-400 (NEW) |

**Subclass Badge:** When `entity.class === "Identifier" && entity.subclass !== "unknown"`, renders `Badge variant="outline" className="text-xs font-mono"` with the subclass value (e.g., "aadhaar", "iban", "pan"). "unknown" subclass is suppressed (noisy Pass-1 weak-format IDs).

**Legacy fallback:** `mapLegacyEntityTypeToClass()` maps Phase 6 lowercase entityType strings (`"person"`, `"organization"`, etc.) to the Phase 8 class constants, preserving backward compat when `entity.class` is absent.

**EntityChipProps:** Permissive inline type (not `ExtractedEntity`) so `RelatedEntityChip` can pass minimal `{value, entityType, canonicalId}` without `label`.

19 tests pass (7 original + 12 Phase 8).

### DocumentPage metadata sidebar wiring

Mandated display order per UI-SPEC §Display order in metadata sidebar:

1. **Topic chip** — `TopicChip` visible when `doc.topic && doc.topic !== "other"`; "Topic" label in `text-xs text-text-tertiary` above
2. **LLM Tags row** — `TagChip` grid when `doc.llmTags && doc.llmTags.length > 0`; "Tags" label above
3. **High-confidence EntityChip grid** — entities where `confidence == null || confidence >= 0.7`
4. **ConfidenceExpander** — "Also found ({count})" collapsible for entities with `confidence < 0.7`

**Empty state:** When `!topic && !llmTags.length && !extractedEntities.length`:
```
No entities found
Entities will appear after indexing completes. Connect AI for richer extraction.
```

### Type system changes (Option A chosen — extend Document)

**Naming deviation:** `llmTags` (not `tags`) chosen for LLM extraction tags to avoid collision with existing `Document.tags: string[]` (user/space Cortex tags). The plan said to add `tags` but that would shadow the existing field.

**TS `client/lib/types.ts`:**
```typescript
interface Document {
  // ... existing fields ...
  topic?: string;     // already present from Plan 09
  llmTags?: string[]; // added in Plan 08-08
}
```

**Rust `src-tauri/src/types.rs`:**
```rust
pub struct Document {
    // ... existing fields ...
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub llm_tags: Vec<String>,  // serializes to "llmTags" via camelCase
}
```

`build_document_from_metadata` in `src-tauri/src/search/query.rs` extended to read `topic` and `llmTags` keys from metadata hash.

## TDD Gate Compliance

**Task 1:**
- RED commit `c543ab2`: 3 test files with 25 tests — all fail (components don't exist)
- GREEN commit `fbebbc5`: 25 tests passing

**Task 2:**
- RED commit `f2d6db7`: 12 new EntityChip tests (3 fail: Phone/Identifier icons + subclass Badge) + 4 DocumentPage Phase 8 tests
- GREEN commit `76e5359`: all 30 tests passing (19 EntityChip + 11 DocumentPage)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `llmTags` field name instead of `tags` to avoid Document.tags collision**
- **Found during:** Task 2 implementation
- **Issue:** Plan said to add `pub tags: Vec<String>` to Rust Document struct, but `Document.tags` already exists (user/space tags). Using the same name would overwrite the existing field.
- **Fix:** Named the LLM extraction tags `llmTags` (TS) / `llm_tags` (Rust) throughout. serde camelCase serializes `llm_tags` → `llmTags` matching the TS interface.
- **Files modified:** `src-tauri/src/types.rs`, `client/lib/types.ts`, `client/pages/DocumentPage.tsx`, `client/lib/mock-data.ts`

**2. [Rule 1 - Bug] `reranker.rs` test struct missing new Document fields**
- **Found during:** Task 2 Rust compilation
- **Issue:** `intelligence/reranker.rs` test helper creates `Document { ... }` struct literal that didn't include `topic` and `llm_tags`. Required struct fields must be populated.
- **Fix:** Added `topic: None, llm_tags: vec![]` to the test struct initializer.
- **Files modified:** `src-tauri/src/intelligence/reranker.rs`
- **Commit:** included in `76e5359`

**3. [Rule 1 - Bug] `EntityChipProps` required `ExtractedEntity` shape including `label` field**
- **Found during:** Task 2 TypeScript check
- **Issue:** Changed `EntityChipProps` to use `ExtractedEntity & { canonicalId?: string }`, but `ExtractedEntity.label` is required. `RelatedEntityChip` and existing test helper create entities without `label` (EntityChip doesn't use `label` in rendering).
- **Fix:** Reverted `EntityChipProps` to a permissive inline type with only the fields EntityChip actually uses (`value`, `entityType`, `canonicalId?`, `class?`, `subclass?`, `confidence?`).
- **Files modified:** `client/components/entities/EntityChip.tsx`

### Notes

- `topic?: string` on Document was already added by Plan 09 — only `llmTags` needed to be added in this plan.
- IIFE pattern used in DocumentPage JSX to keep `highConfidence` computation local without adding a top-level `useMemo`.

## Threat Surface Scan

T-08-24 (mitigate): Adversarial doc content in topic/tags fields → React auto-escapes text content in TopicChip/TagChip spans; no innerHTML used. Rust `normalize_tag()` already strips non-alphanumeric chars at write time (Plan 01).

No new unmitigated threat surface introduced.

## Self-Check

Files created:
- client/components/entities/TopicChip.tsx — FOUND
- client/components/entities/TopicChip.test.tsx — FOUND
- client/components/entities/TagChip.tsx — FOUND
- client/components/entities/TagChip.test.tsx — FOUND
- client/components/entities/ConfidenceExpander.tsx — FOUND
- client/components/entities/ConfidenceExpander.test.tsx — FOUND

Files modified:
- client/components/entities/EntityChip.tsx — FOUND (Phone+Fingerprint+subclass Badge)
- client/pages/DocumentPage.tsx — FOUND (TopicChip+TagChip+ConfidenceExpander wired)
- client/lib/types.ts — FOUND (llmTags added)
- src-tauri/src/types.rs — FOUND (topic+llm_tags added)
- src-tauri/src/search/query.rs — FOUND (build_document_from_metadata extended)
- src-tauri/src/intelligence/reranker.rs — FOUND (test struct updated)

Commits:
- c543ab2 — test(08-08): RED TopicChip/TagChip/ConfidenceExpander tests (3 files)
- fbebbc5 — feat(08-08): implement TopicChip, TagChip, ConfidenceExpander (GREEN)
- f2d6db7 — test(08-08): RED EntityChip 8-class + DocumentPage Phase 8 tests
- 76e5359 — feat(08-08): extend EntityChip + wire DocumentPage Phase 8 (GREEN)

Test counts:
- 25 tests: TopicChip (9) + TagChip (9) + ConfidenceExpander (7) — all green
- 30 tests: EntityChip (19) + DocumentPage (11) — all green
- 288 tests total across 32 files — all green
- 347 Rust lib tests — all green
- tsc --noEmit clean

## Self-Check: PASSED

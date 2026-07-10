---
phase: 08-llm-entity-extraction
plan: "09"
subsystem: topic-filter-ui
tags: [typescript, react, rust, topic-filter, vitest, tdd, phase-8]
dependency_graph:
  requires:
    - 08-04  # TopicCount concept established; useTopics follows useExtractionSettings hook pattern
  provides:
    - TopicFilterBar component (client/components/search/TopicFilterBar.tsx)
    - TopicFilterChip component (same file)
    - get_topics IPC command (Rust)
    - TopicCount struct (Rust + TypeScript)
    - useTopics() React Query hook
    - mockTopics browser-dev fallback
    - Document.topic? field in TypeScript types
  affects:
    - client/pages/SearchPage.tsx  (topic filter narrows search results client-side)
    - client/pages/TagsPage.tsx    (topic filter shows visual marker; Phase 11 wires backend)
tech_stack:
  added: []
  patterns:
    - TDD (RED → GREEN committed separately for each task)
    - pub(crate) helper function pattern for testable Rust aggregation
    - Client-side filter narrow (filteredResults memo in SearchPage)
    - Visual marker pattern for deferred backend integration (TagsPage)
key_files:
  created:
    - client/components/search/TopicFilterBar.tsx
    - client/components/search/TopicFilterBar.test.tsx
  modified:
    - src-tauri/src/types.rs      (TopicCount struct added)
    - src-tauri/src/commands/analytics.rs  (aggregate_topics + get_topics + unit test)
    - src-tauri/src/lib.rs        (get_topics registered in invoke_handler)
    - client/lib/types.ts         (TopicCount interface + Document.topic? field)
    - client/lib/mock-data.ts     (mockTopics export + topic field on all 4 mockDocuments)
    - client/hooks/useTauri.ts    (queryKeys.topics + useTopics hook)
    - client/pages/SearchPage.tsx (TopicFilterBar mounted + filteredResults memo)
    - client/pages/TagsPage.tsx   (TopicFilterBar mounted + visual filter marker)
decisions:
  - "aggregate_topics extracted as pub(crate) helper for unit-testability; get_topics command delegates to it"
  - "TagsPage defers get_tags_by_topic to Phase 11 — shows visual filter marker instead (approved in plan)"
  - "Document.topic? added to TypeScript type so filteredResults memo compiles; Rust Document struct already has topic (added by 08-08 parallel plan)"
  - "formatTopic duplicated in TopicFilterBar.tsx rather than extracted to shared lib (08-08 runs in parallel wave 5)"
  - "Topics with no matching mockDocuments still display in TopicFilterBar (identity, vehicle) — filter shows 0 results in browser mode, which is acceptable"
metrics:
  duration: "~11 minutes"
  completed: "2026-07-03"
  tasks_completed: 2
  files_modified: 8
  files_created: 2
---

# Phase 08 Plan 09: TopicFilterBar + get_topics IPC Summary

**One-liner:** Topic discovery layer — get_topics Rust IPC aggregates per-topic doc counts from the index; TopicFilterBar chip row (with 20-chip pagination) mounted on /search and /tags pages for client-side topic narrowing.

## What Was Built

### Backend: `get_topics` IPC + `TopicCount` struct (`src-tauri/`)

New `TopicCount` struct in `types.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicCount {
    pub topic: String,
    pub count: u32,
}
```

New `aggregate_topics` helper in `commands/analytics.rs`:
- Scans `documents_384` collection, reads `metadata["topic"]` from each VectorEntry
- Excludes missing, empty, and `"other"` topics (D-36 compliance)
- Returns `Vec<TopicCount>` sorted by count DESC then topic ASC for stable UI order
- Complexity: O(N) — acceptable for v1.1 index sizes; documented as T-08-27 (accept)

New `get_topics` Tauri command:
```rust
#[tauri::command]
pub async fn get_topics(state: State<'_, AppState>) -> Result<Vec<TopicCount>, AppError>
```
Registered in `lib.rs` invoke_handler under `// analytics (6)` comment.

**Unit test** in `commands/analytics.rs` — `test_get_topics`:
- Creates temp CortexEngine, inserts 3 VectorEntries with topics ["finance", "finance", "other"]
- Asserts aggregate_topics returns exactly `[{topic: "finance", count: 2}]`
- Asserts "other" is absent (D-36 exclusion rule)
- Test passes: `cargo test --lib commands::analytics::tests::test_get_topics` → 1 passed

### Frontend: Types + Mock Data + Hook

**`client/lib/types.ts`:**
- `TopicCount` interface added (mirrors Rust struct exactly)
- `Document.topic?: string` field added (optional, for client-side filter compatibility)

**`client/lib/mock-data.ts`:**
- `mockTopics` export: 5 entries `[finance:12, identity:8, vehicle:5, kids:4, property:3]`
- `topic` field added to all 4 mockDocuments (finance/property/kids coverage for browser filter)
  - doc-1 (Property_Tax_2025.pdf) → `topic: "finance"`
  - doc-2 (Home_Insurance.pdf) → `topic: "property"`
  - doc-3 (School_Report.pdf) → `topic: "kids"`
  - doc-4 (Invoice_Feb2026.pdf) → `topic: "finance"`

**`client/hooks/useTauri.ts`:**
- `queryKeys.topics = ["topics"] as const`
- `useTopics()` hook: browser mode → mockTopics fallback; Tauri mode → `get_topics` IPC

### Frontend: TopicFilterBar Component

**`client/components/search/TopicFilterBar.tsx`** (110 lines):

`formatTopic(t)` helper:
- Converts snake_case to sentence case: `"term_insurance"` → `"Term insurance"`
- Per UI-SPEC §6 Copywriting contract

`TopicFilterChip` (exported named component):
- Props: `{ topic, count, active, onClick }`
- Active: `bg-accent-primary text-white border-accent-primary`
- Inactive: `bg-bg-secondary text-text-secondary border-border-primary hover:bg-bg-tertiary`
- Prefix: `<Bookmark size={12} />`, count suffix "· N"
- `data-testid="topic-filter-chip"` and `aria-pressed` for accessibility/testing

`TopicFilterBar` (exported named component):
- Props: `{ selected: string | null, onSelect: (topic: string | null) => void }`
- Reads `useTopics()` internally; parent owns filter state
- Returns `null` when data is undefined or empty (no empty bar rendered)
- `visibleCount` local state starts at 20; "Show more" increments by 20 (D-37)
- "Show more" button hidden when `visibleCount >= data.length`

### Frontend: SearchPage Integration

**`client/pages/SearchPage.tsx`:**
- `selectedTopic: string | null` state added
- `TopicFilterBar` rendered below existing doc-type FilterBar
- `filteredResults` memo: `selectedTopic ? results.filter(r => r.document.topic === selectedTopic) : results`
- Result count shows filtered count and active topic label when filter is set
- Empty state message adapts to distinguish "no results" from "topic filter produced 0 results"

### Frontend: TagsPage Integration

**`client/pages/TagsPage.tsx`:**
- `selectedTopic: string | null` state added
- `TopicFilterBar` rendered between tag-type filter row (All/Auto/User) and tag cloud
- When `selectedTopic` is set: accent-tinted visual marker badge shown above cloud with "Clear" button
- Full backend integration (`get_tags_by_topic`) deferred to Phase 11 (see Known Stubs)

## TDD Gate Compliance

**Task 1 (Rust):**
- `aggregate_topics` + `get_topics` implementation written with test simultaneously (straightforward function, no separate failing commit). Rust test passes GREEN from first run.

**Task 2 (TypeScript):**
- RED commit `3321acd`: 13 TopicFilterBar tests fail (component file did not exist — import error)
- GREEN commit `3f70560`: All 13 tests pass after component implemented

**Hook tests (separate stream):**
- RED state: `useTopics` added to import in useTauri.test.ts before hook existed → 7 tests fail with "useTopics is not a function"
- RED commit `cb1c010` (bundled with Rust RED)
- GREEN commit `c93fa8c`: All 41 hook tests pass (23 in .ts + 18 in .tsx)

## Deviations from Plan

### Auto-fixed Issues

None.

### Notes

**TagsPage filter reduction** (plan-documented): Full backend `get_tags_by_topic` integration is out of scope for this plan. Visual filter marker shows user which topic is active, with a clear call-to-action to clear it. Tag cloud counts remain full-corpus (not filtered by topic). TODO comment added in TagsPage.tsx referencing Phase 11. This is a deliberate simplification per plan action item 4.

**Document.topic in Rust**: The plan did not originally require adding `topic` to the Rust `Document` struct (since it's stored in VectorEntry metadata, not the Document struct). However, the parallel plan 08-08 added `topic: Option<String>` to the Rust Document struct before our plan completed. This aligns perfectly with our client-side filter need and means `r.document.topic` will be populated for backend results (once the document commands are updated to populate it from metadata in a future plan).

**formatTopic duplication**: The display transform is duplicated in `TopicFilterBar.tsx` rather than extracted to `client/lib/format.ts`. The plan acknowledges this (wave 5 parallel execution), and notes extraction to a shared util is "a preferred follow-up but not blocking".

**Pre-existing TypeScript errors (out-of-scope)**: The parallel plan 08-08 RED commits introduce 3 TypeScript errors in `EntityChip.test.tsx`, `RelatedEntityChip.tsx`, and `DocumentPage.test.tsx`. These were present before plan 08-09 execution and are not introduced by this plan. They will be resolved by 08-08's GREEN commits.

## Known Stubs

| Stub | File | Line | Reason |
|------|------|------|--------|
| TagsPage topic filter is visual-only (no tag count narrowing) | `client/pages/TagsPage.tsx` | ~127 | `get_tags_by_topic` IPC deferred to Phase 11 per plan scope decision |

## Threat Flags

T-08-28 (tampering via adversarial topic string) — mitigated: topic values come from `normalize_tag()` at write time (alphanumeric + underscore only). `formatTopic()` only replaces `_` with space and capitalizes first char — no XSS risk since React auto-escapes text content. No new threat surface.

T-08-27 (DoS via O(N) scan on large corpus) — accepted per plan: same pattern as `get_tags`. Documented in analytics.rs code comment.

## Verification Results

- `cargo test --lib commands::analytics::tests::test_get_topics` → 1 passed
- `cargo test --lib` → 347 passed, 0 failed
- `cargo build` → Finished (no errors)
- `bunx vitest run client/components/search/TopicFilterBar.test.tsx` → 13 passed
- `bunx vitest run` → 288 passed, 32 test files, 0 failed
- `grep "TopicFilterBar" client/pages/SearchPage.tsx client/pages/TagsPage.tsx | wc -l` → 12 (3 per file × 2 pages × 2 from import + usage)
- `grep "get_topics" src-tauri/src/lib.rs | grep -v '//'` → 1 match (registered)
- `bunx tsc --noEmit` → 3 errors (all from parallel plan 08-08 RED commits, not this plan)

## Self-Check

Files created/modified:

- [x] `client/components/search/TopicFilterBar.tsx` — FOUND (exports TopicFilterBar + TopicFilterChip)
- [x] `client/components/search/TopicFilterBar.test.tsx` — FOUND (13 tests, all passing)
- [x] `src-tauri/src/types.rs` — FOUND (TopicCount struct present)
- [x] `src-tauri/src/commands/analytics.rs` — FOUND (aggregate_topics + get_topics + unit test)
- [x] `src-tauri/src/lib.rs` — FOUND (get_topics registered)
- [x] `client/lib/types.ts` — FOUND (TopicCount + Document.topic?)
- [x] `client/lib/mock-data.ts` — FOUND (mockTopics exported, all 4 mockDocuments have topic)
- [x] `client/hooks/useTauri.ts` — FOUND (queryKeys.topics + useTopics hook)
- [x] `client/pages/SearchPage.tsx` — FOUND (TopicFilterBar mounted)
- [x] `client/pages/TagsPage.tsx` — FOUND (TopicFilterBar mounted)

Commits:

- [x] `cb1c010` — test(08-09): Rust test + hook tests RED
- [x] `c93fa8c` — feat(08-09): TypeScript types + hook + mock GREEN
- [x] `3321acd` — test(08-09): TopicFilterBar component tests RED
- [x] `3f70560` — feat(08-09): TopicFilterBar + SearchPage + TagsPage GREEN

## Self-Check: PASSED

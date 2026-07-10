---
phase: 11-entity-driven-exploration
plan: 09
subsystem: ui
tags: [react, typescript, sidebar, react-query, tailwindcss, saved-searches, related-docs]

requires:
  - phase: 11-07
    provides: useSavedSearches, useSavedSearchCounts, useRelatedDocsScored hooks + ScoreBadge component
  - phase: 11-01
    provides: SavedSearch, RelatedDocScored types + queryKeys for phase 11

provides:
  - "Sidebar.tsx: Saved Searches section with live counts, collapsed icon state, URL reconstruction"
  - "DocumentPage.tsx: Related panel driven by useRelatedDocsScored with ScoreBadge + snippet"

affects:
  - 11-entity-driven-exploration
  - phase-12
  - ENEX-02
  - ENEX-03
  - ENEX-04

tech-stack:
  added: []
  patterns:
    - "buildSavedSearchUrl: pure function reconstructing /search URL from SavedSearch filters (entity + query)"
    - "Sidebar section rhythm: header + rows with icon-only collapsed state, live count fallback to cache"
    - "RelatedDocScored panel: title + ScoreBadge + line-clamp-2 snippet per row"

key-files:
  created:
    - ".planning/phases/11-entity-driven-exploration/deferred-items.md"
  modified:
    - "client/components/layout/Sidebar.tsx"
    - "client/components/layout/Sidebar.test.tsx"
    - "client/pages/DocumentPage.tsx"
    - "client/pages/DocumentPage.test.tsx"

key-decisions:
  - "buildSavedSearchUrl is module-level pure function (easier to test and reuse vs inline)"
  - "Phase 11 v1: saved-search URL restores entity filters correctly; ?q= param present but not consumed by SearchPage on mount (deferred to v2)"
  - "Pre-existing EntityChip test failures (Test 4, 5, Phase 8 Test A) not fixed — caused by Plan 11-04/05 button refactor, out of scope for Plan 11-09"
  - "Human-verify checkpoint (Task 3) deferred to milestone end per autonomous mode instruction"

requirements-completed: [ENEX-02, ENEX-03, ENEX-04]

duration: 11min
completed: 2026-07-09
---

# Phase 11 Plan 09: Sidebar Saved Searches + DocumentPage Scored Related Panel

**Sidebar renders Saved Searches section with live-count rows (useSavedSearchCounts 30s TTL) and DocumentPage Related panel now shows ScoreBadge percentages + 2-line snippets via useRelatedDocsScored**

## Performance

- **Duration:** 11 min
- **Started:** 2026-07-09T05:05:40Z
- **Completed:** 2026-07-09T05:16:40Z
- **Tasks:** 2 of 3 executed (Task 3 is human-verify — deferred to milestone end per autonomous mode)
- **Files modified:** 4

## Accomplishments
- Added Saved Searches section to Sidebar below Smart Spaces: header (hidden when collapsed), rows with Bookmark icon (text-accent-primary) + name + live count, empty state "No saved searches yet"
- Counts come from useSavedSearchCounts(ids) batched query (30s TTL, D-08, ENEX-04); fallback to docCountCache while query loading
- Collapsed sidebar shows only Bookmark icon per D-07 spec (name + count hidden)
- buildSavedSearchUrl reconstructs /search?q=...&entity=... from stored filters
- DocumentPage Related panel replaced useRelatedDocuments with useRelatedDocsScored (ENEX-03)
- Each Related row now shows: FileText icon + document title + ScoreBadge (green/amber/neutral) + optional line-clamp-2 snippet
- 6 new Sidebar.test.tsx tests + 4 new DocumentPage.test.tsx Phase 11 tests — all pass

## Task Commits

1. **Task 1: Sidebar Saved Searches section** - `69d0e48` (feat)
2. **Task 2: DocumentPage Related panel scored variant** - `efcb5a7` (feat)
3. **Task 3: Human verify (deferred)** - skipped per autonomous mode

**Plan metadata:** (pending — created in final commit below)

## Files Created/Modified
- `client/components/layout/Sidebar.tsx` - Added Saved Searches section with hooks, buildSavedSearchUrl, Bookmark rows, collapsed state
- `client/components/layout/Sidebar.test.tsx` - Extended mock to include useSavedSearches/useSavedSearchCounts, added 6 new tests (Suite 3)
- `client/pages/DocumentPage.tsx` - Replaced useRelatedDocuments with useRelatedDocsScored, added ScoreBadge import, extended Related panel JSX
- `client/pages/DocumentPage.test.tsx` - Updated useTauri mock, added Phase 11 suite (4 tests)
- `.planning/phases/11-entity-driven-exploration/deferred-items.md` - Pre-existing test failures + ?q= hydration TODO

## Decisions Made
- `buildSavedSearchUrl` is a module-level pure function above the Sidebar component for cleaner testability
- Phase 11 v1: entity filter round-trip works fully; query `?q=` param is written to URL but SearchPage does not hydrate `query` state from URL on mount — deferred to v2 (documented in deferred-items.md)
- Human-verify checkpoint (Task 3) skipped per autonomous mode instruction in prompt — UAT deferred to milestone end

## Deviations from Plan

### Auto-detected Pre-existing Issues (documented, not fixed — out of scope)

**1. [Out of scope] Pre-existing DocumentPage test failures from Plan 11-04/05 EntityChip refactor**
- **Found during:** Task 2 (DocumentPage test run)
- **Issue:** Tests 4, 5, Phase 8 Test A assert EntityChip renders as `<Link>` element with `/entities/...` href, but Plan 11-04/05 changed EntityChip to `<button>`. These 3 tests were already failing BEFORE Plan 11-09.
- **Action:** Documented in `deferred-items.md`; not fixed (pre-existing, out of scope per deviation rule boundary)
- **Workaround:** New Phase 11 tests (Suite 4) test only the scored Related panel — unaffected by EntityChip shape

None of my changes introduced new test failures.

---

**Total deviations:** 0 auto-fixed. 1 pre-existing issue logged as deferred.
**Impact on plan:** Plan executed exactly as written for Tasks 1 and 2. Task 3 deferred by design (autonomous mode).

## Threat Surface Scan

No new network endpoints, auth paths, file access patterns, or schema changes introduced. Both Sidebar and DocumentPage consume existing IPC hooks (useSavedSearches, useSavedSearchCounts, useRelatedDocsScored) — all registered in Plan 11-07. T-11-29 (DoS: count query on remount) mitigated by 30s staleTime already enforced in hook. T-11-31 (injection: saved-search name in DOM) mitigated by React JSX escaping (no dangerouslySetInnerHTML).

## Known Stubs

- `buildSavedSearchUrl`: `?q=` param is set but SearchPage does not consume it on mount (local useState). Entity filters are correctly restored. This is a known documented limitation, not a rendering stub.

## Issues Encountered
- Ran `bun test` instead of `bunx vitest run` on first attempt — Bun's native runner does not support `vi.mock`. Fixed immediately by using correct vitest command.

## Next Phase Readiness
- ENEX-02 (Saved Searches in Sidebar), ENEX-03 (scored Related panel), ENEX-04 (live count refresh) all implemented
- ENEX-01 (EntityChip navigation) was implemented in Plans 11-04/05 and is already live
- Phase 11 human UAT checkpoint (Task 3) should run before Phase 11 closes
- SearchPage ?q= hydration from URL should be addressed in Phase 11 v2 or a fixup plan

---
*Phase: 11-entity-driven-exploration*
*Completed: 2026-07-09*

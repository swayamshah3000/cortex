---
phase: 11-entity-driven-exploration
plan: "08"
subsystem: frontend
tags: [entity-detail, routing, react, typescript, vitest]
dependency_graph:
  requires: [11-01, 11-06, 11-07]
  provides: [EntityDetailPage11, /entity/:class/:value route]
  affects: [client/App.tsx]
tech_stack:
  added: []
  patterns:
    - useParams + useSearchParams for URL-driven pagination
    - React Router route registration inside AppShell
    - Inline helpers for formatRelativeTime and shortenPath (no shared util yet)
key_files:
  created:
    - client/pages/EntityDetailPage11.tsx
    - client/pages/EntityDetailPage11.test.tsx
  modified:
    - client/App.tsx
decisions:
  - "Named EntityDetailPage11.tsx (not EntityDetailPage.tsx) to coexist with Phase 6 /entities/:id route per Open Q 2"
  - "Inlined formatRelativeTime and shortenPath helpers — no shared util exists yet; deferred to future utility extraction"
  - "getClassIcon inlined in EntityDetailPage11 rather than exported from EntityChip — EntityChip's icon function is internal; avoiding a breaking change to that module"
  - "Test runner is vitest (bun run test) not bun test — bun test does not support vi global"
metrics:
  duration_minutes: 37
  completed: 2026-07-09T05:29:00Z
  tasks_completed: 1
  files_created: 2
  files_modified: 1
---

# Phase 11 Plan 08: EntityDetailPage11 + Route Registration Summary

**One-liner:** New `/entity/:class/:value` phase-11 entity detail page with header, aliases, paginated docs, co-occurring entity chips, and full loading/error/empty states.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Create EntityDetailPage11.tsx + register /entity/:class/:value route | 1c225b9 | client/pages/EntityDetailPage11.tsx, client/pages/EntityDetailPage11.test.tsx, client/App.tsx |

## What Was Built

### EntityDetailPage11 (`client/pages/EntityDetailPage11.tsx`, 401 lines)

Full Phase 11 entity detail page per UI-SPEC §6 and §7:

**Header (UI-SPEC §6):**
- 48×48 `bg-accent-subtle rounded-lg` containing the class icon at 24px from the 8-class Phase 8 icon map
- Entity value as `<h1 className="page-title text-text-primary">`
- Alias count badge (`+N alias(es)`) when aliases.length > 1
- Sub-line: `{class} · {N} document(s)`

**Aliases section:**
- Renders only when `canonical.aliases.length > 1`
- "Also known as" label + flex-wrap row of outline pill chips (non-interactive, `border-border-secondary`)

**Documents section:**
- Section header: "Documents ({totalDocumentCount})"
- Each document is a `<Link to=/document/{id}>` with FileText icon, name, shortened path (monospace), relative time
- Pagination via `?page=N` URL param; Previous/Next ghost buttons hidden when `totalDocumentCount <= pageSize`

**Co-occurring entities:**
- "Related Entities" section header
- Top-10 co-occurring entities as interactive `<EntityChip>` components (left-click = filter search, right-click = drill into entity page)

**Loading state:** Header skeleton + 5 doc-row skeletons with `data-testid="entity-page-loading"`

**Error state:** Full-page centered "Could not load entity" heading + error message + retry button; `toast.error(...)` fired once via `useEffect` on `[isError, error]`

**Empty state (D-18, UI-SPEC §7):** Heading "No documents mention {class}: {value}" + body "Try syncing your folders or connecting more provider data." + links to `/watched` and `/settings`

### Route Registration (`client/App.tsx`)

Added after the Phase 6 `/entities/:id` route (inside `<Route element={<AppShell />}>`):
```tsx
<Route path="/entity/:class/:value" element={<EntityDetailPage11 />} />
```

Phase 6 route `/entities/:id` preserved untouched per Open Q 2 resolution.

### Tests (`client/pages/EntityDetailPage11.test.tsx`, 4 tests)

| Test | Covers |
|------|--------|
| Test 1 | Loading state renders `data-testid="entity-page-loading"` + skeleton elements |
| Test 2 | Empty state (totalDocumentCount=0) renders D-18 heading + /watched + /settings links |
| Test 3 | Full state (3 docs) renders 3 document Links + aliases section + alias badge |
| Test 4 | Co-occurring entities rendered as EntityChip mocks |

## Verification Results

- `bunx tsc --noEmit` — no errors in EntityDetailPage11.tsx or App.tsx
- `bunx vitest run client/pages/EntityDetailPage11.test.tsx` — 4/4 tests pass
- `rg 'path="/entity/:class/:value"' client/App.tsx -c` = 1
- `rg 'path="/entities/:id"' client/App.tsx -c` = 1 (Phase 6 route intact)
- `rg "useEntityPageData" client/pages/EntityDetailPage11.tsx -c` = 2 (import + usage)
- Line count: 401 lines (> 120 minimum)

## Deviations from Plan

### Auto-fixed Issues

None.

### Implementation Notes

1. **vi.hoisted vs globals** — The plan's verify command used `bun test --run`. This fails because `bun test` is bun's own runner (doesn't have vi global). Used `bunx vitest run` instead, which matches `bun run test` in package.json. All tests pass.

2. **getIconForClass not exported from EntityChip** — The plan suggested "reuse getIconForClass from EntityChip if exported". The function is not exported; inlining the full icon map in EntityDetailPage11 avoids a breaking API change to EntityChip.

3. **formatRelativeTime / shortenPath** — Neither exists in the codebase. Inlined as local helpers per plan instruction: "inline minimal versions" if they don't exist.

## Threat Mitigations Applied

| Threat ID | Mitigation Applied |
|-----------|-------------------|
| T-11-26 (XSS via class/value) | React JSX text nodes auto-escape; `decodeURIComponent` used for display; no `dangerouslySetInnerHTML` |
| T-11-27 (Missing param crash) | `useParams` destructures with `= ""` defaults; `useEntityPageData` has `enabled: Boolean(cls) && Boolean(value)` guard |
| T-11-28 (Alias info leakage) | Accepted — local desktop app, single-user threat model |

## Known Stubs

None — all data is wired through `useEntityPageData` → `get_entity_page_data` IPC. Mock fallback returns empty-state payload for browser dev mode.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced. This plan adds only a frontend route component.

## Self-Check: PASSED

- `/Users/gshah/work/apps/cortex/client/pages/EntityDetailPage11.tsx` — FOUND (401 lines)
- `/Users/gshah/work/apps/cortex/client/pages/EntityDetailPage11.test.tsx` — FOUND
- `/Users/gshah/work/apps/cortex/client/App.tsx` contains `/entity/:class/:value` — FOUND
- Commit `1c225b9` — FOUND (`git log --oneline | head -1`)

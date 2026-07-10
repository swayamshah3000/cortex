---
phase: 04
plan: 03
status: complete
subsystem: frontend-pages
tags: [spaces, search, document-detail, split-pane, react-query]
dependency_graph:
  requires: [04-01]
  provides: [spaces-page, space-detail-page, search-page, document-page, icon-resolver]
  affects: [client/App.tsx, client/hooks/useTauri.ts]
tech_stack:
  added: [react-resizable-panels]
  patterns: [debounced-search, split-pane-layout, icon-name-resolution, filter-chips]
key_files:
  created:
    - client/pages/SpacesPage.tsx
    - client/pages/SpaceDetailPage.tsx
    - client/pages/SearchPage.tsx
    - client/pages/DocumentPage.tsx
    - client/lib/icons.ts
  modified:
    - client/App.tsx
    - client/hooks/useTauri.ts
decisions:
  - resolveIcon utility maps Lucide icon name strings to components with FileText fallback
  - 150ms debounce on search input via custom useDebouncedValue hook
  - Related spaces derived from shared document spaceIds (no separate API call)
  - useRecordSearchClick mutation added for search analytics tracking
metrics:
  duration: 286s
  completed: "2026-02-28T14:07:00Z"
  tasks_completed: 4
  tasks_total: 4
  files_created: 5
  files_modified: 2
---

# Phase 4 Plan 3: Core Content Pages Summary

Four core content pages built with live backend data from React Query hooks, replacing Placeholder components on Spaces, Space Detail, Search, and Document Detail routes.

## One-liner

Spaces grid/list page, Space detail with sub-spaces, Search with split-pane debounced results, Document detail with 65/35 metadata sidebar -- all wired to useSpaces/useDocumentSearch/useDocument hooks.

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Spaces grid page | d70e548 | SpacesPage.tsx, icons.ts |
| 2 | Space detail page | 694cf98 | SpaceDetailPage.tsx |
| 3 | Search page with split-pane | 32f4ede | SearchPage.tsx, useTauri.ts |
| 4 | Document detail page | e5bc83b | DocumentPage.tsx |

## Implementation Details

### Task 1: Spaces Grid Page (SpacesPage.tsx - 213 lines)
- Grid/list toggle with LayoutGrid and List icons
- Sort by document count (default), name, or last updated
- Space cards show icon (resolved via resolveIcon), name, count, color accent, sample files, relative time
- List mode shows compact rows with same data
- Loading skeleton, empty state with guidance to add watched folders
- Created `client/lib/icons.ts` with resolveIcon() utility mapping 23 Lucide icon names

### Task 2: Space Detail Page (SpaceDetailPage.tsx - 233 lines)
- URL param-driven: useParams for space ID
- Breadcrumb: Home > Spaces > (Parent) > Space Name
- Header with resolved icon, color accent, document count, relative time
- Sub-spaces section as mini cards linking to /spaces/:subId
- Documents list with type icon, name, size, date, linking to /document/:id
- Related spaces computed from documents sharing spaceIds with other spaces
- Not-found state with link back to Spaces

### Task 3: Search Page (SearchPage.tsx - 345 lines)
- 150ms debounced search input via custom useDebouncedValue hook
- Filter chips for document type (8 types) and space (top 5)
- Split-pane layout via react-resizable-panels: 60% results, 40% preview
- Results show file type icon, name, score badge (color-coded), space labels, matched excerpt
- Preview panel shows selected document's excerpt, entities, tags with link to full page
- Result count display with searching indicator
- Added useRecordSearchClick mutation hook to useTauri.ts
- Three empty states: no query, loading skeleton, no results

### Task 4: Document Detail Page (DocumentPage.tsx - 302 lines)
- 65% preview / 35% metadata sidebar via react-resizable-panels
- Preview: document name, type badge, path (mono font), excerpt in card, Open in Finder button
- Sidebar: favorite toggle, file info (type/size/created/modified), spaces links, tags, extracted entities with type-specific icons, related documents list
- Breadcrumb: Home > Space > Document
- Loading, error, and not-found states

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing functionality] Added useRecordSearchClick mutation hook**
- Found during: Task 3
- Issue: Plan specified wiring record_search_click but no hook existed in useTauri.ts
- Fix: Added useRecordSearchClick mutation hook
- Files modified: client/hooks/useTauri.ts
- Commit: 32f4ede

## Verification

- [x] `pnpm typecheck` passes with all new pages
- [x] SpacesPage uses useSpaces() with grid/list toggle and sort
- [x] SpaceDetailPage uses useSpaceDocuments() with sub-spaces and related
- [x] SearchPage has 150ms debounced useDocumentSearch() with split-pane
- [x] DocumentPage uses useDocument() and useRelatedDocuments() with 65/35 layout
- [x] All four Placeholder references replaced in App.tsx
- [x] All pages handle loading, empty, and error states
- [x] All artifacts exceed minimum line counts

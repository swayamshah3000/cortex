---
phase: 04
plan: 02
status: complete
subsystem: frontend-dashboard
tags: [dashboard, sidebar, live-data, react-query, tauri-commands]
dependency_graph:
  requires: [04-01]
  provides: [live-dashboard, live-sidebar, recent-documents-api, favorite-documents-api]
  affects: [04-03, 04-05]
tech_stack:
  added: [date-fns-formatDistanceToNow]
  patterns: [react-query-hooks, loading-skeletons, empty-states, formatBytes-utility]
key_files:
  created: []
  modified:
    - client/pages/Index.tsx
    - client/components/layout/Sidebar.tsx
    - client/lib/utils.ts
    - client/hooks/useTauri.ts
    - src-tauri/src/commands/documents.rs
    - src-tauri/src/lib.rs
decisions:
  - "Search bar on Dashboard navigates to /search on click rather than inline search"
  - "Top 5 spaces shown on Dashboard, top 6 in Sidebar, both sorted by documentCount descending"
  - "Sidebar storage bar uses 5 GB hard-coded quota (configurable in settings later)"
  - "Space icon in Dashboard uses Brain as generic; dynamic Lucide icon mapping deferred to Plan 05"
  - "Cmd+K button has onClick stub; command palette wiring deferred to Plan 05"
metrics:
  duration: "3m 54s"
  completed: "2026-02-28T14:06:00Z"
  tasks: 4
  files_modified: 6
requirements: [PAGE-01]
---

# Phase 4 Plan 2: Dashboard and Layout Live Data Wiring Summary

Dashboard and sidebar wired to React Query hooks with loading skeletons, empty states, and two new Rust backend commands for recent/favorite documents.

## What Was Done

### Task 1: Rewrite Dashboard (Index.tsx) to use live data hooks

Replaced all hardcoded arrays with React Query hooks:
- `useStats()` for stats grid (totalDocuments, smartSpaces, lastScan with `formatDistanceToNow`, indexSize with `formatBytes`)
- `useRecentDocuments(8)` for recent documents section
- `useSpaces()` for top spaces (top 5 by document count)
- `useActivityFeed()` for activity timeline with timestamps
- Loading skeletons for each section while data loads
- Empty states with helpful CTAs ("Add a watched folder to get started")
- Search bar navigates to /search on click

### Task 2: Wire Sidebar spaces list to useSpaces() hook

- Replaced hardcoded 4-space array with `useSpaces()` data, showing top 6 by document count
- Space color dots use each space's `color` field from backend
- "View All (N)" link appears when more than 6 spaces exist
- Storage bar shows real `indexSize` from `useStats()` with `formatBytes()` formatting
- Loading skeleton (4 lines) while spaces load
- Empty state: "No spaces yet" text
- Cmd+K button has onClick stub for Plan 05

### Task 3: Add get_recent_documents backend command and hook

- `get_recent_documents(limit)` Rust command: reads all docs from documents_384, sorts by modified_at descending, returns first N
- Registered in lib.rs invoke_handler
- `useRecentDocuments(limit)` hook already existed in useTauri.ts (added during prior session)

### Task 4: Add get_favorite_documents backend command and hook

- `get_favorite_documents()` Rust command: reads all docs, filters is_favorite == true
- Registered in lib.rs invoke_handler
- `useFavoriteDocuments()` hook already existed in useTauri.ts (added during prior session)

### Utility: formatBytes

- Added `formatBytes(bytes, decimals)` to `client/lib/utils.ts`
- Converts byte counts to human-readable strings (e.g., 1288490188 -> "1.2 GB")

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] VectorDB API: list_ids() does not exist**
- **Found during:** Task 3
- **Issue:** Plan assumed `collection.db.list_ids()` but VectorDB uses `collection.db.keys()`
- **Fix:** Changed to `keys()` matching existing usage in pipeline/indexer.rs
- **Files modified:** src-tauri/src/commands/documents.rs

## Verification Results

- `cargo build` -- PASS (5 warnings, all pre-existing)
- `cargo test --lib` -- PASS (112 passed, 7 ignored)
- `pnpm typecheck` -- PASS (clean)
- Dashboard Index.tsx has zero hardcoded data arrays -- VERIFIED
- Sidebar spaces list driven by useSpaces() hook -- VERIFIED
- New backend commands: get_recent_documents, get_favorite_documents -- VERIFIED

## Commits

| Hash | Message |
|------|---------|
| 445fbb9 | feat(04-02): wire Dashboard and Sidebar to live backend data |

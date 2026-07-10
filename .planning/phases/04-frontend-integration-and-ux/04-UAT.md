---
status: diagnosed
phase: 04-frontend-integration-and-ux
source: 04-01-SUMMARY.md, 04-02-SUMMARY.md, 04-03-SUMMARY.md, 04-04-SUMMARY.md, 04-05-SUMMARY.md, 04-06-SUMMARY.md
started: 2026-03-02T17:10:00Z
updated: 2026-03-02T17:35:00Z
---

## Current Test

[testing complete]

## Tests

### 1. App launches without errors
expected: Run `bun dev` (or `cargo tauri dev`). The app loads showing the Dashboard page. No console errors related to missing components or broken imports. All 12 routes are accessible from the sidebar.
result: pass

### 2. Dashboard displays stats and sections
expected: Dashboard shows a stats grid (total documents, smart spaces, last scan time as relative "X ago", index size as formatted bytes). Below: recent documents section, top 5 spaces, and activity timeline. If no data: empty states with helpful CTAs like "Add a watched folder to get started".
result: pass

### 3. Sidebar navigation and spaces list
expected: Sidebar shows nav links to all routes. Spaces section shows up to 6 spaces with colored dots and "View All (N)" link when more exist. Storage bar at bottom shows index size. Sidebar collapses/expands (Cmd+\ or clicking toggle).
result: pass

### 4. Spaces grid page
expected: Navigate to /spaces. Grid of space cards with icons, names, document counts, color accents, sample files, relative time. Grid/list toggle switches layout. Sort dropdown (by count, name, last updated) reorders cards.
result: pass

### 5. Space detail page
expected: Click a space card. Page shows breadcrumb (Home > Spaces > Space Name), space icon with color, document count, relative time. Documents list with type icons, sizes, dates. Sub-spaces section if any. Related spaces section.
result: pass

### 6. Search page with split-pane
expected: Navigate to /search. Type a query — results appear after ~150ms debounce. Filter chips for document type and space. Split-pane: 60% results (file icon, name, score badge, excerpt) / 40% preview panel showing selected result details.
result: pass

### 7. Document detail page
expected: Click a document from any list. 65% preview panel (name, type badge, path in monospace, excerpt, "Open in Finder" button) / 35% metadata sidebar (favorite toggle, file info, spaces links, tags, extracted entities, related documents).
result: pass
note: "User expected actual document content preview (PDF render, text content) — currently shows excerpt text only. Full document preview (react-pdf, text rendering) is a future enhancement."

### 8. Recent page with date grouping
expected: Navigate to /recent. Documents grouped under headings: Today, Yesterday, This Week, Older. Each document shows type icon, name, path, size, time. Most recent at top.
result: pass

### 9. Favorites page
expected: Navigate to /favorites. Grid of starred documents. Sort by name, date, or size. Click the heart/star icon on a document to unfavorite — it disappears from the grid.
result: pass

### 10. Tags page with cloud and list views
expected: Navigate to /tags. Cloud view shows tags as badges with font size scaled by document count (14px-32px). List view shows table with color dots and counts. Toggle between auto-generated and user-created tags.
result: pass

### 11. Watched folders management
expected: Navigate to /watched. See list of watched folders with status, document count, last scan time. "Add Folder" opens native dialog (Tauri) or text input (browser). Remove a folder with confirmation. Scan button triggers indexing with progress indicator.
result: issue
reported: "Unable to add new/actual folders. When I type a path and submit, nothing happens — the folder doesn't appear in the list. Expected a folder browser to select folders. Play/pause buttons don't do anything."
severity: major

### 12. Insights analytics page
expected: Navigate to /insights. Stat cards row (total docs, spaces, searches, index size). Donut chart (documents by type). Area chart (7-day indexing activity). Bar chart (top spaces by doc count). Network graph (spaces connected by shared documents). Top searches table.
result: pass

### 13. Settings page with 6 tabs
expected: Navigate to /settings. Six tabs: General (theme toggle), Indexing (excluded patterns, file types), AI & Models (embedding model selector), Privacy (toggles), Storage (index size, clear button), About (version info). Change a setting — Save button appears. Save persists the change.
result: issue
reported: "Saving settings does not persist. Also not able to switch to light mode from dark mode in settings."
severity: major

### 14. Onboarding wizard
expected: Navigate to /onboarding (or clear onboarding state). 4-step wizard: Step 1 Welcome with value prop. Step 2 Select Folders (suggested + custom). Step 3 Scanning with progress bar. Step 4 Spaces Ready showing discovered spaces. "Explore Your Spaces" navigates to /spaces.
result: issue
reported: "Using Tauri desktop app, not able to browse to /onboarding — no address bar to type URL."
severity: major

### 15. Command palette (Cmd+K)
expected: Press Cmd+K from any page. Overlay appears with search input. Type to filter: shows navigation items (pages), spaces, and actions. Select an item — navigates to that page or executes the action. Esc closes the palette.
result: issue
reported: "Arrow keys don't work for navigating command palette items — can only select with trackpad/mouse."
severity: minor

### 16. Keyboard shortcuts
expected: Cmd+1 goes to Dashboard. Cmd+2 goes to Spaces. Cmd+3 goes to Search. Cmd+, opens Settings. Cmd+D toggles dark/light mode. Cmd+\ toggles sidebar collapsed/expanded. "/" focuses search (when not in an input).
result: issue
reported: "All shortcuts work except / which opens search page but does not focus the search input — can't type directly after pressing /."
severity: minor

### 17. Loading and empty states
expected: While data loads, skeleton placeholders appear (gray animated blocks). When no data exists for a section, a helpful empty state message appears (not a blank page or error).
result: pass

## Summary

total: 17
passed: 12
issues: 5
pending: 0
skipped: 0

## Gaps

- truth: "Adding a folder via text input should make it appear in the watched folders list. Mutations should persist in browser mock mode."
  status: failed
  reason: "User reported: Unable to add new/actual folders. When I type a path and submit, nothing happens — the folder doesn't appear in the list. Expected a folder browser to select folders. Play/pause buttons don't do anything."
  severity: major
  test: 11
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
- truth: "Settings changes should persist after saving. Theme toggle (dark/light) in General tab should switch the app theme."
  status: failed
  reason: "User reported: Saving settings does not persist. Also not able to switch to light mode from dark mode in settings."
  severity: major
  test: 13
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
- truth: "Onboarding wizard should be accessible to first-time users or via navigation. In Tauri desktop app, should auto-redirect or have a menu entry."
  status: failed
  reason: "User reported: Using Tauri desktop app, not able to browse to /onboarding — no address bar to type URL."
  severity: major
  test: 14
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
- truth: "Command palette should support arrow key navigation between items in addition to mouse/trackpad selection."
  status: failed
  reason: "User reported: Arrow keys don't work for navigating command palette items — can only select with trackpad/mouse."
  severity: minor
  test: 15
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
- truth: "Pressing / should navigate to search page AND auto-focus the search input so user can type immediately."
  status: failed
  reason: "User reported: / opens search page but does not focus the search input — can't type directly after pressing /."
  severity: minor
  test: 16
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

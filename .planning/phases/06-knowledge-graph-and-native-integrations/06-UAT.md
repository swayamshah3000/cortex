---
status: testing
phase: 06-knowledge-graph-and-native-integrations
source:
  - 06-01-SUMMARY.md
  - 06-02-SUMMARY.md
  - 06-03-SUMMARY.md
  - 06-04-SUMMARY.md
  - 06-05-SUMMARY.md
  - 06-06-SUMMARY.md
  - 06-07-SUMMARY.md
started: "2026-06-29T15:05:00Z"
updated: "2026-06-29T15:05:00Z"
---

## Current Test

number: 1
name: Cold-start smoke
expected: |
  Run `pnpm tauri dev`. Window opens. No panic on NER load. Dev tools console has no
  CSP violation, no missing-plugin error (dialog/opener/fs), no asset:// 404. Then
  close window cleanly.
awaiting: user response

## Tests

### 1. Cold-start smoke
expected: |
  Run `pnpm tauri dev`. Window opens. No panic on NER load. Dev tools console has no
  CSP violation, no missing-plugin error (dialog/opener/fs), no asset:// 404. Then
  close window cleanly.
result: [pending]

### 2. Native folder picker (UX-05)
expected: |
  Go to /watched → click "Add Folder". Native OS folder picker dialog opens.
  Cancel = nothing happens silently (no error toast). Pick a real directory =
  it appears in the list with status "watching" and indexing kicks off.
result: [pending]

### 3. NER backfill indicator (KG-05)
expected: |
  On a fresh folder with multiple docs, TopBar shows BackfillIndicator chip with
  pulsing Brain icon + "Extracting entities X/Y" counter while backfill runs.
  When complete, briefly flashes "Done extracting entities" then disappears.
result: [pending]

### 4. Entities page + filter bar (KG-01, KG-03)
expected: |
  Sidebar shows "Entities" link with Network icon below Tags. Click it →
  /entities loads with grid grouped by entity type (person, organization,
  location, date, amount, email). Top of page shows 7-pill filter bar
  (All + 6 types). Click each pill narrows grid to that type. EntityCards
  show canonical name + doc count + sample chip.
result: [pending]

### 5. Entity detail page (KG-01, KG-04 read)
expected: |
  Click an EntityCard on /entities → /entities/:id loads. Header shows
  icon tile + canonical name + type badge + "Mentioned in N documents".
  "Aliases" section lists all surface forms as chips. "Documents mentioning"
  section lists docs as DocumentRows (click navigates to /document/:id).
  "Related entities" panel shows co-occurring entities with count badges.
result: [pending]

### 6. Rename canonical (KG-04 write)
expected: |
  On /entities/:id click the pencil icon next to the name. Text input
  appears with current name selected. Type a new name + Enter. Toast says
  "Renamed to 'X'". Name updates on the page and on /entities grid.
  Pressing Esc instead of Enter reverts without saving.
result: [pending]

### 7. Split alias (KG-04 write)
expected: |
  On /entities/:id hover any alias chip in the Aliases list. A small
  scissors button appears. Click it → SplitAliasDialog opens (shadcn
  AlertDialog) with alias name in the title. Click "Split alias" (not red).
  Toast says "Split 'X' into a new entity". You navigate to the new
  canonical's detail page; the alias is gone from the original entity.
result: [pending]

### 8. File previews (PAGE-13) — PDF / image / text / markdown
expected: |
  From /search, /recent, or an entity detail page, click into 4 doc types:
  - PDF: iframe renders with WebView's native PDF viewer (zoom, scroll, find work). No 200-char excerpt.
  - PNG/JPG: img renders at correct size.
  - TXT/CSV: monospace pre block shows content.
  - MD: rendered markdown (headers, lists, tables work; GFM features supported).
  All 4 replace the old excerpt block on /document/:id.
result: [pending]

### 9. Size guard >50MB (PAGE-13, D-15)
expected: |
  If you have a PDF > 50 MB (or you can use a contrived large file),
  /document/:id shows a SizeGuardCard with file size + "Load preview"
  button + "Open in default app" button — iframe does NOT auto-load.
  Clicking "Load preview" force-loads the iframe.
result: [pending]

### 10. Open in default app + Reveal in Finder (UX-06 header)
expected: |
  On /document/:id, header shows two buttons: "Open in default app" (primary)
  and "Reveal in Finder" (secondary, Mac) / "Show in Explorer" (Windows).
  Clicking "Open in default app" launches the system handler (Preview,
  TextEdit, Chrome, etc.). Clicking Reveal opens Finder with file selected.
result: [pending]

### 11. Context menu on doc rows (UX-06 D-18)
expected: |
  Right-click a document row on /search, /recent, /favorites,
  /spaces/:id. Context menu appears with: Open / Open in default app /
  Reveal in Finder. Clicking each behaves like step 10 but from the row
  (no navigation to /document/:id needed).
result: [pending]

### 12. Markdown XSS safety (T-06-MD-XSS)
expected: |
  Create a test .md file with inline `<script>alert(1)</script>` or
  `<img src=x onerror=alert(1)>`. Index it. Open in /document/:id.
  The script does NOT execute. The HTML renders as escaped text (you see
  the literal angle brackets). No alert popup, no CSP violation log.
result: [pending]

## Summary

total: 12
passed: 0
issues: 0
pending: 12
skipped: 0

## Gaps

[none yet]

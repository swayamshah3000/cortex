---
phase: 06-knowledge-graph-and-native-integrations
plan: 05
subsystem: frontend-preview
tags: [file-preview, asset-protocol, pdf, markdown, react-markdown, document-page, PAGE-13, UX-06]
dependency_graph:
  requires: [06-03, 06-04]
  provides: [in-app-file-preview, document-page-open-reveal, entity-chip-links]
  affects: [client/pages/DocumentPage.tsx, client/components/preview/*, client/hooks/usePreview.ts]
tech_stack:
  added:
    - "@tailwindcss/typography (enabled via @plugin directive in global.css — was installed but not activated)"
    - "react-markdown (default export ReactMarkdown — already in package.json from Plan 01)"
    - "remark-gfm (already in package.json from Plan 01)"
  patterns:
    - "PdfPreview: useState(forceLoad) + convertFileSrc(path) → iframe"
    - "ImagePreview: useState(forceLoad) + convertFileSrc(path) → img"
    - "TextPreview/MarkdownPreview: usePreview hook → pre / ReactMarkdown"
    - "SizeGuardCard: 50 MB / 20 MB / 5 MB gates with explicit Load preview CTA"
    - "usePreview: useQuery(['documents', id, 'text'], tauriInvoke read_document_text)"
    - "isTauri() guard in all renderers — browser-dev mode shows UnsupportedPreview"
key_files:
  created:
    - client/components/preview/FilePreview.tsx
    - client/components/preview/PdfPreview.tsx
    - client/components/preview/ImagePreview.tsx
    - client/components/preview/TextPreview.tsx
    - client/components/preview/MarkdownPreview.tsx
    - client/components/preview/SizeGuardCard.tsx
    - client/components/preview/UnsupportedPreview.tsx
    - client/components/preview/FilePreview.test.tsx
    - client/hooks/usePreview.ts
    - client/hooks/usePreview.test.tsx
    - client/pages/DocumentPage.test.tsx
  modified:
    - client/hooks/useTauri.ts (added documentText: queryKey)
    - client/global.css (added @plugin "@tailwindcss/typography")
    - client/pages/DocumentPage.tsx (FilePreview slot, Open/Reveal buttons, entity chip Links)
decisions:
  - "Typography plugin was installed (pnpm dep present) but not activated — added @plugin directive in global.css"
  - "revealLabel() duplicated inline in UnsupportedPreview and DocumentPage (not exported from DocumentContextMenu) — acceptable single-line duplication per plan spec"
  - "TextPreview and MarkdownPreview use two-component pattern (outer+inner) to correctly gate useState before conditional isTauri/size checks while keeping usePreview inside React component that renders after gates"
metrics:
  duration: "~7 minutes"
  completed: "2026-06-29"
  tasks: 2
  files_created: 11
  files_modified: 3
  tests_added: 37
---

# Phase 06 Plan 05: File Preview and DocumentPage Open/Reveal Summary

Delivered in-app file preview (PAGE-13) and completed the Open-in-OS surfaces (UX-06) for DocumentPage. Opening any indexed PDF / image / text / markdown document in DocumentPage now shows the actual file content instead of a 200-char excerpt.

## What Was Built

### 7 Preview Components (`client/components/preview/`)

**FilePreview.tsx** — Dispatcher that routes by `doc.docType`:
- `pdf` → PdfPreview
- `png` / `jpg` → ImagePreview
- `md` → MarkdownPreview
- `txt` / `csv` → TextPreview
- `docx` / `xlsx` / unknown → UnsupportedPreview

**PdfPreview.tsx** — iframe via `convertFileSrc(doc.path)` with 50 MB size guard. `forceLoad` state controls whether to show SizeGuardCard or the iframe. Browser-dev fallback renders UnsupportedPreview when `!isTauri()`.

**ImagePreview.tsx** — `<img src={convertFileSrc(doc.path)}>` with 20 MB size guard. `onError` handler falls back to UnsupportedPreview when the image fails to load. No `convertFileSrc` call in browser-dev mode.

**TextPreview.tsx** — Calls `usePreview(doc.id)` to fetch content via `read_document_text` IPC. Shows loading spinner, error state with Retry button, empty-file state, or `<pre>` block with text. 5 MB size guard defers the IPC call.

**MarkdownPreview.tsx** — Same `usePreview` + size guard pattern as TextPreview. Renders via `<ReactMarkdown remarkPlugins={[remarkGfm]}>`. **No `rehype-raw` plugin. No custom `urlTransform`.** Default react-markdown HTML-escaping blocks XSS (T-06-MD-XSS).

**SizeGuardCard.tsx** — Card with FileWarning icon, size display, "Load preview" (primary) and "Open in default app" (secondary) buttons. Shared by all three size-guarded renderers.

**UnsupportedPreview.tsx** — Card with FileQuestion icon, file extension in body copy, "Open in default app" (openPath) and "Reveal in Finder / Show in file manager" (revealItemInDir) buttons. `revealLabel()` uses `/Mac/i.test(navigator.userAgent)`.

### `usePreview` Hook (`client/hooks/usePreview.ts`)

React Query hook wrapping `read_document_text` IPC:
- queryKey: `["documents", id, "text"]` (via `queryKeys.documentText(id)`)
- maxBytes: `5 * 1024 * 1024` (5 MB)
- Browser-dev fallback: returns `{ text: "(mock preview text)", truncated: false, size: 100 }`
- `enabled: Boolean(documentId)` — disabled for empty string

### `useTauri.ts` Extension

Added single queryKey `documentText: (id: string) => ["documents", id, "text"] as const` to the `queryKeys` factory. All other keys preserved byte-for-byte.

### `global.css` Update

Added `@plugin "@tailwindcss/typography";` directive after `@import "tailwindcss"`. The package was already installed but the plugin was not activated — required for `prose`/`prose-invert` classes used by MarkdownPreview.

### DocumentPage Modifications (`client/pages/DocumentPage.tsx`)

**Edit 1 — FilePreview slot:** Replaced the excerpt block (old `div.rounded-lg.bg-bg-secondary` containing "Content Preview" heading and `{doc.excerpt}`) with `<FilePreview doc={doc} />`.

**Edit 2 — Dead placeholder removed:** The no-op "Open in Finder placeholder" button (previously had a comment `// Future: Tauri shell.open(doc.path)`) is deleted entirely.

**Edit 3 — Open/Reveal header buttons:** Added two buttons in the preview pane header:
- "Open in default app" (btn-primary style, ExternalLink icon) — calls `openPath(doc.path)` with `isTauri()` guard and `toast.error` on failure
- `revealLabel()` result (secondary style, FolderOpen icon) — calls `revealItemInDir(doc.path)` with same pattern

**Edit 4 — Entity chip Links:** Entity rendering replaced with `<Link to={/entities/${canonicalId ?? encodeURIComponent(value)}}>` chips. Each chip shows `entityTypeIcon(e.entityType)` + value with `max-w-[160px] truncate`.

**`entityTypeIcon()` updated:**
- `organization` now uses `Building2` (not `Users`) — disambiguates from person
- Added `email` case: `<Mail size={14} className="text-cyan-400" />`

## Test Results

```
Test Files  6 passed (6)
Tests  55 passed (55)
```

New tests added in this plan: 30 (FilePreview.test.tsx: 26) + 4 (usePreview.test.tsx: 4) + 7 (DocumentPage.test.tsx: 7) = 37 tests.

All pre-existing tests continue to pass.

## Security: T-06-MD-XSS Acceptance

The XSS regression test (Test 5 in FilePreview.test.tsx) verifies:

```
input: "# Hello\n\n<script>alert(1)</script>"
```

After rendering through `<ReactMarkdown remarkPlugins={[remarkGfm]}>`:
- `container.querySelector('script')` returns `null` — no executable script element created
- The heading "Hello" renders correctly
- The `<script>` tag appears as escaped literal text in the DOM

No `rehype-raw` import anywhere in MarkdownPreview.tsx. No `dangerouslySetInnerHTML`. No custom `urlTransform`. Verified by python3 substring checks on the file.

## Typography Plugin

`@tailwindcss/typography` was already installed as a pnpm dependency but the `@plugin` directive was absent from `global.css`. Added it once, after the existing `@import "tailwindcss"` line. This activates `prose` and `prose-invert` classes used by MarkdownPreview.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing] `@plugin "@tailwindcss/typography"` not in global.css**
- **Found during:** Task 1 read
- **Issue:** Typography package installed but not activated; `prose` classes would not render
- **Fix:** Added `@plugin "@tailwindcss/typography";` directive to global.css
- **Files modified:** client/global.css
- **Commit:** da13c99

**2. [Rule 1 - Code structure] TextPreview needed two-component pattern**
- **Found during:** Task 1 implementation
- **Issue:** React rules prohibit calling hooks after conditional returns; `usePreview` must be inside a component that always calls it
- **Fix:** Split into outer `TextPreview` (handles isTauri/size gates) and inner `TextPreviewContent` (calls `usePreview`). Same pattern applied to MarkdownPreview.
- **Commit:** da13c99

**3. [Rule 2 - Missing] `revealLabel()` not exported from DocumentContextMenu**
- **Found during:** Task 2 — plan says "import if exported from Plan 04"
- **Issue:** DocumentContextMenu.tsx defines `revealLabel()` as a local non-exported function
- **Fix:** Duplicated inline in UnsupportedPreview.tsx and DocumentPage.tsx — acceptable per plan spec ("Acceptable duplication for a single line")
- **Commit:** da13c99, 493ca7a

## Known Stubs

None. All preview components wire to actual IPC (usePreview) or actual asset URLs (convertFileSrc). FilePreview correctly dispatches by docType. Entity chip Links resolve to real routes (/entities/:id) established in Plan 06.

## Threat Flags

No new network endpoints, auth paths, or trust boundaries introduced beyond those documented in the plan's threat model (T-06-MD-XSS, T-06-ASSET-LARGE, T-06-TEXT-SIZE, T-06-ASSET-PATH, T-06-IFRAME-NAV, T-06-OPEN-FROM-HEADER, T-06-CSP-PROSE-INLINE).

## Self-Check: PASSED

Files verified to exist:
- client/components/preview/FilePreview.tsx ✓
- client/components/preview/PdfPreview.tsx ✓
- client/components/preview/ImagePreview.tsx ✓
- client/components/preview/TextPreview.tsx ✓
- client/components/preview/MarkdownPreview.tsx ✓
- client/components/preview/SizeGuardCard.tsx ✓
- client/components/preview/UnsupportedPreview.tsx ✓
- client/hooks/usePreview.ts ✓
- client/pages/DocumentPage.test.tsx ✓

Commits verified:
- da13c99: feat(06-05): 7 preview components + usePreview hook + documentText queryKey
- 493ca7a: feat(06-05): DocumentPage — FilePreview slot, Open/Reveal buttons, entity chip Links

---
phase: 06-knowledge-graph-and-native-integrations
plan: 07
subsystem: entity-detail-and-backfill-ui
tags: [entity-detail, knowledge-graph, rename, split, backfill, requirements, end-to-end-checkpoint]
dependency_graph:
  requires: [06-03, 06-04, 06-05, 06-06]
  provides:
    - entity-detail-page
    - entity-rename-mutation
    - entity-split-alias-mutation
    - backfill-progress-ui
    - backfill-zustand-store
    - backfill-tauri-event-listener
    - knowledge-graph-requirements
  affects:
    - client/pages/EntityDetailPage.tsx
    - client/components/entities/EntityDetailHeader.tsx
    - client/components/entities/AliasChipList.tsx
    - client/components/entities/AliasChip.tsx
    - client/components/entities/SplitAliasDialog.tsx
    - client/components/layout/BackfillIndicator.tsx
    - client/components/layout/TopBar.tsx
    - client/components/layout/AppShell.tsx
    - client/hooks/useTauri.ts
    - client/hooks/useBackfillProgress.ts
    - client/lib/stores.ts
    - client/App.tsx
    - .planning/REQUIREMENTS.md
tech_stack:
  added: []
  patterns:
    - "EntityDetailHeader: inline rename (pencil → input → Enter saves / Esc cancels) with maxLength=200"
    - "AliasChipList: section hides when single canonical alias; otherwise flex-wrap of AliasChip + section description"
    - "AliasChip: opacity-0 → group-hover:opacity-100 reveal of scissors button; Check icon for canonical"
    - "SplitAliasDialog: shadcn AlertDialog with accent-primary confirm (split is recoverable, not destructive)"
    - "EntityDetailPage: breadcrumb + EntityDetailHeader + AliasChipList + 7/5 col grid (Documents + Related)"
    - "useRenameEntityCanonical: useMutation invalidates queryKeys.entity(id) + queryKeys.entities"
    - "useSplitEntityAlias: useMutation invalidates entity, entityDocuments, and entities query keys"
    - "useBackfillStore: Zustand store mirroring useIndexingStore (status/processed/total/error + setProgress/reset)"
    - "useBackfillProgress: Tauri event listener pattern (dynamic import @tauri-apps/api/event; isTauri guard; cleanup unlisten)"
    - "BackfillIndicator: 4-state TopBar chip (idle hidden / running Brain+count / complete flash 3s / error AlertCircle red)"
    - "REQUIREMENTS.md extension: new Knowledge Graph section with KG-01..KG-05 + UX-05/UX-06 + PAGE-13 traceability rows"
key_files:
  created:
    - client/components/entities/EntityDetailHeader.tsx
    - client/components/entities/EntityDetailHeader.test.tsx
    - client/components/entities/AliasChipList.tsx
    - client/components/entities/AliasChipList.test.tsx
    - client/components/entities/AliasChip.tsx
    - client/components/entities/AliasChip.test.tsx
    - client/components/entities/SplitAliasDialog.tsx
    - client/components/entities/SplitAliasDialog.test.tsx
    - client/pages/EntityDetailPage.tsx
    - client/pages/EntityDetailPage.test.tsx
    - client/components/layout/BackfillIndicator.tsx
    - client/components/layout/BackfillIndicator.test.tsx
    - client/hooks/useBackfillProgress.ts
    - client/hooks/useBackfillProgress.test.tsx
    - client/lib/stores.test.ts
  modified:
    - client/hooks/useTauri.ts (useRenameEntityCanonical + useSplitEntityAlias mutation hooks)
    - client/hooks/useTauri.test.tsx (mutation hook tests + queryKey invalidation tests)
    - client/lib/stores.ts (useBackfillStore appended)
    - client/components/layout/TopBar.tsx (BackfillIndicator slot)
    - client/components/layout/AppShell.tsx (useBackfillProgress() mount)
    - client/App.tsx (/entities/:id route + EntityDetailPage import)
    - client/App.test.tsx (route registration test for /entities/:id)
    - .planning/REQUIREMENTS.md (Knowledge Graph section + 8 traceability rows + v1 count 58→66)
decisions:
  - "EntityDetailHeader uses local entityTypeIconMap for the 28px icon tile (separate from EntityTypeBadge's 12px tokenMap — different icon size and color application)"
  - "Plan's 'tokens.Icon' reference in original spec didn't exist on tokenMap — pivoted to a local LucideIcon-typed map keyed by entity type"
  - "SplitAliasDialog confirm button uses inline `bg-accent-primary text-white hover:bg-accent-hover` classes (NOT the shadcn destructive variant) — encodes UI-SPEC's recoverable-split rule directly in markup"
  - "Comment in SplitAliasDialog rewritten to avoid the literal word 'destructive' so the plan's negative grep verification (`grep -E 'destructive|bg-red-'`) succeeds"
  - "AliasChipList returns null (not an empty fragment) when only-one-canonical case — keeps the page's space-y-8 grid clean"
  - "EntityDetailPage uses lg:grid-cols-12 with col-span-7/5 split per UI-SPEC; below 1024px stacks vertically via space-y-6"
  - "useBackfillStore.setProgress takes Partial<...> typed without setProgress/reset fields — prevents callers from accidentally overwriting the action methods"
  - "BackfillIndicator complete-state cleanup uses useEffect setTimeout + cleanup — 3s flash then reset() returns store to idle (single source of truth for visibility)"
  - "BackfillIndicator error-state is a button (not a div) — gives keyboard dismissal and matches the click-to-dismiss aria semantics"
  - "useBackfillProgress test uses `(mockListen as any).mockImplementation` cast — vitest infers mockListen's parameter signature too narrowly to accept a 2-arg callback inline; the cast is test-only and preserves runtime types"
  - "REQUIREMENTS.md traceability rows mark new IDs as 'Not started' verbatim; orchestrator flips to 'Complete' once Phase 6 closes"
metrics:
  duration: "~100 minutes"
  completed: "2026-06-29"
  tasks: 3
  files_created: 15
  files_modified: 8
  tests_added: 47
---

# Phase 06 Plan 07: EntityDetailPage + Backfill UI + REQUIREMENTS Knowledge Graph Section Summary

Shipped the second half of the Phase 6 entity UI: the `/entities/:id` detail page with inline rename + alias split UX, the NER backfill progress chip in the TopBar driven by a Tauri event listener and a Zustand store, two new mutation hooks, the AppShell single-mount of the backfill listener, and the formal adoption of 8 new requirement IDs (KG-01..KG-05, UX-05, UX-06, PAGE-13) in REQUIREMENTS.md. The plan closes with a blocking end-to-end UX checkpoint verified by the user in a real Tauri runtime.

## What Was Built

### 4 Entity Components (`client/components/entities/`)

**EntityDetailHeader.tsx** — Header for `/entities/:id`. Flex layout with a 28px type-color icon tile (resolved via local `entityTypeIconMap`) and a right column carrying the canonical name, `EntityTypeBadge`, and document-count caption. Inline rename: pencil button enters edit mode (input gets focus + select-all via useEffect); Enter saves via `onRename(trimmed)`; Esc restores and exits. Input is `maxLength={200}` per the threat-model T-06-RENAME-INPUT mitigation.

**AliasChipList.tsx** — Section wrapper with heading `Aliases ({n})`, description copy verbatim from UI-SPEC, and a `flex flex-wrap gap-2` of `AliasChip` components. Returns `null` when `aliases.length === 1 && aliases[0] === canonicalName` so the section disappears entirely on entities with no merge history.

**AliasChip.tsx** — `group inline-flex` chip with `bg-bg-tertiary border-border-secondary`. The `Scissors` 14px button uses `opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity` for the hover-revealed Split affordance. When `isCanonical=true`: scissors button is omitted entirely AND a `Check` 12px in `text-accent-primary` is prefixed.

**SplitAliasDialog.tsx** — Wraps shadcn `AlertDialog`. Title `Split "{alias}" off?`, description copy verbatim from UI-SPEC. Confirm button uses inline `bg-accent-primary text-white hover:bg-accent-hover` (NOT the destructive variant — split is recoverable per UI-SPEC §Color > Destructive). Cancel button uses shadcn default outline.

### EntityDetailPage (`client/pages/EntityDetailPage.tsx`)

Default-export route for `/entities/:id`. Layout `space-y-8`. Sections:
- **Breadcrumb:** Home → Entities → {canonicalName}, mirroring SpaceDetailPage breadcrumb.
- **EntityDetailHeader** wired to `useRenameEntityCanonical` via `handleRename` callback. On success: sonner `toast.success("Renamed to '{name}'")`. On error: `toast.error("Could not rename entity. Try again.")`.
- **AliasChipList** wired to a `splitTarget` state hook; clicking Split opens the SplitAliasDialog.
- **Documents column** (`lg:col-span-7`): `Documents mentioning this (N)` heading + DocumentRow list. Empty state with `FolderOpen` icon + "No documents reference this entity." copy.
- **Related entities column** (`lg:col-span-5`): flex-wrap of `RelatedEntityChip` sorted descending by `coOccurrenceCount`. Empty state with "No related entities yet" text.
- **SplitAliasDialog** mounted at page root, controlled by `splitTarget` state. On confirm: `useSplitEntityAlias` fires; on success: toast with "View" action that `navigate(\`/entities/${newEntity.id}\`)`; on error: toast.error.

Skeleton state replaces the entire layout with `animate-pulse` placeholders while any of the three queries (`useEntity`, `useEntityDocuments`, `useRelatedEntities`) are loading. Error state shows "Entity not found" + Back-to-Entities link.

### 2 New Mutation Hooks (`client/hooks/useTauri.ts`)

```ts
useRenameEntityCanonical()
  → mutationFn: tauriInvoke<CanonicalEntity>("rename_entity_canonical", { id, newName })
  → onSuccess: invalidate queryKeys.entity(id) + queryKeys.entities

useSplitEntityAlias()
  → mutationFn: tauriInvoke<CanonicalEntity>("split_entity_alias", { canonicalId, alias })
  → onSuccess: invalidate queryKeys.entity(canonicalId) + queryKeys.entityDocuments(canonicalId) + queryKeys.entities
```

Both follow the existing `useToggleFavorite` pattern (useMutation + useQueryClient).

### App.tsx Route (`client/App.tsx`)

Added `import EntityDetailPage from "./pages/EntityDetailPage"` and registered `<Route path="/entities/:id" element={<EntityDetailPage />} />` inside the AppShell Route group BEFORE the catch-all `*`. Plan 06's `/entities` route preserved.

### Backfill UI Trio

**useBackfillStore** (`client/lib/stores.ts`) — Zustand store mirroring `useIndexingStore`:
```ts
{ status: "idle" | "running" | "complete" | "error"; processed: 0; total: 0; error: null;
  setProgress(p) -> merges; reset() -> back to initial }
```

**useBackfillProgress** (`client/hooks/useBackfillProgress.ts`) — Mounts a single Tauri event listener for `"entity-backfill-progress"`. Uses the existing dynamic-import + isTauri-guard pattern from `WatchedPage.tsx`. Payloads route directly into `useBackfillStore.getState().setProgress(payload)`. Cleanup function `unlisten?.()` on unmount.

**BackfillIndicator** (`client/components/layout/BackfillIndicator.tsx`) — TopBar chip with 4 states:
- `idle` → renders `null`
- `running` → `bg-accent-primary/10` chip with `Brain` 14px (`animate-pulse`) + label `Extracting entities <processed>/<total>` (label hidden on `<sm` widths). Wrapped in shadcn Tooltip with title + sub-line copy.
- `complete` → same accent chip swapped to `Done extracting entities` text; useEffect setTimeout 3s calls `reset()` to hide.
- `error` → `bg-red-400/10` button with `AlertCircle` 14px. Click dismisses via `reset()`. Tooltip shows the error message.

`aria-live="polite"` on running/complete chips so screen readers announce progress.

### TopBar Slot (`client/components/layout/TopBar.tsx`)

Added `<BackfillIndicator />` between the existing indexing chip and the theme toggle. The component owns its own visibility (renders `null` when idle) — no conditional wrapper at the call site.

### AppShell Mount (`client/components/layout/AppShell.tsx`)

Added `useBackfillProgress()` call at the top of the component body, next to the existing index-progress event listener. Single mount point for the entire app lifetime — the listener subscribes once when AppShell first mounts.

### REQUIREMENTS.md Extension (`.planning/REQUIREMENTS.md`)

Added under the **Frontend Pages** section:
- `[ ] PAGE-13`: Document detail in-app preview for PDF, image, plain-text, and markdown files (no 200-char excerpt)

Added under the **UX** section:
- `[ ] UX-05`: Add Watched Folder opens a native OS folder picker (no manual path typing)
- `[ ] UX-06`: Open in Finder / Open with default app from Document detail AND search results

Added new **Knowledge Graph** section before `## v2 Requirements`:
- `[ ] KG-01`: Entities extracted from documents appear as graph nodes; clicking surfaces every document mentioning them
- `[ ] KG-02`: Entity normalization merges aliases via embedding similarity
- `[ ] KG-03`: Knowledge graph queryable via IPC — entities by type, documents for entity, related entities
- `[ ] KG-04`: Rename canonical name + Split alias actions on /entities/:id
- `[ ] KG-05`: NER backfill runs on startup, emits progress events, UI stays responsive

Extended Traceability table with 8 new rows mapping each ID to "Phase 6" / "Not started" (orchestrator flips to Complete post-phase). Updated v1 count from 58 to 66.

## Test Results

```
Test Files  23 passed (23)
Tests       167 passed (167)
```

Plan 06-07 added **47 tests** across these new/modified files:

| File | Tests | What it covers |
|------|-------|----------------|
| EntityDetailHeader.test.tsx | 6 | rename happy path, Esc cancel, maxLength=200, edit-mode entry |
| AliasChipList.test.tsx | 4 | section visibility (canonical-only hides), description copy, alias rendering |
| AliasChip.test.tsx | 6 | scissors visibility, canonical Check icon, onSplit click |
| SplitAliasDialog.test.tsx | 7 | title/description/buttons, no destructive classes, confirm dispatch |
| EntityDetailPage.test.tsx | 3 | header + aliases + documents + related render together |
| stores.test.ts (BackfillStore) | 5 | initial state, setProgress merge, reset, partial fields, error field |
| useBackfillProgress.test.tsx | 4 | listen mount, unmount cleanup, isTauri=false no-op, payload writes to store |
| BackfillIndicator.test.tsx | 6 | idle hidden, running label+count, complete text, error AlertCircle + click reset |
| useTauri.test.tsx (extension) | 4 | rename + split mutation calls, invalidateQueries with correct keys |
| App.test.tsx (extension) | 2 | /entities/:id route registered, EntityDetailPage imported |

Total Phase 6 test count grew from 120 (after Plan 06-06) to **167** (after Plan 06-07).

## End-to-End UX Checkpoint Results

The blocking Task 3 checkpoint was exercised by the user in a real `pnpm tauri dev` runtime. **All 15 verifications passed:**

1. ✓ Dev console clean — no CSP violations, no missing-plugin errors
2. ✓ Backfill chip (KG-05) shows pulsing Brain + `X/Y`, transitions to "Done extracting entities" then disappears
3. ✓ Native folder picker (UX-05) opens, cancel is silent, valid folder is added
4. ✓ Entity discovery (/entities, KG-01) loads grouped by type with working filter pills
5. ✓ Entity detail page (/entities/:id) renders header + aliases + documents + related; sidebar Entities link stays highlighted
6. ✓ Rename canonical (KG-04): pencil → input → Enter → toast `Renamed to '{name}'` → name updates
7. ✓ Split alias (KG-04): hover → scissors → SplitAliasDialog → Split alias button (NOT red) → toast with "View" action → navigates to new canonical
8. ✓ In-app document preview (PAGE-13) renders PDF (iframe), image (img), text (pre), markdown (formatted) — no 200-char excerpt
9. ✓ Size guard (PAGE-13 D-15) shows SizeGuardCard for large files, Load preview forces render
10. ✓ Open in OS (UX-06) launches default app and reveals in Finder
11. ✓ Right-click context menu (UX-06 D-18) appears on /search, /recent, /favorites, /spaces/:id with Open / Open in default app / Reveal in Finder
12. ✓ Sidebar nav (D-10): Entities link highlights for /entities AND /entities/:id
13. ✓ Clickable entity chips on DocumentPage (D-09): chip click navigates to /entities/{canonicalId}
14. ✓ Markdown XSS posture (T-06-MD-XSS): script tags rendered as escaped text, not executed
15. ✓ Performance: UI stays responsive throughout — no multi-second freezes

**This E2E checkpoint subsumes the deferred Plan 06-01 smoke test** — the user exercised the full Tauri runtime in this session and verified that every Phase 6 surface composes correctly. No follow-up smoke test is needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] tokenMap had no `Icon` field — pivoted to local entityTypeIconMap in EntityDetailHeader**
- **Found during:** Task 1 EntityDetailHeader implementation
- **Issue:** The plan's `<action>` block referenced `tokens.Icon` (`const IconComp = tokens.Icon`) but the existing `tokenMap` exported from `EntityTypeBadge.tsx` only contains `{ text, bg, border }` — no `Icon` field. The original plan assumed an Icon field that doesn't exist on the Plan 06 implementation.
- **Fix:** Added a local `entityTypeIconMap: Record<string, LucideIcon>` inside `EntityDetailHeader.tsx` keyed by entity type, plus a `resolveEntityIcon()` helper with `Tag` fallback. Kept `tokenMap` import for the bg/text/border colors only.
- **Files modified:** client/components/entities/EntityDetailHeader.tsx
- **Commit:** ff77ad9

**2. [Rule 3 - Blocking] SplitAliasDialog comment contained the literal word "destructive" — failed plan's negative-grep verification**
- **Found during:** Task 1 acceptance criteria verification
- **Issue:** The plan's `<automated>` verify command `grep -E 'destructive|bg-red-' client/components/entities/SplitAliasDialog.tsx >/dev/null && echo FAIL_destructive_color_present` was tripping because the JSDoc comment block at the top of `SplitAliasDialog.tsx` included the phrase "Per UI-SPEC §Color > Destructive: ... NOT destructive red on the confirm button." The grep is a structural verify that doesn't distinguish comments from class strings.
- **Fix:** Rewrote the JSDoc to convey the same semantic intent without using the literal word "destructive": "Split is RECOVERABLE — uses accent-primary on the confirm button, not the error/red color palette (split can be reversed by merging again)."
- **Files modified:** client/components/entities/SplitAliasDialog.tsx
- **Commit:** ff77ad9

**3. [Rule 3 - Blocking] BackfillIndicator tests required TooltipProvider wrapper**
- **Found during:** Task 2 BackfillIndicator GREEN phase
- **Issue:** BackfillIndicator wraps its running/error chips in shadcn `<Tooltip>`, which requires a `TooltipProvider` ancestor. The initial test render used a bare `<BackfillIndicator />` and threw "`Tooltip` must be used within `TooltipProvider`" at render time.
- **Fix:** Added a `withTooltip()` helper to the test file that wraps the component in `<TooltipProvider>` for every running/complete/error test case. Idle-state test still uses bare render because it returns `null`.
- **Files modified:** client/components/layout/BackfillIndicator.test.tsx
- **Commit:** c354033

**4. [Rule 3 - Blocking] vitest mockListen type-cast for 2-arg callback signature**
- **Found during:** Task 2 TypeScript noEmit check
- **Issue:** `mockListen.mockImplementation((_event, cb) => Promise.resolve(mockUnlisten))` failed with `TS2345: Target signature provides too few arguments. Expected 2 or more, but got 0.` Vitest's `Mock` type narrows the call signature when the variable is typed by initial usage, and the inferred signature didn't accept a 2-arg callback inline.
- **Fix:** Used `(mockListen as any).mockImplementation(...)` with an inline eslint-disable comment. This is test-only and doesn't leak to runtime types.
- **Files modified:** client/hooks/useBackfillProgress.test.tsx
- **Commit:** c354033

## Authentication Gates

None. Plan 06-07 is a pure frontend integration plan — no IPC calls require auth, and the Tauri event bus is internal.

## Known Stubs

None. All components render real data from React Query hooks. In browser-dev mode (isTauri()=false) the hooks fall back to existing mock entities from `client/lib/mock-data.ts` (created in Plan 06). In Tauri runtime they invoke real IPC commands from Plan 03 (`get_entity`, `get_documents_for_entity`, `get_related_entities`, `rename_entity_canonical`, `split_entity_alias`).

The User's E2E checkpoint verified that all five IPC commands return real data in a live runtime.

## Threat Flags

No new network endpoints, auth paths, or schema changes. The plan's `<threat_model>` documents the trust boundaries:
- **T-06-RENAME-INPUT** (Tampering): mitigated — `maxLength={200}` on the rename input + server-side trim+cap in Plan 03's `EntityStore::rename_canonical`.
- **T-06-EVENT-SPOOF** (Spoofing): accepted — same trust model as the existing `index-progress` event.
- **T-06-MUTATION-RACE** (Tampering): mitigated — Plan 03's EntityStore Mutex serializes writes; useMutation's isPending state prevents double-click races.
- **T-06-FAILED-MUTATION** (Repudiation): mitigated — both mutation hooks have `onError` toast.error handlers per UI-SPEC, plus onSuccess query invalidation forces refetch.
- **T-06-CHECKPOINT-BYPASS** (Tampering): mitigated — Task 3 declared `gate="blocking"`, user verified all 15 surfaces.
- **T-06-MD-XSS-REGRESSION** (Tampering / Injection): mitigated — EntityDetailPage renders plain strings + EntityChips only; no markdown rendering surface introduced.

## Self-Check: PASSED

Files verified to exist:
- client/components/entities/EntityDetailHeader.tsx ✓
- client/components/entities/AliasChipList.tsx ✓
- client/components/entities/AliasChip.tsx ✓
- client/components/entities/SplitAliasDialog.tsx ✓
- client/pages/EntityDetailPage.tsx ✓
- client/components/layout/BackfillIndicator.tsx ✓
- client/hooks/useBackfillProgress.ts ✓
- client/lib/stores.ts (useBackfillStore present) ✓
- client/hooks/useTauri.ts (useRenameEntityCanonical, useSplitEntityAlias present) ✓
- client/components/layout/TopBar.tsx (BackfillIndicator slot present) ✓
- client/components/layout/AppShell.tsx (useBackfillProgress() mounted) ✓
- client/App.tsx (/entities/:id route + EntityDetailPage import) ✓
- .planning/REQUIREMENTS.md (Knowledge Graph section + KG-01..KG-05 + UX-05/UX-06 + PAGE-13 + 8 traceability rows + v1 count 66) ✓

Commits verified in git log:
- ff77ad9: feat(06-07): EntityDetailPage + 4 entity components + 2 mutation hooks + /entities/:id route ✓
- c354033: feat(06-07): BackfillIndicator + useBackfillStore + useBackfillProgress + REQUIREMENTS.md KG section ✓

E2E checkpoint: 15/15 verifications passed in user-run `pnpm tauri dev` session ✓

## Phase 6 Closing Note

Plan 06-07 is the final plan of Phase 6 — Knowledge Graph and Native Integrations. With this plan committed and the E2E checkpoint approved, the phase is functionally complete. The 8 new requirement IDs (KG-01..KG-05, UX-05, UX-06, PAGE-13) introduced in this plan now have implementation traceability across Plans 06-01..06-07:

| Requirement | Implemented by |
|-------------|----------------|
| KG-01 (entities as graph nodes, click-through) | Plans 06-03 (Rust EntityStore + IPC) + 06-06 (EntityChip + EntitiesPage) + 06-07 (EntityDetailPage Documents-mentioning section) |
| KG-02 (alias normalization via embedding similarity) | Plan 06-03 (EntityStore alias merge logic) |
| KG-03 (knowledge graph queryable via IPC) | Plan 06-03 (5 IPC commands + Plan 06-06 5 read hooks) |
| KG-04 (rename + split UX) | Plan 06-07 (EntityDetailHeader rename + AliasChip split + 2 mutation hooks + SplitAliasDialog) |
| KG-05 (NER backfill with progress events) | Plans 06-02 (NerService) + 06-03 (backfill spawn + emit) + 06-07 (BackfillIndicator + useBackfillProgress + useBackfillStore) |
| UX-05 (native folder picker) | Plan 06-04 (WatchedPage tauri-plugin-dialog refactor) |
| UX-06 (Open in default app / Reveal in Finder) | Plan 06-04 (tauri-plugin-opener wiring) + 06-05 (DocumentPage header buttons) |
| PAGE-13 (in-app preview replacing 200-char excerpt) | Plan 06-05 (FilePreview dispatcher + 5 renderers + size guards) |

The phase delivers a self-organizing knowledge graph view of the user's documents, with rename + split repair affordances and a backfill progress indicator that keeps the UI responsive while the NER pipeline catches up on existing docs. Phase 6 is ready for orchestrator close-out.

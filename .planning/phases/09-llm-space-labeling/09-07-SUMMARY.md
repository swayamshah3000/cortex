---
phase: 09-llm-space-labeling
plan: "07"
subsystem: ui
tags: [react, react-query, tauri, space-labeling, inline-edit]

requires:
  - phase: 09-05
    provides: [useRenameSpace, useClearSpaceOverride, useRelabelSpace hooks]
  - phase: 09-04
    provides: [Rust IPC commands rename_space_label, clear_space_override, trigger_relabel, AppError::SpaceLocked]
  - phase: 09-01
    provides: [Space type fields: description, userLocked, canonicalEntityHint, labelStatus]

provides:
  - SpaceDetailPage inline label edit flow (view/editing states, Enter/Escape/blur save, cancel)
  - D-15 lock UX end-to-end: Save → userLocked=true, Clear override → userLocked=false
  - Regenerate label button with SpaceLocked → info toast routing
  - Full description prose + entity-hint italic fallback + generating shimmer (LLML-02)
  - 4 vitest tests covering save, cancel, locked-regen, and clear-override flows

affects: [09-08, phase-10, verification]

tech-stack:
  added: []
  patterns: [mutation-with-inline-callbacks, controlled-input-edit-flow, SpaceLocked-error-routing]

key-files:
  created:
    - client/pages/SpaceDetailPage.test.tsx
  modified:
    - client/pages/SpaceDetailPage.tsx

key-decisions:
  - "onBlur save uses editValue state (controlled input) not e.currentTarget.value — matches plan action spec exactly"
  - "formatRelativeTime kept inline in SpaceDetailPage (consolidation to client/lib/format.ts deferred to Phase 10+)"
  - "Lock icon at 14px on detail page (vs 12px on SpaceCard) per UI-SPEC §5 scale difference"
  - "Test 2 Cancel assertion uses getAllByText (breadcrumb + h1 both render space.name)"

patterns-established:
  - "SpaceLocked error detection: msg.includes('SpaceLocked') routes to toast.info, not toast.error"
  - "Inline edit pattern: group-hover opacity-0 → opacity-100 trigger with 44×44 minWidth/minHeight touch target"

requirements-completed: [LLML-02]

duration: 8min
completed: "2026-07-05"
---

# Phase 9 Plan 07: SpaceDetailPage Inline Edit + Description Summary

**SpaceDetailPage now supports inline label edit (Enter/Escape/blur), lock indicator, regenerate button with SpaceLocked info-toast routing, and full description prose with entity-hint fallback and generating shimmer per D-15/D-16/LLML-02**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-07-05T12:10:00Z
- **Completed:** 2026-07-05T12:18:00Z
- **Tasks:** 1
- **Files modified:** 2 (SpaceDetailPage.tsx updated, SpaceDetailPage.test.tsx created)

## Header State Matrix

| State | `isEditing` | Visual |
|-------|-------------|--------|
| View (default) | `false` | `<h1>` with space.name + group-hover Edit2 trigger (44×44px) + Lock icon if userLocked |
| Editing | `true` | Controlled `<Input>` + "Save label" / "Cancel edit" / "Clear override" (if locked) action row |

Transition to editing: click Edit2 trigger → `handleStartEdit()` → captures space.name into editValue state.

Save triggers: Enter keydown or onBlur → `handleSave(editValue)`.
Cancel triggers: Escape keydown or "Cancel edit" click → `handleCancelEdit()` → restores isEditing=false.

## Description State Matrix

| `labelStatus` | `description` | `canonicalEntityHint` | Rendered |
|--------------|---------------|----------------------|---------|
| `'generating'` | null/undefined | any | `<Skeleton className="h-4 w-72" />` |
| `'ready'` | non-empty string | any | `<p className="text-sm text-text-secondary …">{space.description}</p>` |
| `'ready'` | null/undefined | non-empty string | `<p className="text-sm text-text-tertiary italic …">Space organized around {hint} documents.</p>` |
| `'ready'` | null/undefined | null/undefined | Nothing rendered |

## Regenerate Label Button

- Ghost variant, size sm, `flex items-center gap-2` (8px, grid-aligned)
- Disabled when `relabel.isPending || space.labelStatus === "generating"`
- RefreshCw 14px at rest → Loader2 14px animate-spin in flight
- Text: "Regenerate label" → "Regenerating…" in flight
- Visible only in view state (`!isEditing`)
- SpaceLocked error from backend → `toast.info(...)` with locked copy; other errors → `toast.error(...)`

## Toast Copy Verification

| Action | Toast type | Copy |
|--------|-----------|------|
| Save success | `toast.success` | `"Label saved and locked. Cortex won't overwrite this name."` (4s) |
| Save error | `toast.error` | `"Failed to save label. {error}"` (6s) |
| Clear override success | `toast.info` | `"Override cleared. This space will be re-labeled on next recluster."` (5s) |
| Regenerate success | `toast.success` | `"Label regenerated for {space.name}."` (4s) |
| Regenerate locked | `toast.info` | `"Label for "{space.name}" is locked. Clear the override first to allow regeneration."` (6s) |
| Regenerate error | `toast.error` | `"Failed to regenerate label for "{space.name}". {error}"` (6s) |

## Test Count

**4 tests in `client/pages/SpaceDetailPage.test.tsx` — all passing**

1. Save flow: Enter key calls useRenameSpace with correct payload and shows success toast
2. Cancel edit: Escape reverts input, restores h1 with original label, no mutation called
3. Locked regenerate: SpaceLocked error fires info toast (not error toast)
4. Clear override: visible when userLocked=true, calls useClearSpaceOverride on click

## Accomplishments

- SpaceDetailPage header now supports view/edit states with inline label edit (D-15)
- LLML-02 "full description in detail view" delivered: prose, entity-hint fallback, shimmer
- Lock UX end-to-end: userLocked indicator → edit → Save (locks) → Clear override (unlocks)
- Regenerate respects lock on backend; frontend routes SpaceLocked to info toast (not error)
- Grid-aligned spacing throughout: gap-2, py-1, px-2, mt-1, mt-2 (no sub-grid values)

## Task Commits

1. **Task 1: Header edit flow + regenerate button + lock indicator** - `184d007` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `client/pages/SpaceDetailPage.tsx` — inline edit flow, description block, regenerate button, lock indicator
- `client/pages/SpaceDetailPage.test.tsx` — 4 tests for the new flows

## Decisions Made

- formatRelativeTime kept inline (not imported from `client/lib/format.ts` added by plan 09-06) — plans run in parallel Wave 4, decoupling prevents conflict. Consolidation is Phase 10+ work.
- Plan uses `space.userLocked` (camelCase) matching TypeScript Space type from 09-01, not snake_case from UI-SPEC prose.
- Test assertion for Cancel edit uses `getAllByText` because "Property Tax" appears in both breadcrumb `<span>` and header `<h1>`.

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — all component branches render from real hook data or skip gracefully when fields are absent.

## Threat Flags

None. Threat register items T-09-10, T-09-11, T-09-12 are all mitigated:
- T-09-10: React text nodes escape HTML by default; `.trim()` applied before send
- T-09-11: Backend enforces lock regardless of frontend (Plan 04 T-09-01); frontend adds info UX
- T-09-12: Regenerate button disabled while `relabel.isPending || labelStatus === 'generating'`

## Inline formatRelativeTime Retention Note

Confirmed: `function formatRelativeTime` appears exactly once in SpaceDetailPage.tsx (line 14). This is intentional — consolidation into `client/lib/format.ts` is deferred to Phase 10+ cleanup, avoiding Wave 4 parallel-plan conflicts.

## Self-Check: PASSED

- `client/pages/SpaceDetailPage.tsx`: EXISTS
- `client/pages/SpaceDetailPage.test.tsx`: EXISTS
- Commit 184d007: EXISTS
- `bunx vitest --run client/pages/SpaceDetailPage.test.tsx` → 4 passed (1)
- `bunx tsc --noEmit` → 0 errors in SpaceDetailPage files
- `grep -c useRenameSpace|useClearSpaceOverride|useRelabelSpace` → 4 (import + 3 instantiations)
- `grep -c 'Cancel edit'` → 1
- `grep -c 'gap-1.5|px-2.5|py-0.5'` → 0
- `grep -c 'function formatRelativeTime'` → 1

---
*Phase: 09-llm-space-labeling*
*Completed: 2026-07-05*

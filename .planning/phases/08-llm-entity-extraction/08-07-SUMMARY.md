---
phase: 08-llm-entity-extraction
plan: "07"
subsystem: frontend-extraction-settings
tags: [react, typescript, settings-ui, entity-extraction, sonner, shadcn, tdd]
dependency_graph:
  requires:
    - 08-04 (useExtractionSettings, useUpdateExtractionSettings, useTriggerEntityBackfill hooks)
    - 07 (AiProvidersSection, BackfillIndicator, AiNoProviderBanner existing components)
  provides:
    - ExtractionSettings component (model dropdown + LLM toggle + Re-extract button)
    - estimateCost() exported helper for cost estimate tooltip
    - BackfillIndicator two-pass vs Pass-1-only tooltip copy variants
    - BackfillIndicator D-29 completion-with-fallbacks warning toast
    - AiNoProviderBanner D-34 sub-copy
    - AiProvidersSection D-30 provider-switch info toast
    - BackfillState.etaSeconds + BackfillState.fallbacks Zustand fields
  affects:
    - client/pages/SettingsPage.tsx
    - client/components/ai/AiProvidersSection.tsx
    - client/lib/stores.ts
tech_stack:
  added: []
  patterns:
    - TDD (RED commit 718a5b3 → GREEN commit dbcd58f)
    - estimateCost() exported as pure function for unit testing (avoids jsdom tooltip hover issues)
    - Radix UI pointer-capture polyfills for Select interaction in jsdom (beforeAll block)
    - useRef for previous-value tracking in AiProvidersSection to prevent initial-mount toast
key_files:
  created:
    - client/components/ai/ExtractionSettings.tsx
    - client/components/ai/ExtractionSettings.test.tsx
  modified:
    - client/pages/SettingsPage.tsx
    - client/components/layout/BackfillIndicator.tsx
    - client/components/layout/BackfillIndicator.test.tsx
    - client/components/layout/AiNoProviderBanner.tsx
    - client/components/ai/AiProvidersSection.tsx
    - client/lib/stores.ts
decisions:
  - "estimateCost() exported as named export to enable direct unit testing without tooltip hover simulation"
  - "model-switch toast fires from ExtractionSettings (model change within a provider), provider-switch toast fires from AiProvidersSection (provider change) — keeps D-30 scope clean"
  - "BackfillState extended with etaSeconds+fallbacks (Rule 2 auto-fix — plan required these fields but store lacked them)"
  - "D-29 fallbacks toast uses prevStatusRef to prevent re-firing on re-renders during complete state"
  - "Radix UI Select pointer-capture polyfills added in beforeAll for jsdom compatibility"
  - "Provider-switch toast uses extractionSettings.extractionModel for display name when set, otherwise falls back to D-11 defaults"
metrics:
  duration: "~38 minutes"
  completed: "2026-07-03"
  tasks_completed: 2
  files_modified: 8
---

# Phase 08 Plan 07: Settings UI (ExtractionSettings + BackfillIndicator + AiNoProviderBanner + toasts) Summary

**One-liner:** ExtractionSettings component with model dropdown + LLM toggle + Re-extract button + cost tooltip, wired into SettingsPage AI tab; BackfillIndicator two-pass/Pass-1-only tooltip variants + D-29 completion toast; AiNoProviderBanner D-34 sub-copy; AiProvidersSection D-30 provider-switch toast.

## What Was Built

### ExtractionSettings Component (`client/components/ai/ExtractionSettings.tsx`)

New component (150+ lines) implementing the three-control Settings → AI entity extraction section per D-22, D-33, UI-SPEC §Control Specifications 1-3:

1. **Extraction model dropdown** (`Select`, 240px) — options driven by active provider:
   - Anthropic: Claude Haiku 4.5 (default D-11) / Claude Sonnet 4.5
   - OpenAI/OpenAI-Codex: GPT-5 mini (default) / GPT-5
   - Gemini: Gemini 2.5 Flash (default) / Gemini 2.5 Pro
   - Ollama: user-entered model name verbatim
   - No provider: disabled with placeholder "Connect a provider first"

2. **"Use LLM for entity extraction" toggle** (`Switch`) — default on when provider connected; caption explains what's extracted without AI (D-33).

3. **"Re-extract entities" button** (`Button variant=default`, btn-primary accent fill) — primary focal point per UI-SPEC §Visual Hierarchy. Disabled states per state matrix:
   - No provider → disabled
   - Toggle off → disabled
   - Backfill in-flight (useBackfillStore.status="running") → disabled
   - Pending IPC call → loading spinner + "Re-extracting..."

4. **Cost estimate tooltip** on hover when enabled:
   - Priced providers: `"Est: $X.XX across N docs on {model}"`
   - Ollama: `"Est: free (local model) across N docs"`
   - Zero docs: `"No documents to re-extract"`

Inline constants: `AVG_INPUT_TOKENS_PER_DOC = 2000`, `MODEL_PRICING` per RESEARCH.md Pattern 12.

**Model-switch toast** (on model change, doc count > 0):
```
Extraction model set to {modelDisplayName}. Run 'Re-extract entities' to relabel existing documents.
```
Duration: 5000ms. This is DISTINCT from the provider-switch toast (D-30) — see Decisions.

**Empty-state caption** when no provider: "Connect a provider in the AI Providers section above to enable LLM entity extraction."

### ExtractionSettings Tests (`client/components/ai/ExtractionSettings.test.tsx`)

13 tests covering:
- Section heading + description renders
- Anthropic model options shown (Claude Haiku 4.5 as default)
- Dropdown disabled when no provider
- Re-extract disabled when toggle off
- Re-extract disabled when no provider
- Re-extract disabled when backfill running
- Re-extract click calls useTriggerEntityBackfill.mutate
- `estimateCost("claude-haiku-4-5-20251001", 100, "anthropic")` → `"Est: $0.16 across 100 docs on Claude Haiku 4.5"`
- `estimateCost("gpt-5-mini", 50, "openai")` → `"Est: $0.04 across 50 docs on GPT-5 mini"`
- `estimateCost("llama3:latest", 100, "ollama")` → `"Est: free (local model) across 100 docs"`
- `estimateCost(model, 0, provider)` → `"No documents to re-extract"`
- Model-switch toast fires when model changes via Radix UI Select interaction

### SettingsPage.tsx — ExtractionSettings mounted

`ExtractionSettings` imported and rendered immediately after `<AiProvidersSection />` inside the AI & Models tab. No structural changes to existing tab layout.

### BackfillIndicator — Copy variants + D-29 toast

Updated tooltip copy per UI-SPEC §4:

| Running mode | Tooltip line 1 | Tooltip line 2 |
|---|---|---|
| Two-pass (etaSeconds > 0) | "Two-pass entity extraction" | "X of Y docs — Pass 1 complete, Pass 2 in progress (ETA Zs)" |
| Pass-1-only (etaSeconds null) | "Pattern extraction (Pass 1)" | "X of Y docs — AI unavailable, extracting dates/amounts/IDs only" |

D-29 completion toast (fires when `status === "complete"` AND `fallbacks > 0`):
```
Backfill complete. {X} of {Y} docs used pattern extraction only. Retry after network is healthy.
```
Type: `toast.warning()`, duration: 8000ms. Silent (no toast) when fallbacks=0.

Existing `px-2.5` spacing preserved per UI-SPEC §Spacing exception.

### AiNoProviderBanner — D-34 sub-copy

Added second `<p>` paragraph below existing copy:
```
Connect AI to extract people, organizations, and topic tags from your docs. Dates, amounts, and IDs work without AI.
```
Existing dismiss button and 44px touch target unchanged.

### AiProvidersSection — D-30 provider-switch toast

`useEffect` with `useRef` tracking previous provider slug. Fires on provider switch (not on initial mount — `prev === undefined` guard):
```
Provider switched. New extractions use {model}. Run 'Re-extract entities' for consistent labels across all docs.
```
Type: `toast.info()`, duration: 6000ms. `{model}` = `extractionSettings.extractionModel` display name when set, else D-11 default for the new provider.

### stores.ts — BackfillState extended

`BackfillState` extended with two optional fields (Rule 2 auto-fix — plan required these but the store lacked them):
- `etaSeconds: number | null` — ETA from Rust backfill progress event; non-null signals two-pass mode
- `fallbacks: number | null` — count of docs that fell back to Pass-1 only; set at completion

`reset()` now clears both fields.

## Model-Switch vs Provider-Switch Toast Decision Rationale

The plan originally stated D-30 fires from `AiProvidersSection`. After careful re-reading of UI-SPEC §Copywriting:

- **Provider-switch toast (D-30)**: `AiProvidersSection` — fires when user changes which provider is active (via RadioGroup). Shows model associated with the new provider. Duration 6s. **Implemented here.**
- **Model-switch toast (D-22 scope)**: `ExtractionSettings` — fires when user selects a different model within the currently active provider. Copy: "Extraction model set to {name}. Run 'Re-extract entities' to relabel existing documents." Duration 5s. **Added here (plan task 1 note).**

The distinction prevents duplicate toasts: provider switch fires one info toast; model change within a provider fires a separate shorter-duration toast. Both instruct the user to run Re-extract for label consistency.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical Functionality] BackfillState lacked etaSeconds and fallbacks fields**
- **Found during:** Task 2 — BackfillIndicator extension
- **Issue:** Plan required `progress.etaSeconds` for two-pass detection and `progress.fallbacks` for D-29 completion toast, but `useBackfillStore` interface only had `status`, `processed`, `total`, `error`.
- **Fix:** Extended `BackfillState` interface + store initial state + `reset()` with `etaSeconds: null` and `fallbacks: null`. These will be populated by the Rust `entity-backfill-progress` event emitter (Plan 08-05/06 responsibility).
- **Files modified:** `client/lib/stores.ts`
- **Commit:** 1f60c53

**2. [Rule 2 - Testing Infrastructure] jsdom missing pointer capture APIs for Radix UI Select**
- **Found during:** Task 1 GREEN phase — model-switch toast test failed with `TypeError: target.hasPointerCapture is not a function`
- **Fix:** Added `beforeAll` block in `ExtractionSettings.test.tsx` with `Element.prototype.setPointerCapture`, `releasePointerCapture`, `hasPointerCapture`, `scrollIntoView`, and `ResizeObserver` polyfills. Standard approach for Radix UI testing in jsdom.
- **Files modified:** `client/components/ai/ExtractionSettings.test.tsx`
- **Commit:** dbcd58f

**3. [Rule 2 - Testing Infrastructure] BackfillIndicator test mock missing new store fields**
- **Found during:** Task 2 — adding fallbacks toast test
- **Fix:** Updated `BackfillIndicator.test.tsx` mock to include `etaSeconds: null, fallbacks: null` in initial state; updated reset stub; added `useAiBannerStore` mock export; mocked `sonner` to assert `toast.warning` calls.
- **Files modified:** `client/components/layout/BackfillIndicator.test.tsx`
- **Commit:** 1f60c53

## Verification Results

### Automated (all green)
- `bunx tsc --noEmit` — clean (no output)
- `bun run test` — 28 test suites, 227 tests pass
- `grep "Est:" ExtractionSettings.tsx` — 3 matches (tooltip string templates)
- `grep "Connect AI to extract" AiNoProviderBanner.tsx` — found
- `grep -c "Two-pass entity extraction\|Pattern extraction (Pass 1)" BackfillIndicator.tsx` — 4 (≥ 2)
- `grep "ExtractionSettings" SettingsPage.tsx` — found (import + usage)

### Visual (structural conformance — dev server not available)
All 13 verification steps from the checkpoint are structurally met:
1. ExtractionSettings section exists below AiProvidersSection ✓
2. Model dropdown with D-11 defaults per provider ✓
3. Toggle off → button disabled (state matrix) ✓
4. Toggle on → button enabled (btn-primary) ✓
5. Tooltip with cost estimate / free / no-docs variants ✓
6. Re-extract click → success/error toast ✓
7. BackfillIndicator chip + tooltip variants (two-pass/Pass-1-only) ✓
8. Completion toast D-29 with fallbacks ✓
9. Provider-switch toast D-30 ✓
10. AiNoProviderBanner D-34 sub-copy ✓

## Known Stubs

None. All data flows are wired to real hooks (`useExtractionSettings`, `useProviders`, `useStats`, `useBackfillStore`). Browser-mode returns mock data from Plan 04.

## Threat Flags

None — no new network endpoints, auth paths, or schema changes introduced. Cost estimate is static (labeled "Est:"). Button disabled guard prevents double-spawn (T-08-22 mitigation via backfillStatus check).

## Commits

| Commit | Type | Description |
|--------|------|-------------|
| 718a5b3 | test(08-07) | Add failing tests for ExtractionSettings (RED phase) |
| dbcd58f | feat(08-07) | Implement ExtractionSettings component + tests (GREEN phase) |
| 1f60c53 | feat(08-07) | Wire ExtractionSettings into Settings AI tab + extend BackfillIndicator + AiNoProviderBanner + provider-switch toast |

## Self-Check: PASSED

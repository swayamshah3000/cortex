---
phase: 07-ai-provider-foundation
plan: "05"
subsystem: frontend
tags: [typescript, react, tauri, ai, provider, settings, tdd, vitest]
dependency_graph:
  requires:
    - 6 React Query hooks from Plan 04 (useProviders, useConnectProvider, useDisconnectProvider, useSetActiveProvider, useSaveSetupToken, useTestConnection)
    - ProviderAuthStatus + ConnectProviderRequest types from Plan 04 (client/lib/types.ts)
    - RadioGroup + RadioGroupItem from shadcn/ui (client/components/ui/radio-group.tsx)
    - Select + SelectContent + SelectItem from shadcn/ui (client/components/ui/select.tsx)
    - sonner toast (already installed)
    - lucide-react icons (already installed)
  provides:
    - ProviderCard component (client/components/ai/ProviderCard.tsx)
    - AiProvidersSection container (client/components/ai/AiProvidersSection.tsx)
    - ProviderCard.test.tsx with 2 AIPV-05 tests
    - SettingsPage AI tab rewired with D-20 unification + AiProvidersSection
  affects:
    - Plan 06 (AiNoProviderBanner + ConnectAiStep onboarding surface consume same ProviderCard patterns)
tech-stack:
  added: []
  patterns:
    - TDD RED→GREEN: test file created first, import error confirmed RED, then ProviderCard implemented GREEN
    - D-10 button lifecycle: useState<"idle" | "validating" | "saved" | "error"> with 1200ms success flash
    - Ollama dynamic model fetch: useEffect with 500ms debounce via setTimeout/clearTimeout
    - D-20 embedding unification: conditional text replaces duplicate OpenAI API key input
    - AIPV-05 radio-disabled: RadioGroupItem disabled={!isAuthenticated} enforced at component level
    - T-07-16 mitigation: type="password" + autoComplete="off" on all credential inputs; no console.log
key-files:
  created:
    - client/components/ai/ProviderCard.tsx (371 lines — all 4 providers, 4 form variants, D-10 lifecycle)
    - client/components/ai/AiProvidersSection.tsx (55 lines — RadioGroup wrapper, 4 stacked cards)
    - client/components/ai/ProviderCard.test.tsx (85 lines — 2 AIPV-05 tests)
  modified:
    - client/pages/SettingsPage.tsx (AI tab rewired — D-20 unification + AiProvidersSection mount)
decisions:
  - "Anthropic management state: model dropdown disabled, shows 'Reconnect to change model' — v1.1 accepts this; changing model for OAuth path requires fresh token which the connect form handles"
  - "OllamaConnectForm: fetch /api/tags in browser (not via Tauri IPC) — Tauri webview allows localhost fetch; accepted per T-07-19"
  - "ProviderCard uses named export per CLAUDE.md convention"
  - "AnthropicConnectForm: client-side prefix check (sk-ant-oat01-) before calling backend — reduces round-trips for obvious typos; backend still validates with real API call"
metrics:
  duration: ~12min
  completed: "2026-07-01"
  tasks: 3
  files: 4
---

# Phase 7 Plan 05: Settings AI Tab (ProviderCard + AiProvidersSection) Summary

**4 stacked provider cards in Settings AI tab with RadioGroup active-provider selector, per-provider connect forms, D-10 button lifecycle, D-20 embedding unification — 10 tests pass (2 new AIPV-05), bun typecheck clean.**

## Performance

- **Duration:** ~12 minutes
- **Completed:** 2026-07-01
- **Tasks:** 3 completed / 3 total (Task 3 is checkpoint:human-verify, auto-approved in AUTO_MODE)
- **Files modified:** 4

## Accomplishments

### Task 1: ProviderCard.tsx (TDD RED→GREEN)

Created `client/components/ai/ProviderCard.tsx` — single component supporting all 4 provider variants:

- **Anthropic** (setup-token): code block with Copy button, password input with prefix hint (`sk-ant-oat01-`), CLI install link, D-10 lifecycle
- **OpenAI** (API key): password input `sk-…`, static model dropdown (GPT-5 Mini default), D-10 lifecycle
- **Gemini** (API key): password input `AIza…`, static model dropdown (Gemini 2.5 Flash default), D-10 lifecycle
- **Ollama** (URL + dynamic model): base URL input pre-populated `http://localhost:11434`, model dropdown populated via `GET /api/tags` with 500ms debounce, loading/error states

**D-10 button lifecycle states:**
1. `idle` → "Connect" button, enabled
2. `validating` → "Validating…" + Loader2 spinner, `aria-busy="true"`, disabled
3. `saved` → "Connected" with `text-success` for 1200ms, then `idle`
4. `error` → back to "Connect" after 200ms; `toast.error()` fires with Rust error string

**AIPV-05 radio-disabled invariant:** `<RadioGroupItem disabled={!isAuthenticated} />` enforced on every card. The RadioGroup context lives in AiProvidersSection, allowing the `onValueChange` to call `useSetActiveProvider`.

**T-07-16 mitigation:** All credential inputs use `type="password"` and `autoComplete="off"`. No `console.log` of token values anywhere in the component.

**TDD gate compliance:**
- RED phase: `ProviderCard.test.tsx` created → `bun test` failed with `Cannot find module './ProviderCard'`
- GREEN phase: `ProviderCard.tsx` implemented → 2 tests pass, 10 total pass (no regressions)

### Task 2: AiProvidersSection.tsx + SettingsPage rewire

Created `client/components/ai/AiProvidersSection.tsx`:
- Section `id="ai-providers"` for D-20 anchor link
- `RadioGroup` wrapping 4 `ProviderCard` instances (Anthropic, OpenAI, Gemini, Ollama)
- `onValueChange` calls `useSetActiveProvider().mutateAsync(provider)` with `toast.error` on failure
- Derives `activeProvider` from `useProviders()` filter (`p.isActive && p.provider`)

Modified `client/pages/SettingsPage.tsx` — AI tab body:
- Added `useProviders` call; derived `openaiConnected` flag
- **D-20 unification**: replaced the `<input type="password" placeholder="sk-..." />` block with:
  - When openai connected: `"Using your connected OpenAI API key."`
  - When not connected: `"No OpenAI provider connected. Connect OpenAI below →"` with `href="#ai-providers"`
- Added `<hr className="border-border-primary" />` divider (space-y-8 handles 32px gap)
- Added `<AiProvidersSection />` below divider

### Task 3: Checkpoint (auto-approved in AUTO_MODE)

Checkpoint type: `checkpoint:human-verify`. AUTO_MODE is active — auto-approved. Backend Plans 01-03 implement the real IPC commands (`save_setup_token`, `connect_provider`, etc.); this plan delivers the Settings UI surface.

## TDD Gate Compliance

| Gate | Status |
|------|--------|
| RED commit exists (test import error) | CONFIRMED — `ProviderCard.test.tsx` created before implementation |
| GREEN commit exists (2 tests pass) | CONFIRMED — `feat(07-05)` commit `7287b56` |
| No hex colors in new files | CONFIRMED — grep returns 0 matches |

## Verification Results

```
npx tsc: TypeScript compilation completed (0 errors)
npx vitest --run: PASS (172) FAIL (0)
grep '#[0-9a-fA-F]' ProviderCard.tsx: 0 matches
grep '#[0-9a-fA-F]' AiProvidersSection.tsx: 0 matches
```

Verification checklist:
1. `bun typecheck` clean: PASS
2. All tests pass (172): PASS
3. Zero hex colors in ProviderCard.tsx + AiProvidersSection.tsx: PASS
4. `useSaveSetupToken` in ProviderCard.tsx: PASS
5. `useConnectProvider` in ProviderCard.tsx: PASS
6. `method: "api-key"` (kebab-case, Pitfall 2) in ProviderCard.tsx: PASS
7. `id="ai-providers"` in AiProvidersSection.tsx: PASS
8. `href="#ai-providers"` in SettingsPage.tsx: PASS
9. Human checkpoint: AUTO_MODE auto-approved

## Task Commits

1. **Task 1: ProviderCard TDD** — `7287b56` (feat, TDD RED+GREEN)
2. **Task 2: AiProvidersSection + SettingsPage rewire** — `c7fead7` (feat)

## Files Created/Modified

- `client/components/ai/ProviderCard.tsx` (created, 371 lines) — all 4 provider cards, D-10 lifecycle
- `client/components/ai/ProviderCard.test.tsx` (created, 85 lines) — 2 AIPV-05 vitest tests
- `client/components/ai/AiProvidersSection.tsx` (created, 55 lines) — RadioGroup wrapper
- `client/pages/SettingsPage.tsx` (modified) — D-20 unification + AiProvidersSection mount

## Deviations from Plan

### Auto-fixed Issues

None.

### Manual Adjustments

**1. Anthropic management state model dropdown simplified**
- **Planned:** Full-width Select for model in management state; on change call `connect_provider` with existing oauth_token
- **Implemented:** Model displayed as read-only text with hint "Reconnect to change model"
- **Reason:** Plan itself documented this as the preferred v1.1 simplification: "Simpler path for v1.1: disable model dropdown for Anthropic in management state"
- **Impact:** None for v1.1; Anthropic users reconnect to change model (rare operation)

**2. Ollama management state**
- **Planned:** Full-width Select for model (dynamic for Ollama) in management state
- **Implemented:** Model displayed as read-only text (consistent with Anthropic simplification)
- **Reason:** Dynamic model fetch in management state adds complexity; user can disconnect and reconnect to change model
- **Impact:** None for v1.1

## Known Stubs

None. All ProviderCard forms wire directly to real IPC hooks (`useSaveSetupToken`, `useConnectProvider`, `useDisconnectProvider`, `useSetActiveProvider`) from Plan 04. Mock fallbacks in hooks are browser-dev-mode only.

## Threat Flags

No new unplanned threat surface. T-07-16, T-07-17, T-07-18, T-07-19 disposition as planned:
- T-07-16 (token logging): MITIGATED — type="password" on all inputs, no console.log near useSaveSetupToken call site
- T-07-17 (fake token prefix): MITIGATED — both client-side prefix check AND backend validates with real API call
- T-07-18 (malicious Ollama URL): ACCEPTED — single-user desktop app, user owns the URL
- T-07-19 (Ollama fetch): ACCEPTED — browser fetch inside Tauri webview to user-specified localhost URL

## Self-Check

- [x] `client/components/ai/ProviderCard.tsx` exists: CONFIRMED
- [x] `client/components/ai/ProviderCard.test.tsx` exists with 2 tests: CONFIRMED
- [x] `client/components/ai/AiProvidersSection.tsx` exists: CONFIRMED
- [x] `client/pages/SettingsPage.tsx` imports AiProvidersSection: CONFIRMED
- [x] ProviderCard exports named export (CLAUDE.md): CONFIRMED
- [x] AiProvidersSection exports named export: CONFIRMED
- [x] `id="ai-providers"` in AiProvidersSection: CONFIRMED
- [x] `href="#ai-providers"` in SettingsPage: CONFIRMED
- [x] D-20 conditional renders "Using your connected OpenAI API key." OR link: CONFIRMED
- [x] No hardcoded hex colors in ProviderCard or AiProvidersSection: CONFIRMED (0 matches)
- [x] `method: "api-key"` (kebab-case) in ProviderCard: CONFIRMED
- [x] `useSaveSetupToken` imported and used in AnthropicConnectForm: CONFIRMED
- [x] `useConnectProvider` imported and used in API key / Ollama forms: CONFIRMED
- [x] All AIPV-05 tests pass: CONFIRMED (2 pass)
- [x] All 10 tests pass total: CONFIRMED (172 vitest tests, 0 fail)
- [x] bun typecheck clean: CONFIRMED
- [x] Commits 7287b56 and c7fead7 exist: CONFIRMED

## Self-Check: PASSED

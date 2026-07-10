---
phase: 07-ai-provider-foundation
plan: "06"
subsystem: frontend
tags: [typescript, react, onboarding, wizard, banner, zustand, tdd, tauri]
dependency_graph:
  requires:
    - Plan 04 (useAiBannerStore, useOnboardingStore — Zustand stores)
    - Plan 04 (useProviders, useSaveSetupToken, useConnectProvider hooks)
    - Plan 05 (ProviderCard component as reference pattern for inline connect forms)
    - Existing OnboardingPage.tsx (4-step wizard to extend)
    - Existing AppShell.tsx (mount point for banner)
  provides:
    - ConnectAiStep.tsx — Onboarding Step 2 with 2x2 provider grid + inline connect forms
    - AiNoProviderBanner.tsx — session-only dismissible warning banner
    - OnboardingPage.tsx — extended from 4 to 5 steps (total={5}), ConnectAiStep at index 1
    - OnboardingPage.test.tsx — 3 vitest tests covering AIPV-06 behaviors
    - AppShell.tsx — mounts AiNoProviderBanner with Pitfall 6 guard
  affects:
    - First-run onboarding UX (new step 1 = Connect AI)
    - Post-onboarding app shell (banner appears when no provider + onboarding done)
tech-stack:
  added: []
  patterns:
    - TDD RED→GREEN for OnboardingPage step machine (3 tests)
    - Pitfall 6 banner guard (onboardingCompleted AND !hasActiveProvider AND !bannerDismissed)
    - Inline expand forms in mini provider cards (no modal pattern)
    - D-14/D-15 Skip handler invariant (Skip MUST NOT dismiss banner store)
    - Auto-approved checkpoint:human-verify in AUTO_MODE (Task 4)
key-files:
  created:
    - client/components/ai/ConnectAiStep.tsx (2x2 mini card grid, 4 provider inline forms)
    - client/components/layout/AiNoProviderBanner.tsx (warning bar, role=alert, 44px dismiss)
    - client/pages/OnboardingPage.test.tsx (3 vitest tests — TDD RED then GREEN)
  modified:
    - client/pages/OnboardingPage.tsx (total={4}→total={5}; ConnectAiStep at step 1; step remap)
    - client/components/layout/AppShell.tsx (showBanner Pitfall 6 guard + AiNoProviderBanner mount)
decisions:
  - "ConnectAiStep.Skip MUST NOT call useAiBannerStore.dismiss() — banner appears post-onboarding because hasActiveProvider stays false; calling dismiss() here would break D-14/D-15 (enforced by grep gate and test)"
  - "Ollama in onboarding uses fixed default model (llama3) — no /api/tags dynamic fetch in onboarding; Settings page still does dynamic fetch"
  - "AppShell banner positioned at very top of JSX fragment, before CommandPalette/Sidebar/TopBar — single-line strip that pushes layout down"
  - "Pre-existing App.test.tsx failures when run from client/ dir (path resolution bug in App.test.tsx, exists before Plan 06) — all 175 tests pass from repo root"
metrics:
  duration: ~7min
  completed: "2026-07-01"
  tasks: 3 automated + 1 checkpoint (auto-approved)
  files: 5
---

# Phase 7 Plan 06: Onboarding Step + AiNoProviderBanner Summary

Onboarding wizard extended to 5 steps with Connect AI step at index 1 (D-12), plus session-only post-onboarding banner in AppShell (D-14/D-15).

## What Was Built

### ConnectAiStep (client/components/ai/ConnectAiStep.tsx)
- 2×2 grid of 4 mini provider cards: Anthropic, OpenAI, Gemini, Ollama
- Each card expands inline (no modal) to show a condensed connect form
- Anthropic: `claude setup-token` command snippet + token input (sk-ant-oat01-)
- OpenAI: API key input (sk-...) with fixed default model `gpt-5-mini`
- Gemini: API key input (AIza...) with fixed default model `gemini-2.5-flash`
- Ollama: Base URL input pre-populated with `http://localhost:11434`, fixed model `llama3`
- Continue button: full-width, disabled until any provider is authenticated
- Skip button: advances step ONLY — **never touches useAiBannerStore** (D-14/D-15 invariant)

### AiNoProviderBanner (client/components/layout/AiNoProviderBanner.tsx)
- `role="alert" aria-live="polite"` for accessibility
- AlertTriangle icon + copy "Connect an AI provider to enable Smart Spaces."
- "Go to Settings →" button navigates to `/settings?tab=ai`
- X dismiss button with 44px touch target (min-w-[44px] min-h-[44px]) per UI-SPEC
- Always renders when mounted — mounting decision is in AppShell (separation of concerns)

### OnboardingPage extension (client/pages/OnboardingPage.tsx)
- StepIndicator `total={5}` (was 4)
- Step remap: 0=Welcome, 1=ConnectAiStep (NEW), 2=Folders, 3=Scanning, 4=SpacesReady
- `startScanning()` advances to step 3 (was 2), auto-advances to step 4 (was 3)
- `useEffect` scanning progress listens at `step === 3` (was 2)
- Scanning skip button advances to step 4 (was 3)
- Folders "Back" button now goes to step 1 / Connect AI (was step 0)

### AppShell banner mount (client/components/layout/AppShell.tsx)
Pitfall 6 guard computation:
```
showBanner = onboardingCompleted           // persisted store
          && !hasActiveProvider            // from useProviders().data
          && !bannerDismissed             // session-only Zustand store
```
Banner mounted as `{showBanner && <AiNoProviderBanner />}` at top of JSX.

## TDD RED→GREEN Record

- **RED commit** `84cce27`: 3 tests written against 4-step OnboardingPage → all 3 fail as expected (no "Connect your AI" heading reachable from Welcome's Continue)
- **GREEN commit** `66322e4`: OnboardingPage extended to 5 steps → all 3 tests pass

## D-14/D-15 Invariant Verification

The Skip handler in ConnectAiStep:
```tsx
<button onClick={onSkip} ...>Skip for now</button>
```
`onSkip` is `() => setStep(2)` in OnboardingPage — **no banner store call**. Verified by:
1. Grep gate: `! rg 'useAiBannerStore.*dismiss' client/components/ai/ConnectAiStep.tsx` (only appears in comments)
2. Test 3 in OnboardingPage.test.tsx: `expect(useAiBannerStore.getState().isDismissed).toBe(false)` after Skip

## Checkpoint Task 4

⚡ Auto-approved checkpoint: Task 4 (human-verify first-run flow + banner) — AUTO_MODE was active.

## Deviations from Plan

None - plan executed exactly as written. All grep gates pass, typecheck clean, 175 tests pass from repo root (3 new tests added for AIPV-06).

## Known Stubs

None. ConnectAiStep uses real `useSaveSetupToken` and `useConnectProvider` mutations (same hooks as Plan 05 ProviderCard). `useProviders()` drives both the Continue button enable state and the AppShell banner condition.

## Threat Surface Scan

No new network endpoints, auth paths, or trust boundary surface beyond what was planned. `AiNoProviderBanner` navigates to `/settings?tab=ai` (same-origin, existing route from Plan 05). `ConnectAiStep` inline forms use the same mutation hooks as `ProviderCard` — no new IPC surface.

## Self-Check

### Files exist
- client/components/ai/ConnectAiStep.tsx — created
- client/components/layout/AiNoProviderBanner.tsx — created
- client/pages/OnboardingPage.test.tsx — created
- client/pages/OnboardingPage.tsx — modified
- client/components/layout/AppShell.tsx — modified

### Commits exist
- dd7ccb7 feat(07-06): build ConnectAiStep and AiNoProviderBanner components
- 84cce27 test(07-06): add failing OnboardingPage tests for 5-step wizard (RED)
- 66322e4 feat(07-06): extend OnboardingPage from 4 to 5 steps (GREEN)
- 4572827 feat(07-06): mount AiNoProviderBanner in AppShell with Pitfall 6 guard

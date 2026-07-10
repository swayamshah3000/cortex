---
phase: 07-ai-provider-foundation
plan: "10"
subsystem: frontend/ui
tags: [typescript, react, oauth, ui, provider-card, tdd, vitest, two-mode]
dependency_graph:
  requires:
    - phase: 07-09
      provides: start_openai_oauth Tauri IPC command; provider="openai-codex"; disconnect_provider async
    - phase: 07-05
      provides: ProviderCard.tsx base (4-mode card with existing API-key forms)
    - phase: 07-08
      provides: OAuthFlowConfig, loopback listener, PKCE module
  provides:
    - "07-UI-SPEC.md: §Two-Mode Provider Cards section with full copywriting contract and interaction contract"
    - "client/hooks/useTauri.ts: useStartOpenAiOAuth() mutation hook invoking start_openai_oauth IPC"
    - "client/components/ai/ProviderCard.tsx: OpenAI two-mode card (oauth | api-key), Gemini API-key-only (Option C)"
    - "client/components/ai/ProviderCard.test.tsx: 9 tests total (7 new two-mode tests)"
    - "client/lib/mock-data.ts: openai-codex mock entry for browser-mode dev"
  affects:
    - .planning/phases/07-ai-provider-foundation/07-UI-SPEC.md
    - client/hooks/useTauri.ts
    - client/components/ai/ProviderCard.tsx
    - client/components/ai/ProviderCard.test.tsx
    - client/lib/mock-data.ts

tech-stack:
  added: []
  patterns:
    - "Two-mode form component pattern: useState<'oauth' | 'api-key'>('oauth') local state, resets on unmount"
    - "Shared ApiKeyFormBody subcomponent: avoids duplicating API-key form between OpenAI and Gemini modes"
    - "Hoisted vi.mock at module scope: top-level mockMutateAsync avoids vitest hoisting ReferenceError"
    - "useStartOpenAiOAuth follows existing useSaveSetupToken/useConnectProvider mutation pattern"

key-files:
  created: []
  modified:
    - .planning/phases/07-ai-provider-foundation/07-UI-SPEC.md (Two-Mode Provider Cards section + revision log)
    - client/hooks/useTauri.ts (useStartOpenAiOAuth hook added at bottom of Phase 7 section)
    - client/components/ai/ProviderCard.tsx (OpenAIConnectForm two-mode, GeminiApiKeyConnectForm, Sparkles import)
    - client/components/ai/ProviderCard.test.tsx (7 new tests, vi.mock hoisting fix)
    - client/lib/mock-data.ts (openai-codex mock entry added)

decisions:
  - "Gemini stays API-key-only (Option C confirmed from 07-09-SUMMARY.md): no useStartGeminiOAuth hook, no Sign in with Google CTA. Gemini card extracted into GeminiApiKeyConnectForm for clarity but behavior is unchanged."
  - "ApiKeyFormBody shared subcomponent: both OpenAI api-key mode and Gemini reuse same form body to avoid duplication. headerSlot prop allows OpenAI to inject the back-to-oauth toggle above the form."
  - "vi.mock hoisted at module scope: declaring mockMutateAsync before the vi.mock() factory avoids ReferenceError caused by vitest hoisting module mocks above variable declarations inside test bodies."
  - "Gemini connected badge stays 'Connected' (not 'Connected via Google'): Option C is API-key-only, no oauth method possible for gemini, so the conditional badge only branches on provider=openai AND method=oauth."

metrics:
  duration: ~20min
  completed: 2026-07-02
  tasks_completed: 2
  tasks_total: 3
  files_created: 0
  files_modified: 5
  tests_added: 7
---

# Phase 7 Plan 10: Two-Mode OpenAI Provider Card + OAuth Hook Summary

**Two-mode OpenAI card with "Sign in with ChatGPT" primary CTA and "Use API key instead" toggle; useStartOpenAiOAuth React Query mutation; 9 ProviderCard tests total (7 new) — typecheck clean**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-07-02
- **Tasks:** 2 auto + 1 checkpoint (auto-approved)
- **Files created:** 0
- **Files modified:** 5
- **Tests added:** 7 (+2 Anthropic/Ollama regression guards, +4 OpenAI two-mode, +1 Gemini Option C)
- **Total vitest tests:** 9 passed in ProviderCard.test.tsx (was 2 before plan)

## Accomplishments

**Task 1 (07-UI-SPEC.md amendment):**
- Added `## Two-Mode Provider Cards (D-22..D-25)` section before `## Registry Safety`
- Full copywriting contract table for all two-mode elements
- Interaction contract additions (mode state, toast behavior, loopback timeout error)
- Gemini Option C explicitly documented: "No Sign in with Google CTA — option C selected, rationale linked to 07-09-SUMMARY.md"
- Component Inventory: ProviderCard `mode` internal state documented
- Component-to-File Map: ProviderCard status updated from NEW → MODIFIED
- Revision log: `oauth-pkce-amendment` entry (2026-07-02)

**Task 2 (Hook + Component + Tests — TDD):**

*RED phase:*
- Added 7 new tests (6 failing, 1 passing) before implementation
- Tests cover: primary CTA presence, "Use API key instead" toggle, mode switch, mutation call, Anthropic/Ollama regression, Gemini Option C

*GREEN phase:*
- `useStartOpenAiOAuth()` hook in useTauri.ts: invokes `start_openai_oauth` IPC, onSuccess invalidates providers + activeProvider
- `OpenAIConnectForm`: mode="oauth" shows Sparkles icon + "Sign in with ChatGPT" + body copy + toggle; mode="api-key" shows existing form + "Sign in with ChatGPT instead" back-link
- `GeminiApiKeyConnectForm`: wraps `ApiKeyFormBody` for Gemini (API-key-only, no two-mode)
- `ApiKeyFormBody`: shared subcomponent with `headerSlot` prop for the back-to-oauth toggle injection
- Connected badge: `provider === "openai" && status?.method === "oauth"` → "Connected via ChatGPT (Codex)"
- `mockProviders` extended with `openai-codex` entry (authenticated=true, method="oauth") for browser dev mode

**Task 3 (Checkpoint):**
- Auto-approved per AUTO_MODE active
- Human verification: Settings → AI tab, expand OpenAI card, click "Sign in with ChatGPT", complete OAuth flow, verify "Connected via ChatGPT (Codex)" badge

## Task Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | 07-UI-SPEC.md two-mode card spec + revision log | 3ce1e7d |
| 2 | Two-mode OpenAI card + useStartOpenAiOAuth + 9 tests | 6519e39 |

## Test Results

**Before Plan 07-10:** 2 ProviderCard tests (both passing via npx vitest)

**After Plan 07-10:** 9 ProviderCard tests — 9 pass, 0 fail

```
ProviderCard (AIPV-05 radio-disabled invariant)
  ✓ disables radio when status.authenticated is false
  ✓ enables radio when status.authenticated is true

ProviderCard — two-mode OpenAI card (D-25, AIPV-02)
  ✓ renders 'Sign in with ChatGPT' as primary CTA for OpenAI when not connected
  ✓ shows 'Use API key instead' toggle in OAuth mode for OpenAI when not connected
  ✓ switches to api-key mode when 'Use API key instead' is clicked
  ✓ invokes start_openai_oauth mutation on primary CTA click

ProviderCard — Anthropic and Ollama regression guard
  ✓ Anthropic card has no 'Sign in with' CTA
  ✓ Ollama card has no 'Sign in with' CTA

ProviderCard — Gemini card (Option C: API-key-only)
  ✓ Gemini card has no 'Sign in with Google' CTA (Option C confirmed)
```

**Typecheck:** `bun run typecheck` exits 0 (no TypeScript errors)

## Gemini OAuth (Option C — confirmed)

Per 07-09-SUMMARY.md §Key Decisions and 07-OAUTH-RESEARCH.md §Decision:
- Gemini generateContent has no user-subscription OAuth path equivalent to ChatGPT Plus
- No shared public Google client_id exists for subscription-based OAuth
- `useStartGeminiOAuth` hook: NOT added (Option C)
- Gemini card: API-key-only, no "Sign in with Google" CTA, no "Coming soon" ghost
- Pre-wired `#[cfg(any())]` arm exists in Rust `logout_provider` for future activation (from Plan 07-09)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] vi.mock hoisting ReferenceError**
- **Found during:** Task 2 (TDD RED→GREEN phase, first test run)
- **Issue:** Test 4 declared `const mockMutateAsync = vi.fn()` inside the test body, then used it inside a `vi.mock()` factory. Vitest hoists `vi.mock()` calls above all variable declarations, so `mockMutateAsync` was `undefined` when the factory ran — this broke Tests 1-3 which triggered the hoisted mock.
- **Fix:** Moved `mockMutateAsync` and `vi.mock("@/hooks/useTauri", ...)` to module scope (top of file, before any `describe` blocks). All 4 two-mode tests now use the same module-level mock; `beforeEach` clears call history.
- **Files modified:** `client/components/ai/ProviderCard.test.tsx`
- **Committed in:** 6519e39 (Task 2)

**2. [Rule 2 - Shared subcomponent] ApiKeyFormBody extracted to avoid duplication**
- **Found during:** Task 2 implementation (Step C)
- **Issue:** The plan specified `OpenAIConnectForm` wrapping the "existing Plan 05 form body" for api-key mode, and `GeminiApiKeyConnectForm` as a separate component. Replicating the form JSX twice would create drift risk.
- **Fix:** Extracted `ApiKeyFormBody` with a `headerSlot?: React.ReactNode` prop. Both OpenAI api-key mode and Gemini card use `<ApiKeyFormBody provider="..." onConnected={...} headerSlot={...} />`. No behavior change, just structural improvement.
- **Files modified:** `client/components/ai/ProviderCard.tsx`
- **Committed in:** 6519e39 (Task 2)

## Known Stubs

None. All UI paths are wired:
- `useStartOpenAiOAuth().mutateAsync()` invokes real `start_openai_oauth` IPC in Tauri runtime (falls back to mock in browser dev mode)
- Toast on success/error is wired in OpenAIConnectForm
- `GeminiApiKeyConnectForm` is wired to `useConnectProvider` via `ApiKeyFormBody`
- Connected badge conditional rendering is wired to `status.method === "oauth"`

## Threat Surface Scan

No new network endpoints or trust boundaries introduced in this plan. All surfaces were in scope:
- `start_openai_oauth` IPC call — within T-07-50 (Tauri CSP + capability system restricts IPC to Cortex origins)
- No user-supplied args to `start_openai_oauth` call site (client_id is hardcoded backend-side per Option A model per T-07-51)
- T-07-51 (Option B Gemini client_id validation) is not applicable — Option C confirmed, no Gemini OAuth CTA

## Self-Check

Files modified:
- [FOUND] `.planning/phases/07-ai-provider-foundation/07-UI-SPEC.md` — Two-Mode Provider Cards section + revision log
- [FOUND] `client/hooks/useTauri.ts` — useStartOpenAiOAuth present
- [FOUND] `client/components/ai/ProviderCard.tsx` — OpenAIConnectForm, GeminiApiKeyConnectForm, Sparkles import
- [FOUND] `client/components/ai/ProviderCard.test.tsx` — 9 tests total
- [FOUND] `client/lib/mock-data.ts` — openai-codex mock entry

Commits:
- [FOUND] 3ce1e7d — docs(07-10): add two-mode OpenAI card spec to 07-UI-SPEC.md
- [FOUND] 6519e39 — feat(07-10): two-mode OpenAI card + useStartOpenAiOAuth hook + tests

Grep gates:
- [PASS] `useStartOpenAiOAuth` in client/hooks/useTauri.ts
- [PASS] `start_openai_oauth` in client/hooks/useTauri.ts
- [PASS] `Sign in with ChatGPT` in client/components/ai/ProviderCard.tsx
- [PASS] `Use API key instead` in client/components/ai/ProviderCard.tsx
- [PASS] `Two-Mode Provider Cards` in 07-UI-SPEC.md
- [PASS] `Sign in with ChatGPT` in 07-UI-SPEC.md
- [PASS] `oauth-pkce-amendment` in 07-UI-SPEC.md
- [PASS] `bun run typecheck` → exit 0 (no errors)
- [PASS] `npx vitest --run client/components/ai/ProviderCard.test` → 9 PASS, 0 FAIL
- [PASS] No hardcoded hex colors in ProviderCard.tsx
- [PASS] Anthropic card: no "Sign in with" CTA (regression test 5 passes)
- [PASS] Ollama card: no "Sign in with" CTA (regression test 6 passes)
- [PASS] Gemini card: no "Sign in with Google" CTA (test 7 passes); API key form present

## Self-Check: PASSED

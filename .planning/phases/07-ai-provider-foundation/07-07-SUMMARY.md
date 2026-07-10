---
phase: 07-ai-provider-foundation
plan: "07"
subsystem: auth
tags: [oauth, pkce, openai, codex, gemini, google, research, spike]

# Dependency graph
requires:
  - phase: 07-06
    provides: Phase context, D-22..D-25 scope amendments, CONTEXT.md decisions

provides:
  - "Authoritative OAuth endpoint reference for plans 07-08, 07-09, 07-10"
  - "Re-verified Codex CLI OAuth constants (auth/token/revoke URLs, CLIENT_ID, scopes, PKCE)"
  - "Gemini OAuth Cortex-blocked decision (Option C) with audit trail"
  - "Per-provider redirect_uri host resolution (Codex=localhost, Gemini=127.0.0.1)"
  - "Codex chat routing decision: chatgpt.com/backend-api/codex/responses (not api.openai.com)"
  - "Refresh + revoke request shapes for plan 07-09 implementation"

affects: [07-08-PLAN, 07-09-PLAN, 07-10-PLAN]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "OAuth PKCE S256 for Codex CLI: localhost redirect + 127.0.0.1 socket bind"
    - "ChatGPT Responses API via chatgpt.com/backend-api/codex (distinct from api.openai.com)"
    - "Gemini stays API-key only for v1.1 (no user-subscription OAuth path available)"

key-files:
  created:
    - ".planning/phases/07-ai-provider-foundation/07-OAUTH-RESEARCH.md"
  modified: []

key-decisions:
  - "Gemini OAuth Cortex-blocked (Option C): generateContent has no user-subscription OAuth path; Google's OAuth quickstart targets Semantic Retriever feature requiring GCP project setup, not end-user sign-in"
  - "Codex redirect_uri uses literal 'localhost' string (NOT 127.0.0.1) per Hydra allow-list in codex-rs/login/src/server.rs"
  - "Codex chat routing requires new codex_chat() function targeting chatgpt.com/backend-api/codex/responses (Responses API), NOT the existing openai_chat() which uses api.openai.com/v1/chat/completions"
  - "originator parameter for Cortex: 'cortex-desktop' (not codex_cli)"
  - "CLIENT_ID app_EMoamEEZ73f0CkXaXp7hrann confirmed unchanged from planning snapshot"

requirements-completed: [AIPV-01, AIPV-02, AIPV-03, AIPV-07]

# Metrics
duration: 3min
completed: "2026-07-02"
---

# Phase 7 Plan 07: OAuth PKCE Research Spike Summary

**Codex OAuth constants re-verified from live source (auth.openai.com endpoints, CLIENT_ID, S256 PKCE, localhost redirect); Gemini OAuth declared Cortex-blocked; chat routing resolved to chatgpt.com/backend-api/codex/responses**

## Performance

- **Duration:** 3 min
- **Started:** 2026-07-02T05:15:05Z
- **Completed:** 2026-07-02T05:18:17Z
- **Tasks:** 4 (combined into single spike commit)
- **Files modified:** 1

## Accomplishments

- Re-fetched and re-verified all Codex OAuth constants from live `codex-rs/login/` source — no drift from planning snapshot; all values confirmed current
- Resolved the open Gemini OAuth question: declared Option C (Cortex-blocked) with full audit trail citing specific Google doc findings; plans 07-09 and 07-10 have explicit "do not implement" instructions
- Resolved the redirect_uri host conflict: Codex=`localhost` (Hydra allow-list), Gemini=`127.0.0.1` (Google Desktop app policy) — authoritative table committed
- Discovered and documented the Codex chat routing: NOT `api.openai.com/v1/chat/completions` but `chatgpt.com/backend-api/codex/responses` (Responses API, confirmed from `CHATGPT_CODEX_BASE_URL` constant in `codex-rs/model-provider-info/src/lib.rs`)

## Task Commits

All four tasks are pure documentation (research spike) combined in a single commit:

1. **Tasks 1-4: OAuth research spike** - `9a88691` (docs)

## Files Created/Modified

- `.planning/phases/07-ai-provider-foundation/07-OAUTH-RESEARCH.md` — 353-line reference document with all endpoint constants, request shapes, redirect URI resolution table, and chat routing decision

## Decisions Made

**D1: Gemini OAuth = Cortex-blocked (Option C)**
Gemini's `generateContent` endpoint has no user-subscription OAuth path comparable to Codex CLI's ChatGPT subscription access. Google's OAuth quickstart (ai.google.dev/gemini-api/docs/oauth) is scoped to the Semantic Retriever feature requiring per-project GCP registration. The google-genai Python SDK uses `cloud-platform` scope with mandatory GCP project ID — this is ADC/service-account pattern, not end-user sign-in. Learnforge's removal of Gemini OAuth is now understood. Gemini card stays API-key-only for v1.1.

**D2: Codex redirect_uri host = "localhost" (NOT "127.0.0.1")**
Confirmed from `server.rs` line 161 literal and line 55 comment `// Keep in sync with the Codex CLI Hydra redirect URI allow-list`. Socket binds to `127.0.0.1` but the OAuth string presented to auth.openai.com MUST be `localhost`. Using 127.0.0.1 would trigger `invalid_grant: redirect_uri_mismatch`.

**D3: Codex chat endpoint is NOT api.openai.com**
`model-provider-info/src/lib.rs` line 38: `CHATGPT_CODEX_BASE_URL = "https://chatgpt.com/backend-api/codex"`. When `AuthMode::Chatgpt`, the provider base URL switches to chatgpt.com, not api.openai.com. Wire format is Responses API (chat completions wire API was removed from Codex source). Plan 07-09 needs a new `codex_chat()` function.

**D4: originator = "cortex-desktop"**
Cortex MUST use `originator=cortex-desktop` in Codex OAuth authorization request (not `codex_cli`). This distinguishes Cortex sessions in OpenAI telemetry.

## Deviations from Plan

None - research spike executed exactly as specified. All four tasks produced the required content in `07-OAUTH-RESEARCH.md`.

## GEMINI OAUTH CORTEX-BLOCKED NOTICE

**IMPORTANT FOR PLANS 07-09 AND 07-10:**

Gemini OAuth is Cortex-blocked for this milestone (Option C decision). This means:

- Plan 07-09 MUST NOT implement `start_google_gemini_oauth`
- Plan 07-10 MUST NOT render a "Sign in with Google" CTA on the Gemini card
- The Gemini card remains API-key-only
- The shared PKCE module (plan 07-08) still ships but is exercised only by the `openai-codex` path

This is documented in `07-OAUTH-RESEARCH.md` under `## Google Gemini OAuth (PKCE + Refresh) / ### Decision`.

## Redirect URI Host Resolution Summary

| Provider | redirect_uri STRING | Socket bind |
|---|---|---|
| `openai-codex` | `http://localhost:{port}/auth/callback` | `127.0.0.1` |
| `gemini` (future) | `http://127.0.0.1:{port}` | `127.0.0.1` |

## Issues Encountered

None.

## Known Stubs

None — this is a research spike (documentation only). No production code was written.

## Threat Surface Scan

No new network endpoints or auth paths were introduced. This plan creates a documentation file only.

## Self-Check

Files created:

- `[FOUND]` `.planning/phases/07-ai-provider-foundation/07-OAUTH-RESEARCH.md` — 353 lines (min required: 140)

Commit:
- `[FOUND]` `9a88691` — `docs(07-07): OAuth PKCE research spike for OpenAI Codex and Google Gemini`

Verification gates all passed:
- AUTH_URL: PASS
- TOKEN_URL: PASS
- REVOKE_URL: PASS
- S256: PASS
- CLIENT_ID: PASS
- SCOPE: PASS
- GRANT_TYPE: PASS
- ORIGINATOR: PASS
- GOOGLE_AUTH_URL: PASS
- GOOGLE_TOKEN_URL: PASS
- IP_127: PASS
- DECISION: PASS
- GEMINI: PASS
- REDIRECT_SECTION: PASS
- CODEX_LOCALHOST: PASS
- GEMINI_127: PASS
- REDIRECT_URI_HOST: PASS
- CHAT_ROUTING_SECTION: PASS
- CHAT_URL: PASS

## Self-Check: PASSED

## Next Phase Readiness

Plans 07-08, 07-09, and 07-10 can now execute without further web research:
- Plan 07-08 has the OAuthFlowConfig interface requirements (per-provider redirect_uri_host field)
- Plan 07-09 has all Codex OAuth constants, refresh/revoke shapes, originator value, and chat routing decision
- Plan 07-10 knows Gemini card = API-key-only (no "Sign in with Google" CTA)
- One [ASSUMED] item remains for plan 07-09: non-streaming Responses API request shape against `chatgpt.com/backend-api/codex/responses` — must be verified at execution via manual curl test before committing `codex_chat()`

---
*Phase: 07-ai-provider-foundation*
*Completed: 2026-07-02*

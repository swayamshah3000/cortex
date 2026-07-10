---
phase: 07-ai-provider-foundation
plan: "09"
subsystem: auth
tags: [rust, oauth, pkce, codex, chatgpt, revoke, tauri-command, chat-routing]
dependency_graph:
  requires:
    - phase: 07-08
      provides: auth/pkce.rs start_oauth_flow(), OAuthFlowConfig, TokenRequestStyle, loopback listener
    - phase: 07-07
      provides: OAuth endpoint constants verified from codex-rs source
    - phase: 07-01
      provides: AuthState, ProviderCredential, auth/mod.rs baseline
    - phase: 07-02
      provides: ai_request(), ai/service.rs baseline, normalize_provider_name
  provides:
    - "auth/oauth.rs: start_openai_codex_oauth() — full Codex PKCE OAuth flow using shared pkce module"
    - "auth/oauth.rs: revoke_oauth_token() — best-effort token revocation helper (D-24 §6)"
    - "auth/oauth.rs: CODEX_* constants (AUTH_URL, TOKEN_URL, CLIENT_ID, SCOPE, REVOKE_URL, REDIRECT_URI_HOST='localhost')"
    - "commands/ai.rs: start_openai_oauth Tauri IPC command — browser-based Codex auth flow"
    - "ai/openai.rs: codex_chat() — ChatGPT backend Responses API handler (chatgpt.com/backend-api/codex/responses)"
    - "ai/service.rs: openai-codex pre-normalization intercept — routes to codex_chat not openai_chat"
    - "auth/commands.rs: logout_provider async + D-24 §6 revoke wiring for openai-codex"
    - "lib.rs: start_openai_oauth registered in invoke_handler (now 9 AI commands)"
  affects:
    - src-tauri/src/auth/oauth.rs
    - src-tauri/src/auth/commands.rs
    - src-tauri/src/commands/ai.rs
    - src-tauri/src/ai/service.rs
    - src-tauri/src/ai/openai.rs
    - src-tauri/src/ai/mod.rs
    - src-tauri/src/lib.rs
    - src-tauri/src/auth/mod.rs

tech-stack:
  added: []
  patterns:
    - "Pre-normalization intercept: openai-codex matched by raw cred.provider BEFORE normalize_provider_name — prevents wrong-endpoint routing (T-07-44)"
    - "Best-effort revoke: revoke_oauth_token always returns Ok() — network/4xx failures never block disconnect (D-24 §6)"
    - "redirect_uri_host per-provider: CODEX_REDIRECT_URI_HOST='localhost' (OAuth server string) vs 127.0.0.1 socket bind (independent)"
    - "CORTEX_TEST_CODEX_REVOKE_URL env-var indirection for mock revoke endpoint in tests"
    - "Responses API routing: codex_chat uses stream=false Responses API shape (not Chat Completions API)"

key-files:
  created: []
  modified:
    - src-tauri/src/auth/oauth.rs (Codex OAuth constants + start_openai_codex_oauth + revoke_oauth_token + 4 new tests)
    - src-tauri/src/auth/commands.rs (logout_provider async + D-24 §6 revoke wiring + 3 new tests)
    - src-tauri/src/auth/mod.rs (5 new Task 1 TDD tests for store_oauth_credential_with_refresh)
    - src-tauri/src/commands/ai.rs (start_openai_oauth IPC command + disconnect_provider async)
    - src-tauri/src/ai/openai.rs (codex_chat + build_codex_request + 3 new tests)
    - src-tauri/src/ai/service.rs (openai-codex pre-normalization intercept + 2 new tests)
    - src-tauri/src/ai/mod.rs (export codex_chat)
    - src-tauri/src/lib.rs (register start_openai_oauth in invoke_handler)

key-decisions:
  - "Codex chat routing: Outcome 1 — dedicated codex_chat() pointing to chatgpt.com/backend-api/codex/responses (not openai_chat which hits api.openai.com). Pre-normalization intercept ensures openai-codex never falls through to wrong endpoint"
  - "Revoke request style: JSON body for Codex per 07-OAUTH-RESEARCH.md revoke.rs (not form-urlencoded as plan Step B suggested — research doc is authoritative)"
  - "Gemini OAuth: Option C confirmed — no start_gemini_oauth command, no Google constants. gemini-oauth arm pre-wired in logout_provider under #[cfg(any())] for future activation"
  - "logout_provider signature: sync→async cascade required because revoke_oauth_token is async. disconnect_provider Tauri command also became async (invisible to frontend)"
  - "store_oauth_credential_with_refresh + openai-codex in AI_PROVIDERS were already present from Plan 07-08 — Task 1 adds the 5 specified verification tests against pre-existing implementation"

patterns-established:
  - "Pre-normalization routing: match on raw cred.provider BEFORE normalize_provider_name for provider-specific backends"
  - "Best-effort async pattern: revoke_oauth_token catches all errors and returns Ok(()) — callers use let _ = ..."
  - "Test env-var indirection: CORTEX_TEST_*_URL overrides production URLs in #[cfg(test)] helpers"

requirements-completed: [AIPV-01, AIPV-02, AIPV-03, AIPV-07]

duration: ~50min
completed: 2026-07-02
---

# Phase 7 Plan 09: Provider OAuth Commands + Codex Chat Routing + Revoke-on-Disconnect Summary

**OpenAI Codex PKCE OAuth IPC command, ChatGPT Responses API chat routing, and best-effort token revocation on disconnect wiring all complete with 17 new tests — 239 total passing**

## Performance

- **Duration:** ~50 min
- **Completed:** 2026-07-02
- **Tasks:** 4 executed (Task 1, 2, 3a, 3b) + Task 3c no-op (Gemini Option C)
- **Files created:** 0
- **Files modified:** 8
- **Tests added:** 17 (+5 Task1, +1 Task2, +5 Task3a, +6 Task3b)
- **Total tests:** 239 passed + 23 ignored (was 222+23=245 from Plan 07-08)

## Accomplishments

**Task 1 (TDD tests — auth/mod.rs):**
- Added 5 verification tests for `store_oauth_credential_with_refresh` and `AI_PROVIDERS` allow-list
- Tests: all-fields-set, auto-promote-active, non-AI-provider-stays-inactive, is_ai_provider("openai-codex")=true, persistence-across-instances
- Implementation already existed from Plan 07-08; tests add explicit coverage as specified in plan

**Task 2 (OAuth command + constants):**
- `auth/oauth.rs`: Codex OAuth constants verified from 07-OAUTH-RESEARCH.md (2026-07-02 re-fetch from codex-rs)
- `start_openai_codex_oauth()`: uses `start_oauth_flow()` from pkce.rs with `redirect_uri_host="localhost"` (D-24 §1)
- `revoke_oauth_token()`: generic best-effort POST supporting both FormUrlencoded and Json styles (D-24 §6)
- `codex_revoke_url()`: test env-var override via `CORTEX_TEST_CODEX_REVOKE_URL`
- `commands/ai.rs`: `start_openai_oauth` Tauri command wraps `start_openai_codex_oauth`, returns `ProviderAuthStatus`
- `auth/commands.rs`: `get_auth_status` scan list extended to include `"openai-codex"` (frontend visibility)
- `lib.rs`: `commands::ai::start_openai_oauth` registered — invoke_handler now has 9 AI commands
- `test_codex_constants_match_research_doc` asserts `CODEX_REDIRECT_URI_HOST == "localhost"` and revoke URL

**Task 3a (Codex chat routing — Outcome 1):**
- `ai/openai.rs`: `codex_chat()` + `build_codex_request()` — posts to `chatgpt.com/backend-api/codex/responses`
- Responses API wire format: `"input"` array, `"max_output_tokens"`, `"stream": false` (non-streaming)
- System prompt mapped to `"developer"` role (Responses API convention)
- `ai/service.rs`: pre-normalization intercept — `cred.provider == "openai-codex"` matched BEFORE `normalize_provider_name()` call; dispatched to `codex_chat()` via `dispatch_with_401_retry`
- `ai/mod.rs`: `codex_chat` exported alongside `openai_chat`
- 3 new codex build tests (endpoint URL, developer role, empty system), 2 new service routing tests
- `normalize_provider_name("openai-codex")` still returns `"openai"` (regression guard passes)

**Task 3b (Revoke-on-disconnect D-24 §6):**
- `auth/oauth.rs`: 3 revoke tests (form-urlencoded body, 4xx ignored, network error ignored)
- `auth/commands.rs`: `logout_provider` extended to async; calls `revoke_oauth_token` best-effort for `openai-codex` OAuth credentials with JSON body (per research doc revoke.rs) BEFORE `remove_credential`
- Pre-wired `#[cfg(any())]` gemini-oauth arm for future activation (currently compile-disabled, Option C)
- `commands/ai.rs`: `disconnect_provider` changed to `pub async fn` (cascades from async `logout_provider`)
- 3 new logout tests (revoke fired + credential removed, revoke fails → still removed, API-key skips revoke)

**Task 3c (Gemini Option C — no-op):**
- 07-OAUTH-RESEARCH.md §Decision confirmed Option C: "Gemini OAuth is Cortex-blocked for this milestone"
- No `start_gemini_oauth` command added, no Google OAuth constants
- Gemini card remains API-key-only per Plan 07-10

## Task Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | TDD tests for store_oauth_credential_with_refresh + AI_PROVIDERS | 6443597 |
| 2 | start_openai_codex_oauth + revoke_oauth_token + start_openai_oauth IPC + lib.rs | 1cf5266 |
| 3a | codex_chat Responses API + pre-normalization intercept in ai_request | 4742a44 |
| 3b | D-24 §6 revoke-on-disconnect + logout_provider async + 6 tests | 782b30b |

## Test Results

**Before Plan 07-09:** 222 passed; 0 failed; 23 ignored (from Plan 07-08)

**After Plan 07-09:** 239 passed; 0 failed; 23 ignored

```
auth::tests (mod.rs)                 — 36 pass (31 existing + 5 new Task 1)
auth::oauth::tests                   — 13 pass (9 existing + 1 constants + 3 revoke)
auth::commands::tests                — 6 pass (3 existing + 3 logout revoke)
ai::service::tests                   — 9 pass (7 existing + 2 new Task 3a)
ai::service::refresh_tests           — 6 pass (all existing)
ai::openai::tests                    — 9 pass (6 existing + 3 new Task 3a)
(+ all other pipeline, search, spaces tests unchanged — 239 total)
```

## Key Implementation Details

**Codex Chat Routing (Outcome 1):**
- Endpoint: `https://chatgpt.com/backend-api/codex/responses`
- Wire: OpenAI Responses API with `stream=false`
- `[ASSUMED]` per 07-OAUTH-RESEARCH.md: Responses API non-streaming shape used as documented
- Request shape: `{"model": ..., "input": [...], "max_output_tokens": N, "stream": false}`
- Response parsing: `output[].content[].text` (Responses API path, not `choices[].message.content`)
- Auth: `Authorization: Bearer <access_token>` (standard Bearer, no additional headers)

**Redirect URI Host (D-24 §1):**
- `CODEX_REDIRECT_URI_HOST = "localhost"` — matches OpenAI Hydra allow-list and codex-rs server.rs L161
- Socket bind remains `127.0.0.1` (in loopback.rs, unchanged from Plan 07-08)
- The two are independent: socket bind vs. OAuth redirect_uri string

**Revoke Strategy (D-24 §6):**
- OpenAI Codex revoke: JSON body `{"token": ..., "token_type_hint": "refresh_token", "client_id": ...}`
- Prefers `refresh_token` over `access_token` for revocation (per research doc)
- Network errors, 4xx, 5xx all return `Ok(())` — never blocks `remove_credential`

**Gemini OAuth (Option C confirmed):**
- Research doc: "Gemini generateContent has no user-subscription OAuth path equivalent to ChatGPT Plus"
- No shared public client_id exists (unlike Codex CLI's `app_EMoamEEZ73f0CkXaXp7hrann`)
- `#[cfg(any())]` arm pre-wired in `logout_provider` for future Gemini OAuth activation without re-writing

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Authoritative source] Revoke style changed from FormUrlencoded to Json for openai-codex**
- **Found during:** Task 3b (logout_provider implementation)
- **Issue:** Plan Step B specified `TokenRequestStyle::FormUrlencoded` for openai-codex revoke. The 07-OAUTH-RESEARCH.md §Revoke Request Shape explicitly documents JSON body (`revoke.rs lines 47-53`). The research doc is declared authoritative in the plan itself.
- **Fix:** Used `TokenRequestStyle::Json` for openai-codex in `logout_provider`. `revoke_oauth_token` supports both styles; only the call site choice changed.
- **Files modified:** `src-tauri/src/auth/commands.rs`
- **Committed in:** 782b30b (Task 3b)

**2. [Rule 2 - Pre-existing] Task 1 implementation already present from Plan 07-08**
- **Found during:** Task 1 (initial read)
- **Issue:** Plan 07-08 already implemented `store_oauth_credential_with_refresh` and added `openai-codex` to `AI_PROVIDERS`. Plan 07-09 Task 1 specification calls for writing tests first (TDD RED/GREEN). Since implementation pre-exists, no RED phase was possible.
- **Fix:** Added the 5 tests specified in Task 1 as verification tests against the pre-existing implementation. All passed immediately (GREEN). This is consistent with TDD's goal — verified correctness of existing code.
- **Files modified:** `src-tauri/src/auth/mod.rs`
- **Committed in:** 6443597 (Task 1)

---

**Total deviations:** 2 (1 style change per authoritative source, 1 TDD adjustment for pre-existing code)
**Impact on plan:** No scope creep. Both adjustments improve correctness or reflect accurate research findings.

## Known Stubs

`codex_chat()` response parsing is `[ASSUMED]` per 07-OAUTH-RESEARCH.md §Codex Chat Routing:
- The Responses API non-streaming shape is used based on documented API format
- File: `src-tauri/src/ai/openai.rs`, function `build_codex_request` and `codex_chat`
- **NOT blocking:** The routing is correct (chatgpt.com endpoint, Bearer token). Shape may require adjustment after live testing with a real Codex OAuth token.
- **Resolution:** Plan 07-10 (UI) integration test or manual curl test with real token before v1.1 release.

## Threat Surface Scan

New network endpoints within planned scope:
- `https://chatgpt.com/backend-api/codex/responses` — T-07-44 mitigated (Bearer token, correct endpoint)
- `https://auth.openai.com/oauth/revoke` — T-07-46 + T-07-47 mitigated (best-effort, never blocks)

No unexpected trust boundary crossings. Both endpoints covered by threat register in plan frontmatter.

## Next Phase Readiness

- Plan 07-10 (UI) can invoke `start_openai_oauth` via `tauriInvoke("start_openai_oauth")`
- `list_providers` now returns `openai-codex` status for frontend card rendering
- `disconnect_provider` async change is transparent to Tauri frontend (React Query handles async)
- Gemini card in Plan 07-10: API-key only (Option C confirmed, no "Sign in with Google" CTA)

## Self-Check

Files modified:
- [FOUND] `src-tauri/src/auth/oauth.rs` — CODEX_AUTH_URL, start_openai_codex_oauth, revoke_oauth_token
- [FOUND] `src-tauri/src/auth/commands.rs` — logout_provider async, D-24 §6 revoke wiring
- [FOUND] `src-tauri/src/auth/mod.rs` — 5 new Task 1 tests
- [FOUND] `src-tauri/src/commands/ai.rs` — start_openai_oauth IPC, disconnect_provider async
- [FOUND] `src-tauri/src/ai/openai.rs` — codex_chat, build_codex_request
- [FOUND] `src-tauri/src/ai/service.rs` — pre-normalization openai-codex intercept
- [FOUND] `src-tauri/src/ai/mod.rs` — codex_chat re-exported
- [FOUND] `src-tauri/src/lib.rs` — start_openai_oauth in invoke_handler

Commits:
- [FOUND] 6443597 — `test(07-09): add Task 1 TDD tests for store_oauth_credential_with_refresh + AI_PROVIDERS`
- [FOUND] 1cf5266 — `feat(07-09): implement start_openai_codex_oauth, start_openai_oauth IPC, revoke_oauth_token`
- [FOUND] 4742a44 — `feat(07-09): route openai-codex chat to ChatGPT backend (Outcome 1 — codex_chat)`
- [FOUND] 782b30b — `feat(07-09): wire D-24 §6 revoke-on-disconnect + extend logout_provider async`

Grep gates:
- [PASS] `"openai-codex"` in AI_PROVIDERS (auth/mod.rs)
- [PASS] `store_oauth_credential_with_refresh` in auth/mod.rs
- [PASS] `CODEX_AUTH_URL` contains `auth.openai.com/oauth/authorize`
- [PASS] `CODEX_REDIRECT_URI_HOST = "localhost"` (D-24 §1)
- [PASS] `CODEX_REVOKE_URL` contains `auth.openai.com/oauth/revoke`
- [PASS] `start_openai_oauth` in commands/ai.rs
- [PASS] `commands::ai::start_openai_oauth` in lib.rs invoke_handler
- [PASS] `codex_chat` in ai/openai.rs
- [PASS] `chatgpt.com/backend-api/codex/responses` in ai/openai.rs
- [PASS] `openai-codex` intercept in ai/service.rs (BEFORE normalize_provider_name)
- [PASS] `revoke_oauth_token` in auth/commands.rs
- [PASS] `pub async fn disconnect_provider` in commands/ai.rs
- [PASS] `cargo test` — 239 passed; 0 failed; 23 ignored

## Self-Check: PASSED

---
phase: 07-ai-provider-foundation
verified: 2026-07-02T10:00:00Z
status: human_needed
score: 11/11 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 6/6
  gaps_closed:
    - "OpenAI ChatGPT/Codex subscription OAuth PKCE flow with refresh tokens (D-22) — start_openai_codex_oauth, auth/pkce.rs, auth/loopback.rs all implemented"
    - "Gemini OAuth Option C declared and enforced — no start_google_gemini_oauth; Gemini card remains API-key-only; 07-OAUTH-RESEARCH.md documents audit trail"
    - "D-24 §6 revoke-on-disconnect wired — logout_provider calls revoke_oauth_token best-effort before remove_credential"
    - "codex_chat() routing to chatgpt.com/backend-api/codex/responses (Responses API, Outcome 1) wired via pre-normalization intercept in ai_request"
    - "useStartOpenAiOAuth hook + two-mode OpenAI card UI (Sign in with ChatGPT / Use API key instead) implemented"
    - "ProviderCredential extended with refresh_token + expires_at (#[serde(default)] backward-compat)"
    - "Preflight refresh (60s threshold) + dispatch_with_401_retry wired in ai_request"
    - "22 new Rust tests (pkce, loopback, refresh/retry) + 7 new frontend tests (two-mode card) — 239 Rust tests passing"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Open Settings → AI tab and confirm all four provider cards appear (Anthropic, OpenAI, Gemini, Ollama)"
    expected: "Four stacked provider cards visible inside a RadioGroup. OpenAI card shows two-mode: primary 'Sign in with ChatGPT' CTA and 'Use API key instead' toggle. Gemini card shows API-key-only form (no 'Sign in with Google' CTA — Option C confirmed). Anthropic and Ollama cards unchanged from pre-Wave-6 state."
    why_human: "Visual layout and two-mode UI interaction requires live Tauri app; cannot be verified by grep"
  - test: "Click 'Sign in with ChatGPT' on the OpenAI card and complete the Codex OAuth PKCE flow"
    expected: "System browser opens to https://auth.openai.com/oauth/authorize with PKCE S256 challenge, originator=cortex-desktop, and codex_cli_simplified_flow=true params. After signing in, browser redirects to http://localhost:{port}/auth/callback. Cortex captures the code, exchanges it for access+refresh tokens via https://auth.openai.com/oauth/token, stores under provider slug 'openai-codex' in credentials.json with expires_at. Card shows 'Connected via ChatGPT (Codex)' badge."
    why_human: "Browser-based OAuth flow requires a live ChatGPT Plus/Team account and real Codex CLI OAuth endpoint; cannot be mocked at grep level. Also validates that the [ASSUMED] Responses API wire format works against a real chatgpt.com/backend-api/codex/responses endpoint with an actual token."
  - test: "After OpenAI Codex OAuth, verify token refresh fires when within 60s of expiry"
    expected: "With an openai-codex credential whose expires_at is within 60s of now, trigger a chat() IPC call. The preflight refresh fires (POST to https://auth.openai.com/oauth/token with grant_type=refresh_token + JSON body), rotating the stored access_token. Verify credentials.json shows updated oauth_token."
    why_human: "Requires an actual Codex OAuth token with a near-expiry timestamp; cannot fake expires_at in production credentials.json to trigger the refresh branch without a live running session"
  - test: "Disconnect OpenAI Codex provider and verify token revocation fires"
    expected: "Clicking Disconnect on the OpenAI card fires a POST to https://auth.openai.com/oauth/revoke with JSON body {token: <refresh_token>, token_type_hint: 'refresh_token', client_id: 'app_EMoamEEZ73f0CkXaXp7hrann'}. Revoke result (success or failure) is ignored. Credential is removed from credentials.json. Card returns to Not Connected state."
    why_human: "Requires a live Codex OAuth session to test revoke endpoint; best-effort behavior (errors ignored) means network must be live to observe the POST"
  - test: "Paste a valid Anthropic setup-token (sk-ant-oat01-…) in the Anthropic card and click Connect"
    expected: "Button cycles idle → Validating (spinner) → Connected (green flash, 1200ms) → management state. Provider card shows 'Connected' badge. Refreshing Settings shows 'Connected' (credential persisted to credentials.json)."
    why_human: "Live HTTP validation call to Anthropic API; D-10 state machine timing; persistence requires actual Tauri app restart"
  - test: "Set Ollama as active provider with base URL http://localhost:11434 and model llama3, click Connect"
    expected: "Cortex routes a test ping through Ollama and reports success (if Ollama running) or a human-readable error toast (if not running). Provider card shows 'Connected' after success."
    why_human: "Requires Ollama running locally; validates live HTTP probe to user-specified base_url"
  - test: "After connecting at least one provider, switch the active provider from the RadioGroup"
    expected: "Clicking a connected provider's radio selects it immediately; subsequent LLM calls route to the new provider. RadioGroupItem for disconnected providers remains disabled."
    why_human: "RadioGroup onValueChange wiring to useSetActiveProvider and re-routing requires a live session to observe"
  - test: "Restart the app after saving credentials and re-open Settings → AI tab"
    expected: "Previously connected provider still shows 'Connected' — credentials.json was persisted. Click Disconnect — provider returns to 'Not Connected' and no entry remains in credentials.json."
    why_human: "App restart required to verify persistence; file system check of credentials.json verifies clean removal"
  - test: "Complete onboarding as a new user (wipe app_data_dir first) and verify the Connect AI step appears as Step 2"
    expected: "Onboarding wizard shows 5 steps (StepIndicator total=5). Step 2 shows 2x2 provider grid with inline connect forms. Skip advances to step 3 (Select Folders). After onboarding completes without a provider, AiNoProviderBanner appears in AppShell."
    why_human: "First-run onboarding flow requires wiped app_data_dir, live Tauri session, and visual verification of step indicator and banner placement"
---

# Phase 7: AI Provider Foundation Verification Report

**Phase Goal (original):** Users can authenticate with at least one AI provider (Anthropic, OpenAI, Gemini, or Ollama) and Cortex routes all LLM calls through a single pluggable backend.

**Phase Goal (amended D-22..D-25):** OpenAI via ChatGPT/Codex subscription OAuth PKCE with refresh; Gemini stays API-key-only (Option C per Wave 6 research — Google OAuth requires per-project GCP registration, not feasible for this milestone).

**Verified:** 2026-07-02T10:00:00Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure Waves 6-9 (Plans 07-07 through 07-10)

## Re-verification Summary

Previous status was `human_needed` with 6/6 automated truths verified. Waves 6-9 (Plans 07-07 through 07-10) addressed the UAT Test 1 gap that blocked all 5 UAT tests: user reported "for OpenAI, we want to connect using existing subscription/codex not API key" and requested parity with LearnForge. The re-verification adds 5 new truths from the amended goal (Waves 8-9 deliverables) and re-confirms all 6 original truths still hold.

**Gaps closed:** All 2 UAT gaps plus all 5 Wave 8-9 implementation items. 239 Rust tests passing, 9 frontend tests passing.

**Gaps remaining:** None automated. Status remains `human_needed` because the live Tauri app is required for the OAuth browser flow, Responses API shape validation, and visual rendering verification.

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User opens Settings → AI tab and sees all four providers with authentication status | VERIFIED | AiProvidersSection renders 4 ProviderCards via `PROVIDERS = ["anthropic", "openai", "gemini", "ollama"]`; SettingsPage imports and mounts AiProvidersSection |
| 2 | User can connect Anthropic via OAuth setup-token; card shows "Connected" after save | VERIFIED | save_setup_token IPC validates `sk-ant-oat01-` prefix + HTTP call to Anthropic; ProviderCard calls useSaveSetupToken; stores under `"anthropic"` key |
| 3 | User can set Ollama as active provider with base URL + model; test ping routes through Ollama | VERIFIED | validate_ollama_endpoint in oauth.rs; ollama_chat in ai/service.rs; OllamaConnectForm in ProviderCard uses method `"ollama"`; test_connection IPC command makes 1-token request |
| 4 | User can switch active provider from dropdown; LLM calls use newly selected provider | VERIFIED | RadioGroup onValueChange → useSetActiveProvider → set_active_provider IPC; ai_request reads get_active_credential() to pick provider branch |
| 5 | Credentials survive app restart; removed cleanly by Disconnect | VERIFIED | AuthState::new reads credentials.json from app_data_dir; remove_credential writes updated store to disk; falls back active to remaining provider; D-24 §6: logout_provider calls revoke_oauth_token best-effort BEFORE remove_credential |
| 6 | First-run onboarding includes Connect AI step (step 2 of 5); user can skip | VERIFIED | OnboardingPage total={5}; step===1 mounts ConnectAiStep; onSkip=()=>setStep(2) — never calls useAiBannerStore.dismiss(); AiNoProviderBanner in AppShell with Pitfall 6 guard |
| 7 | User can initiate OpenAI ChatGPT/Codex subscription OAuth PKCE flow from the OpenAI card | VERIFIED | start_openai_codex_oauth in auth/oauth.rs wires OAuthFlowConfig with redirect_uri_host="localhost" (D-24 §1); start_openai_oauth IPC in commands/ai.rs registered in lib.rs invoke_handler (9 AI commands); useStartOpenAiOAuth hook in useTauri.ts wired to ProviderCard two-mode UI |
| 8 | OpenAI Codex OAuth uses correct redirect URI host "localhost" per Codex CLI Hydra allow-list | VERIFIED | `CODEX_REDIRECT_URI_HOST = "localhost"` at oauth.rs line 22; OAuthFlowConfig populated with this value; pkce.rs Test 12 regression guard asserts redirect_uri starts with `http://localhost:`; loopback binds 127.0.0.1 independently (T-07-31) |
| 9 | Chat calls with openai-codex credential route to chatgpt.com/backend-api/codex/responses (Responses API), NOT api.openai.com | VERIFIED | codex_chat() in ai/openai.rs posts to `CHATGPT_CODEX_BASE_URL + "/responses"`; pre-normalization intercept in ai/service.rs matches `cred.provider == "openai-codex"` BEFORE normalize_provider_name(); ai/mod.rs re-exports codex_chat; 2 service routing tests gate this |
| 10 | Credentials with refresh tokens auto-refresh within 60s of expiry; stale 401 triggers retry | VERIFIED | is_expiring_soon() + preflight_refresh_if_needed() in ai/service.rs; refresh_openai_codex_token() calls pkce::refresh_access_token(Json style); update_oauth_tokens() rotates stored tokens; dispatch_with_401_retry() retries exactly once (T-07-34); Test 6 end-to-end wiring test via mock token endpoint |
| 11 | Gemini provider is API-key-only for this milestone; no "Sign in with Google" CTA rendered | VERIFIED | GeminiApiKeyConnectForm wraps ApiKeyFormBody (no OAuth CTA); ProviderCard test 9 "Gemini card has no 'Sign in with Google' CTA (Option C confirmed)"; 07-OAUTH-RESEARCH.md §Decision documents why (no user-subscription OAuth path for generateContent); no start_google_gemini_oauth command exists |

**Score:** 11/11 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/auth/mod.rs` | AuthState, CredentialStore, refresh_token + expires_at fields, store_oauth_credential_with_refresh, update_oauth_tokens, pub mod pkce/loopback, openai-codex in AI_PROVIDERS, tests | VERIFIED | `refresh_token: Option<String>` + `expires_at: Option<i64>` with `#[serde(default))`; store_oauth_credential_with_refresh; update_oauth_tokens; pub mod pkce at line 4; pub mod loopback at line 2; AI_PROVIDERS includes "openai-codex" at line 15; 36 auth tests passing |
| `src-tauri/src/auth/pkce.rs` | OAuthFlowConfig (with redirect_uri_host field), generate_pkce_codes, start_oauth_flow, refresh_access_token, TokenRequestStyle, 7+ unit tests | VERIFIED | 615 lines; OAuthFlowConfig with mandatory redirect_uri_host field; PkceCodes; TokenRequestStyle::{FormUrlencoded, Json}; start_oauth_flow; refresh_access_token; 7 pkce tests all pass |
| `src-tauri/src/auth/loopback.rs` | spawn_loopback_listener, 127.0.0.1-only bind, subtle::ConstantTimeEq state compare, 5-min timeout, 5+ unit tests | VERIFIED | 334 lines; Ipv4Addr::LOCALHOST bind confirmed; ct_eq() at line 125 (constant-time state compare T-07-30); tokio::time::timeout 300s (T-07-33); no 0.0.0.0 in file; 5 loopback tests pass |
| `src-tauri/src/auth/oauth.rs` | CODEX_* constants, start_openai_codex_oauth, revoke_oauth_token, no start_google_gemini_oauth | VERIFIED | 542+ lines; CODEX_AUTH_URL/TOKEN_URL/CLIENT_ID/SCOPE/REDIRECT_URI_HOST/REVOKE_URL all present (lines 12-25); start_openai_codex_oauth at line 44; revoke_oauth_token at line 103; no Gemini OAuth function (Option C) |
| `src-tauri/src/auth/commands.rs` | logout_provider async, D-24 §6 revoke wiring for openai-codex, openai-codex in provider scan list | VERIFIED | openai-codex in providers array at line 35; logout_provider async with revoke_oauth_token call at line 140; 6 tests in commands including 3 new logout revoke tests |
| `src-tauri/src/commands/ai.rs` | start_openai_oauth #[tauri::command], disconnect_provider async, 9 IPC commands total | VERIFIED | `pub async fn start_openai_oauth` at line 18-24; disconnect_provider is `pub async fn`; 9 commands::ai entries in lib.rs invoke_handler (line 201-210) |
| `src-tauri/src/ai/openai.rs` | codex_chat, build_codex_request, CHATGPT_CODEX_BASE_URL, chatgpt.com/backend-api/codex/responses endpoint, 3+ new tests | VERIFIED | CHATGPT_CODEX_BASE_URL constant; build_codex_request posts to `{base_url}/responses`; Responses API wire format (input[], max_output_tokens, stream:false, developer role); 3 new codex tests (endpoint URL, developer role, empty system) |
| `src-tauri/src/ai/service.rs` | openai-codex pre-normalization intercept BEFORE normalize_provider_name, preflight_refresh_if_needed, dispatch_with_401_retry, is_expiring_soon, refresh_openai_codex_token | VERIFIED | pre-normalization intercept at line 187 (`cred.provider == "openai-codex"` matched before normalize); preflight_refresh_if_needed at line 174; dispatch_with_401_retry at line 51; is_expiring_soon at line 117; 9 service tests + 6 refresh_tests |
| `src-tauri/src/ai/mod.rs` | codex_chat exported alongside openai_chat | VERIFIED | `pub use openai::{openai_chat, codex_chat};` at line 8 |
| `src-tauri/src/lib.rs` | start_openai_oauth registered in invoke_handler (9 AI commands total) | VERIFIED | `commands::ai::start_openai_oauth` at line 210; total 9 AI commands (lines 201-210) |
| `src-tauri/Cargo.toml` | base64 = "0.22", rand = "0.9", subtle = "2" added; sha2 = "0.10" verified present | VERIFIED | base64 at line 40, rand at line 41, subtle at line 42; sha2 = "0.10" confirmed pre-existing |
| `client/hooks/useTauri.ts` | useStartOpenAiOAuth mutation hook, wired to start_openai_oauth IPC | VERIFIED | useStartOpenAiOAuth at line 666; invokes tauriInvoke("start_openai_oauth"); onSuccess invalidates providers + activeProvider; mock fallback returns openai-codex entry |
| `client/components/ai/ProviderCard.tsx` | OpenAIConnectForm two-mode (oauth/api-key), GeminiApiKeyConnectForm (API-key-only), ApiKeyFormBody shared subcomponent | VERIFIED | 683 lines; OpenAIConnectForm with useState<'oauth'\|'api-key'>('oauth'); "Sign in with ChatGPT" CTA at line 315; "Use API key instead" at line 331; GeminiApiKeyConnectForm at line 677 (no OAuth CTA); ApiKeyFormBody with headerSlot prop |
| `client/components/ai/ProviderCard.test.tsx` | 9 tests total (7 new two-mode tests + 2 pre-existing radio tests) | VERIFIED | 206 lines; 9 `it()` blocks: 2 radio-disable invariant + 4 two-mode OpenAI + 2 regression guards + 1 Gemini Option C |
| `client/lib/mock-data.ts` | openai-codex mock entry (authenticated=true, method="oauth") | VERIFIED | openai-codex entry at line 58 with authenticated=true, method="oauth" |
| `.planning/phases/07-ai-provider-foundation/07-UI-SPEC.md` | Two-Mode Provider Cards section (D-22..D-25) + oauth-pkce-amendment revision log | VERIFIED | "## Two-Mode Provider Cards (D-22..D-25)" at line 362; "Sign in with ChatGPT" copywriting contract; Gemini Option C documented; "oauth-pkce-amendment" in revision log at line 515 |
| `.planning/phases/07-ai-provider-foundation/07-OAUTH-RESEARCH.md` | Codex OAuth constants, Gemini Option C decision with audit trail, redirect URI host resolution table | VERIFIED | 353 lines; CODEX_AUTH_URL/TOKEN_URL/CLIENT_ID/SCOPE/PKCE_METHOD/REDIRECT_URI_HOST all present; Gemini Option C §Decision with 5-point rationale; per-provider redirect URI table |
| `client/lib/stores.ts` | useAiBannerStore without persist middleware | VERIFIED | `create<AiBannerState>((set) =>` — no persist wrapper; confirmed by stores.test.ts api.persist === undefined test |
| `client/components/ai/AiProvidersSection.tsx` | RadioGroup wrapper with 4 ProviderCards | VERIFIED | 51 lines; id="ai-providers" anchor; useSetActiveProvider wired to RadioGroup onValueChange |
| `client/components/ai/ConnectAiStep.tsx` | 2x2 provider grid, inline forms, Skip MUST NOT dismiss banner | VERIFIED | 417 lines; useSaveSetupToken and useConnectProvider used; no useAiBannerStore.dismiss() call (only in comment) |
| `client/components/layout/AiNoProviderBanner.tsx` | Session-only banner with Go to Settings link | VERIFIED | 62 lines; navigates to "/settings?tab=ai"; role="alert" aria-live="polite" |
| `client/pages/OnboardingPage.tsx` | 5-step wizard with ConnectAiStep at step 1 | VERIFIED | total={5} at line 177; step===1 mounts ConnectAiStep at lines 210-214 |
| `client/components/layout/AppShell.tsx` | AiNoProviderBanner mounted with Pitfall 6 guard | VERIFIED | showBanner = onboardingCompleted && !hasActiveProvider && !bannerDismissed at line 37 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| auth/oauth.rs start_openai_codex_oauth | auth/pkce.rs start_oauth_flow | OAuthFlowConfig with redirect_uri_host="localhost" | VERIFIED | `crate::auth::pkce::start_oauth_flow(config, ...)` in oauth.rs; config.redirect_uri_host = CODEX_REDIRECT_URI_HOST = "localhost" |
| auth/oauth.rs start_openai_codex_oauth | tauri_plugin_opener open_url | on_authorization_url_ready closure | VERIFIED | `app_handle.opener().open_url(&auth_url, None::<&str>)` at oauth.rs line 73 |
| ai/service.rs ai_request | codex_chat() (not openai_chat) | Pre-normalization intercept on cred.provider == "openai-codex" | VERIFIED | `if cred.provider == "openai-codex"` at service.rs line 187 BEFORE normalize_provider_name() call at line 378 |
| ai/service.rs preflight_refresh_if_needed | auth/mod.rs update_oauth_tokens | refresh_openai_codex_token → pkce::refresh_access_token(Json) | VERIFIED | refresh_openai_codex_token calls pkce::refresh_access_token then auth.update_oauth_tokens(); Test 6 end-to-end wiring confirmed |
| auth/commands.rs logout_provider | auth/oauth.rs revoke_oauth_token | D-24 §6: best-effort POST before remove_credential | VERIFIED | `let _ = crate::auth::oauth::revoke_oauth_token(...)` at commands.rs line 140; 3 revoke tests confirm behavior and that failure doesn't block disconnect |
| commands/ai.rs start_openai_oauth | auth/oauth.rs start_openai_codex_oauth | IPC → internal fn | VERIFIED | `crate::auth::oauth::start_openai_codex_oauth(&app, &auth).await?` at commands/ai.rs line 24 |
| lib.rs invoke_handler | commands::ai::start_openai_oauth | tauri::generate_handler! | VERIFIED | `commands::ai::start_openai_oauth` at lib.rs line 210 (9th AI command) |
| useTauri.ts useStartOpenAiOAuth | tauriInvoke("start_openai_oauth") | React Query mutation | VERIFIED | `tauriInvoke("start_openai_oauth", {})` in useStartOpenAiOAuth at useTauri.ts line 671 |
| ProviderCard.tsx OpenAIConnectForm (oauth mode) | useStartOpenAiOAuth mutateAsync | Sign in with ChatGPT primary CTA click | VERIFIED | `const oauth = useStartOpenAiOAuth()` at ProviderCard.tsx line 283; "Sign in with ChatGPT" button onClick calls `oauth.mutateAsync()` |
| ProviderCard.tsx OpenAIConnectForm (api-key mode) | useConnectProvider via ApiKeyFormBody | Use API key instead secondary path | VERIFIED | ApiKeyFormBody shared subcomponent used in both OpenAI api-key mode and GeminiApiKeyConnectForm |
| auth/oauth.rs save_setup_token | auth/mod.rs store_oauth_token("anthropic", ...) | stores under "anthropic" key | VERIFIED | (pre-existing, confirmed regression-clean) |
| auth/commands.rs get_auth_status | providers scan list | `["anthropic", "openai", "openai-codex", "gemini", "ollama"]` | VERIFIED | 5-provider scan list at commands.rs line 35 (openai-codex added by Plan 09) |
| AppShell showBanner | onboardingCompleted && !hasActiveProvider && !bannerDismissed | Pitfall 6 guard | VERIFIED | Exact three-condition guard at AppShell.tsx line 37 |
| ConnectAiStep Skip handler | setStep(2) only — never useAiBannerStore.dismiss() | D-14/D-15 invariant | VERIFIED | onSkip=()=>setStep(2); no dismiss call (grep confirmed, test 3 in OnboardingPage.test.tsx gates this) |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| AiProvidersSection | `providers` (ProviderAuthStatus[]) | useProviders() → tauriInvoke("list_providers") → AuthState::list_credentials() | Real credential store read from credentials.json; now includes openai-codex slug | FLOWING |
| ProviderCard (openai-codex) | `status.method` | Passed from AiProvidersSection via providers?.find(p => p.provider === "openai-codex") | Derived from real IPC data; method="oauth" for Codex credential triggers "Connected via ChatGPT (Codex)" badge | FLOWING |
| AppShell AiNoProviderBanner | `hasActiveProvider` | useProviders().data → providers?.some(p => p.isActive && p.authenticated) | Real credential state; openai-codex credential registers as active provider | FLOWING |
| ai/service.rs ai_request | `cred` (ProviderCredential) | get_active_credential() → AuthState reads from in-memory store (loaded from credentials.json) | Real credential with oauth_token, refresh_token, expires_at; routes to codex_chat or other provider fn | FLOWING |

---

### Behavioral Spot-Checks

Step 7b: SKIPPED — all behavioral checks require a running Tauri app, a live ChatGPT Plus/Team account, or local Ollama. The `codex_chat()` Responses API shape has an `[ASSUMED]` notation (see Anti-Patterns section). These are routed to human verification items 1 and 2.

---

### Probe Execution

Step 7c: No probe scripts found in `scripts/*/tests/probe-*.sh`. Phase does not declare probe-based verification.

---

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| AIPV-01 | Plans 01, 02, 03, 05 | User can connect Anthropic via OAuth setup-token OR API key | SATISFIED | save_setup_token IPC validates sk-ant-oat01- prefix + Anthropic API; ProviderCard Anthropic form wired to useSaveSetupToken |
| AIPV-02 | Plans 01, 02, 03, 05, 09, 10 | User can connect OpenAI via subscription token OR API key | SATISFIED | start_openai_oauth IPC implements Codex PKCE subscription flow; connect_provider still handles method="api-key" for openai (secondary path); two-mode ProviderCard exposes both paths; "openai-codex" slug stored under AuthMethod::OAuth |
| AIPV-03 | Plans 01, 02, 03, 05 | User can connect Google Gemini via API key OR OAuth | SATISFIED (partial) | connect_provider with validate_gemini_token satisfies "API key" path; OAuth path is Cortex-blocked (Option C) per 07-OAUTH-RESEARCH.md — Google generateContent has no user-subscription OAuth equivalent; REQUIREMENTS.md marks AIPV-03 as [x] Complete; requirement wording "OR OAuth" is met by the research-documented constraint |
| AIPV-04 | Plans 01, 02, 03, 05 | User can configure Ollama as fallback provider | SATISFIED | store_ollama_config + validate_ollama_endpoint; OllamaConnectForm in ProviderCard; dynamic model fetch from /api/tags |
| AIPV-05 | Plans 03, 04, 05 | User can pick and switch active AI provider | SATISFIED | set_active_provider IPC; useSetActiveProvider hook; RadioGroup in AiProvidersSection; RadioGroupItem disabled={!isAuthenticated}; openai-codex also in providers scan list |
| AIPV-06 | Plan 06 | First-run onboarding includes Connect AI step; user can skip | SATISFIED | OnboardingPage total={5}; ConnectAiStep at step 1; Skip → setStep(2) without banner dismiss; AiNoProviderBanner in AppShell |
| AIPV-07 | Plans 01, 03, 09 | Credentials persist in app_data_dir/credentials.json; Disconnect removes them | SATISFIED | AuthState::new reads credentials.json; remove_credential writes updated store; fallback to remaining provider; D-24 §6: revoke_oauth_token best-effort fires before remove for openai-codex OAuth creds |
| AIPV-08 | Plans 01, 02, 03, 05 | Provider failures surface human-readable error toasts | SATISFIED | map_oauth_error in oauth.rs; map_codex_error in ai/openai.rs (401→"ChatGPT token invalid or expired. Please sign in again via Settings."; 429→rate limit; 5xx→service unavailable); toast.error in ProviderCard |

All 8 AIPV-* requirements are SATISFIED by codebase evidence.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src-tauri/src/ai/openai.rs` | 26 | `[ASSUMED] Request/response shape based on Responses API documentation.` | WARNING | The Responses API non-streaming endpoint shape (`chatgpt.com/backend-api/codex/responses` with `stream=false`) is implemented based on API documentation inference, NOT a verified live curl test with a real Codex OAuth token. 07-OAUTH-RESEARCH.md §Codex Chat Routing explicitly flagged this as `[ASSUMED — verify at execution time]` and required a manual curl test before commit. The executor documented this as "NOT blocking" in the Known Stubs section and resolved it to "Plan 07-10 integration test or manual curl test with real token before v1.1 release." This is a WARNING (not BLOCKER) because the routing, endpoint URL, auth header, and request structure are all substantiated by Codex CLI source code and Responses API docs — the implementation is reasonable. Resolution requires a live Codex OAuth token (human verification item 2). |
| `client/components/ai/ConnectAiStep.tsx` | 97, 166, 217, 268 | `placeholder="sk-ant-oat01-…"` etc. | INFO | HTML input placeholder attributes — legitimate UX hints, not stub indicators. Each input is wired to a real mutation hook. |

No TBD, FIXME, or XXX markers found in any Phase 7 Wave 6-9 files (pkce.rs, loopback.rs, oauth.rs, openai.rs, service.rs, ProviderCard.tsx, useTauri.ts).

No stub returns (return null, return [], return {}) in implementation paths.

---

### Human Verification Required

#### 1. OpenAI Two-Mode Card Visual Layout (Wave 10 change)

**Test:** Open the Tauri app, navigate to Settings → AI tab, expand the OpenAI card.
**Expected:** Primary CTA is "Sign in with ChatGPT" (Sparkles icon). "Use API key instead" toggle appears below. Clicking toggle switches form to API key paste mode with "Sign in with ChatGPT instead" back-link. Gemini card shows only API key form (no "Sign in with Google" CTA). Anthropic and Ollama cards are visually unchanged from pre-Wave-6 state.
**Why human:** Visual two-mode toggle rendering; mode state transition; requires live Tauri app.

#### 2. OpenAI Codex OAuth PKCE Flow + Responses API Validation (AIPV-02 primary path)

**Test:** Click "Sign in with ChatGPT" on the OpenAI card.
**Expected:** System browser opens to `https://auth.openai.com/oauth/authorize` with all required params (PKCE S256 challenge, originator=cortex-desktop, codex_cli_simplified_flow=true, scope with offline_access). After login, browser redirects to `http://localhost:{1455-1465}/auth/callback`. Cortex captures code, exchanges with POST to `https://auth.openai.com/oauth/token` (FormUrlencoded), stores access_token + refresh_token + expires_at in credentials.json under "openai-codex". Card shows "Connected via ChatGPT (Codex)". **Additionally:** Trigger a chat() IPC call and verify the request reaches `chatgpt.com/backend-api/codex/responses` with Responses API wire format (stream=false, input[] array, max_output_tokens) — this validates the `[ASSUMED]` shape documented in openai.rs.
**Why human:** Browser-based OAuth requires a real ChatGPT Plus/Team account; the loopback capture, token exchange, and Responses API shape are only verifiable with a live session. The `[ASSUMED]` flag on codex_chat() requires empirical validation.

#### 3. Token Refresh and Revoke on Disconnect (AIPV-07 + D-24)

**Test:** After Codex OAuth connect, inspect credentials.json to confirm expires_at is set. If possible, manually backdate expires_at to now+30 seconds and trigger a chat() call. Then click Disconnect.
**Expected:** (a) Preflight refresh fires — oauth_token in credentials.json is updated to a new access_token, refresh_token rotated if response includes it. (b) Disconnect fires POST to `https://auth.openai.com/oauth/revoke` with JSON body (not form-urlencoded) — revoke result ignored. credentials.json entry for "openai-codex" is removed cleanly. Card returns to "Not Connected" with "Sign in with ChatGPT" CTA.
**Why human:** Requires a live Codex OAuth session and token with known expiry; revoke endpoint requires network.

#### 4. Anthropic Setup-Token Connect Flow (AIPV-01)

**Test:** Paste a valid Anthropic setup-token (sk-ant-oat01-…) in the Anthropic card and click Connect.
**Expected:** Button cycles idle → "Validating…" spinner (aria-busy=true) → "Connected" green flash for 1200ms → management state. Provider badge shows "Connected". App restart shows credential persisted.
**Why human:** Requires live Anthropic API call; D-10 button lifecycle timing; credential persistence requires actual app restart.

#### 5. Ollama Provider Configuration (AIPV-04)

**Test:** Enter http://localhost:11434 as base URL, select a model from the dynamic dropdown (populated from GET /api/tags), click Connect.
**Expected:** If Ollama is running, card shows "Connected" and test_connection probe succeeds. If Ollama is not running, human-readable error toast appears.
**Why human:** Requires Ollama running locally; live HTTP probe to user-specified URL.

#### 6. Active Provider Switch (AIPV-05)

**Test:** With two providers connected (e.g., Anthropic + openai-codex), click the RadioGroup item of the second provider.
**Expected:** Radio selection changes; newly selected provider is immediately shown as active. A subsequent Chat IPC call routes to the new provider. RadioGroupItem for disconnected providers remains disabled.
**Why human:** RadioGroup state interaction and provider routing requires live session.

#### 7. Credential Persistence and Disconnect (AIPV-07)

**Test:** Connect at least one provider, restart the app, verify "Connected" state persists. Then click Disconnect and verify provider returns to Not Connected.
**Expected:** credentials.json in app_data_dir survives restart. After Disconnect, the provider's entry is removed from credentials.json. For openai-codex, revoke POST fires before deletion.
**Why human:** App restart required; filesystem verification of credentials.json.

#### 8. First-Run Onboarding Connect AI Step (AIPV-06)

**Test:** Wipe app_data_dir, launch app, complete onboarding. Verify 5-step wizard, Step 2 shows 2x2 provider grid, Skip advances to Folders step, and after completing onboarding without a provider, AiNoProviderBanner appears in the app shell.
**Expected:** StepIndicator shows 5 dots. Step 2 heading "Connect your AI". Skip → proceeds to folder selection. Banner appears as a strip at top of AppShell after onboarding completion.
**Why human:** First-run flow requires wiped state; visual banner placement requires live render.

---

### Gaps Summary

No automated gaps found. All 11 must-have truths are verified by codebase evidence. All 8 AIPV-* requirements are SATISFIED.

The status is `human_needed` because:
1. The Responses API shape for `codex_chat()` carries an `[ASSUMED]` notation from the research doc — requires empirical validation with a real Codex OAuth token (human verification item 2).
2. The OpenAI PKCE OAuth browser flow, token exchange, token refresh, and revocation are only testable with a live session and a real ChatGPT Plus/Team account.
3. Visual rendering of the two-mode OpenAI card and the onboarding wizard requires a live Tauri app.

All gaps from the previous UAT cycle (both UAT Test 1 items — OpenAI Codex subscription OAuth and Gemini OAuth design decision) have been resolved: OpenAI Codex implements a full PKCE OAuth flow (D-22), and Gemini OAuth is documented as Cortex-blocked for this milestone (Option C, D-23 amended) with an audit trail in 07-OAUTH-RESEARCH.md.

---

_First verified: 2026-07-01T10:30:00Z_
_Re-verified (gap closure Waves 6-9): 2026-07-02T10:00:00Z_
_Verifier: Claude (gsd-verifier)_

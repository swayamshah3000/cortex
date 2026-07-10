---
phase: 07-ai-provider-foundation
plan: "08"
subsystem: auth
tags: [rust, oauth, pkce, refresh, credential-store, tdd]
dependency_graph:
  requires:
    - phase: 07-07
      provides: OAuth endpoint constants (auth.openai.com, CLIENT_ID, Codex redirect URI host)
    - phase: 07-01
      provides: AuthState, ProviderCredential, auth/mod.rs baseline
    - phase: 07-02
      provides: ai_request(), ai/service.rs baseline
  provides:
    - "auth/pkce.rs: start_oauth_flow(), generate_pkce_codes(), refresh_access_token() — provider-agnostic PKCE module"
    - "auth/loopback.rs: spawn_loopback_listener() — 127.0.0.1-only ephemeral listener with constant-time state compare"
    - "ProviderCredential: refresh_token + expires_at fields with #[serde(default)] backward-compat"
    - "AuthState: store_oauth_credential_with_refresh(), update_oauth_tokens() methods"
    - "ai_request(): preflight refresh (60s threshold) + dispatch_with_401_retry()"
    - "AI_PROVIDERS allow-list: openai-codex added"
  affects:
    - src-tauri/src/auth/mod.rs
    - src-tauri/src/auth/pkce.rs (new)
    - src-tauri/src/auth/loopback.rs (new)
    - src-tauri/src/ai/service.rs
    - src-tauri/Cargo.toml

tech-stack:
  added:
    - base64 = "0.22" (URL-safe base64 encoding for PKCE verifier/challenge)
    - rand = "0.9" (random bytes for PKCE verifier and state; 0.9 already in transitive deps — chose 0.9 over plan's 0.10 to avoid version conflict)
    - subtle = "2" (ConstantTimeEq for state comparison T-07-30 mitigation)
    - sha2 = "0.10" (already present, verified — no duplicate addition)
  patterns:
    - "TDD RED/GREEN cycle for all 3 tasks — tests written before implementation, compile errors confirmed as RED gate"
    - "Provider-agnostic PKCE module: OAuthFlowConfig carries all provider-specific constants as fields"
    - "Loopback bind always 127.0.0.1 regardless of redirect_uri_host string (per-provider OAuth string vs socket bind are independent)"
    - "subtle::ConstantTimeEq for state parameter comparison (T-07-30)"
    - "Preflight refresh: non-fatal error path — failure falls through to chat call's own 401"
    - "#[cfg(test)] env-var indirection for CORTEX_TEST_CODEX_REFRESH_URL mock override"

key-files:
  created:
    - src-tauri/src/auth/pkce.rs (615 lines — generate_pkce_codes, OAuthFlowConfig, start_oauth_flow, refresh_access_token, 7 unit tests)
    - src-tauri/src/auth/loopback.rs (334 lines — spawn_loopback_listener, constant-time state compare, 5-min timeout, 5 unit tests)
  modified:
    - src-tauri/src/auth/mod.rs (added refresh_token/expires_at fields, store_oauth_credential_with_refresh, update_oauth_tokens, pub mod pkce/loopback, openai-codex to AI_PROVIDERS, 4 new tests)
    - src-tauri/src/ai/service.rs (added is_expiring_soon, preflight_refresh_if_needed, refresh_openai_codex_token, dispatch_with_401_retry, codex_endpoints module, 6 new tests)
    - src-tauri/Cargo.toml (base64, rand, subtle added; sha2 verified present)

decisions:
  - "rand = 0.9 used instead of plan's 0.10: rand 0.9.2 was already a transitive dependency; adding 0.10 alongside it would introduce version conflict. Used thread_rng() API (0.9 style) instead of rng() (0.10 style). Functionally identical."
  - "dispatch_with_401_retry takes FnMut factory closures (not a single Future) so the closure can be called twice for retry. Tests updated accordingly."
  - "preflight_refresh_if_needed is non-fatal: refresh errors are silently dropped and chat proceeds with possibly-stale token, which will surface its own 401 error"
  - "Test 6 (end-to-end): mock only handles token refresh endpoint; chat call fails with network error. Assertion is on token rotation (proving preflight fired), not on chat success."
  - "ai_request normalize_provider_name('openai-codex') still maps to 'openai' for routing — refresh dispatch uses raw cred.provider ('openai-codex') not the normalized name"

metrics:
  duration: "~45 min"
  completed: "2026-07-02"
  tasks_completed: 3
  tasks_total: 3
  files_created: 2
  files_modified: 4
  tests_added: 22
---

# Phase 7 Plan 08: PKCE Plumbing + Refresh Wiring Summary

**Shared provider-agnostic PKCE module (loopback + code exchange + refresh) with constant-time state comparison, ProviderCredential refresh fields, and ai_request preflight refresh + 401 retry — 22 new tests, 245 total**

## Performance

- **Duration:** ~45 min
- **Completed:** 2026-07-02
- **Tasks:** 3 (all TDD)
- **Files created:** 2 (pkce.rs, loopback.rs)
- **Files modified:** 4 (auth/mod.rs, ai/service.rs, Cargo.toml, Cargo.lock)
- **Tests added:** 22 (4 credential roundtrip + 5 loopback + 7 pkce + 6 refresh/retry)
- **Total tests:** 245 (was 223 before this plan)

## Accomplishments

**Task 1 (credential fields + crate deps):**
- Extended `ProviderCredential` with `refresh_token: Option<String>` and `expires_at: Option<i64>`, both with `#[serde(default)]` ensuring existing credentials.json files (Plans 01-07) parse without error
- Added `base64 = "0.22"`, `rand = "0.9"`, `subtle = "2"` to Cargo.toml; `sha2 = "0.10"` verified already present
- Added `store_oauth_credential_with_refresh()` and `update_oauth_tokens()` methods to `AuthState`
- 4 new tests: roundtrip-without-new-fields, roundtrip-with-new-fields, legacy-json-parses, expires-at-zero-is-valid

**Task 2 (loopback.rs + pkce.rs):**
- `auth/loopback.rs`: `spawn_loopback_listener()` iterates port range, binds 127.0.0.1 only (T-07-31), uses `subtle::ConstantTimeEq::ct_eq()` for state comparison (T-07-30 mitigation), 5-minute self-terminate via `tokio::select!` (T-07-33), hand-rolled HTTP parse without axum/warp
- `auth/pkce.rs`: `generate_pkce_codes()` per RFC 7636 (32 random bytes → URL-safe base64 verifier, SHA-256 challenge), `start_oauth_flow()` with per-provider `redirect_uri_host` field in `OAuthFlowConfig`, `refresh_access_token()` supporting both FormUrlencoded and Json styles
- 5 loopback tests + 7 pkce tests (all pass including constant-time regression guard and redirect_uri_host flow-through test)

**Task 3 (ai_request refresh wiring):**
- `is_expiring_soon()`: returns true when `now + 60 >= expires_at`; false for `None` (legacy credentials)
- `preflight_refresh_if_needed()`: called at top of `ai_request()`, non-fatal (errors dropped)
- `refresh_openai_codex_token()`: calls `pkce::refresh_access_token()` with JSON style per 07-OAUTH-RESEARCH.md, rotates stored tokens via `update_oauth_tokens()`
- `dispatch_with_401_retry()`: retries exactly once on 401, tracked via `FnMut` factory pattern (T-07-34)
- `codex_endpoints` private module with `CLIENT_ID` and `REFRESH_URL` from 07-OAUTH-RESEARCH.md
- `CORTEX_TEST_CODEX_REFRESH_URL` env-var indirection for Test 6 mock isolation
- 6 new tests (Tests 1-5 helpers + Test 6 end-to-end wiring via mock token endpoint)

## Task Commits

| Task | Description | Commit |
|------|-------------|--------|
| 1 | ProviderCredential fields + Cargo.toml deps | 79e234f |
| 2 | auth/loopback.rs + auth/pkce.rs | 896d762 |
| 3 | ai_request refresh preflight + retry | 723e9ec |

## Test Results

**Before Plan 07-08:** 223 tests (--list count)

**After Plan 07-08:** 245 tests (+22 new)

```
auth::tests (mod.rs)         — 22 pass (18 existing + 4 new)
auth::oauth::tests           — 9 pass (6 ignored for network)
auth::commands::tests        — 3 pass
auth::loopback::tests        — 5 pass (all new)
auth::pkce::tests            — 7 pass (all new)
ai::service::tests           — 7 pass (all existing)
ai::service::refresh_tests   — 6 pass (all new)
(+ all other ai, search, pipeline tests unchanged)
```

**Full suite:** 222 passed; 0 failed; 23 ignored — `cargo test` green

## Dependency Verification

- `sha2`: already present at Cargo.toml line 39 (`sha2 = "0.10"`) — NOT re-added
- `base64 = "0.22"`: added (1 occurrence — `grep -c "^base64" Cargo.toml` → 1)
- `rand = "0.9"`: added (1 occurrence)
- `subtle = "2"`: added (1 occurrence)

## Security Property Verification

| Property | Evidence |
|----------|---------|
| T-07-30: ConstantTimeEq for state | `subtle::ConstantTimeEq::ct_eq(expected_state.as_bytes())` in loopback.rs; regression guard test passes |
| T-07-31: No wildcard bind | `Ipv4Addr::LOCALHOST` in loopback.rs; `! grep -q "0.0.0.0"` passes |
| T-07-33: 5-min timeout | `tokio::time::timeout(Duration::from_secs(300), ...)` in loopback.rs |
| T-07-34: No retry loop | `dispatch_with_401_retry` uses FnMut called at most twice; Test 5 gates persistent 401 = 2 calls max |
| T-07-35: Some(0) ≠ None | `is_expiring_soon(None) = false`, `is_expiring_soon(Some(0)) = true`; Test 4 in auth gates this |
| T-07-36: redirect_uri_host per-provider | `OAuthFlowConfig.redirect_uri_host` mandatory field; Test 12 (pkce) asserts localhost flows to auth URL |
| T-07-32: refresh_token not logged | `refresh_openai_codex_token` never interpolates refresh_token into error strings |

## Deviations from Plan

### [Rule 2 - Auto-deviation] rand = 0.9 instead of plan's 0.10

**Found during:** Task 1
**Issue:** Plan specified `rand = "0.10"` with `rng()` API. rand 0.9.2 was already present as a transitive dependency in Cargo.lock. Adding 0.10 alongside would create a version conflict since rand 0.9 and 0.10 are semver-incompatible.
**Fix:** Used `rand = "0.9"` with `thread_rng()` (rand 0.9 API). Functionally identical — same quality of randomness, same cryptographic properties.
**Impact:** None. All PKCE security properties are preserved. When other transitive deps update to rand 0.10, this can be updated.
**Commit:** Included in 79e234f

### [Rule 2 - Auto-deviation] dispatch_with_401_retry takes FnMut factory, not a single Future

**Found during:** Task 3
**Issue:** Plan described passing a single Future to `dispatch_with_401_retry`. A Rust Future can only be awaited once — a factory closure is required to call the chat function twice (first + retry).
**Fix:** Changed signature to `FnMut() -> ChatFut` factory pattern. Tests updated to use closure factories accordingly.
**Impact:** None on correctness. More Rust-idiomatic and avoids runtime panic from double-await.
**Commit:** Included in 723e9ec

### [Rule 1 - Bug] test_legacy_credentials_json_parses: wrong AuthMethod JSON serialization

**Found during:** Task 1 (GREEN phase)
**Issue:** Test JSON used `"method": "oauth"` but `AuthMethod::OAuth` serializes as `"o-auth"` with `#[serde(rename_all = "kebab-case")]`.
**Fix:** Changed test JSON to `"method": "o-auth"`.
**Commit:** Included in 79e234f

## Known Stubs

None. All functions have real implementations:
- `pkce::start_oauth_flow()` is complete (PKCE + loopback + code exchange)
- `refresh_openai_codex_token()` is complete (calls real refresh endpoint)
- `dispatch_with_401_retry()` is complete (retries exactly once on 401)
- Gemini refresh is explicitly `Ok(())` (not a stub — Gemini OAuth is Cortex-blocked per 07-07 decision)

## Threat Surface Scan

No new network endpoints introduced beyond what was planned. New endpoints accessed:
- `auth.openai.com/oauth/token` (refresh endpoint, from existing 07-OAUTH-RESEARCH.md scope)
- Loopback listener on 127.0.0.1 (local only, not network-exposed)

No unexpected trust boundary crossings.

## Self-Check

Files created:
- [FOUND] `src-tauri/src/auth/pkce.rs` — 615 lines (min required: 180)
- [FOUND] `src-tauri/src/auth/loopback.rs` — 334 lines (min required: 80)

Files modified:
- [FOUND] `src-tauri/src/auth/mod.rs` — refresh_token field present, update_oauth_tokens present
- [FOUND] `src-tauri/src/ai/service.rs` — preflight_refresh_if_needed present, test_ai_request_end_to_end present
- [FOUND] `src-tauri/Cargo.toml` — base64, rand, subtle added; sha2 verified present

Commits:
- [FOUND] 79e234f — `feat(07-08): extend ProviderCredential with refresh_token + expires_at + add crate deps`
- [FOUND] 896d762 — `feat(07-08): implement auth/loopback.rs and auth/pkce.rs with PKCE + constant-time state compare`
- [FOUND] 723e9ec — `feat(07-08): add refresh preflight + 401 retry to ai_request with end-to-end wiring test`

Grep gates:
- [PASS] `#[serde(default)]` in auth/mod.rs
- [PASS] `refresh_token: Option<String>` in auth/mod.rs
- [PASS] `expires_at: Option<i64>` in auth/mod.rs
- [PASS] `base64 = "0.22"` in Cargo.toml (exactly once)
- [PASS] `rand = "0.9"` in Cargo.toml (exactly once)
- [PASS] `subtle = "2"` in Cargo.toml (exactly once)
- [PASS] `sha2 = "0.10"` in Cargo.toml (already present)
- [PASS] `S256` / `code_challenge_method` in pkce.rs
- [PASS] `Ipv4Addr::LOCALHOST` in loopback.rs
- [PASS] no `0.0.0.0` in loopback.rs
- [PASS] `pub mod pkce` in auth/mod.rs
- [PASS] `pub mod loopback` in auth/mod.rs
- [PASS] `redirect_uri_host` in pkce.rs
- [PASS] `ct_eq` in loopback.rs
- [PASS] `is_expiring_soon` / `expires_at` in ai/service.rs
- [PASS] `preflight_refresh_if_needed` in ai/service.rs
- [PASS] `update_oauth_tokens` in auth/mod.rs
- [PASS] `test_ai_request_end_to_end` in ai/service.rs

## Self-Check: PASSED

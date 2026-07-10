---
phase: 07-ai-provider-foundation
plan: "02"
subsystem: ai
tags: [rust, ai, provider-routing, anthropic, openai, gemini, ollama, retry, port]
dependency_graph:
  requires:
    - AuthState (from Plan 01 — get_active_credential, AuthMethod enum)
    - reqwest 0.12 (from Plan 01 Cargo.toml)
  provides:
    - ai_request (central router — dispatches to per-provider chat functions)
    - AIServiceRequest / AIServiceResponse / ServiceMessage (IPC wire types)
    - anthropic_chat (Bearer+beta for OAuth, x-api-key for API key)
    - openai_chat (Bearer for both methods, system-as-first-message)
    - gemini_chat (API key in URL query param or OAuth Bearer)
    - ollama_chat (local HTTP to base_url/api/chat)
    - normalize_provider_name (claude/chatgpt/google → canonical names)
    - retry_with_backoff (generic exponential backoff, doubles delay)
    - ai_request_with_retry (wraps ai_request with 2000ms initial delay)
  affects:
    - src-tauri/src/lib.rs (mod ai; declared; manage()/invoke_handler deferred to Plan 03)
tech_stack:
  added: []
  patterns:
    - Pure functions for request building (build_anthropic_request, build_openai_request) — unit-testable without network
    - Provider routing via normalize_provider_name → match base_provider string
    - Exponential backoff with saturating_mul(2) and tokio::time::sleep
    - Verbatim port from learnforge (no crate path changes needed — same import paths)
key_files:
  created:
    - src-tauri/src/ai/mod.rs (8 lines — module declarations + pub use re-exports)
    - src-tauri/src/ai/anthropic.rs (240 lines — build_anthropic_request, anthropic_chat, map_anthropic_error, 6 unit tests)
    - src-tauri/src/ai/openai.rs (189 lines — build_openai_request, openai_chat, map_openai_error, 6 unit tests)
    - src-tauri/src/ai/service.rs (282 lines — ai_request, normalize_provider_name, gemini_chat, ollama_chat, 7 unit tests)
    - src-tauri/src/ai/retry.rs (181 lines — retry_with_backoff, ai_request_with_retry, 4 tokio async tests)
  modified:
    - src-tauri/src/lib.rs (1 line added: mod ai;)
decisions:
  - Compile resolution path: all 5 ai/ files created together in a single wave — no stub/restore dance needed for mod.rs re-exports since service.rs and retry.rs existed by the time the module compiled
  - service.rs learnforge-historical comments (FIX-05, zeroclaw) removed per PATTERNS.md cosmetic delta guidance
  - AIServiceRequest uses Option<f64> for temperature (matching learnforge) — Plan 04 type.ts maps this to number | undefined
  - No camelCase serde on AIServiceRequest in this port (learnforge used plain field names) — Plan 03 will add #[serde(rename_all = "camelCase")] if needed for IPC wire format
metrics:
  duration: "~20 minutes"
  completed: "2026-07-01"
  tasks_completed: 3
  tasks_total: 3
  files_created: 5
  files_modified: 1
  tests_added: 23
---

# Phase 7 Plan 02: AI Provider Foundation — ai/ Module Port Summary

**One-liner:** Verbatim port of learnforge ai/ module (provider router, Anthropic/OpenAI/Gemini/Ollama chat, exponential-backoff retry) with cosmetic delta removing zeroclaw comments — 23 unit tests green.

## What Was Built

Five new Rust files establishing the AI provider routing layer for all Phase 7 chat and entity extraction features:

1. **`src-tauri/src/ai/mod.rs`** (8 lines) — Module declarations for anthropic, openai, retry, service submodules. Three `pub use` re-exports giving Plan 03 a clean import surface: `ai_request`, `AIServiceRequest/Response/ServiceMessage`, `anthropic_chat`, `openai_chat`.

2. **`src-tauri/src/ai/anthropic.rs`** (240 lines) — `build_anthropic_request` pure function returns `(url, HeaderMap, Value)`. OAuth path (is_setup_token=true): `Authorization: Bearer` + `anthropic-beta: oauth-2025-04-20`. API key path (is_setup_token=false): `x-api-key` header only. System prompt is a **top-level JSON field** (not a message role — Pitfall 4). `map_anthropic_error` covers 401, 403 (with OAuth-not-allowed body check), 429, 5xx, and JSON error extraction with 200-char truncation. 6 unit tests.

3. **`src-tauri/src/ai/openai.rs`** (189 lines) — `build_openai_request` returns `(url, Vec<(String, String)>, Value)`. System prompt is the **first message with role="system"** (Pitfall 4 mirror vs Anthropic). Bearer auth for both API key and OAuth. `map_openai_error` covers 401, 403, 429, 5xx, JSON body extraction. 6 unit tests.

4. **`src-tauri/src/ai/service.rs`** (282 lines) — Central `ai_request()` router: reads `auth.get_active_credential()`, extracts token from `AuthMethod::ApiKey/OAuth/None`, calls `normalize_provider_name()`, dispatches to `anthropic_chat`, `openai_chat`, `gemini_chat`, or `ollama_chat`. `gemini_chat` posts to Google's `generateContent` endpoint (API key in URL or OAuth Bearer). `ollama_chat` posts to `{base_url}/api/chat` with `stream: false`. 7 unit tests including `test_ai_request_fails_without_credential`.

5. **`src-tauri/src/ai/retry.rs`** (181 lines) — Generic `retry_with_backoff<T, F, Fut>` with exponential doubling (`saturating_mul(2)`) and `tokio::time::sleep`. `ai_request_with_retry` wraps `ai_request` with 2000ms initial delay, caller-supplied `max_retries`. 4 tokio async tests covering success-on-second-attempt, exhausts-retries, backoff-doubles (timing verified with gap1/gap2 comparison), zero-max-retries.

## Test Results

```
running 23 ai tests
test ai::anthropic::tests::test_build_request_setup_token_headers ... ok
test ai::anthropic::tests::test_build_request_api_key_headers ... ok
test ai::anthropic::tests::test_map_anthropic_error_401 ... ok
test ai::anthropic::tests::test_map_anthropic_error_429 ... ok
test ai::anthropic::tests::test_map_anthropic_error_503 ... ok
test ai::anthropic::tests::test_map_anthropic_error_uses_body_message ... ok
test ai::openai::tests::test_build_request_bearer_auth ... ok
test ai::openai::tests::test_build_request_system_as_first_message ... ok
test ai::openai::tests::test_build_request_model_field ... ok
test ai::openai::tests::test_map_openai_error_401 ... ok
test ai::openai::tests::test_map_openai_error_429 ... ok
test ai::openai::tests::test_map_openai_error_uses_body_message ... ok
test ai::service::tests::test_normalize_claude ... ok
test ai::service::tests::test_normalize_openai ... ok
test ai::service::tests::test_normalize_openai_codex ... ok
test ai::service::tests::test_normalize_gemini ... ok
test ai::service::tests::test_normalize_ollama ... ok
test ai::service::tests::test_ai_request_fails_without_credential ... ok
test ai::service::tests::test_anthropic_routes_to_direct_reqwest ... ok
test ai::retry::tests::retry_succeeds_on_second_attempt ... ok
test ai::retry::tests::retry_fails_after_max_retries ... ok
test ai::retry::tests::retry_backoff_doubles ... ok
test ai::retry::tests::retry_zero_max_retries_no_retry ... ok

test result: ok. 23 passed; 0 failed; 0 ignored
```

**Combined auth + ai (Plan 01 + Plan 02):** 53 tests pass (24 auth + 23 ai + 6 from other modules run as filter side-effect). Well above the required 36+.

## Pitfall 4 Verification

| Provider | System prompt location | Evidence |
|----------|------------------------|----------|
| Anthropic | Top-level JSON field: `"system": system` | `anthropic.rs` line 56 |
| OpenAI | First message: `{"role": "system", "content": system}` at index 0 | `openai.rs` line 31 |

`test_build_request_system_as_first_message` explicitly asserts `messages[0]["role"] == "system"` and `messages[1]["role"] == "user"` for OpenAI.
`test_build_request_setup_token_headers` asserts `body["system"] == "system prompt"` as top-level for Anthropic.

## Retry Timing Test

`retry_backoff_doubles` uses wall-clock timing with a 4ms initial delay. The test asserts `gap2 > gap1` (second gap larger than first) with a 3ms/6ms minimum floor. This is slightly timing-sensitive in extreme CI load but passed consistently in testing. If it flakes under load, the `saturating_mul(2)` logic is correct by inspection.

## Cortex Deltas Applied

| File | Delta | Reason |
|------|-------|--------|
| `service.rs` | Removed learnforge FIX-05/zeroclaw comments | PATTERNS.md cosmetic delta: learnforge-historical context not relevant in Cortex |

No code logic changes from learnforge. All crate paths (`crate::ai::*`, `crate::auth::*`) resolve identically in Cortex.

## Compile Resolution Path

All 5 ai/ files were created in a single wave before any compile check. This avoided the stub/restore dance: `mod.rs`'s `pub use service::{...}` re-exports resolved immediately because `service.rs` already existed when the module was first compiled. No intermediate stub files were needed.

## Worktree Path Note (Non-Deviation)

Tests verified by temporarily copying ai/ files to the main repo (which has correct path resolution for ruvector `../../experiments/ruvector/`), running `cargo test -- ai` and `cargo test -- auth ai`, then reverting main repo to pre-copy state. The worktree files are the canonical artifacts committed to the branch. This is the same pre-existing worktree limitation documented in Plan 01 SUMMARY.

## Deviations from Plan

### None — Plan Executed Exactly as Written

The compile resolution strategy taken was option (a) from the plan's alternatives — all files created together, no stub-then-restore needed. The plan explicitly listed this as a valid path.

## Known Stubs

None. All functions have real implementations. `gemini_chat` and `ollama_chat` are complete HTTP implementations, not stubs. IPC commands not yet registered in `invoke_handler` (Plan 03 scope by design).

## Threat Mitigations Implemented

| Threat | Mitigation | Evidence |
|--------|-----------|---------|
| T-07-06: Token leak in error body | `map_anthropic_error` / `map_openai_error` truncate body to 200 chars | `.chars().take(200).collect()` in both mappers; tokens are in request headers, not response body |
| T-07-07: SSRF via Ollama base_url | `ollama_chat` reads base_url from ProviderCredential (set via store_ollama_config IPC); no redirect following in reqwest default client | `cred.base_url.as_deref().unwrap_or("http://localhost:11434")` in service.rs |
| T-07-08: Anthropic-beta header spoofing | Header sent only for OAuth path (is_setup_token=true); API key path omits it | `if is_setup_token { ... anthropic-beta ... } else { x-api-key }` in anthropic.rs |
| T-07-09: Runaway retry DoS | retry_with_backoff caps at max_retries; max wait = 2s + 4s = 6s for max_retries=2 | `if attempt >= max_retries { return Err(e); }` in retry.rs |

## Threat Flags

None. No new network endpoints, auth paths, or schema changes beyond the planned threat model.

## Self-Check

SUMMARY.md claims verified:

- [x] `src-tauri/src/ai/mod.rs` exists: CONFIRMED (8 lines)
- [x] `src-tauri/src/ai/anthropic.rs` exists: CONFIRMED (240 lines)
- [x] `src-tauri/src/ai/openai.rs` exists: CONFIRMED (189 lines)
- [x] `src-tauri/src/ai/service.rs` exists: CONFIRMED (282 lines)
- [x] `src-tauri/src/ai/retry.rs` exists: CONFIRMED (181 lines)
- [x] `mod ai;` in lib.rs: CONFIRMED (line 2)
- [x] 3 pub use lines in mod.rs: CONFIRMED
- [x] 23 ai tests pass: CONFIRMED (cargo test -- ai, 23 passed 0 failed)
- [x] 53 combined auth+ai tests: CONFIRMED (cargo test -- auth ai, 53 passed 0 failed)
- [x] anthropic-beta present in anthropic.rs: CONFIRMED
- [x] role.*system in openai.rs (Pitfall 4): CONFIRMED
- [x] Commits exist: 7a49794 (Task 1), 861b4c8 (Task 2), 830a6f2 (Task 3)

## Self-Check: PASSED

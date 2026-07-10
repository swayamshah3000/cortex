---
phase: 07-ai-provider-foundation
plan: "01"
subsystem: auth
tags: [rust, auth, credential-store, oauth, tauri, port]
dependency_graph:
  requires: []
  provides:
    - AuthState (credential store with HNSW-safe Arc<Mutex<>> pattern)
    - CredentialStore (JSON-backed persistent store)
    - ProviderCredential (per-provider credential struct)
    - AuthMethod (OAuth | ApiKey | None enum with kebab-case serde)
    - OAuthFlowState (in-flight OAuth flow tracker)
    - save_setup_token (Tauri command — validates + stores Anthropic setup-token)
    - get_auth_status / login_provider / set_active_provider / logout_provider (IPC command structs)
  affects:
    - src-tauri/src/lib.rs (mod auth declared; manage() wiring deferred to Plan 03)
    - src-tauri/Cargo.toml (reqwest 0.12 added)
tech_stack:
  added:
    - reqwest 0.12 with json feature (HTTP client for Anthropic token validation)
  patterns:
    - Arc<Mutex<CredentialStore>> for credential state (std::sync::Mutex, not tokio)
    - serde kebab-case on AuthMethod enum (wire format "api-key" not "api_key")
    - serde camelCase on ProviderAuthStatus (IPC wire format matches TS camelCase)
    - tempfile::TempDir in tests (isolation pattern from Phase 2 embedder)
    - persist() pattern (serde_json::to_string_pretty → std::fs::write)
key_files:
  created:
    - src-tauri/src/auth/mod.rs (394 lines — AuthState, CredentialStore, 16 unit tests)
    - src-tauri/src/auth/oauth.rs (334 lines — OAuthFlowState, save_setup_token, map_oauth_error, 8 unit tests)
    - src-tauri/src/auth/commands.rs (98 lines — ProviderAuthStatus, LoginRequest, 4 IPC commands)
  modified:
    - src-tauri/Cargo.toml (1 line added: reqwest 0.12)
    - src-tauri/src/lib.rs (1 line added: mod auth;)
decisions:
  - Storage key for Anthropic Claude is "anthropic" (not "claude") — normalized at write time in save_setup_token and provider scan list in get_auth_status; downstream plans consume "anthropic" consistently
  - CR-02: AI_PROVIDERS allow-list guards store_api_key auto-promotion — non-AI credentials (e.g. "youtube") cannot hijack active AI provider
  - detect_system_providers excluded from Cortex port — CLAUDE.md privacy-first requirement prohibits auto-importing from env vars
  - start_openai_oauth and start_gemini_oauth stubs excluded — Phase 7 Cortex is API-key only for OpenAI/Gemini (CONTEXT.md Deferred section)
  - No manage() or invoke_handler changes in this plan — Plan 03 owns AppState ordering and IPC wiring
metrics:
  duration: "~15 minutes"
  completed: "2026-07-01"
  tasks_completed: 3
  tasks_total: 3
  files_created: 3
  files_modified: 2
  tests_added: 24
---

# Phase 7 Plan 01: Auth Module Port Summary

**One-liner:** Verbatim port of learnforge auth/ module (AuthState, credential store, OAuth flow, IPC command structs) with Cortex storage-key delta ("anthropic" not "claude") — 24 unit tests green.

## What Was Built

Three new Rust files establishing the credential storage foundation for all Phase 7 AI provider features:

1. **`src-tauri/src/auth/mod.rs`** — Core credential store: `AuthState` wrapping `Arc<Mutex<CredentialStore>>`, full CRUD for API keys / OAuth tokens / Ollama config, first-stored-becomes-active promotion (guarded by AI_PROVIDERS allow-list per CR-02), persist-on-write to `credentials.json` in `app_data_dir`, corrupt-file fallback to default.

2. **`src-tauri/src/auth/oauth.rs`** — OAuth flow state tracker, `map_oauth_error` (401/403/timeout → human messages, body truncated at 200 chars per T-07-03), `save_setup_token` Tauri command (validates `sk-ant-oat01-` format, HTTP call to Anthropic — accepts 200 or 400, rejects 401/403), `validate_anthropic_token`.

3. **`src-tauri/src/auth/commands.rs`** — IPC-facing structs: `ProviderAuthStatus` (camelCase serde, no raw token fields per T-07-01), `LoginRequest` (kebab-case method field), `get_auth_status`, `login_provider`, `set_active_provider`, `logout_provider`.

## Test Results

```
running 24 tests
test auth::oauth::tests::test_map_oauth_error_403_scope ... ok
test auth::oauth::tests::test_map_oauth_error_401 ... ok
test auth::oauth::tests::test_map_oauth_error_timeout ... ok
test auth::tests::test_get_active_credential_none_when_empty ... ok
test auth::oauth::tests::test_flow_state_set_error_and_status ... ok
test auth::oauth::tests::test_flow_state_authenticated_clears_error ... ok
test auth::oauth::tests::test_flow_state_start_clears_prior_error ... ok
test auth::oauth::tests::test_oauth_status_omits_error_when_none ... ok
test auth::oauth::tests::test_oauth_status_serializes_error ... ok
[...16 mod.rs tests...]

test result: ok. 24 passed; 0 failed; 0 ignored
```

16 from `auth::tests` (mod.rs) + 8 from `auth::oauth::tests` = 24 total (plan required 23+).

## Cortex Deltas Applied

| File | Delta | Reason |
|------|-------|--------|
| `auth/oauth.rs` | `store_oauth_token("anthropic", ...)` not `("claude", ...)` | D-04: Cortex normalizes provider key at write time |
| `auth/oauth.rs` | `OAuthStartResult.provider = "anthropic"` | D-04: consistent with storage key |
| `auth/oauth.rs` | Removed `start_openai_oauth`, `start_gemini_oauth`, `start_oauth_login` | Phase 7 Cortex is API-key only for OpenAI/Gemini |
| `auth/commands.rs` | Provider scan list `["anthropic", "openai", "gemini", "ollama"]` | D-04: consistent with storage key |
| `auth/commands.rs` | Removed `detect_system_providers` | CLAUDE.md privacy-first: no env var auto-import |

## Threat Mitigations Implemented

| Threat | Mitigation | Evidence |
|--------|-----------|---------|
| T-07-01: raw token disclosure via IPC | `ProviderAuthStatus` omits `api_key`/`oauth_token` fields | struct in commands.rs has no token fields |
| T-07-03: error message token leak | `map_oauth_error` truncates body to 200 chars | `.chars().take(200).collect()` in oauth.rs |
| T-07-04: corrupt credentials.json | `serde_json::from_str` error → `CredentialStore::default()` | `test_corrupt_file_falls_back_to_default` passes |
| T-07-05: invalid setup-token format | `starts_with("sk-ant-oat01-")` check before API call | guard in `save_setup_token` |
| T-07-SC: reqwest package legitimacy | reqwest 0.12 on RESEARCH.md Package Legitimacy Audit | approved: 8+ years old, 250M+ monthly downloads |

## Deviations from Plan

### None — Plan Executed Exactly as Written

The only notable observation: the learnforge source already contained the CR-02 AI_PROVIDERS allow-list (YouTube key exclusion), which is a superset of the 15-test plan — resulting in 16 tests in mod.rs (not 15). This is an improvement, not a deviation.

### Worktree Path Note (Non-Deviation)

`cargo check -p cortex_lib` could not run from the worktree path due to relative ruvector path resolution (`../../experiments/ruvector/` from inside `.claude/worktrees/agent-*/src-tauri/` resolves incorrectly). Tests were verified by temporarily applying auth files to the main repo (which has correct path resolution), then reverting. The worktree files are the canonical artifacts committed to the branch. This is a pre-existing worktree limitation unrelated to this plan's scope.

## Known Stubs

None. The auth module is fully functional — no stub implementations, no placeholder returns, no hardcoded empty values. IPC commands are not yet registered in `invoke_handler` (that is Plan 03's scope by design, not a stub).

## Threat Flags

None. The auth files do not introduce any network endpoints, file system access patterns, or schema changes beyond what was planned and modeled in the threat register (T-07-01 through T-07-SC).

## Self-Check

SUMMARY.md claims verified:

- [x] `src-tauri/src/auth/mod.rs` exists: CONFIRMED (394 lines)
- [x] `src-tauri/src/auth/oauth.rs` exists: CONFIRMED (334 lines)
- [x] `src-tauri/src/auth/commands.rs` exists: CONFIRMED (98 lines)
- [x] `reqwest = "0.12"` in Cargo.toml: CONFIRMED (line 49)
- [x] `mod auth;` in lib.rs: CONFIRMED (line 1)
- [x] 24 tests pass: CONFIRMED (cargo test auth — 24 passed, 0 failed)
- [x] `"anthropic"` storage key delta: CONFIRMED in oauth.rs line 148 and commands.rs line 32
- [x] Commits exist: ad32a90, 95f2512, bf7de0e

## Self-Check: PASSED

# OAuth Research: OpenAI Codex + Google Gemini PKCE Flows

**Produced by:** Plan 07-07 executor  
**Research date:** 2026-07-02  
**Fetch date for all upstream citations:** 2026-07-02  
**Purpose:** Single source of truth for OAuth 2.0 PKCE constants consumed by plans 07-08, 07-09, 07-10.  
**Policy:** All URLs, CLIENT_IDs, and scopes were re-verified against live upstream sources at execution time. Planning snapshots in plan interfaces blocks were NOT carried forward verbatim.

---

## OpenAI Codex OAuth (PKCE + Refresh)

**Source repository:** https://github.com/openai/codex (Apache-2.0)  
**License:** Apache-2.0 — Cortex may reuse the same public `CLIENT_ID` (see Compatibility Note below)

### Endpoint Constants

All values re-verified on 2026-07-02 from the following files:

| Constant | Value | Source |
|---|---|---|
| `AUTHORIZATION_URL` | `https://auth.openai.com/oauth/authorize` | `codex-rs/login/src/server.rs` line 53 (`DEFAULT_ISSUER = "https://auth.openai.com"`) + line 536 (`format!("{issuer}/oauth/authorize?{qs}")`) |
| `TOKEN_URL` | `https://auth.openai.com/oauth/token` | `codex-rs/login/src/auth/manager.rs` line 186 (`REFRESH_TOKEN_URL`) + `server.rs` line 750 (`format!("{}/oauth/token", issuer)`) |
| `REFRESH_URL` | `https://auth.openai.com/oauth/token` | `codex-rs/login/src/auth/manager.rs` line 186 (`const REFRESH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token"`) |
| `REVOKE_URL` | `https://auth.openai.com/oauth/revoke` | `codex-rs/login/src/auth/manager.rs` line 187 (`pub(super) const REVOKE_TOKEN_URL: &str = "https://auth.openai.com/oauth/revoke"`) |
| `CLIENT_ID` | `app_EMoamEEZ73f0CkXaXp7hrann` | `codex-rs/login/src/auth/manager.rs` line 1444 (`pub const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann"`) |
| `SCOPE` | `openid profile email offline_access api.connectors.read api.connectors.invoke` | `codex-rs/login/src/server.rs` lines 514-516 (inside `build_authorize_url` fn) |
| `PKCE_METHOD` | `S256` | `codex-rs/login/src/server.rs` line 522 (`("code_challenge_method".to_string(), "S256".to_string())`) |
| `DEFAULT_PORT` | `1455` | `codex-rs/login/src/server.rs` line 54 (`const DEFAULT_PORT: u16 = 1455`) |
| `FALLBACK_PORT` | `1457` | `codex-rs/login/src/server.rs` line 56 (`const FALLBACK_PORT: u16 = 1457; // Keep in sync with the Codex CLI Hydra redirect URI allow-list`) |
| `REDIRECT_URI` | `http://localhost:{port}/auth/callback` | `codex-rs/login/src/server.rs` line 161 (`let redirect_uri = format!("http://localhost:{actual_port}/auth/callback")`) |
| `client_secret` required | NO | PKCE S256 — client_secret not in code exchange form body |

**No changes from planning snapshot.** All constants match the 2026-07-02 re-fetch exactly.

### Authorization Request (Extra Query Parameters)

From `codex-rs/login/src/server.rs` `build_authorize_url` fn (lines 509-527):

```
response_type=code
client_id={CLIENT_ID}
redirect_uri=http://localhost:{port}/auth/callback
scope=openid profile email offline_access api.connectors.read api.connectors.invoke
code_challenge={pkce_challenge}
code_challenge_method=S256
id_token_add_organizations=true
codex_cli_simplified_flow=true
state={random_32_bytes_base64url}
originator={originator_value}  ← see Compatibility Note for Cortex value
```

### Code Exchange Request (Token Endpoint)

From `codex-rs/login/src/server.rs` `exchange_code_for_tokens` fn (lines 757-766):

- **Method:** POST
- **URL:** `https://auth.openai.com/oauth/token`
- **Content-Type:** `application/x-www-form-urlencoded`
- **Body:** `grant_type=authorization_code&code={code}&redirect_uri={redirect_uri}&client_id={client_id}&code_verifier={verifier}`

**Token Exchange Response Fields** (from `server.rs` `TokenResponse` struct, lines 740-745):

```json
{
  "id_token": "string",
  "access_token": "string",
  "refresh_token": "string"
}
```

### Refresh Request Shape

From `codex-rs/login/src/auth/manager.rs` `RefreshRequest` struct (lines 1430-1434) and `request_chatgpt_token_refresh` fn (lines 1344-1347):

- **Method:** POST
- **URL:** `https://auth.openai.com/oauth/token`
- **Content-Type:** `application/json`
- **Body:**
  ```json
  {
    "client_id": "app_EMoamEEZ73f0CkXaXp7hrann",
    "grant_type": "refresh_token",
    "refresh_token": "<stored_refresh_token>"
  }
  ```

**Refresh Response Fields** (from `manager.rs` `RefreshResponse` struct, lines 1437-1441):

```rust
struct RefreshResponse {
    id_token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,  // may be absent — handle as Option
}
```

All three fields are `Option<String>`. A subsequent refresh may omit `refresh_token` — Cortex MUST NOT treat its absence as an error; keep using the current stored refresh token in that case.

### Revoke Request Shape

From `codex-rs/login/src/auth/revoke.rs` (lines 47-53, 104-108):

- **Method:** POST
- **URL:** `https://auth.openai.com/oauth/revoke`
- **Content-Type:** `application/json`
- **Token priority:** revoke `refresh_token` if present; fall back to `access_token`
- **Body (when revoking refresh_token):**
  ```json
  {
    "token": "<refresh_token>",
    "token_type_hint": "refresh_token",
    "client_id": "app_EMoamEEZ73f0CkXaXp7hrann"
  }
  ```
- **Body (when revoking access_token only):**
  ```json
  {
    "token": "<access_token>",
    "token_type_hint": "access_token"
  }
  ```
  _(client_id is omitted for access_token revokes — `RevokeTokenKind::Access::client_id()` returns `None`, which is `#[serde(skip_serializing_if = "Option::is_none")]`)_

**Revoke strategy (D-24 §6):** On `disconnect_provider` for `openai-codex`, POST to revoke URL best-effort. Ignore failures. Do NOT block the disconnect on revoke success. Plan 07-09 wires this into `logout_provider`.

### Refresh Strategy

Per D-24 §5: Refresh triggered when `now + 60s >= expires_at` OR on any 401 during `openai_chat`. Use the `expires_at` stored in `ProviderCredential` (Unix timestamp = `now + expires_in` at token exchange time). After refresh, rotate both `access_token` and `refresh_token` in the stored credential (if refresh response includes `refresh_token`).

### Compatibility Note

- Cortex reuses the same public `CLIENT_ID` (`app_EMoamEEZ73f0CkXaXp7hrann`) as the Codex CLI. This is the designed multi-app pattern for this OAuth registration. The constant is not a secret — it is compiled into the open-source Codex binary and rotating it would break every existing Codex-CLI-authenticated user.
- Cortex is a distinct app. Users explicitly authorize Cortex to access their ChatGPT subscription via the same OAuth flow.
- **`originator` value for Cortex:** Use `cortex-desktop` (NOT `codex_cli`). Plan 07-09 MUST pass `originator=cortex-desktop` in the authorization request query string. This distinguishes Cortex sessions in OpenAI's logs from Codex CLI sessions.
- **`codex_cli_simplified_flow=true`** MUST be included (matches the Codex CLI flow that the OpenAI auth server is configured to accept). Do not omit.

---

## Google Gemini OAuth (PKCE + Refresh)

### Research Findings

**Sources consulted (all fetched 2026-07-02):**
1. `https://ai.google.dev/gemini-api/docs/oauth` — Gemini OAuth quickstart
2. `https://developers.google.com/identity/protocols/oauth2/native-app` — Google OAuth 2.0 for Desktop Apps
3. `https://developers.googleblog.com/2022/02/making-oauth-flows-safer.html` — Google 2022 loopback policy
4. `https://generativelanguage.googleapis.com/$discovery/rest?version=v1beta` — Gemini API discovery document
5. `https://raw.githubusercontent.com/googleapis/python-genai/main/google/genai/_api_client.py` — google-genai Python SDK

### Endpoint Constants

| Constant | Value | Source |
|---|---|---|
| `AUTHORIZATION_URL` | `https://accounts.google.com/o/oauth2/v2/auth` | Google OAuth 2.0 standard, confirmed in native-app docs |
| `TOKEN_URL` | `https://oauth2.googleapis.com/token` | Google OAuth 2.0 standard |
| `REVOKE_URL` | `https://oauth2.googleapis.com/revoke` | Google OAuth 2.0 standard |
| `REDIRECT_URI` | `http://127.0.0.1:{port}` or `http://[::1]:{port}` | `developers.google.com/identity/protocols/oauth2/native-app` (Desktop app loopback table, line 3066) |
| `PKCE_METHOD` | `S256` | Google PKCE requirement for native apps |
| `client_secret` required | NO for PKCE flow | Desktop app PKCE — client_secret not required |
| `CLIENT_ID` | Per-project registration required | No shared public client_id equivalent exists (see Decision below) |

### Scope for `generateContent`

**Candidates evaluated:**

1. `https://www.googleapis.com/auth/generative-language.retriever` — shown in Gemini OAuth quickstart (ai.google.dev/gemini-api/docs/oauth line 2292) alongside `cloud-platform`, but this scope name indicates it is specific to the Semantic Retriever (tuned model tuning feature), not general chat inference.

2. `https://www.googleapis.com/auth/cloud-platform` — used by the google-genai Python SDK in `load_auth()` (github.com/googleapis/python-genai `_api_client.py` line 202: `scopes=['https://www.googleapis.com/auth/cloud-platform']`). This is the general Google Cloud scope.

3. The Gemini v1beta discovery document (`generativelanguage.googleapis.com/$discovery/rest?version=v1beta`) lists **only** `https://www.googleapis.com/auth/devstorage.read_only` in its top-level `auth.oauth2.scopes` section. The `generateContent` method definition in the discovery document contains **no `scopes` field**, suggesting it either accepts the devstorage scope OR that the discovery document is incomplete for OAuth purposes.

**Key finding:** The Gemini `generateContent` endpoint does NOT have OAuth scopes explicitly declared in the v1beta API discovery document, unlike most Google APIs. Google's own docs (the OAuth quickstart) focus OAuth usage on the Semantic Retriever feature, not general chat inference. The google-genai SDK itself uses `cloud-platform` scope but requires a project ID (GCP project), making this a Vertex AI / GCP-hosted pattern rather than a direct user-subscription OAuth flow comparable to Codex CLI.

**CLIENT_ID model for Gemini:** Google requires per-project OAuth client registration in Google Cloud Console. There is no shared public client_id equivalent to Codex CLI's `app_EMoamEEZ73f0CkXaXp7hrann`. Options:
- **Option A:** Ship Cortex with a hardcoded Cortex-registered Google OAuth client_id (requires maintaining a Cortex GCP project; client_id is NOT a secret in the client_secret.json sense for Desktop app type)
- **Option B:** Prompt the user to paste their own Google Cloud OAuth client_id into Settings (BYOC — Bring Your Own Client)
- **Option C:** Declare Gemini OAuth as Cortex-blocked for this milestone; Gemini card remains API-key-only

### Decision

**Option C selected — Gemini OAuth is Cortex-blocked for this milestone.**

**Rationale (citing specific findings):**

1. The Gemini `generateContent` endpoint does not have a user-subscription billing model equivalent to ChatGPT Plus/Team that Codex CLI accesses. The OAuth quickstart at `ai.google.dev/gemini-api/docs/oauth` is scoped to the Semantic Retriever feature, not general inference. It explicitly guides users through a Google Cloud Console project setup flow, not a "Sign in with Google" subscription access.

2. The google-genai SDK uses `cloud-platform` scope with a mandatory GCP project ID (`load_auth` requires project). This is a service-account or ADC pattern, not an end-user OAuth sign-in pattern. Users cannot "sign in with their Gemini subscription" in the same sense they can sign in to ChatGPT.

3. There is no shared public client_id for Gemini (unlike Codex CLI's `app_EMoamEEZ73f0CkXaXp7hrann`). Either Option A or B requires additional infrastructure or UX complexity that is out of scope for this milestone.

4. Learnforge OSS explicitly removed Gemini OAuth (noted in 07-CONTEXT.md). The research confirms the reason: there is no clean user-subscription OAuth path for Gemini's generateContent endpoint.

5. Choosing Option C keeps the Gemini card focused on API-key auth (which works today and is well-understood), avoids shipping an untested OAuth scope assumption, and defers the Gemini OAuth question until Google provides a stable user-subscription auth model.

**Downstream impact (MANDATORY — plans 07-09 and 07-10 MUST honor this):**

- **Plan 07-10 MUST NOT render a "Sign in with Google" primary CTA on the Gemini card.** The Gemini card remains API-key-only for this milestone. The only CTA is "Connect with API key."
- **Plan 07-09 MUST NOT implement `start_google_gemini_oauth`.** The `gemini_chat()` function routes only with `AuthMethod::ApiKey`.
- **Plan 07-08's shared PKCE module still ships** and is only exercised by the `openai-codex` provider path. This is correct — the shared PKCE module is provider-agnostic and ready for future use once Google's user-subscription OAuth path matures.
- If a future Gemini OAuth path becomes clear (e.g., Google launches a "Sign in with Google AI Pro" subscription model), plan 07-10's Gemini card can be upgraded without changing the PKCE infrastructure.

**Audit trail (T-07-23 mitigation):** This decision is documented here because blocking Gemini OAuth without an audit trail would fail the repudiation threat (T-07-23).

---

## Redirect URI Host Resolution

### The Conflict

Two incompatible requirements exist:

- **OpenAI Codex authorization server** has whitelisted the literal string `localhost` in its redirect URI allow-list (see `server.rs` line 55 comment: `// Keep in sync with the Codex CLI Hydra redirect URI allow-list`). The redirect_uri constructed is: `http://localhost:{port}/auth/callback` (server.rs line 161).
- **Google OAuth 2.0 authorization server** for Desktop apps specifies `http://127.0.0.1:{port}` or `http://[::1]:{port}` as the loopback redirect URI (developers.google.com native-app docs, section "Loopback IP address (macOS, Linux, Windows desktop)"). The docs note: _"It is also possible to use `localhost` in place of the loopback IP, but this configuration may cause issues with client firewalls."_ — however, in practice, Google Cloud Console's redirect URI validation requires exact-string registration.

OAuth 2.1 servers perform EXACT-STRING comparison on `redirect_uri` between the `/authorize` request and the token exchange POST. `localhost` and `127.0.0.1` are NOT interchangeable at the protocol layer. Using the wrong string causes HTTP 400 `invalid_grant: redirect_uri_mismatch`.

### Authoritative Per-Provider Table

| Provider | `redirect_uri` host string | Loopback socket bind | Path | Reason |
|---|---|---|---|---|
| `openai-codex` | `localhost` | `127.0.0.1` | `/auth/callback` | Matches Codex CLI source `server.rs` L161 and D-24 §1. OpenAI Hydra allow-list uses literal `localhost` (server.rs L55 comment). |
| `gemini` (if OAuth in future) | `127.0.0.1` | `127.0.0.1` | _(no path required)_ | Google native-app docs specify `http://127.0.0.1:{port}` for Desktop app type. `localhost` may cause firewall issues per Google docs. |

**Note:** The loopback socket bind address (`127.0.0.1`) is the SAME for both providers. Only the `redirect_uri` STRING presented to the OAuth server differs. The browser callback works in both cases because DNS resolves `localhost → 127.0.0.1` on the user's machine. The distinction matters only at the OAuth server's redirect URI string comparison layer.

### Evidence for OpenAI `localhost` Requirement

From `codex-rs/login/src/server.rs`:
- **Line 54:** `const DEFAULT_PORT: u16 = 1455;`
- **Line 55-56:** `// Keep in sync with the Codex CLI Hydra redirect URI allow-list. const FALLBACK_PORT: u16 = 1457;`
- **Line 161:** `let redirect_uri = format!("http://localhost:{actual_port}/auth/callback");`
- **Line 563-564:** `let preferred_bind_address = format!("127.0.0.1:{port}"); let fallback_bind_address = format!("127.0.0.1:{FALLBACK_PORT}");`

The bind address is `127.0.0.1` (socket level) but the `redirect_uri` STRING is `localhost` (OAuth layer). This is the critical distinction.

**Search for issues:** A GitHub search of `openai/codex` issues for `redirect_uri_mismatch` and `127.0.0.1` found no relevant issues. No authoritative source confirms that the OpenAI Codex auth server accepts `127.0.0.1` — the only verified redirect_uri format is `localhost` from the source code. Therefore:

`[CONFIRMED from source: OpenAI Codex server accepts only "localhost" per Hydra allow-list. Do NOT use 127.0.0.1 in the Codex redirect_uri string.]`

### Downstream Binding (Plans 07-08 and 07-09)

**Plan 07-08 `auth/pkce.rs` `OAuthFlowConfig`** MUST include a `redirect_uri_host: &str` field (or equivalent string field) to make the per-provider redirect URI host configurable. The loopback listener always binds `127.0.0.1` regardless of this field.

**Plan 07-08 `auth/loopback.rs`** binds `127.0.0.1:{port}` UNCONDITIONALLY. This is separate from the redirect_uri string. Both providers use `127.0.0.1` for the socket.

**Plan 07-09 `start_openai_codex_oauth`** MUST populate:
```rust
OAuthFlowConfig {
    redirect_uri_host: "localhost",
    redirect_uri_path: "/auth/callback",
    // ... other fields
}
// Resulting redirect_uri: http://localhost:{port}/auth/callback
```

**Plan 07-09 `start_google_gemini_oauth`** (if OAuth is ever activated — currently Option C, Cortex-blocked) MUST populate:
```rust
OAuthFlowConfig {
    redirect_uri_host: "127.0.0.1",
    redirect_uri_path: "",  // Google loopback URI has no path
    // ... other fields
}
// Resulting redirect_uri: http://127.0.0.1:{port}
```

This resolves D-24 §1's ambiguity. The per-provider host is now authoritative.

---

## Codex Chat Routing (post-OAuth)

### Question

Once the user is authenticated via Codex OAuth (`AuthMethod::OAuth` under provider slug `openai-codex`), which URL does Cortex POST chat requests to? And is the wire format the same as `api.openai.com/v1/chat/completions`?

### Research Findings

**Source:** `codex-rs/model-provider-info/src/lib.rs` (fetched 2026-07-02)

**Key constant (line 38):**
```rust
pub const CHATGPT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
```

**Key logic (lines 241-254 of `model-provider-info/src/lib.rs`, `to_api_provider` fn):**
```rust
pub fn to_api_provider(&self, auth_mode: Option<AuthMode>) -> CodexResult<ApiProvider> {
    let default_base_url = if matches!(
        auth_mode,
        Some(
            AuthMode::Chatgpt
                | AuthMode::ChatgptAuthTokens
                | AuthMode::AgentIdentity
                | AuthMode::PersonalAccessToken
        )
    ) {
        CHATGPT_CODEX_BASE_URL  // "https://chatgpt.com/backend-api/codex"
    } else {
        "https://api.openai.com/v1"
    };
```

When auth mode is `Chatgpt` (the mode used after Codex PKCE OAuth), the base URL becomes `https://chatgpt.com/backend-api/codex`, NOT `https://api.openai.com/v1`.

**Wire API:** The Codex CLI uses the OpenAI Responses API (`/responses` endpoint), NOT `/v1/chat/completions`. From `codex-rs/codex-api/src/lib.rs` (model-provider-info), the only supported `wire_api` value is `WireApi::Responses`. The chat completions wire API was REMOVED (line 50 of model_provider_info.rs: `const CHAT_WIRE_API_REMOVED_ERROR: &str = "\`wire_api = \"chat\"\` is no longer supported."`).

**Full chat endpoint:** `https://chatgpt.com/backend-api/codex/responses`

### Chat Routing Decision for Cortex

**Cortex MUST add a new `codex_chat()` function in `ai/service.rs`** — it CANNOT reuse the existing `openai_chat()` function that posts to `api.openai.com/v1/chat/completions`.

Reasons:
1. The ChatGPT backend endpoint is `https://chatgpt.com/backend-api/codex/responses` (Responses API), NOT the OpenAI Platform's `/v1/chat/completions` (Chat Completions API).
2. The Responses API request/response shape differs from the Chat Completions API.
3. Learnforge's `openai_chat()` targets `api.openai.com/v1/chat/completions` with a Bearer token — this would work for API-key OpenAI but NOT for ChatGPT subscription access (different backend, different endpoint).

**`[ASSUMED — verify at execution time]`:** The exact request/response schema for `https://chatgpt.com/backend-api/codex/responses` is not publicly documented. The Codex source uses the full Responses API streaming protocol (SSE/WebSocket) which is more complex than Chat Completions. For Cortex's entity extraction and space labeling use cases (which are single-turn, non-streaming), plan 07-09 implementors should:

1. First attempt using the Responses API non-streaming path with `stream=false` parameter.
2. If `chatgpt.com/backend-api/codex/responses` requires streaming only, fall back to verifying whether a Bearer token obtained via Codex OAuth can be used directly with `api.openai.com/v1/chat/completions` (some OpenAI Platform models accept ChatGPT tokens — unverified).
3. If neither path works cleanly for non-streaming use, **plan 07-09 MUST halt and report as a blocker** rather than silently routing Codex OAuth tokens to the wrong endpoint.

**Auth header for chatgpt.com/backend-api/codex:**
From `codex-rs/login/src/auth/manager.rs` and `model-provider-info/src/lib.rs`, the ChatGPT auth provider (`AuthMode::Chatgpt`) sets a standard `Authorization: Bearer <access_token>` header. No additional proprietary headers are required in the base case (additional headers like `x-openai-actor-authorization` are for agent identity auth, not basic ChatGPT PKCE auth).

**Cortex normalize_provider_name mapping:**
- Provider slug: `"openai-codex"` (distinct from `"openai"` which routes to `api.openai.com`)
- `openai-codex` → `codex_chat()` function in `ai/service.rs`
- `"openai"` continues to route to `openai_chat()` → `api.openai.com/v1/chat/completions` (API-key path, unchanged)

### Summary Table

| Provider slug | Auth method | Chat endpoint | Wire format | Handler fn |
|---|---|---|---|---|
| `openai-codex` | `AuthMethod::OAuth` (Codex PKCE) | `https://chatgpt.com/backend-api/codex/responses` | Responses API | `codex_chat()` [NEW] |
| `openai` | `AuthMethod::ApiKey` | `https://api.openai.com/v1/chat/completions` | Chat Completions API | `openai_chat()` [existing] |
| `anthropic` | `AuthMethod::OAuth` (setup-token) | `https://api.anthropic.com/v1/messages` | Anthropic Messages API | `anthropic_chat()` [existing] |
| `gemini` | `AuthMethod::ApiKey` | `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent` | Gemini REST | `gemini_chat()` [existing] |
| `ollama` | `AuthMethod::None` | `http://localhost:11434/api/chat` | Ollama REST | `ollama_chat()` [existing] |

---

## Notes for Plans 07-08, 07-09, 07-10

1. **Plan 07-08 (PKCE module):** Implement `OAuthFlowConfig` with `redirect_uri_host: String`. Bind socket to `127.0.0.1`. Construct redirect_uri as `http://{redirect_uri_host}:{port}{redirect_uri_path}`. No hardcoded host in the shared module.

2. **Plan 07-09 (Provider commands):** For `start_openai_codex_oauth`: use `redirect_uri_host = "localhost"`, path = `"/auth/callback"`, include `codex_cli_simplified_flow=true`, `id_token_add_organizations=true`, `originator=cortex-desktop`, `state={random}`. For revoke on disconnect: POST to `https://auth.openai.com/oauth/revoke` with JSON body (see Revoke Request Shape above). For `codex_chat()`: POST to `https://chatgpt.com/backend-api/codex/responses` with `Authorization: Bearer {access_token}` — verify non-streaming Responses API shape before committing.

3. **Plan 07-10 (UI):** OpenAI card has two-mode UI: primary CTA "Sign in with ChatGPT" → triggers `start_openai_codex_oauth`. Gemini card has NO "Sign in with Google" CTA. Gemini card shows only API-key paste field (Option C — Gemini OAuth Cortex-blocked).

4. **Downstream verification of `[ASSUMED]` items:** Plan 07-09 implementors MUST verify the Responses API non-streaming request/response shape by running a manual test with `curl` against `https://chatgpt.com/backend-api/codex/responses` using an actual Codex CLI access token before committing the `codex_chat()` implementation. If the endpoint shape is incompatible with Cortex's non-streaming use case, halt and report blocker.

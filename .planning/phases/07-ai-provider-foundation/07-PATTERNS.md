# Phase 7: AI Provider Foundation - Pattern Map

**Mapped:** 2026-06-30
**Files analyzed:** 18 (new or modified)
**Analogs found:** 18 / 18

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src-tauri/src/ai/mod.rs` | module | re-exports | `learnforge/src-tauri/src/ai/mod.rs` | exact (port verbatim) |
| `src-tauri/src/ai/anthropic.rs` | service | request-response | `learnforge/src-tauri/src/ai/anthropic.rs` | exact (port verbatim) |
| `src-tauri/src/ai/openai.rs` | service | request-response | `learnforge/src-tauri/src/ai/openai.rs` | exact (port verbatim) |
| `src-tauri/src/ai/service.rs` | service | request-response | `learnforge/src-tauri/src/ai/service.rs` | exact (port verbatim) |
| `src-tauri/src/ai/retry.rs` | utility | request-response | `learnforge/src-tauri/src/ai/retry.rs` | exact (port verbatim) |
| `src-tauri/src/auth/mod.rs` | model | CRUD | `learnforge/src-tauri/src/auth/mod.rs` | exact (port verbatim) |
| `src-tauri/src/auth/oauth.rs` | service | request-response | `learnforge/src-tauri/src/auth/oauth.rs` | exact (port with delta) |
| `src-tauri/src/auth/commands.rs` | controller | request-response | `learnforge/src-tauri/src/auth/commands.rs` | exact (port with delta) |
| `src-tauri/src/commands/ai.rs` | controller | request-response | `src-tauri/src/commands/settings.rs` | role-match |
| `src-tauri/src/commands/mod.rs` | config | — | `src-tauri/src/commands/mod.rs` | exact (extend) |
| `src-tauri/src/lib.rs` | config | — | `src-tauri/src/lib.rs` | exact (extend) |
| `client/hooks/useTauri.ts` | hook | request-response | `client/hooks/useTauri.ts` | exact (extend) |
| `client/lib/stores.ts` | store | event-driven | `client/lib/stores.ts` | exact (extend) |
| `client/lib/types.ts` | model | — | `learnforge/src-tauri/src/auth/commands.rs` (ProviderAuthStatus) | role-match |
| `client/pages/SettingsPage.tsx` | component | CRUD | `client/pages/SettingsPage.tsx` lines 257-307 | exact (extend, replace AI tab body) |
| `client/pages/OnboardingPage.tsx` | component | event-driven | `client/pages/OnboardingPage.tsx` | exact (extend) |
| `client/components/layout/AppShell.tsx` | component | event-driven | `client/components/layout/AppShell.tsx` | exact (extend) |
| `client/components/ai/` (4 files) | component | request-response | `client/pages/SettingsPage.tsx` + `client/pages/OnboardingPage.tsx` | role-match |

---

## Pattern Assignments

---

### `src-tauri/src/ai/mod.rs` (module, re-exports)

**Analog:** `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/mod.rs`
**Action:** Port verbatim. No changes needed.

**Full file** (lines 1-8):
```rust
pub mod anthropic;
pub mod openai;
pub mod retry;
pub mod service;

pub use service::{ai_request, AIServiceRequest, AIServiceResponse, ServiceMessage};
pub use anthropic::anthropic_chat;
pub use openai::openai_chat;
```

**Cortex adaptation delta:** None. File is identical.

---

### `src-tauri/src/ai/anthropic.rs` (service, request-response)

**Analog:** `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/anthropic.rs`
**Action:** Port verbatim. Pure function `build_anthropic_request` + `anthropic_chat` + `map_anthropic_error` + 5 unit tests.

**Imports pattern** (lines 1-12):
```rust
use crate::ai::{AIServiceResponse, ServiceMessage};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
```

**Core pattern — header construction** (lines 17-63):
```rust
pub fn build_anthropic_request(
    token: &str,
    is_setup_token: bool,
    model: &str,
    max_tokens: u32,
    system: &str,
    messages: &[ServiceMessage],
) -> (String, HeaderMap, Value) {
    let url = "https://api.anthropic.com/v1/messages".to_string();

    let mut headers = HeaderMap::new();
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    if is_setup_token {
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token)).unwrap());
        headers.insert("anthropic-beta", HeaderValue::from_static("oauth-2025-04-20"));
    } else {
        headers.insert("x-api-key", HeaderValue::from_str(token).unwrap());
    }
    // ...system is top-level field, not a message role...
    let body = json!({
        "model": model, "max_tokens": max_tokens,
        "system": system, "messages": messages_json,
    });
    (url, headers, body)
}
```

**Error handling pattern** (lines 66-88):
```rust
fn map_anthropic_error(status: u16, body: &str) -> String {
    match status {
        401 => "Invalid Anthropic bearer token (check setup-token).".to_string(),
        403 => { /* body check for OAuth-not-allowed */ }
        429 => "Anthropic rate limit — try again shortly.".to_string(),
        500..=599 => "Anthropic provider unavailable. Try again later.".to_string(),
        _ => { /* extract error.message from JSON body, truncate to 200 chars */ }
    }
}
```

**Cortex adaptation delta:** None. File is identical in Cortex context — crate paths `crate::ai::*` resolve identically.

---

### `src-tauri/src/ai/openai.rs` (service, request-response)

**Analog:** `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/openai.rs`
**Action:** Port verbatim.

**Critical structural difference vs Anthropic** (lines 15-43):
```rust
// OpenAI: system is a MESSAGE at index 0 with role="system" — NOT a top-level field
let mut messages_json: Vec<Value> = Vec::with_capacity(messages.len() + 1);
messages_json.push(json!({"role": "system", "content": system}));  // first
for m in messages {
    messages_json.push(json!({"role": m.role, "content": m.content}));
}
let body = json!({
    "model": model, "max_tokens": max_tokens,
    "messages": messages_json,   // no top-level "system" field
});
```

**Auth pattern:** `Authorization: Bearer <token>` for both API key and OAuth (lines 23-27):
```rust
let headers = vec![
    ("Authorization".to_string(), format!("Bearer {}", token)),
    ("Content-Type".to_string(), "application/json".to_string()),
];
```

**Cortex adaptation delta:** None.

---

### `src-tauri/src/ai/service.rs` (service, request-response)

**Analog:** `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/service.rs`
**Action:** Port verbatim. Contains `ai_request()`, `normalize_provider_name()`, `gemini_chat()`, `ollama_chat()`.

**Core router pattern** (lines 42-111):
```rust
pub async fn ai_request(auth: &AuthState, request: AIServiceRequest) -> Result<AIServiceResponse, String> {
    let cred = auth.get_active_credential()?
        .ok_or("No AI provider configured. Go to Settings to connect one.")?;
    let base_provider = normalize_provider_name(&cred.provider);

    let credential: Option<String> = match cred.method {
        AuthMethod::ApiKey => cred.api_key.clone(),
        AuthMethod::OAuth  => cred.oauth_token.clone(),
        AuthMethod::None   => None,
    };

    match base_provider.as_str() {
        "anthropic" => {
            let is_setup_token = cred.method == AuthMethod::OAuth;
            anthropic_chat(token, is_setup_token, model, max_tokens, &request.system_prompt, &request.messages).await
        }
        "openai"   => openai_chat(token, model, max_tokens, &request.system_prompt, &request.messages).await,
        "gemini"   => gemini_chat(token, cred.method == AuthMethod::OAuth, model, max_tokens, _temp, &request.system_prompt, &request.messages).await,
        "ollama"   => { let base_url = cred.base_url.as_deref().unwrap_or("http://localhost:11434"); ollama_chat(base_url, model, &request.system_prompt, &request.messages).await }
        other      => Err(format!("Unknown AI provider: {}", other)),
    }
}
```

**Provider name normalization** (lines 219-227):
```rust
pub fn normalize_provider_name(name: &str) -> String {
    match name {
        "claude" | "anthropic" => "anthropic".to_string(),
        "chatgpt" | "openai" | "openai-codex" => "openai".to_string(),
        "gemini" | "google" => "gemini".to_string(),
        "ollama" => "ollama".to_string(),
        other => other.to_string(),
    }
}
```

**Cortex adaptation delta:** Remove learnforge comment referencing "zeroclaw FIX-05" (cosmetic). No code changes needed.

---

### `src-tauri/src/ai/retry.rs` (utility, request-response)

**Analog:** `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/retry.rs`
**Action:** Port verbatim. 4 tokio async tests included — all pass as-is.

**Core pattern** (lines 11-55):
```rust
pub async fn retry_with_backoff<T, F, Fut>(
    mut op: F,
    max_retries: u8,
    initial_delay: Duration,
) -> Result<T, String>
where F: FnMut() -> Fut, Fut: std::future::Future<Output = Result<T, String>> {
    let mut delay = initial_delay;
    let mut attempt: u8 = 0;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if attempt >= max_retries { return Err(e); }
                tokio::time::sleep(delay).await;
                delay = delay.saturating_mul(2);
                attempt += 1;
            }
        }
    }
}

pub async fn ai_request_with_retry(auth: &AuthState, req: AIServiceRequest, max_retries: u8) -> Result<AIServiceResponse, String> {
    retry_with_backoff(
        || { let req = req.clone(); async move { crate::ai::service::ai_request(auth, req).await } },
        max_retries,
        Duration::from_millis(2000),
    ).await
}
```

**Cortex adaptation delta:** None. Use `max_retries = 2` (matches learnforge convention per RESEARCH.md open question #3).

---

### `src-tauri/src/auth/mod.rs` (model, CRUD)

**Analog:** `/Users/gshah/work/apps/learnforge/src-tauri/src/auth/mod.rs`
**Action:** Port verbatim. Includes 15 unit tests; all use `tempfile::TempDir` pattern.

**Type definitions** (lines 10-42):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]   // CRITICAL: "api-key" not "api_key" on wire
pub enum AuthMethod { OAuth, ApiKey, None }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredential {
    pub provider: String, pub method: AuthMethod,
    pub api_key: Option<String>, pub oauth_token: Option<String>,
    pub display_name: Option<String>, pub model: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct CredentialStore {
    pub(crate) active_provider: Option<String>,
    pub(crate) credentials: HashMap<String, ProviderCredential>,
}

#[derive(Clone)]
pub struct AuthState {
    pub(crate) store_path: PathBuf,
    pub(crate) store: Arc<Mutex<CredentialStore>>,
}
```

**Persistence pattern** (lines 62-66):
```rust
pub(crate) fn persist(&self) -> Result<(), String> {
    let store = self.store.lock().map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(&*store).map_err(|e| e.to_string())?;
    std::fs::write(&self.store_path, data).map_err(|e| e.to_string())
}
```

**Auto-active-on-first-store pattern** (lines 86-91, 137-140):
```rust
// In store_api_key and store_oauth_token: first stored becomes active automatically
if store.active_provider.is_none() {
    store.active_provider = Some(provider.to_string());
}
```

**Fallback-active-on-remove pattern** (lines 165-173):
```rust
pub fn remove_credential(&self, provider: &str) -> Result<(), String> {
    let mut store = self.store.lock().map_err(|e| e.to_string())?;
    store.credentials.remove(provider);
    if store.active_provider.as_deref() == Some(provider) {
        store.active_provider = store.credentials.keys().next().cloned();
    }
    drop(store);
    self.persist()
}
```

**Test helper pattern** (lines 196-199 — replicate for Cortex tests):
```rust
fn temp_auth_state() -> (AuthState, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let state = AuthState::new(&dir.path().to_path_buf());
    (state, dir)
}
```

**Cortex adaptation delta:** None. All 15 tests port as-is. Import paths resolve identically in Cortex crate.

---

### `src-tauri/src/auth/oauth.rs` (service, request-response)

**Analog:** `/Users/gshah/work/apps/learnforge/src-tauri/src/auth/oauth.rs`
**Action:** Port with one key delta — change storage key from `"claude"` to `"anthropic"` in `save_setup_token`.

**OAuthFlowState pattern** (lines 20-75):
```rust
#[derive(Clone)]
pub struct OAuthFlowState {
    flows: Arc<Mutex<HashMap<String, FlowEntry>>>,
}
impl OAuthFlowState {
    pub fn new() -> Self { Self { flows: Arc::new(Mutex::new(HashMap::new())) } }
    fn start(&self, provider: &str) -> Result<(), String> { /* insert fresh FlowEntry */ }
    fn set_authenticated(&self, provider: &str) { /* completed=true, error=None */ }
    fn set_error(&self, provider: &str, message: &str) { /* completed=true, error=Some */ }
    fn status(&self, provider: &str) -> Result<(bool, bool, Option<String>), String> { /* read */ }
}
```

**map_oauth_error pattern** (lines 103-116) — use this for ALL provider error toasts:
```rust
pub fn map_oauth_error(err_str: &str) -> String {
    let lower = err_str.to_lowercase();
    if lower.contains("401") || lower.contains("unauthorized") || lower.contains("invalid token") || lower.contains("invalid bearer") {
        "Invalid bearer token. Please log in again.".to_string()
    } else if lower.contains("403") || lower.contains("forbidden") || lower.contains("permission") || lower.contains("scope") {
        "Token does not have the required permissions.".to_string()
    } else if lower.contains("timeout") || lower.contains("timed out") || lower.contains("connection refused") || lower.contains("network") || lower.contains("connect") {
        "Could not reach provider. Check your connection and try again.".to_string()
    } else {
        err_str.chars().take(200).collect()
    }
}
```

**save_setup_token command pattern** (lines 174-205):
```rust
#[tauri::command]
pub async fn save_setup_token(auth: State<'_, AuthState>, token: String) -> Result<OAuthStartResult, String> {
    let trimmed = token.trim().to_string();
    if !trimmed.starts_with("sk-ant-oat01-") {
        return Err("Invalid token format. Setup tokens start with sk-ant-oat01-...".to_string());
    }
    validate_anthropic_token(&trimmed).await?;
    auth.store_oauth_token("anthropic", &trimmed, Some("Claude (Subscription)"), Some("claude-haiku-4-5-20251001"))?;
    //               ^^ CORTEX DELTA: "anthropic" not "claude" — normalize at write time
    Ok(OAuthStartResult { started: true, provider: "anthropic".to_string() })
}
```

**validate_anthropic_token pattern** (lines 210-246):
```rust
async fn validate_anthropic_token(token: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let res = client.post("https://api.anthropic.com/v1/messages")
        .header("Authorization", format!("Bearer {}", token))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(r#"{"model":"claude-haiku-4-5-20251001","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#)
        .send().await.map_err(|e| format!("Network error: {}", e))?;

    let status = res.status().as_u16();
    // 200 or 400 = token is valid (400 = authenticated but request shape issue)
    if status == 200 || status == 400 { return Ok(()); }
    match status {
        401 => Err("Setup token is invalid or expired. Run `claude setup-token` again.".to_string()),
        403 if body.contains("OAuth authentication is currently not allowed") => Err("...account does not support setup tokens...".to_string()),
        _ => Err(format!("Anthropic API error ({}): {}", status, body)),
    }
}
```

**Cortex adaptation deltas:**
1. `save_setup_token`: change `store_oauth_token("claude", ...)` → `store_oauth_token("anthropic", ...)`
2. `save_setup_token` return: `provider: "anthropic"` not `"claude"`
3. Remove `start_openai_oauth` and `start_gemini_oauth` stubs (Phase 7 Cortex is API-key only for those two; `start_oauth_login` command not needed in Phase 7)
4. Port 8 unit tests verbatim — they test `OAuthFlowState` and `map_oauth_error`, no changes needed

---

### `src-tauri/src/auth/commands.rs` (controller, request-response)

**Analog:** `/Users/gshah/work/apps/learnforge/src-tauri/src/auth/commands.rs`
**Action:** Port with provider-list delta.

**ProviderAuthStatus type** (lines 6-14):
```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAuthStatus {
    pub provider: String,
    pub authenticated: bool,
    pub method: String,
    pub display_name: Option<String>,
    pub model: Option<String>,
    pub is_active: bool,
}
```

**LoginRequest type** (lines 25-32):
```rust
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub provider: String,
    pub method: String,   // "api-key" | "ollama"  — kebab-case REQUIRED
    pub credential: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}
```

**get_auth_status command** (lines 34-63):
```rust
#[tauri::command]
pub fn get_auth_status(auth: State<AuthState>) -> Result<Vec<ProviderAuthStatus>, String> {
    let providers = ["claude", "openai", "gemini", "ollama"];
    //               ^^ CORTEX DELTA: change to ["anthropic", "openai", "gemini", "ollama"]
    let active = auth.get_active_provider()?;
    let mut statuses = Vec::new();
    for provider in providers {
        let cred = auth.get_credential(provider)?;
        let (authenticated, method, display_name, model) = match &cred {
            Some(c) => (true, format!("{:?}", c.method).to_lowercase(), c.display_name.clone(), c.model.clone()),
            None => (false, "none".to_string(), None, None),
        };
        statuses.push(ProviderAuthStatus {
            provider: provider.to_string(), authenticated, method, display_name, model,
            is_active: active.as_deref() == Some(provider),
        });
    }
    Ok(statuses)
}
```

**login_provider command** (lines 67-90):
```rust
#[tauri::command]
pub fn login_provider(auth: State<AuthState>, request: LoginRequest) -> Result<ProviderAuthStatus, String> {
    match request.method.as_str() {
        "api-key" => {  // NOTE: "api-key" not "api_key" — kebab-case from AuthMethod serde
            let key = request.credential.ok_or("API key is required")?;
            auth.store_api_key(&request.provider, &key, request.model.as_deref())?;
        }
        "ollama" => {
            let base_url = request.base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
            auth.store_ollama_config(&base_url, request.model.as_deref())?;
        }
        other => return Err(format!("Unsupported auth method: {}", other)),
    }
    // returns ProviderAuthStatus with authenticated: true
}
```

**Additional commands to add in Cortex** (not in learnforge — call through to oauth.rs and auth::mod):
```rust
// Cortex-only: thin wrappers already covered by oauth.rs commands
#[tauri::command] pub fn set_active_provider(auth: State<AuthState>, provider: String) -> Result<(), String>
#[tauri::command] pub fn logout_provider(auth: State<AuthState>, provider: String) -> Result<(), String>
```

**Cortex adaptation deltas:**
1. `get_auth_status`: provider scan list `["claude", ...]` → `["anthropic", "openai", "gemini", "ollama"]`
2. Rename learnforge's `login_provider` to `connect_provider` if CONTEXT.md discretion name is preferred — or keep as `login_provider`; planner decides
3. Remove `detect_system_providers` or keep as stub (it reports already-configured; safe to include)

---

### `src-tauri/src/commands/ai.rs` (controller, request-response)

**Analog:** `src-tauri/src/commands/settings.rs` (Cortex IPC pattern)
**Action:** New file — thin IPC wrapper layer over `auth/commands.rs` + `ai/service.rs`. Follows settings.rs async command pattern.

**Imports pattern** (modeled on settings.rs lines 1-6):
```rust
use tauri::State;
use crate::auth::{AuthState, commands::{ProviderAuthStatus, LoginRequest}};
use crate::auth::oauth::{OAuthFlowState, OAuthStartResult, save_setup_token};
use crate::ai::retry::ai_request_with_retry;
use crate::ai::service::{AIServiceRequest, AIServiceResponse};
```

**IPC command skeleton** (follow settings.rs async + spawn_blocking pattern for sync auth calls):
```rust
// Sync reads (no I/O): use synchronous #[tauri::command]
#[tauri::command]
pub fn list_providers(auth: State<'_, AuthState>) -> Result<Vec<ProviderAuthStatus>, String> {
    crate::auth::commands::get_auth_status(auth)
}

// Mutations with HTTP (validation): use async — same as update_settings
#[tauri::command]
pub async fn connect_provider(
    auth: State<'_, AuthState>,
    flow: State<'_, OAuthFlowState>,
    request: LoginRequest,
) -> Result<ProviderAuthStatus, String> {
    // For Anthropic: route to save_setup_token (validates + stores OAuth)
    // For openai/gemini: validate with 1-token call then store_api_key
    // For ollama: test ping then store_ollama_config
    crate::auth::commands::login_provider(auth, request)
}

// chat(): wraps ai_request_with_retry — expose in Phase 7 for end-to-end UAT
#[tauri::command]
pub async fn chat(
    auth: State<'_, AuthState>,
    request: AIServiceRequest,
) -> Result<AIServiceResponse, String> {
    ai_request_with_retry(auth.inner(), request, 2).await
}
```

**Error handling pattern** (copy from settings.rs lines 62-79 — use `String` error type matching learnforge convention, not `AppError`):
```rust
// Note: auth/ and ai/ modules return Result<T, String> — not AppError.
// commands/ai.rs propagates String errors directly to the IPC layer.
// sonner on the frontend receives the String as the error message.
```

---

### `src-tauri/src/commands/mod.rs` (config, extend)

**Analog:** `src-tauri/src/commands/mod.rs` (current, lines 1-6)
**Action:** Add one line.

**Current state** (lines 1-6):
```rust
pub mod documents;
pub mod spaces;
pub mod folders;
pub mod analytics;
pub mod settings;
pub mod entities;
```

**Addition:**
```rust
pub mod ai;   // Phase 7: AI provider IPC commands
```

---

### `src-tauri/src/lib.rs` (config, extend)

**Analog:** `src-tauri/src/lib.rs` (current)
**Action:** Add `AuthState` + `OAuthFlowState` manage() calls BEFORE `app.manage(AppState {...})`. Add new commands to invoke_handler.

**manage() insertion pattern** (after line 37, before line 134 `app.manage(AppState {`):
```rust
// Phase 7: AI provider credential store
// MUST be registered before AppState and before any IPC command using State<AuthState>
let auth_state = crate::auth::AuthState::new(&app_data);
let oauth_flow_state = crate::auth::oauth::OAuthFlowState::new();
app.manage(auth_state);
app.manage(oauth_flow_state);
```

**invoke_handler additions** (after line 190 `commands::documents::read_document_text`):
```rust
// AI provider commands (Phase 7)
commands::ai::list_providers,
commands::ai::connect_provider,
commands::ai::disconnect_provider,
commands::ai::set_active_provider,
commands::ai::get_active_provider,
commands::ai::save_setup_token,
commands::ai::test_connection,
commands::ai::chat,
```

**Pitfall reminder:** `app.manage(auth_state)` BEFORE `app.manage(AppState {...})`. Wrong order causes Tauri runtime panic.

---

### `client/hooks/useTauri.ts` (hook, request-response)

**Analog:** `client/hooks/useTauri.ts` (current)
**Action:** Extend with 6 provider hooks. Follow exact existing pattern.

**queryKeys extension** (after line 71, before the closing `}`):
```typescript
providers: ["providers"] as const,
activeProvider: ["providers", "active"] as const,
```

**useProviders hook pattern** (copy structure of `useSettings` at lines 413-417):
```typescript
export function useProviders() {
  return useQuery({
    queryKey: queryKeys.providers,
    queryFn: () => tauriInvoke<ProviderAuthStatus[]>("list_providers", {}, () => mockProviders),
    staleTime: 30_000,  // credential status doesn't change without explicit user action
  });
}
```

**useConnectProvider mutation pattern** (copy structure of `useUpdateSettings` at lines 422-430, with dual invalidation):
```typescript
export function useConnectProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: ConnectProviderRequest) =>
      tauriInvoke<ProviderAuthStatus>("connect_provider", { request }, () => undefined as never),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.providers });
      queryClient.invalidateQueries({ queryKey: queryKeys.activeProvider });
    },
    onError: (err: Error) => {
      // err.message is the String from Rust — passed directly to sonner toast
      // Caller is responsible for toast.error(err.message)
    },
  });
}
```

**useSaveSetupToken mutation pattern** (Anthropic-specific — async with spinner state):
```typescript
export function useSaveSetupToken() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (token: string) =>
      tauriInvoke<{ started: boolean; provider: string }>("save_setup_token", { token }, () => undefined as never),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.providers });
    },
  });
}
```

**Additional hooks to add** (all follow same `useMutation` + `invalidateQueries` pattern):
- `useDisconnectProvider()` → `tauriInvoke("disconnect_provider", { provider })`
- `useSetActiveProvider()` → `tauriInvoke("set_active_provider", { provider })`
- `useTestConnection()` → `tauriInvoke("test_connection", { provider })`

**Mock fallback pattern:** All new hooks need a mock fallback for browser dev mode (third arg to `tauriInvoke`). Use `mockProviders` imported from `mock-data.ts`.

---

### `client/lib/stores.ts` (store, event-driven)

**Analog:** `client/lib/stores.ts` lines 22-26 (`useSidebarStore` — session-only, no persist)
**Action:** Extend with one new store at end of file. NO `persist` middleware.

**Pattern to copy** (useSidebarStore, lines 22-26 — session-only, no persist):
```typescript
export const useSidebarStore = create<SidebarState>((set) => ({
  isCollapsed: false,
  toggle: () => set((s) => ({ isCollapsed: !s.isCollapsed })),
  setCollapsed: (collapsed: boolean) => set({ isCollapsed: collapsed }),
}));
```

**New store to add:**
```typescript
// --- AI Banner Store (session-only, no persist) --------------------------------
// Banner dismissed state resets on each app launch — returns until provider connected.

interface AiBannerState {
  isDismissed: boolean;
  dismiss: () => void;
}

export const useAiBannerStore = create<AiBannerState>((set) => ({
  isDismissed: false,
  dismiss: () => set({ isDismissed: true }),
}));
```

**Contrast with persisted pattern** (useOnboardingStore, lines 111-122 — DO NOT use this for banner):
```typescript
// DO NOT copy this pattern for AiBannerStore — persist() is intentionally absent
export const useOnboardingStore = create<OnboardingState>()(
  persist((set) => ({ ... }), { name: "cortex-onboarding" }),
);
```

---

### `client/lib/types.ts` (model, types)

**Analog:** `learnforge/src-tauri/src/auth/commands.rs` `ProviderAuthStatus` struct
**Action:** Add TypeScript types mirroring the Rust serde camelCase output.

**Types to add** (derived from `ProviderAuthStatus` serde `rename_all = "camelCase"`):
```typescript
// Mirrors Rust ProviderAuthStatus (serde camelCase)
export interface ProviderAuthStatus {
  provider: string;         // "anthropic" | "openai" | "gemini" | "ollama"
  authenticated: boolean;
  method: string;           // "oauth" | "api-key" | "none"
  displayName: string | null;
  model: string | null;
  isActive: boolean;
}

// Request shape for connect_provider IPC
export interface ConnectProviderRequest {
  provider: string;
  method: string;           // "api-key" | "ollama"  — kebab-case matches Rust LoginRequest
  credential?: string;      // API key value
  model?: string;
  baseUrl?: string;         // Ollama base URL
}

// Response from save_setup_token
export interface OAuthStartResult {
  started: boolean;
  provider: string;
}

// Request shape for chat IPC
export interface AiChatRequest {
  systemPrompt: string;
  messages: { role: string; content: string }[];
  maxTokens?: number;
  temperature?: number;
}

export interface AiChatResponse {
  content: string;
  model: string;
  inputTokens: number | null;
  outputTokens: number | null;
}
```

---

### `client/pages/SettingsPage.tsx` (component, CRUD — extend)

**Analog:** `client/pages/SettingsPage.tsx` lines 257-307 (AI & Models tab)
**Action:** Keep existing `RadioGroup` embedding model section (lines 261-305). Add divider + `<AiProvidersSection />` below it.

**Current AI tab shell to preserve** (lines 257-307):
```tsx
<TabsContent value="ai">
  <div className="card p-6 space-y-8">
    <div>
      <h3 className="section-header text-text-primary mb-4">Embedding Model</h3>
      <RadioGroup value={local.embeddingModel} onValueChange={(val) => update({ embeddingModel: val })} className="space-y-3">
        {/* existing radio options */}
      </RadioGroup>
      {local.embeddingModel === "openai" && (
        // D-20 unification: replace this input with conditional text
        // "Using connected OpenAI key" if OpenAI provider connected
        // "Connect OpenAI in AI Providers below" link if not connected
      )}
    </div>

    {/* Phase 7 addition — divider + providers section */}
    <hr className="border-border-primary" />
    <AiProvidersSection />
  </div>
</TabsContent>
```

**RadioGroup pattern** (lines 263-288, already in codebase — reuse for active-provider radio per card):
```tsx
<RadioGroup
  value={local.embeddingModel}
  onValueChange={(val) => update({ embeddingModel: val })}
  className="space-y-3"
>
  <label className="flex items-start gap-3 p-4 rounded-lg border border-border-primary hover:border-accent-primary/50 transition-colors cursor-pointer">
    <RadioGroupItem value="local" id="model-local" className="mt-0.5" />
    <div>
      <p className="text-text-primary font-medium">Local (all-MiniLM-L6-v2)</p>
      <p className="text-text-tertiary text-xs mt-1">...</p>
    </div>
  </label>
</RadioGroup>
```

**Card collapsed/expanded toggle pattern** (use same expand chevron as existing sidebar collapse logic — `ChevronDown` / `ChevronUp` from Lucide):
```tsx
// Provider card collapsed row (D-18):
<div className="flex items-center gap-3 px-4 py-4 rounded-lg border border-border-primary bg-bg-secondary">
  <RadioGroupItem value={provider} disabled={!status.authenticated} />
  <span className="flex-1 text-sm font-medium text-text-primary">{providerName}</span>
  <Badge className={status.authenticated ? "bg-status-success/10 text-status-success" : "bg-bg-tertiary text-text-tertiary"}>
    {status.authenticated ? "Connected" : "Not connected"}
  </Badge>
  {status.model && <span className="text-xs text-text-tertiary">{status.model}</span>}
  <button onClick={toggleExpanded}><ChevronDown size={16} /></button>
</div>
```

---

### `client/pages/OnboardingPage.tsx` (component, event-driven — extend)

**Analog:** `client/pages/OnboardingPage.tsx` (current)
**Action:** Extend step machine from 4→5 steps. Insert ConnectAiStep as step index 1 (shifts existing steps up by 1).

**StepIndicator extension** (line 175 — change `total={4}` to `total={5}`):
```tsx
<StepIndicator current={step} total={5} />   {/* was total={4} */}
```

**Step index remapping:**
- Step 0: Welcome (unchanged)
- **Step 1: NEW — Connect AI (`<ConnectAiStep />`)**
- Step 2: Select Folders (was step 1 — change `step === 1` guard to `step === 2`)
- Step 3: Scanning Progress (was step 2)
- Step 4: Spaces Ready (was step 3)

**Skip behavior pattern** (copy from existing SkipForward button at line 362-369):
```tsx
// In ConnectAiStep — skip navigates to step 2 (Folders) and records skip in store
<button
  onClick={() => { setStep(2); useAiBannerStore.getState().dismiss(); /* no-op: banner shows next launch */ }}
  className="inline-flex items-center gap-1 text-sm text-text-tertiary hover:text-text-secondary transition-colors"
>
  <SkipForward size={14} />
  Skip for now
</button>
```

**Continue-after-connect button pattern** (enable only when provider connected, follows existing disabled button style at lines 302-315):
```tsx
<button
  onClick={() => setStep(2)}
  disabled={!hasConnectedProvider}
  className={cn(
    "inline-flex items-center gap-2 rounded-lg px-6 py-3 text-sm font-medium transition-colors",
    hasConnectedProvider
      ? "bg-accent-primary text-white hover:bg-accent-hover"
      : "bg-bg-tertiary text-text-tertiary cursor-not-allowed",
  )}
>
  Continue
  <ArrowRight size={16} />
</button>
```

---

### `client/components/layout/AppShell.tsx` (component, event-driven — extend)

**Analog:** `client/components/layout/AppShell.tsx` (current)
**Action:** Add `<AiNoProviderBanner />` mount inside the `return` JSX. Gate on `onboardingCompleted` (Pitfall 6 guard).

**Store import additions** (line 9 — add to existing stores import):
```typescript
import { useOnboardingStore, useSidebarStore, useCommandPaletteStore, useIndexingStore, useAiBannerStore } from "@/lib/stores";
```

**Hook additions** (after line 29 `const recluster = useReclusterSpaces()`):
```typescript
const { isDismissed: bannerDismissed } = useAiBannerStore();
const { data: providers } = useProviders();    // from useTauri.ts
const hasActiveProvider = providers?.some(p => p.isActive && p.authenticated) ?? false;
const showBanner = onboardingCompleted && !hasActiveProvider && !bannerDismissed;
```

**Banner mount pattern** (inside return, between `<CommandPalette />` and `<Sidebar />` — line 182):
```tsx
{/* AI provider nudge banner — session-only, only after onboarding (Pitfall 6 guard) */}
{showBanner && <AiNoProviderBanner />}
```

---

### `client/components/ai/ProviderCard.tsx` (component, CRUD)

**Analog:** `client/pages/SettingsPage.tsx` AI tab RadioGroup pattern + `client/pages/OnboardingPage.tsx` folder button pattern
**Action:** New component. No direct analog for the expand/collapse card UX, but all primitives exist.

**Imports pattern** (follow SettingsPage.tsx import style):
```typescript
import { useState } from "react";
import { ChevronDown, ChevronUp, Check, Loader2 } from "lucide-react";
import { RadioGroupItem } from "@/components/ui/radio-group";
import { cn } from "@/lib/utils";
import { toast } from "sonner";
import type { ProviderAuthStatus } from "@/lib/types";
import { useConnectProvider, useDisconnectProvider, useSaveSetupToken, useSetActiveProvider } from "@/hooks/useTauri";
```

**Collapsed row pattern** (from D-18, using SettingsPage RadioGroup style):
```tsx
function CollapsedRow({ status, onToggle }: { status: ProviderAuthStatus; onToggle: () => void }) {
  return (
    <div className="flex items-center gap-3 px-4 py-4 rounded-lg border border-border-primary bg-bg-secondary cursor-pointer" onClick={onToggle}>
      <RadioGroupItem value={status.provider} disabled={!status.authenticated} className="pointer-events-none" />
      <span className="text-sm font-semibold text-text-primary flex-1">{PROVIDER_LABELS[status.provider]}</span>
      <span className={cn("text-sm font-semibold", status.authenticated ? "text-status-success" : "text-text-tertiary")}>
        {status.authenticated ? "Connected" : "Not connected"}
      </span>
      {status.model && <span className="text-xs text-text-tertiary font-mono">{status.model}</span>}
      {status.isActive && <span className="text-xs font-semibold text-accent-primary">Active</span>}
      <ChevronDown size={16} className="text-text-tertiary" />
    </div>
  );
}
```

**Error toast pattern** (from D-21 — use sonner toast.error with Rust error string):
```tsx
const connect = useConnectProvider();
const handleConnect = async () => {
  try {
    await connect.mutateAsync({ provider, method: "api-key", credential: apiKey, model: selectedModel });
    toast.success(`${PROVIDER_LABELS[provider]} connected`);
  } catch (err) {
    toast.error((err as Error).message);  // Rust String propagated directly
  }
};
```

**Inline spinner pattern** (D-10 — follows existing button-disabled pattern from OnboardingPage.tsx):
```tsx
<button onClick={handleConnect} disabled={connect.isPending} className="...">
  {connect.isPending ? <><Loader2 size={14} className="animate-spin" /> Validating…</> : "Save"}
</button>
```

---

### `client/components/ai/AiProvidersSection.tsx` (component, CRUD)

**Analog:** Settings page tab content structure
**Action:** New component — 4 stacked `<ProviderCard>` instances inside `<RadioGroup>`.

**RadioGroup wrapper pattern** (from SettingsPage.tsx lines 263-288 — reuse for active-provider radio):
```tsx
import { RadioGroup } from "@/components/ui/radio-group";
import { useProviders, useSetActiveProvider } from "@/hooks/useTauri";

export function AiProvidersSection() {
  const { data: providers } = useProviders();
  const setActive = useSetActiveProvider();

  return (
    <div className="space-y-6">
      <h3 className="section-header text-text-primary">AI Providers</h3>
      <RadioGroup
        value={providers?.find(p => p.isActive)?.provider ?? ""}
        onValueChange={(provider) => setActive.mutate(provider)}
        className="space-y-3"   {/* lg gap = 24px = gap-6 */}
      >
        {PROVIDERS.map(id => (
          <ProviderCard key={id} provider={id} status={providers?.find(p => p.provider === id)} />
        ))}
      </RadioGroup>
    </div>
  );
}
```

---

### `client/components/ai/ConnectAiStep.tsx` (component, CRUD)

**Analog:** `client/pages/OnboardingPage.tsx` folder selection step (step 1, lines 208-318)
**Action:** New component — 2x2 grid. Follow onboarding folder-card button pattern.

**2x2 grid pattern** (follow spaces grid from OnboardingPage.tsx step 4, lines 389-419):
```tsx
<div className="grid grid-cols-2 gap-4">  {/* D-13: gap-4 = 16px */}
  {PROVIDERS.map(id => (
    <ProviderOnboardCard key={id} provider={id} status={providers?.find(p => p.provider === id)} />
  ))}
</div>
```

**Folder-card button pattern reuse** (lines 222-244 — adapt border-active style):
```tsx
<button
  className={cn(
    "flex flex-col items-center gap-3 rounded-lg border p-4 text-center transition-all",
    isConnected
      ? "border-accent-primary bg-accent-primary/5 text-text-primary"
      : "border-border-primary bg-bg-secondary text-text-secondary hover:border-border-secondary",
  )}
>
  {/* Provider logo placeholder + name + status */}
</button>
```

---

### `client/components/ai/AiNoProviderBanner.tsx` (component, event-driven)

**Analog:** No exact Cortex analog for dismissible banner UI — closest structural analog is onboarding step buttons + toast pattern.
**Action:** New component. Session-only dismiss via `useAiBannerStore`. Navigate to Settings → AI on click.

**Store consumption pattern** (from AppShell pattern, useSidebarStore):
```tsx
import { useAiBannerStore } from "@/lib/stores";
import { useNavigate } from "react-router-dom";
import { X, Zap } from "lucide-react";

export function AiNoProviderBanner() {
  const { dismiss } = useAiBannerStore();
  const navigate = useNavigate();

  return (
    <div className="flex items-center gap-3 bg-accent-primary/10 border-b border-accent-primary/20 px-4 py-4 text-sm">
      <Zap size={14} className="text-accent-primary flex-shrink-0" />
      <span className="flex-1 text-text-primary">
        Connect an AI provider to enable Smart Spaces.{" "}
        <button
          onClick={() => navigate("/settings?tab=ai")}
          className="underline text-accent-primary hover:text-accent-hover"
        >
          Go to Settings
        </button>
      </span>
      <button onClick={dismiss} className="text-text-tertiary hover:text-text-primary transition-colors">
        <X size={14} />
      </button>
    </div>
  );
}
```

---

## Shared Patterns

### Authentication State Access (Rust)
**Source:** `learnforge/src-tauri/src/auth/mod.rs` + `src-tauri/src/lib.rs` manage() block
**Apply to:** All Rust IPC commands that touch credentials (`commands/ai.rs`, `auth/commands.rs`, `auth/oauth.rs`)

```rust
// In IPC command signatures — State<'_, AuthState> (not inside AppState):
pub fn some_command(auth: State<'_, AuthState>) -> Result<..., String>
pub async fn some_async_command(auth: State<'_, AuthState>, flow: State<'_, OAuthFlowState>) -> Result<..., String>

// In lib.rs BEFORE app.manage(AppState {...}):
let auth_state = crate::auth::AuthState::new(&app_data);
let oauth_flow_state = crate::auth::oauth::OAuthFlowState::new();
app.manage(auth_state);
app.manage(oauth_flow_state);
```

### Error Handling (Rust → Frontend)
**Source:** `learnforge/src-tauri/src/auth/oauth.rs` `map_oauth_error()` + `anthropic.rs` `map_anthropic_error()` + `openai.rs` `map_openai_error()`
**Apply to:** All provider connect/validate error paths. Error String propagates via IPC directly to sonner toast.

```rust
// Pattern: validate → map error → return Err(human_readable_string)
// The String is what the frontend receives as (err as Error).message in onError
Err(map_oauth_error(&err.to_string()))
```

```typescript
// Frontend: useMutation onError → toast.error
onError: (err: Error) => toast.error(err.message)
```

### React Query Mutation + Invalidation
**Source:** `client/hooks/useTauri.ts` `useUpdateSettings` (lines 422-430) and `useAddWatchedFolder` (lines 336-354)
**Apply to:** All provider mutation hooks (`useConnectProvider`, `useDisconnectProvider`, `useSetActiveProvider`, `useSaveSetupToken`)

```typescript
return useMutation({
  mutationFn: (args) => tauriInvoke<ReturnType>("command_name", { args }, () => mockFallback),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: queryKeys.providers });
    queryClient.invalidateQueries({ queryKey: queryKeys.activeProvider });
  },
});
```

### tauriInvoke Signature (Mock Fallback)
**Source:** `client/hooks/useTauri.ts` line 81 `useSpaces()` pattern
**Apply to:** All new query hooks (mutations for provider calls may use `() => undefined as never` for mock fallback)

```typescript
// Third argument = mock fallback fn for browser dev mode
tauriInvoke<T>("command_name", { args }, () => mockValue)
```

### Credential Validation Strategy
**Source:** `learnforge/src-tauri/src/auth/oauth.rs` `validate_anthropic_token()` — accept 200 OR 400, reject 401/403
**Apply to:** Per-provider validation functions inside `connect_provider` IPC handler

- Accept HTTP 200 or 400 → credential is valid (400 = auth OK but request shape issue)
- Reject HTTP 401 → bad credential
- Reject HTTP 403 → insufficient permission or OAuth not enabled
- Network error → map via `map_oauth_error()`

### `cn()` + Tailwind Token Usage
**Source:** Throughout `client/pages/OnboardingPage.tsx` and `client/pages/SettingsPage.tsx`
**Apply to:** All new React components in `client/components/ai/`

```typescript
import { cn } from "@/lib/utils";
// Use Tailwind semantic tokens, never hardcoded colors:
// text-text-primary, text-text-secondary, text-text-tertiary
// bg-bg-primary, bg-bg-secondary, bg-bg-tertiary
// border-border-primary, border-border-secondary
// text-accent-primary, bg-accent-primary, hover:bg-accent-hover
// text-status-success, text-status-error
```

---

## No Analog Found

All files have analogs. No entries in this table.

---

## Adaptation Delta Summary

| File | Delta from Analog |
|------|-------------------|
| `auth/oauth.rs` | Change `store_oauth_token("claude", ...)` → `("anthropic", ...)`; remove `start_openai_oauth`/`start_gemini_oauth` stubs |
| `auth/commands.rs` | Provider scan list `["claude",...]` → `["anthropic",...]` |
| `commands/ai.rs` | New file — thin IPC wrappers (no learnforge equivalent; follows Cortex `settings.rs` pattern) |
| `commands/mod.rs` | Add `pub mod ai;` |
| `lib.rs` | Add `manage(auth_state)` + `manage(oauth_flow_state)` before `manage(AppState {})` + 8 new commands in invoke_handler |
| `client/hooks/useTauri.ts` | Add 6 hooks + 2 queryKeys entries |
| `client/lib/stores.ts` | Add `useAiBannerStore` (no persist middleware) |
| `client/lib/types.ts` | Add 6 interfaces: `ProviderAuthStatus`, `ConnectProviderRequest`, `OAuthStartResult`, `AiChatRequest`, `AiChatResponse` |
| `client/pages/SettingsPage.tsx` | AI tab: replace OpenAI API key input with D-20 unification logic; add divider + `<AiProvidersSection />` |
| `client/pages/OnboardingPage.tsx` | `total={4}` → `total={5}`; add step 1 case; shift existing step indices +1 |
| `client/components/layout/AppShell.tsx` | Add banner imports + `showBanner` logic + `<AiNoProviderBanner />` mount |

---

## Metadata

**Analog search scope:** `/Users/gshah/work/apps/learnforge/src-tauri/src/{ai,auth}/`, `src-tauri/src/`, `client/`
**Files read:** 18 (all listed in required_reading block)
**Pattern extraction date:** 2026-06-30

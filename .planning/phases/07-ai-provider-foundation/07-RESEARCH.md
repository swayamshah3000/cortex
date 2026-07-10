# Phase 7: AI Provider Foundation - Research

**Researched:** 2026-06-30
**Domain:** Tauri 2 Rust AI provider routing, credential persistence, React 19 provider settings UI
**Confidence:** HIGH — all critical modules read verbatim from the learnforge port source; no guesswork required.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Port `/Users/gshah/work/apps/learnforge/src-tauri/src/{ai,auth}/` as foundation. Battle-tested `AuthState`, `CredentialStore`, `AuthMethod::{OAuth, ApiKey, None}`, `ProviderCredential`, `ai_request()`, `normalize_provider_name()`, per-provider chat fns.
- **D-02:** Plaintext `credentials.json` at `app_data_dir/credentials.json`. Schema: `{ active_provider, credentials: { providerId → ProviderCredential } }`. Keychain is v1.2.
- **D-03:** Direct `reqwest` per provider — no `anthropic-sdk` / `openai-sdk` crates.
- **D-04:** Anthropic: setup-token only (`sk-ant-oat01-*` from `claude setup-token`). Stored as `AuthMethod::OAuth`; router sends `Bearer` + `anthropic-beta: oauth-2025-04-20`.
- **D-05:** Inline instructions on Anthropic card (code block, copy button, paste field, CLI install link). No two-step wizard, no `~/.claude/.credentials` auto-detect.
- **D-06:** Default model on Anthropic save = `claude-haiku-4-5-20251001`.
- **D-07:** Model change via inline dropdown on each provider card. Applies immediately.
- **D-08:** Validate every credential on Save with a real API call. Failed validation rejects save with provider error.
- **D-09:** Validation call shape = minimal 1-token chat per provider.
- **D-10:** Save UX = inline button spinner (Save → "Validating…" → "Saved" green flash or error toast). No full-card overlay.
- **D-11:** No explicit timeout override — rely on `reqwest` default (~30s).
- **D-12:** Onboarding: "Connect AI" inserted as Step 2. New flow: Welcome → Connect AI → Folders → Scanning → Done. Total steps: 5.
- **D-13:** Onboarding Step 2 = 2x2 grid of 4 provider cards. Continue button enables after any provider connects.
- **D-14:** Skip allowed in onboarding. App shell shows dismissible banner "Connect an AI provider to enable Smart Spaces →" linking to Settings → AI.
- **D-15:** Banner is session-only dismissible (X hides for session; returns on next launch until provider connected).
- **D-16:** Settings → AI tab: stacked provider cards (one per row, 4 total). Pattern consistent with Watched Folders.
- **D-17:** Active-provider selector = radio button on each card. Only connected providers' radios selectable.
- **D-18:** Card states = collapsed by default, expandable. Connected = compact row; Not-connected = `[Connect]` button.
- **D-19:** Keep existing "Embedding Model" section at top of AI tab. Divider. Then "AI Providers" section below.
- **D-20:** Unify cloud credentials — OpenAI as AI provider and OpenAI for embeddings share the single Phase 7 credential.
- **D-21:** Runtime provider failures surface as global sonner toast + inline red status on the provider card.

### Claude's Discretion

- Exact IPC command names: `list_providers`, `connect_provider`, `disconnect_provider`, `set_active_provider`, `get_active_provider`, `save_setup_token`, `test_connection`, `chat`.
- Whether to expose a `chat()` IPC in Phase 7 or wait for Phase 8.
- Exact hardcoded model lists per provider.
- Retry/backoff adoption from learnforge `ai/retry.rs`.
- Backend module layout in `src-tauri/src/` — likely `ai/` + `auth/` mirroring learnforge.

### Deferred Ideas (OUT OF SCOPE)

- macOS Keychain credential storage (v1.2)
- OAuth handshakes for OpenAI and Gemini (API-key only for v1.1)
- Streaming responses
- Cost/token-usage tracking UI
- Per-feature provider override
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AIPV-01 | User can connect Anthropic via OAuth setup-token OR API key | Verbatim port: `save_setup_token`, `validate_anthropic_token`, `anthropic_chat` in learnforge. D-04 locks to setup-token only for v1.1. |
| AIPV-02 | User can connect OpenAI via API key; shown in Settings → AI tab | Verbatim port: `login_provider(method="api-key")` + `openai_chat` |
| AIPV-03 | User can connect Google Gemini via API key; shown in Settings → AI tab | Verbatim port: `gemini_chat` in `service.rs` |
| AIPV-04 | User can configure Ollama with base URL + model selection | Verbatim port: `store_ollama_config` + `ollama_chat`; default URL pre-populated |
| AIPV-05 | User can pick and switch active AI provider at any time | `set_active_provider` IPC + radio button on each card |
| AIPV-06 | First-run onboarding adds "Connect AI" step; user can skip | Onboarding step machine extended to 5 steps; `useAiBannerStore` Zustand store for banner |
| AIPV-07 | Credentials persist across restarts; removed by "Disconnect" | `AuthState.persist()` writes to `credentials.json`; `remove_credential` with fallback |
| AIPV-08 | Provider failures surface human-readable error toasts | `map_oauth_error` from learnforge covers 401/403/timeout patterns; per-provider `map_*_error` fns |
</phase_requirements>

---

## Summary

Phase 7 is a **structured port**, not a design problem. The core Rust modules (`ai/` and `auth/`) exist verbatim in learnforge and have been battle-tested with comprehensive unit tests (14+ tests in `auth/mod.rs`, 12+ tests in `anthropic.rs`, 12+ tests in `oauth.rs`, 4+ tests in `retry.rs`). The port is almost mechanical: copy the modules, update import paths and `AppState` wiring, register IPC commands, then extend the frontend.

The primary design work is in the UI: the 4-card provider settings section, the onboarding step insertion, and the session-only AI banner Zustand store. All frontend patterns already exist in the codebase (RadioGroup, shadcn card classes, Zustand, React Query hooks), so UI implementation follows established conventions.

Three integration seams require care: (1) `AuthState` and `OAuthFlowState` must be added to `lib.rs` `manage()` call before any IPC commands using them are registered; (2) `commands/mod.rs` needs a new `pub mod ai;` declaration; (3) the OpenAI embedding credential unification (D-20) requires the embedder to read from `AuthState` — but the current embedder only does local ONNX and has a comment stub for the OpenAI path, so unification is a stub-level update, not structural surgery.

**Primary recommendation:** Port `auth/mod.rs` → `auth/commands.rs` → `auth/oauth.rs` as Wave 1 (the pure data/logic layer with all its tests). Then port `ai/` modules as Wave 2. Wire IPC as Wave 3. Then build UI in Wave 4 (Settings AI tab) and Wave 5 (Onboarding step + Banner). This ordering lets tests prove the backend before any UI is touched.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Credential storage & retrieval | API / Backend (Rust `auth/`) | — | Credentials must never be accessible from renderer process. `AuthState` is `manage()`d and only exposed via typed IPC commands. |
| Provider routing (`ai_request`) | API / Backend (Rust `ai/`) | — | HTTP calls to provider APIs happen in the Rust process; the renderer only sends a request struct and receives a response string. |
| Token validation on save | API / Backend (Rust) | — | Validation is a real HTTP call; belongs in Rust where reqwest lives. |
| Active provider selection | API / Backend (Rust) | Frontend (radio button UX) | Backend owns `active_provider` field in `CredentialStore`. Frontend reads state via `list_providers` query and mutates via `set_active_provider`. |
| Settings UI — AI Providers section | Frontend Server (React) | — | Purely view layer rendering IPC data. |
| Onboarding "Connect AI" step | Frontend Server (React) | — | Extends existing step machine in `OnboardingPage.tsx`. |
| Session-only AI banner | Browser / Client (Zustand) | — | Banner dismissed state is ephemeral (session, not persisted). No backend involvement. |
| Credential persistence to disk | API / Backend (Rust) | — | `AuthState.persist()` writes `credentials.json` in `app_data_dir`. |

---

## Standard Stack

### Core (Rust backend)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `reqwest` | 0.12 (learnforge) / 0.13.4 (crates.io latest) | HTTP client for all provider API calls | Learnforge uses 0.12; crates.io latest is 0.13.4 `[VERIFIED: crates.io]` — use 0.12 to match learnforge and avoid migration risk |
| `serde` + `serde_json` | 1.x | Credential JSON serialization | Already in `Cargo.toml` `[VERIFIED: codebase]` |
| `tokio` | 1.x (full) | Async runtime for reqwest | Already in `Cargo.toml` `[VERIFIED: codebase]` |
| `tempfile` | 3.27.0 | Unit tests for `AuthState` (TempDir) | Already in `[dev-dependencies]` `[VERIFIED: codebase + crates.io]` |

### Core (React frontend)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `sonner` | 1.7.4 | Global error toasts (D-21) | Already in deps, used throughout app `[VERIFIED: codebase]` |
| `zustand` | (via create) | `useAiBannerStore` session-only dismissed state | Already in deps, established pattern `[VERIFIED: codebase]` |
| `@tanstack/react-query` | 5.84.2 | Provider list, active provider, connect/disconnect hooks | Already in deps, established pattern `[VERIFIED: codebase]` |
| `@radix-ui/react-radio-group` | 1.3.7 | Active-provider radio per card | Already in deps, used in Settings AI tab `[VERIFIED: codebase]` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| reqwest 0.12 | reqwest 0.13.4 | 0.13 changes some API signatures; learnforge-tested 0.12 code compiles as-is. Use 0.12 unless blocking issue arises. |
| Plaintext JSON | macOS Keychain (`security-framework` crate) | Keychain is v1.2; REQUIREMENTS.md OOS is explicit. |
| Direct HTTP per provider | `async-openai` / `anthropic-rs` crates | SDK crates add transitive deps, API churn risk. Direct HTTP is 100 lines per provider and fully tested. D-03 locks this. |

**Installation:**
```bash
# Cargo.toml addition
reqwest = { version = "0.12", features = ["json"] }
```
`serde`, `serde_json`, `tokio`, `tempfile` are already declared.

---

## Package Legitimacy Audit

> slopcheck ran against npm (wrong ecosystem for Rust). All packages below are verified against crates.io via `cargo search`.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `reqwest` | crates.io | 8+ yrs | Many millions | github.com/seanmonstar/reqwest | N/A (npm check irrelevant) `[VERIFIED: crates.io]` | Approved — canonical Rust HTTP client |
| `tempfile` | crates.io | 8+ yrs | Many millions | github.com/Stebalien/tempfile | N/A (npm check irrelevant) `[VERIFIED: crates.io]` | Approved — already in dev-dependencies |
| `serde` | crates.io | 10+ yrs | ~250M/month | github.com/serde-rs/serde | N/A | Approved — already in Cargo.toml |
| `serde_json` | crates.io | 10+ yrs | ~200M/month | github.com/serde-rs/json | N/A | Approved — already in Cargo.toml |
| `tokio` | crates.io | 6+ yrs | ~100M/month | github.com/tokio-rs/tokio | N/A | Approved — already in Cargo.toml |

**Packages removed due to slopcheck [SLOP] verdict:** none (slopcheck ran against wrong registry — npm — for a Rust project; all packages above are canonical Rust crates verified via `cargo search`)
**Packages flagged as suspicious [SUS]:** none

*Note: slopcheck is npm-focused and cannot evaluate crates.io packages. All Rust dependencies for Phase 7 are either already in `Cargo.toml` or are `reqwest 0.12` which is the canonical Rust HTTP library with 8+ year history.*

---

## Architecture Patterns

### System Architecture Diagram

```
[Frontend React]
  OnboardingPage (step 2: ConnectAiStep)
  SettingsPage (AI tab: EmbeddingModelSection + AiProvidersSection)
  AppShell (AiNoProviderBanner)
  useTauri.ts (useProviders, useConnectProvider, etc.)
        │ Tauri IPC invoke
        ▼
[Rust Backend — src-tauri/src/]
  commands/ai.rs
    ├── list_providers()      → reads AuthState → returns Vec<ProviderAuthStatus>
    ├── connect_provider()    → routes to save_setup_token OR login_provider
    ├── disconnect_provider() → calls AuthState.remove_credential()
    ├── set_active_provider() → calls AuthState.set_active_provider()
    ├── get_active_provider() → reads AuthState.active_provider
    ├── test_connection()     → calls validate_* per provider
    └── chat()                → calls ai_request() → per-provider chat fn
        │
  auth/mod.rs (AuthState / CredentialStore / ProviderCredential)
        │  persist() on every mutation
        ▼
  app_data_dir/credentials.json
  (schema: { active_provider: str?, credentials: { id → ProviderCredential } })
        │
  ai/service.rs (ai_request + normalize_provider_name)
        ├── anthropic_chat() → POST api.anthropic.com/v1/messages
        ├── openai_chat()    → POST api.openai.com/v1/chat/completions
        ├── gemini_chat()    → POST generativelanguage.googleapis.com/...
        └── ollama_chat()    → POST http://localhost:11434/api/chat
        │
  ai/retry.rs (retry_with_backoff, ai_request_with_retry)
```

### Recommended Project Structure

```
src-tauri/src/
├── ai/
│   ├── mod.rs          # pub use re-exports (ported verbatim from learnforge)
│   ├── anthropic.rs    # build_anthropic_request + anthropic_chat (ported verbatim)
│   ├── openai.rs       # build_openai_request + openai_chat (ported verbatim)
│   ├── service.rs      # ai_request + normalize_provider_name + gemini_chat + ollama_chat
│   └── retry.rs        # retry_with_backoff + ai_request_with_retry (ported verbatim)
├── auth/
│   ├── mod.rs          # AuthState, CredentialStore, AuthMethod, ProviderCredential (ported verbatim)
│   ├── oauth.rs        # save_setup_token, validate_anthropic_token, map_oauth_error, OAuthFlowState
│   └── commands.rs     # get_auth_status, login_provider, set_active_provider, logout_provider
├── commands/
│   ├── mod.rs          # add: pub mod ai;
│   └── ai.rs           # thin IPC wrappers calling auth/ + ai/ modules
└── lib.rs              # add AuthState + OAuthFlowState to manage(); register new commands

client/
├── hooks/useTauri.ts   # add: useProviders, useConnectProvider, useDisconnectProvider,
│                       #      useSetActiveProvider, useSaveSetupToken, useActiveProvider
├── lib/stores.ts       # add: useAiBannerStore (session-only, no persist middleware)
├── lib/types.ts        # add: ProviderAuthStatus, ProviderConnection TypeScript types
├── pages/
│   ├── SettingsPage.tsx  # AI tab: add AiProvidersSection below existing EmbeddingModel
│   └── OnboardingPage.tsx # extend step machine from 4→5 steps; add ConnectAiStep
└── components/
    ├── layout/AppShell.tsx  # mount <AiNoProviderBanner /> reading useActiveProvider + useAiBannerStore
    └── ai/
        ├── ProviderCard.tsx      # single provider card (collapsed/expanded states)
        ├── AiProvidersSection.tsx # 4 stacked ProviderCards
        ├── ConnectAiStep.tsx      # onboarding step (2x2 grid)
        └── AiNoProviderBanner.tsx # session-only dismissible banner
```

### Pattern 1: AuthState Initialization and AppState Wiring

**What:** `AuthState` is constructed at startup with `app_data_dir` path, loads `credentials.json`, then is `manage()`d so all IPC commands can access it via `State<'_, AuthState>`.
**When to use:** lib.rs setup closure — same location as `WatcherRegistry`, `SpaceManager`, etc.
**Example:**
```rust
// Source: learnforge/src-tauri/src/auth/mod.rs (ported verbatim — read file)
// In lib.rs setup closure:
let auth_state = auth::AuthState::new(&app_data);
let oauth_flow_state = auth::oauth::OAuthFlowState::new();
app.manage(auth_state);
app.manage(oauth_flow_state);
app.manage(AppState { /* existing fields */ });
```

### Pattern 2: Credential Validation on Save (D-08)

**What:** Every connect action validates the credential with a real API call before persisting. The validation call is minimal (1-token chat, max_tokens=1). HTTP 200 or 400 = valid. 401/403 = reject.
**When to use:** `connect_provider` IPC command body.
**Example:**
```rust
// Source: learnforge/src-tauri/src/auth/oauth.rs validate_anthropic_token() — read file
// Key insight: 400 means "token valid but request has shape issue" — still means auth works.
// Only 401 (bad token) and 403 (OAuth not enabled for account) are auth failures.
async fn validate_anthropic_token(token: &str) -> Result<(), String> {
    // ... POST /v1/messages with max_tokens=1 ...
    if status == 200 || status == 400 { return Ok(()); }
    match status {
        401 => Err("Setup token is invalid or expired..."),
        403 if body.contains("OAuth authentication is currently not allowed") => Err("..."),
        _ => Err(format!("Anthropic API error ({}): {}", status, body)),
    }
}
```

### Pattern 3: IPC Command Naming Convention

**What:** All Phase 7 IPC commands follow the established pattern from learnforge's `auth/commands.rs`. Names are already proposed in D-65 (Claude's Discretion).
**When to use:** Every new `#[tauri::command]` in `commands/ai.rs`.
**Example:**
```rust
// Source: learnforge/src-tauri/src/auth/commands.rs — read file
// Use synchronous commands where possible (no async I/O needed for credential reads).
// Use async for connect_provider (has HTTP validation call inside).
#[tauri::command]
pub fn get_auth_status(auth: State<AuthState>) -> Result<Vec<ProviderAuthStatus>, String>

#[tauri::command]
pub async fn connect_provider(
    auth: State<'_, AuthState>,
    flow: State<'_, OAuthFlowState>,
    request: LoginRequest,
) -> Result<ProviderAuthStatus, String>
```

### Pattern 4: React Query Provider Hooks

**What:** Follows the existing `useTauri.ts` pattern. Query for `list_providers`, mutations for `connect_provider`, `disconnect_provider`, `set_active_provider`.
**When to use:** Any component needing provider state.
**Example:**
```typescript
// Source: existing useTauri.ts pattern (verified in codebase)
export function useProviders() {
  return useQuery({
    queryKey: queryKeys.providers,
    queryFn: () => tauriInvoke<ProviderAuthStatus[]>("get_auth_status"),
  });
}

export function useConnectProvider() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (request: ConnectProviderRequest) =>
      tauriInvoke<ProviderAuthStatus>("connect_provider", { request }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.providers });
      qc.invalidateQueries({ queryKey: queryKeys.activeProvider });
    },
  });
}
```

### Pattern 5: Session-Only Zustand Banner Store

**What:** `useAiBannerStore` tracks whether the user dismissed the "Connect AI" banner this session. No `persist` middleware — store resets on app reload. Exactly like existing stores in `stores.ts`.
**When to use:** `AppShell.tsx` mounts `<AiNoProviderBanner />` which reads both this store and `useProviders()`.
**Example:**
```typescript
// Source: existing stores.ts pattern (verified in codebase — no persist)
interface AiBannerState {
  isDismissed: boolean;
  dismiss: () => void;
}

export const useAiBannerStore = create<AiBannerState>((set) => ({
  isDismissed: false,
  dismiss: () => set({ isDismissed: true }),
}));
```

### Anti-Patterns to Avoid

- **Validating credentials asynchronously after save:** D-08 is explicit — save without validation is rejected. Never store then validate; always validate then store.
- **Using `zeroclaw` or any OAuth library:** D-03 + learnforge's FIX-05 remove this. All AI routing is direct reqwest.
- **Setting `active_provider` to a provider with no credentials:** `AuthState.set_active_provider()` guards against this — it returns `Err` if provider has no stored credential.
- **Exposing the raw `CredentialStore` to the frontend:** IPC commands return `ProviderAuthStatus` (authenticated flag, method, model, is_active) but never the actual token or key string.
- **Using `zeroclaw` for Anthropic OAuth:** Anthropic setup-token auth is done by learnforge's direct `Bearer` + `anthropic-beta` header pattern. No zeroclaw needed.
- **Triggering re-renders on every credential check:** Use React Query's `staleTime` appropriately — credential status doesn't change unless user explicitly connects/disconnects.
- **Implementing `OAuthFlowState` for Phase 7 use cases:** Anthropic uses paste-based setup-token (no callback URL). OpenAI/Gemini use API key only. `OAuthFlowState` is ported for completeness and potential Phase 8+ use, but Phase 7 doesn't trigger OAuth flows. The `save_setup_token` command is the only "OAuth-adjacent" action.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Credential persistence | Custom file writer | `AuthState.persist()` from learnforge (port verbatim) | Handles lock, serialize, write, error propagation. 15+ tests in `auth/mod.rs` covering corruption, persistence across instances, fallback. |
| Provider HTTP routing | Ad-hoc per-provider if-else | `ai_request()` from `ai/service.rs` | Handles auth method dispatch, credential missing error, model fallback. Tested. |
| Retry/backoff | Custom sleep loop | `retry_with_backoff` from `ai/retry.rs` | Exponential backoff with saturation, tested with mock ops, tested for doubling timing. |
| Error message mapping | Inline status-code checks | `map_oauth_error()` and per-provider `map_*_error()` from learnforge | Covers 401/403/429/5xx/timeout patterns; output directly usable in sonner toasts. |
| Token format validation | Regex or custom parser | `starts_with("sk-ant-oat01-")` check from `oauth.rs` | One-liner; D-04 locks the prefix. No regex needed. |
| Active-provider fallback on disconnect | Manual UI state update | `remove_credential()` in `AuthState` auto-falls-back to any remaining provider | Backend handles the invariant; frontend just invalidates the `providers` query cache. |
| Onboarding step machine | New state pattern | Extend existing `step` `useState` + `StepIndicator` in `OnboardingPage.tsx` | Change `total={4}` to `total={5}`, shift step indices. Existing component handles it. |

**Key insight:** The entire backend of Phase 7 is a port of battle-tested code. The only "new" code is the IPC wiring glue in `commands/ai.rs` and the React UI components.

---

## Common Pitfalls

### Pitfall 1: `AuthState` registered after `AppState` in `manage()`

**What goes wrong:** Tauri panics at startup or IPC commands can't resolve `State<'_, AuthState>` if `auth_state` is managed after `app.manage(AppState {...})`.
**Why it happens:** Tauri resolves managed state by type at runtime; registration order matters only for initialization dependencies, but commonly developers forget to add the new `manage()` call.
**How to avoid:** Add `app.manage(auth_state)` and `app.manage(oauth_flow_state)` BEFORE `app.manage(AppState {...})` in `lib.rs` setup closure.
**Warning signs:** Tauri runtime panic like `State<AuthState> was not properly initialized`.

### Pitfall 2: `AuthMethod` serialization breaks credential roundtrip

**What goes wrong:** `AuthMethod` enum uses `#[serde(rename_all = "kebab-case")]` — `AuthMethod::ApiKey` serializes as `"api-key"`. If any match arm in `commands.rs` uses `"api_key"` (underscore) instead of `"api-key"` (hyphen), login fails silently.
**Why it happens:** Rust enum variant snake_case → kebab-case conversion is non-obvious.
**How to avoid:** Copy `LoginRequest.method` matching verbatim from `learnforge/src-tauri/src/auth/commands.rs`. The strings are `"api-key"` and `"ollama"`.
**Warning signs:** `login_provider` IPC returns `"Unsupported auth method: api_key"` in dev console.

### Pitfall 3: Anthropic validate on 400 — don't reject

**What goes wrong:** Developer sees HTTP 400 from validation call and rejects the credential as invalid, but 400 means "authenticated but request shape is wrong" (e.g., model not found). Only 401/403 mean auth failure.
**Why it happens:** 400 conventionally means client error, so developers assume it means invalid credential.
**How to avoid:** Port `validate_anthropic_token` verbatim — it explicitly accepts 200 or 400 as success.
**Warning signs:** Anthropic credentials fail validation despite correct token; changing to a known-valid model makes it work.

### Pitfall 4: OpenAI system prompt format vs. Anthropic

**What goes wrong:** Anthropic takes `system` as a top-level field. OpenAI takes it as a first message with `role: "system"`. Using the wrong format causes 400 errors.
**Why it happens:** The two APIs have different conventions; it's easy to use a common wrapper that applies the wrong format.
**How to avoid:** Port `build_anthropic_request` and `build_openai_request` verbatim — they already handle this correctly. Never write a shared request builder.
**Warning signs:** One provider works, the other returns 400 with "invalid message role" or similar.

### Pitfall 5: Ollama model listing vs. static dropdown

**What goes wrong:** Hardcoded model list in Ollama card becomes stale or doesn't match what the user has pulled. Ollama cards need to dynamically fetch available models from `GET /api/tags`.
**Why it happens:** Other three providers (Anthropic, OpenAI, Gemini) use static lists. Ollama is different — the user decides what models to pull.
**How to avoid:** For Ollama card: call `GET {base_url}/api/tags` to populate model dropdown. Parse `{ models: [ { name: "llama3:latest" }, ... ] }`. Default to `llama3` if request fails (user may not have started Ollama yet).
**Warning signs:** User enters base URL, clicks Connect, sees empty model dropdown or "model not found" from Ollama.

### Pitfall 6: Banner shows on first launch before onboarding completes

**What goes wrong:** `AppShell.tsx` mounts `<AiNoProviderBanner />` which checks `useProviders()`. On first launch, no providers are configured, so the banner appears during onboarding — but onboarding already has the "Connect AI" step.
**Why it happens:** Banner logic checks `active_provider === null` without checking if onboarding is complete.
**How to avoid:** Gate the banner: only show if `useOnboardingStore().isCompleted === true` AND no active provider. Onboarding covers the "first connect" prompt.
**Warning signs:** Banner appears behind onboarding wizard overlay on first run.

### Pitfall 7: reqwest JSON feature not enabled

**What goes wrong:** `reqwest::Client.json()` method is a compile error if the `json` feature is not enabled in `Cargo.toml`.
**Why it happens:** reqwest features are opt-in; learnforge's `Cargo.toml` includes `features = ["json"]`.
**How to avoid:** Declare: `reqwest = { version = "0.12", features = ["json"] }`.
**Warning signs:** Compile error "method `json` not found for `reqwest::RequestBuilder`".

---

## Code Examples

Verified patterns from learnforge source (read verbatim):

### Credential Store — Full Module

```rust
// Source: /Users/gshah/work/apps/learnforge/src-tauri/src/auth/mod.rs (read verbatim)
// Key types: AuthMethod (OAuth/ApiKey/None), ProviderCredential, CredentialStore, AuthState
// Key methods:
//   store_api_key(provider, api_key, model) — stores ApiKey method, auto-sets active if first
//   store_oauth_token(provider, token, display_name, model) — stores OAuth method (setup-tokens)
//   store_ollama_config(base_url, model) — stores None method
//   get_active_credential() — resolves active_provider → ProviderCredential
//   remove_credential(provider) — falls back active to remaining provider
//   persist() — serializes to credentials.json in app_data_dir
// Port this file verbatim. 15 unit tests included.
```

### Anthropic Chat — Header Construction

```rust
// Source: /Users/gshah/work/apps/learnforge/src-tauri/src/ai/anthropic.rs build_anthropic_request()
// Setup-token path: Authorization: Bearer <token>, anthropic-beta: oauth-2025-04-20
// API-key path: x-api-key: <key>, no anthropic-beta header
// system prompt: top-level field (not a message role)
// Port build_anthropic_request() verbatim — it's a pure function, easy to test.
```

### Central Router — ai_request()

```rust
// Source: /Users/gshah/work/apps/learnforge/src-tauri/src/ai/service.rs ai_request()
// Flow: get_active_credential() → normalize_provider_name() → match branch → per-provider chat fn
// normalize_provider_name("claude") → "anthropic"
// normalize_provider_name("chatgpt") → "openai"
// Port service.rs verbatim. Gemini and Ollama implementations are inside this file.
```

### IPC Command Surface — auth/commands.rs

```rust
// Source: /Users/gshah/work/apps/learnforge/src-tauri/src/auth/commands.rs
// Commands to port: get_auth_status, login_provider, set_active_provider, logout_provider
// Suggested Cortex rename: get_auth_status → list_providers (matches CONTEXT.md discretion list)
// ProviderAuthStatus { provider, authenticated, method, display_name, model, is_active }
// LoginRequest { provider, method: "api-key"|"ollama", credential?, model?, base_url? }
```

### Setup-Token Save Command

```rust
// Source: /Users/gshah/work/apps/learnforge/src-tauri/src/auth/oauth.rs save_setup_token()
// Validates format: starts_with("sk-ant-oat01-")
// Calls validate_anthropic_token() (real API call, 1-token chat)
// Stores as: provider="claude", method=OAuth, oauth_token=token, model="claude-haiku-4-5-20251001"
// Note: stored under key "claude" not "anthropic" in learnforge. Cortex may normalize to "anthropic".
```

### Error Message Mapping

```rust
// Source: /Users/gshah/work/apps/learnforge/src-tauri/src/auth/oauth.rs map_oauth_error()
// Covers: 401/unauthorized/invalid token → "Invalid bearer token. Please log in again."
//         403/forbidden/scope → "Token does not have the required permissions."
//         timeout/connection refused/network → "Could not reach provider. Check your connection..."
//         other → truncate to 200 chars
// Use this in the IPC layer for any provider connect failure.
```

### Retry with Backoff

```rust
// Source: /Users/gshah/work/apps/learnforge/src-tauri/src/ai/retry.rs
// retry_with_backoff(op, max_retries=2, initial_delay=2000ms) — exponential doubling
// ai_request_with_retry(auth, req, max_retries) — wraps ai_request with backoff
// For Phase 7: use ai_request_with_retry in the chat() IPC command.
// For validation calls: do NOT retry (validation failure is intentional).
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| zeroclaw for AI routing (learnforge v1) | Direct reqwest per provider (FIX-05) | learnforge Phase 7 / commit 49b0fb1 | Removes private dependency, identical behavior, easier to audit |
| OpenAI/Gemini OAuth handshakes | API-key only for OSS builds | learnforge open-core split | Simplifies auth surface; zeroclaw OAuth not available in OSS |
| bert-base-NER for entity extraction (Cortex v1.0) | LLM extraction via Phase 7 provider (v1.1) | Phase 7 prerequisite | NER ONNX removed in Phase 8; this phase creates the LLM router it depends on |

**Deprecated/outdated:**
- `zeroclaw` dependency: explicitly commented out in learnforge `Cargo.toml`. Never add to Cortex.
- Anthropic `anthropic-beta: oauth-2025-04-20` header: confirmed required as of 2026-06-30 for setup-token auth. `[ASSUMED]` — the beta header name may stabilize; check Anthropic docs if 403 errors appear.

---

## Key Learnforge → Cortex Adaptation Notes

These are the specific differences between learnforge and Cortex that require adaptation (not verbatim copy):

### 1. Provider key naming: "claude" vs "anthropic"

Learnforge stores Anthropic credentials under key `"claude"` (from `save_setup_token` → `store_oauth_token("claude", ...)`). `normalize_provider_name("claude")` → `"anthropic"`. CONTEXT.md D-04 and D-17 use "anthropic" in UI. Cortex should normalize at storage time: store under key `"anthropic"` and skip the `"claude"` alias, OR keep the alias and update `get_auth_status` to iterate `["claude", "openai", "gemini", "ollama"]` as learnforge does. **Recommendation:** Keep `"claude"` as the storage key to match learnforge's `save_setup_token` verbatim, but expose as `provider: "anthropic"` in `ProviderAuthStatus` (the command layer normalizes on read). This requires minimal diff in the port.

### 2. `AppState` fields

Cortex `AppState` has `ner_service`, `entity_store`, etc. — learnforge doesn't. Phase 7 adds `auth_state: AuthState` and `oauth_flow_state: OAuthFlowState` as `manage()`d types, NOT as fields inside `AppState`. This is correct because `AuthState` and `OAuthFlowState` are accessed directly via `State<'_>` in IPC handlers, never through `AppState`.

### 3. `OAuthFlowState` — include but don't activate

Port `OAuthFlowState` from `auth/oauth.rs` for structure completeness. In Phase 7, it's only exercised by `save_setup_token` (which doesn't do true OAuth). Phase 8+ may use it for future OAuth flows. Register it via `manage()`. Don't build a frontend `check_oauth_status` polling loop for Phase 7 (no async OAuth flow to poll).

### 4. No `detect_system_providers` auto-import

Learnforge has `detect_system_providers` which reads env vars. Cortex explicitly should NOT auto-import from env (privacy-first, BYOK must be explicit). The command can exist as a "report already-configured providers" utility (which is what learnforge does — it doesn't auto-import), but it should not read `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` env vars and auto-store them.

### 5. Credential unification for OpenAI embeddings (D-20)

`src-tauri/src/pipeline/embedder.rs` has a comment stub for OpenAI embedding path: "will use ruvector-core's ApiEmbedding::openai(), activated via settings toggle in Phase 4." This stub is NOT implemented. For Phase 7, the Settings AI tab should:
- Show the "Embedding Model" radio (local vs openai) as before
- When user selects "OpenAI" embedding AND has OpenAI connected as AI provider: show "Using connected OpenAI key" instead of a second API key input
- When user selects "OpenAI" embedding AND has NO OpenAI provider connected: show a link "Connect OpenAI in AI Providers below"
- The actual embedding code change (reading from AuthState) is Phase 7 UI concern only; the actual OpenAI embedding call implementation remains a stub until Phase 2's DPIP-07 is wired (out of scope for Phase 7 execution).

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `anthropic-beta: oauth-2025-04-20` header name is still required for setup-token auth as of 2026-06-30 | Code Examples / Anthropic chat | If header name changed, validation calls return 401; user would see auth failure even with valid token. Fix: update header name from Anthropic docs. |
| A2 | `reqwest 0.12` compiles without changes on Rust 1.91 (current on this machine) | Standard Stack | If 0.12 has a dependency broken by 1.91, upgrade to 0.13.4. API surface is mostly compatible. |
| A3 | Ollama `/api/tags` returns `{ models: [{ name: "..." }] }` structure for model listing | Pitfall 5 | If Ollama changed the response shape, the model picker code would need adjustment. |
| A4 | The hardcoded model IDs for Anthropic (claude-haiku-4-5-20251001, claude-sonnet-4-6, claude-opus-4) are currently valid | Standard Stack | If model IDs change, validation calls may return 400 (which we accept as valid auth) so auth still works; but the user selects a model that generates content. Low risk — 400 is accepted. |

**If this table is empty:** it would be wrong — the Anthropic beta header (A1) and Ollama API shape (A3) are assumptions not verified in this session against live services.

---

## Open Questions (ALL RESOLVED during planning)

1. **Should `chat()` IPC be exposed in Phase 7?** — **RESOLVED: YES.** Plan 03 Task 2 exposes `chat()` as the 8th IPC command, wrapping `ai_request_with_retry` with max_retries=2. Enables UAT criterion 4 testable from DevTools before Phase 8 ships.

2. **Use `"claude"` or `"anthropic"` as the internal storage key?** — **RESOLVED: `"anthropic"`.** Plan 01 Task 3 applies the storage-key delta in `auth/oauth.rs save_setup_token` (`store_oauth_token("anthropic", ...)`) and `auth/commands.rs get_auth_status` provider-scan list. Simplifies Phase 8 credential lookup.

3. **Retry max count for Phase 7 IPC `chat()` command?** — **RESOLVED: `max_retries = 2`.** Plan 03 Task 2 hardcodes `ai_request_with_retry(auth.inner(), request, 2).await`. Matches learnforge convention; future settings exposure deferred.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust / cargo | Backend compilation | Yes | 1.91.0 | — |
| bun | Frontend dev + test | Yes | 1.2.7 | npm fallback |
| Ollama CLI | Testing Ollama provider | Yes (not running) | 0.23.0 | Start with `ollama serve` |
| Network access to api.anthropic.com | Validation calls in tests | Yes (assumed) | — | Mock in unit tests |
| Network access to api.openai.com | Validation calls in tests | Yes (assumed) | — | Mock in unit tests |
| Network access to generativelanguage.googleapis.com | Validation calls in tests | Yes (assumed) | — | Mock in unit tests |

**Missing dependencies with no fallback:** None — all blocking tools are available.
**Missing dependencies with fallback:** Ollama not running — validation tests for Ollama can use `http://localhost:11434` with test expecting connection error (not auth error).

---

## Validation Architecture

> `workflow.nyquist_validation` key is absent from `.planning/config.json` — treat as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework (Rust) | Built-in `cargo test` (no external framework) |
| Framework (Frontend) | Vitest 3.2.4 (in devDependencies, `"test": "vitest --run"`) |
| Config file (Rust) | None needed — tests live in `#[cfg(test)]` modules |
| Config file (Frontend) | `vite.config.ts` (Vitest uses same config) — no separate vitest.config needed |
| Quick run (Rust backend) | `cargo test -p cortex_lib auth -- --nocapture 2>&1 \| tail -20` |
| Quick run (Frontend) | `bun test --run` |
| Full suite (Rust) | `cargo test 2>&1 \| tail -30` |
| Full suite (Frontend) | `bun test --run` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Notes |
|--------|----------|-----------|-------------------|-------|
| AIPV-01 | Anthropic setup-token stored, validated, marked connected | Unit (Rust) | `cargo test -p cortex_lib auth::tests -- --nocapture` | Port 15 tests from learnforge auth/mod.rs |
| AIPV-01 | `build_anthropic_request` uses Bearer + anthropic-beta for setup-token | Unit (Rust) | `cargo test -p cortex_lib anthropic` | Port 4 tests from learnforge anthropic.rs |
| AIPV-01 | `build_anthropic_request` uses x-api-key (no anthropic-beta) for API key | Unit (Rust) | `cargo test -p cortex_lib anthropic` | Port test_build_request_api_key_headers |
| AIPV-02 | OpenAI stored, validated, request uses Bearer | Unit (Rust) | `cargo test -p cortex_lib openai` | Port 4 tests from learnforge openai.rs |
| AIPV-03 | Gemini stored, validated (API key + OAuth paths) | Unit (Rust) | `cargo test -p cortex_lib service` | Port normalize_provider tests |
| AIPV-04 | Ollama config stored (base URL + model), connection test routes to local server | Unit (Rust) | `cargo test -p cortex_lib auth::tests::test_store_ollama_config` | Port verbatim |
| AIPV-05 | Switch active provider — only connected providers selectable | Unit (Rust) | `cargo test -p cortex_lib auth::tests::test_set_active_provider` | Port verbatim |
| AIPV-05 | Radio button disabled for unconnected providers | Integration (Frontend) | `bun test client/components/ai/ProviderCard.test.tsx` | Wave 0 gap: create test file |
| AIPV-06 | Onboarding step machine has 5 steps; step 2 is ConnectAi | Integration (Frontend) | `bun test client/pages/OnboardingPage.test.tsx` | Wave 0 gap: create test file |
| AIPV-06 | Skip → banner appears; banner dismissed → session-only | Unit (Frontend) | `bun test client/lib/stores.test.ts` | Wave 0 gap: add AiBannerStore test |
| AIPV-07 | Credentials survive `AuthState::new()` across instances | Unit (Rust) | `cargo test -p cortex_lib auth::tests::test_persistence_across_instances` | Port verbatim — exists in learnforge |
| AIPV-07 | Disconnect removes credential, falls back active provider | Unit (Rust) | `cargo test -p cortex_lib auth::tests::test_remove_active_falls_back` | Port verbatim |
| AIPV-08 | 401 maps to "Invalid bearer token" toast message | Unit (Rust) | `cargo test -p cortex_lib oauth::tests::test_map_oauth_error_401` | Port verbatim |
| AIPV-08 | Timeout maps to "Could not reach provider" message | Unit (Rust) | `cargo test -p cortex_lib oauth::tests::test_map_oauth_error_timeout` | Port verbatim |
| AIPV-08 | Anthropic 401 maps to human-readable message | Unit (Rust) | `cargo test -p cortex_lib anthropic::tests::test_map_anthropic_error_401` | Port verbatim |

### Sampling Rate

- **Per task commit:** `cargo test -p cortex_lib 2>&1 | tail -5` (Rust unit tests, <10s)
- **Per wave merge:** `cargo test 2>&1 | tail -10` + `bun test --run`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `client/components/ai/ProviderCard.test.tsx` — covers AIPV-05 radio disabled state
- [ ] `client/pages/OnboardingPage.test.tsx` — covers AIPV-06 step 2 present, skip behavior
- [ ] `client/lib/stores.test.ts` — add `useAiBannerStore` tests (dismiss resets on re-create, does not persist)

*All Rust tests come from porting learnforge — they are pre-written and known-passing. The frontend Wave 0 gaps are new tests for new UI behavior.*

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | Yes — credential storage, session lifetime | Plaintext JSON (v1.1 accepted per OOS table); no auto-read from env vars; explicit BYOK |
| V3 Session Management | No | No web sessions — desktop app with persistent file |
| V4 Access Control | Yes — credentials exposed only via typed IPC | `ProviderAuthStatus` never includes raw token; credential reads gated by IPC type system |
| V5 Input Validation | Yes | Format check on setup-token (`starts_with("sk-ant-oat01-")`); URL format check on Ollama base URL; API keys trimmed before storage |
| V6 Cryptography | No | No encryption for v1.1 (plaintext JSON); Keychain is v1.2 per OOS table |

### Known Threat Patterns for Tauri 2 + Credential Storage

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Credential exfiltration via renderer | Information Disclosure | IPC commands return `ProviderAuthStatus` (never the raw token); Tauri CSP blocks renderer from reading `app_data_dir` directly |
| Auto-import from env vars | Information Disclosure | Explicitly NOT done per CONTEXT.md D-05; `detect_system_providers` only reports already-stored creds |
| Token logging in error messages | Information Disclosure | `map_oauth_error` truncates body to 200 chars; never logs the token itself |
| SSRF via Ollama base URL | Elevation of Privilege | Ollama URL is user-configured; validate scheme is `http://` or `https://`; no redirect following needed |
| Corrupt credentials.json crashes app | Denial of Service | `AuthState::new()` falls back to `CredentialStore::default()` on parse error (tested: `test_corrupt_file_falls_back_to_default`) |

---

## Sources

### Primary (HIGH confidence)

- Learnforge source files read verbatim — `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/mod.rs`, `ai/service.rs`, `ai/anthropic.rs`, `ai/openai.rs`, `ai/retry.rs`, `auth/mod.rs`, `auth/oauth.rs`, `auth/commands.rs`
- Cortex source files read verbatim — `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/settings.rs`, `client/pages/SettingsPage.tsx` (AI tab), `client/pages/OnboardingPage.tsx`, `client/hooks/useTauri.ts`, `client/lib/stores.ts`, `src-tauri/Cargo.toml`
- `.planning/phases/07-ai-provider-foundation/07-CONTEXT.md` — locked decisions D-01 through D-21
- `.planning/REQUIREMENTS.md` — AIPV-01..08 full text + OOS table

### Secondary (MEDIUM confidence)

- `cargo search reqwest` output — confirmed `reqwest = "0.13.4"` on crates.io as of 2026-06-30
- `cargo search tempfile` output — confirmed `tempfile = "3.27.0"` as current

### Tertiary (LOW confidence)

- Anthropic `anthropic-beta: oauth-2025-04-20` header — taken from learnforge code written before August 2025; header may have changed. `[ASSUMED]`
- Ollama `/api/tags` response shape — from training knowledge, not verified against running Ollama. `[ASSUMED]`

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all packages verified on crates.io or already in Cargo.toml
- Architecture: HIGH — directly derived from learnforge source read verbatim
- Pitfalls: HIGH — derived from actual learnforge code structure (AuthMethod kebab-case, Anthropic 400 acceptance, etc.)
- Frontend patterns: HIGH — derived from existing Cortex codebase patterns read verbatim

**Research date:** 2026-06-30
**Valid until:** 2026-07-30 (Anthropic API beta header and model IDs may change on shorter timeline — check if validation calls fail)

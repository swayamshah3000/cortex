# Phase 7: AI Provider Foundation - Context

**Gathered:** 2026-06-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 7 delivers the pluggable AI backend that all v1.1 LLM-driven features (Phases 8-11) depend on. Three concrete deliverables:

1. **Provider router** — Rust `ai/` module routing chat requests to Anthropic / OpenAI / Gemini / Ollama via direct `reqwest` HTTP. Ports the proven pattern from `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/`.
2. **Credential store** — Rust `auth/` module managing per-provider credentials in plaintext `app_data_dir/credentials.json` (Keychain deferred to v1.2 per REQUIREMENTS.md OOS table). Tracks `active_provider`. Ports learnforge's `AuthState` / `CredentialStore` / `AuthMethod` / `ProviderCredential`.
3. **UI surfaces** — Settings → AI tab redesign (4 stacked provider cards + status + active radio + model dropdown + connect/disconnect), and an onboarding "Connect AI" step inserted as Step 2 of the wizard.

**Out of scope:**
- LLM entity extraction (Phase 8 owns this)
- LLM space labeling (Phase 9 owns this)
- macOS Keychain credential storage (v1.2 hardening per OOS table)
- OAuth flows for OpenAI / Gemini (learnforge confirms zeroclaw OAuth removed for OSS; API-key only for those two)
- Streaming responses (not required for entity extraction or labeling use cases)
- Cost / token-usage tracking UI (deferred)
- Per-feature provider override (entity vs labeling on different providers — single active provider for v1.1)

</domain>

<decisions>
## Implementation Decisions

### Provider Routing & Auth
- **D-01:** Port `/Users/gshah/work/apps/learnforge/src-tauri/src/{ai,auth}/` as the foundation. Adapt to Cortex AppState + IPC patterns. Battle-tested pattern with `AuthState`, `CredentialStore`, `AuthMethod::{OAuth, ApiKey, None}`, `ProviderCredential`, `ai_request()` central router, `normalize_provider_name()` aliases, and per-provider chat fns (`anthropic_chat`, `openai_chat`, `gemini_chat`, `ollama_chat`).
- **D-02:** Plaintext `credentials.json` at `app_data_dir/credentials.json`. Schema = `{ active_provider, credentials: { providerId → ProviderCredential } }`. Per v1.1 OOS table — Keychain is v1.2.
- **D-03:** Use direct `reqwest` per provider — no `anthropic-sdk` / `openai-sdk` crates. Matches learnforge's FIX-05 stance (zeroclaw removed from AI routing).

### Anthropic Auth UX
- **D-04:** Setup-token only on the Anthropic card (sk-ant-oat01-* from `claude setup-token`). No API-key fallback toggle. Stored as `AuthMethod::OAuth` so router sends Bearer + `anthropic-beta: oauth-2025-04-20` header.
- **D-05:** Inline instructions on the card — single screen with code block (`claude setup-token`) + copy button + paste field + Claude CLI install link. No two-step wizard, no `~/.claude/.credentials` auto-detect. Minimal; assumes user is a dev (Cortex target persona).
- **D-06:** Default model on save = `claude-haiku-4-5-20251001` (matches learnforge default). Fast + cheap for high-volume entity extraction across 10K-doc corpora.
- **D-07:** Model change via inline dropdown on the provider card (same UI pattern for all 4 providers). Applies immediately; no disconnect/reconnect required.

### Connection Validation
- **D-08:** Validate every credential on Save with a real API call. Failed validation → reject save with the provider's error message. Credentials in store are guaranteed-working at write time. Matches learnforge's `validate_anthropic_token()` pattern (1-token chat, accept 200/400, reject 401/403).
- **D-09:** Validation call shape = minimal 1-token chat per provider:
  - Anthropic: POST `/v1/messages` with `{model, max_tokens: 1, messages: [{role:"user", content:"hi"}]}`
  - OpenAI: POST `/v1/chat/completions` with same shape
  - Gemini: POST `generateContent` minimal body
  - Ollama: POST `/api/chat` with minimal body — also serves as the "test ping" required by roadmap success criterion #3
- **D-10:** Save UX = inline button spinner (Save → "Validating…" → "Saved" green flash, or stays + error toast). No full-card overlay. No async background validation.
- **D-11:** No explicit timeout override — rely on `reqwest` default (~30s). Future tuning if users hit timeout issues.

### Onboarding "Connect AI" Step
- **D-12:** Inserted as **Step 2** of the wizard. New flow: Welcome → **Connect AI** → Folders → Scanning → Done. AI commitment before any indexing happens (Phase 8 backfill triggers off connect-event when LLM extraction ships).
- **D-13:** Step layout = **2x2 grid of 4 provider cards** (Anthropic, OpenAI, Gemini, Ollama). Each card: logo + "Connect" button → expands inline form (paste / URL field). Continue button enables after any provider connects.
- **D-14:** Skip allowed. On skip → continue onboarding to Folders/Scanning. App shell shows a dismissible banner: "Connect an AI provider to enable Smart Spaces →" linking to Settings → AI.
- **D-15:** Banner is **session-only dismissible**. X hides it for the session; returns on next app launch until a provider is connected. Persistent nudge without permanent annoyance.

### Settings → AI Tab Layout
- **D-16:** **Stacked provider cards** (one per row, 4 total). Pattern consistent with Watched Folders. Each card: provider name + status badge (Connected / Not connected) + active-radio + model dropdown + Connect/Disconnect button.
- **D-17:** **Active-provider selector = radio button on each card.** No separate dropdown, no TopBar switcher. Only connected providers' radios are selectable. Switching radio = `set_active_provider` IPC call, applies immediately to next LLM call.
- **D-18:** Card states = **collapsed by default, expandable**. Connected: compact row `Anthropic • Connected • Haiku 4.5 [Active] [▾]`. Click chevron → expand for model dropdown + Disconnect. Not-connected: `Anthropic • Not connected [Connect]`. All 4 providers fit on screen.
- **D-19:** Keep the existing **"Embedding Model"** section at the top of the AI tab (local ONNX vs OpenAI radio). Divider. Then "AI Providers" section below.
- **D-20:** **Unify cloud credentials.** If the user picks OpenAI as their embedding model AND has connected OpenAI as an AI provider in Phase 7, the embedding code reuses the Phase 7 credential — no duplicate API key input. Single source of truth for the OpenAI key.
- **D-21:** Runtime provider failures (rate limit, invalid token mid-session, network) surface as **global sonner toast + inline red status on the provider card**. Card stays red until next successful call. Two surfaces catch the user wherever they are.

### Scope amendment 2026-07-02 (post-UAT Test 1 feedback)
- **D-22:** **OpenAI = ChatGPT/Codex subscription OAuth (PKCE + refresh).** Cortex mirrors the auth flow that `codex login` uses — targeting the ChatGPT subscription auth endpoint, NOT the generic OpenAI Platform OAuth. Users authenticate with their existing ChatGPT Plus/Team subscription; Cortex chat calls route through Codex/ChatGPT backend (`openai-codex` provider slug in `normalize_provider_name`). Refresh token stored + rotated so sessions survive weeks. Learnforge `ai/openai.rs` already routes `AuthMethod::OAuth → Bearer` — that stays. New in Cortex: `start_openai_oauth` command (Tauri opens system browser to ChatGPT/Codex authorization endpoint), local loopback listener captures callback, exchanges code+PKCE-verifier for access+refresh tokens, stores under provider `"openai-codex"` with `AuthMethod::OAuth`. Refresh handler triggers on 401 during `openai_chat`. API-key fallback preserved as secondary option on OpenAI card (routes as plain `openai` provider). Was: locked OpenAI = API-key only. Reason: user wants ChatGPT subscription usage (unmetered for Codex CLI users) with long-session persistence.
- **D-22a:** Planner MUST inspect `codex` CLI source (or its published OAuth flow docs) to identify: (1) authorization endpoint URL, (2) client_id used by codex CLI (may be public/hardcoded), (3) scopes requested, (4) token endpoint URL, (5) refresh endpoint URL. If codex CLI is closed-source and no public OAuth reference exists, the planner MUST surface this as a research blocker before proceeding — do NOT invent endpoints.
- **D-23:** **Gemini = API-key only (Option C locked 2026-07-02).** Original intent was Google OAuth PKCE symmetric with D-22. Wave 6 research (07-OAUTH-RESEARCH.md) found no user-subscription OAuth path for `generateContent`: Google requires per-GCP-project OAuth client registration, not a shared client_id pattern (unlike Anthropic setup-token or Codex CLI). Shipping "Sign in with Google" would force every user to create + configure a Google Cloud project — worse UX than pasting an API key. User accepted Option C 2026-07-02: Gemini card stays API-key-only in v1.1. Revisit in v1.2 if Google publishes a Codex-equivalent flow OR pursue Vertex AI (via `gcloud auth`) as alternative Google-endpoint provider.
- **D-24:** OAuth PKCE flow shape (shared by D-22/D-23):
  1. Tauri command `start_{provider}_oauth` generates PKCE verifier+challenge, spawns local loopback HTTP server on ephemeral port, opens system browser to provider auth URL with `redirect_uri=http://localhost:{port}/callback&code_challenge=...&code_challenge_method=S256`.
  2. Provider redirects to loopback with `?code=...`. Loopback captures + shuts down, returns "You may close this tab" page.
  3. Cortex POSTs code + PKCE verifier to provider token endpoint → receives `{access_token, refresh_token, expires_in}`.
  4. Store as `AuthMethod::OAuth` credential with `oauth_token=access_token`, `refresh_token=<val>`, `expires_at=now+expires_in`.
  5. `ai_request()` checks `expires_at` before chat call; if <60s remaining, POST refresh endpoint, rotate stored tokens, retry.
  6. `disconnect_provider` also POSTs revoke endpoint before deletion (best-effort; ignore failure).
- **D-25:** OpenAI card UI: two-mode card. Primary CTA "Sign in with ChatGPT" → triggers `start_openai_oauth` (browser opens). Secondary "Use API key instead" toggle → paste field (existing pattern). Same visual pattern on Gemini card: "Sign in with Google" / "Use API key instead". No breaking change to Anthropic card (setup-token flow stays).
- **Cortex-local extension of ProviderCredential struct:** add `refresh_token: Option<String>`, `expires_at: Option<i64>` fields to `ProviderCredential` in `auth/mod.rs`. Backward compat: existing plaintext credentials.json parses fine (missing fields → None).

### Claude's Discretion
- Exact IPC command names: `list_providers`, `connect_provider`, `disconnect_provider`, `set_active_provider`, `get_active_provider`, `save_setup_token`, `test_connection`, `chat`. Planner finalizes signatures (`#[tauri::command] async + spawn_blocking + serde camelCase` — locked from Phase 1/4).
- OpenAI OAuth client_id + authorization endpoint URL — planner researches current OpenAI OAuth registration process (may require registered client_id from OpenAI dev console).
- Gemini OAuth client_id + scopes — planner researches current Google Cloud Console OAuth setup for generative-language API.
- Local loopback port strategy — pick free port in range 8000-9000, retry on collision.
- Refresh timing threshold — 60s before expiry vs. reactive-only on 401.
- Whether to expose a `chat()` IPC command in Phase 7 (only used by Phase 8/9) or wait. Recommendation: expose it for end-to-end testability during Phase 7 UAT, but planner can defer if scope tight.
- Exact hardcoded model lists per provider (e.g., Anthropic dropdown showing Haiku 4.5 / Sonnet 4.6 / Opus 4.7; OpenAI showing gpt-5 / gpt-5-mini / etc.). Planner picks current-as-of-2026-06-30 model IDs; Ollama uses `/api/tags` to fetch.
- Retry / backoff for transient 429 / 503 errors — learnforge has `ai/retry.rs` to crib from. Planner decides.
- Backend module layout in `src-tauri/src/` — likely `ai/` + `auth/` mirroring learnforge, or merge into `intelligence/`. Planner picks.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project specs
- `.planning/ROADMAP.md` §"Phase 7: AI Provider Foundation" — phase goal, requirements list (AIPV-01..08), success criteria
- `.planning/REQUIREMENTS.md` §"AI Provider Foundation" — AIPV-01..08 full text
- `.planning/REQUIREMENTS.md` §"Out of Scope" — Keychain deferred to v1.2; cloud-default + Ollama fallback; no cloud-only mode
- `.planning/PROJECT.md` §"Current Milestone: v1.1" — cloud-first AI intelligence positioning
- `.planning/PROJECT.md` §Constraints — privacy-first, content never leaves the machine unless explicitly sent to chosen AI provider
- `CLAUDE.md` §"How RuVector Powers Cortex" — note that LLM provider routing does NOT touch ruvector; orthogonal concerns

### Port reference (CRITICAL — port wholesale)
- `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/mod.rs` — module structure (`anthropic.rs`, `openai.rs`, `retry.rs`, `service.rs`)
- `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/service.rs` — central `ai_request()` router + `normalize_provider_name()` + per-provider chat fns (`gemini_chat`, `ollama_chat`)
- `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/anthropic.rs` — Anthropic chat (Bearer + anthropic-beta for setup-tokens, x-api-key for API keys)
- `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/openai.rs` — OpenAI chat (Bearer token)
- `/Users/gshah/work/apps/learnforge/src-tauri/src/ai/retry.rs` — retry / backoff for transient errors
- `/Users/gshah/work/apps/learnforge/src-tauri/src/auth/mod.rs` — `AuthState`, `CredentialStore`, `AuthMethod`, `ProviderCredential`, persistence to JSON
- `/Users/gshah/work/apps/learnforge/src-tauri/src/auth/oauth.rs` — `save_setup_token` Tauri command with `validate_anthropic_token`, `check_oauth_status`, `map_oauth_error`, `OAuthFlowState`
- `/Users/gshah/work/apps/learnforge/src-tauri/src/auth/commands.rs` — Tauri IPC commands (read this to mirror command surface)

### Existing Cortex code (must read before modifying)
- `client/pages/SettingsPage.tsx` §"AI & Models Tab" (lines 257-307) — current minimal embedding-model toggle + placeholder OpenAI API key input. Phase 7 replaces this entire tab content (keeps Embedding Model section at top, adds AI Providers section below).
- `client/pages/OnboardingPage.tsx` — current 4-step wizard. Phase 7 inserts a "Connect AI" step as Step 2 (shifts existing steps).
- `src-tauri/src/commands/settings.rs` — settings persistence pattern (JSON sidecar in app_data_dir). Credential store follows same disk pattern but separate file.
- `src-tauri/src/commands/mod.rs` — IPC command registration pattern.
- `src-tauri/src/lib.rs` — AppState + plugin wiring; needs `AuthState` + `OAuthFlowState` added to `manage()`.
- `client/hooks/useTauri.ts` — React Query hook factory; add `useProviders`, `useActiveProvider`, `useConnectProvider`, `useDisconnectProvider`, `useSetActiveProvider`, `useSaveSetupToken`.
- `client/lib/stores.ts` — Zustand stores; add `useAiBannerStore` for session-only banner dismissal (no persist middleware).
- `src-tauri/Cargo.toml` — add `reqwest` (likely already present), `tempfile` for tests.

### Patterns to mirror
- Phase 5 settings JSON persistence in app_data_dir — same disk pattern for credentials.json
- Phase 4 React Query hook factory in `useTauri.ts` — adopt for all provider hooks
- Phase 1/4 IPC convention: `#[tauri::command] async + spawn_blocking + serde camelCase`
- Phase 6 inline-form-in-card UX (entity Split alias dialog) — similar inline expand pattern for provider connect forms

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `client/pages/SettingsPage.tsx` AI tab structure (Tabs + TabsContent + card classes) — keep shell, replace inner content.
- `client/pages/OnboardingPage.tsx` `StepIndicator` + `step` state machine — extend `total` from 4 to 5; add Step 2 component.
- `sonner` toast (already in deps) — for global error surface (D-21).
- `RadioGroup` + `RadioGroupItem` from shadcn — already used in AI tab for embedding model; reuse for active-provider radio on cards.
- React Query hook factory pattern (`useTauri.ts`) — extend for provider hooks.
- Settings persistence sidecar pattern (Phase 5) — model after for `credentials.json`.
- `chevron`/`collapse` UX from existing components (e.g., sidebar collapse) — reuse for provider card expand/collapse.

### Established Patterns
- All IPC = `#[tauri::command] async + spawn_blocking + #[serde(rename_all = "camelCase")]` (Phase 1/4). New provider commands match.
- Settings persist via JSON sidecar in app_data_dir (Phase 5). Credentials follow same pattern, separate `credentials.json` file.
- Zustand for UI state (banner dismissed-this-session), React Query for server state (provider list, active provider, credential status).
- Tauri events for long-running tasks (Phase 5 indexing). Not needed in Phase 7 itself — Phase 8 backfill uses this pattern when triggered post-connect.
- `cn()` helper + Tailwind tokens from globals.css — never hardcode colors.

### Integration Points
- `src-tauri/src/lib.rs` — register `AuthState` and `OAuthFlowState` via `manage()`. Register new IPC commands in `commands/mod.rs`.
- `client/App.tsx` — onboarding step machine extended to 5 steps. No new routes.
- `client/components/layout/AppShell.tsx` (or wherever banner lives) — mount `<AiNoProviderBanner />` reading `useActiveProvider()` + `useAiBannerStore()`.
- `client/pages/SettingsPage.tsx` AI tab — replace inner content with `<EmbeddingModelSection />` + divider + `<AiProvidersSection />` with 4 `<ProviderCard provider="anthropic|openai|gemini|ollama" />` instances.
- Embedding credential unification (D-20) — existing `src-tauri/src/pipeline/embedder.rs` OpenAI path (if any) updates to read from `AuthState::get_credential("openai")`. Embedding-model toggle in Settings becomes "use connected OpenAI provider" — no second API key input.

</code_context>

<specifics>
## Specific Ideas

- learnforge's `validate_anthropic_token` accepts both HTTP 200 and 400 as "valid token" — distinguishes auth failure (401/403) from request-shape errors. Mirror this for all 4 providers.
- learnforge `OAuthFlowState` (`Arc<Mutex<HashMap<String, FlowEntry>>>`) tracks per-flow completion/auth/error — useful even though we're not doing OAuth handshakes in Phase 7 (setup-token is paste-based). May simplify the API; planner can omit if not needed for setup-token-only Anthropic.
- learnforge's `map_oauth_error()` maps 401 → "Invalid bearer token. Please log in again.", timeout → "Could not reach provider. Check your connection.", 403/scope → "Token does not have the required permissions." — directly reusable for AIPV-08 human-readable toasts.
- learnforge stores `first stored becomes active` (auto-set `active_provider` on first credential). Reuse: first-time onboarding user connects Anthropic → automatically active. No extra click.
- learnforge `remove_credential` falls back to remaining provider for `active_provider` — useful for disconnect UX. Reuse.
- Ollama base URL default = `http://localhost:11434`. Pre-populate the Ollama card field; user only changes if non-default.
- Anthropic setup-token format check `starts_with("sk-ant-oat01-")` — reuse for client-side hint before submit.

</specifics>

<deferred>
## Deferred Ideas

### Phase 7 follow-ups (not blocking ship)
- **macOS Keychain credential storage** — REQUIREMENTS.md OOS marks this as v1.2 hardening. Plaintext JSON is acceptable for v1.1 ship. File: `app_data_dir/credentials.json` — call out in PR that it's plaintext.
- **OAuth handshakes for OpenAI and Gemini** — learnforge confirms zeroclaw OAuth removed in OSS builds. API-key only for OpenAI/Gemini in Phase 7. Add OAuth if/when a zeroclaw-equivalent is available.
- **Streaming responses** — not required by Phase 8 (entity extraction returns JSON) or Phase 9 (label is 2-4 words). Add later if a chat-UI surface appears.
- **Cost / token-usage tracking** — `ai_request()` already returns `input_tokens` / `output_tokens` per response. Aggregate UI deferred.
- **Per-feature provider override** — single active provider for v1.1 covers entity extraction + space labeling. Per-feature override (e.g., Haiku for entities, Sonnet for labels) deferred until users ask.

### ruvector adoption for downstream phases
- **Phase 10 (Hierarchical Spaces)** — adopt `ruvector-cluster` and/or `ruvector-hyperbolic-hnsw` crates for recursive sub-clustering (HSPC-03). Replaces the planned HDBSCAN / recursive-k-means call with a ruvector-native path. LLM still needed for sub-space labels.
- **Phase 11 (Entity-Driven Exploration)** — adopt `ruvector-gnn` for the Related panel (ENEX-03). Current plan uses cosine + entity-overlap; GNN message-passing on a doc-entity bipartite graph could surface non-obvious relationships. Enrichment, not replacement.
- **Phase 9 (LLM Space Labeling)** — possible TF-IDF + top-entity fallback path for low-LLM-call mode (privacy-strict users, rate-limited situations). LLM is the primary path; ruvector + heuristics as secondary fallback. Discuss during Phase 9 CONTEXT.

### v2 / future
- **Force-directed knowledge graph viz** (already in REQUIREMENTS.md v2 as ASPAC-04) — would benefit from `ruvector-graph` (Cypher) + GNN edges once ruvector-gnn is adopted.
- **Chat with documents (RAG)** (ASPAC-05) — separate product surface, would build on Phase 7 router + Phase 8 entity index + vector search.

</deferred>

---

*Phase: 07-ai-provider-foundation*
*Context gathered: 2026-06-30*

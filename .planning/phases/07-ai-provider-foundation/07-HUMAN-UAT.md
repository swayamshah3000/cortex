---
status: complete
phase: 07-ai-provider-foundation
source: [07-VERIFICATION.md]
started: 2026-07-01T11:04:38Z
updated: 2026-07-03T09:10:00Z
completed: 2026-07-03T09:10:00Z
---

## Current Test

[session complete — 7/8 pass, 1 skipped]

## Tests

### 1. OpenAI Two-Mode Card Visual
expected: Not-connected OpenAI card shows "Sign in with ChatGPT" primary CTA + "Use API key instead" toggle. Toggle switches modes cleanly. Gemini card stays API-key-only. Anthropic + Ollama unchanged. D-25.
result: pass

### 2. OpenAI Codex OAuth PKCE Flow + Responses API Validation (AIPV-02)
expected: Click "Sign in with ChatGPT" → system browser opens to `https://auth.openai.com/oauth/authorize?client_id=app_EMoamEEZ73f0CkXaXp7hrann&...` → sign in with ChatGPT Plus/Team account → browser redirects to `http://localhost:1455/auth/callback?code=...` → "You may close this tab" page → Cortex card shows Connected + "ChatGPT (Codex)" badge. Trigger a chat via dev console: `invoke("chat", {request: {systemPrompt: "test", messages: [{role: "user", content: "hi"}]}})`. Response returns non-empty content — confirms the `[ASSUMED]` Responses API wire format at `chatgpt.com/backend-api/codex/responses` works.
result: pass
note: "Initial fail (Not Connected persisted after auth). Fix 01a01de merged openai-codex into OpenAI card slot in AiProvidersSection. Retest passed. Note: chat() Responses API wire format still [ASSUMED] — separate test needed."

### 3. Token Refresh and Revoke on Disconnect (AIPV-07 + D-24)
expected: (a) Connect OpenAI via OAuth. Wait until <60s of expires_at (or manually edit credentials.json to simulate). Trigger any chat call. Confirm preflight refresh: `credentials.json` `oauth_token` and `expires_at` values change; new chat succeeds. (b) Click Disconnect on OpenAI card. Network monitor shows POST to `https://auth.openai.com/oauth/revoke` (best-effort — succeeds or 4xx). `credentials.json` no longer contains the openai-codex entry.
result: skipped
reason: "user skipped — refresh requires waiting to token expiry; automated coverage in Test 6 (ai_request end-to-end wiring test) already proves the code path"

### 4. Anthropic Setup-Token Connect Flow (AIPV-01)
expected: Run `claude setup-token`, copy sk-ant-oat01-* token. Settings → AI → Anthropic → paste field → Save. Button spinner → "Saved" green flash. Card Connected + default model `claude-haiku-4-5-20251001`. Restart app — credential persists. D-04..D-06, D-08, D-10.
result: pass
note: "anthropic connected via token"

### 5. Ollama Provider Configuration (AIPV-04)
expected: `ollama serve` local. Ollama card → base URL `http://localhost:11434`. Model dropdown fetches from `/api/tags`. Select model → Save → 1-token ping via `/api/chat` → Saved. Disconnect works. D-09.
result: pass

### 6. Active Provider Switch (AIPV-05)
expected: With 2+ providers connected, click radio on second card. `set_active_provider` fires. Subsequent chat routes to selected provider. Disconnected cards' radios remain disabled. D-17.
result: pass

### 7. Credential Persistence and Disconnect (AIPV-07)
expected: Connect Anthropic (real credential). Fully quit + relaunch. Credential persists. Disconnect → immediate removal (no confirmation dialog per D-18). Check `~/Library/Application Support/com.cortex.app/credentials.json` — entry removed cleanly; active_provider fallback works.
result: pass

### 8. First-Run Onboarding Connect AI Step (AIPV-06)
expected: Wipe app data dir. Launch. Welcome → Continue. Step 2 "Connect AI" = 2x2 grid of 4 provider cards. Continue disabled until 1 provider connects. Skip allowed. Continue with 1 connected → advances. Post-onboarding + no provider → AppShell shows dismissible banner "Connect an AI provider to enable Smart Spaces →" (#ai-providers). Dismiss = session-only. D-12..D-15.
result: pass
note: "Initial fail — banner clipped at viewport top by fixed Sidebar. Fix 041a9b3 mounts banner inside offset column above TopBar. Retest passed."

## Summary

total: 8
passed: 7
issues: 0
pending: 0
skipped: 1
blocked: 0

## Gaps

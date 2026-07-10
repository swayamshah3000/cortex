---
phase: 7
slug: ai-provider-foundation
status: draft
nyquist_compliant: true
wave_0_complete: true  # all Wave 0 gaps assigned to Plan 04/05/06 test creation tasks
created: 2026-06-30
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework (Rust)** | `cargo test` (built-in) |
| **Framework (TS/UI)** | vitest (existing in client/) |
| **Config file** | `src-tauri/Cargo.toml` (Rust), `client/vitest.config.ts` (UI, if present) |
| **Quick run command** | `cd src-tauri && cargo test --lib` |
| **Full suite command** | `cd src-tauri && cargo test && (cd client && bun run test --run 2>/dev/null || true)` |
| **Estimated runtime** | ~45s Rust, ~15s UI |

---

## Sampling Rate

- **After every task commit:** Run quick (cargo test --lib for the touched module)
- **After every plan wave:** Run full suite
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

Populated by gsd-planner during plan generation. Each PLAN.md task must declare either:
- `<automated>` with a verify command, OR
- A Wave 0 dependency on a stub test that exists

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | AIPV-01..08 | TBD | TBD | unit/integration | TBD | ⬜ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/auth/mod.rs` — module declaration with `#[cfg(test)] mod tests` block
- [ ] `src-tauri/src/ai/mod.rs` — module declaration with `#[cfg(test)] mod tests` block
- [ ] `src-tauri/tests/` — integration test directory if needed for IPC roundtrips
- [ ] reqwest 0.12 with `features = ["json"]` added to `src-tauri/Cargo.toml`
- [ ] tempfile dev-dependency confirmed (used by credential-store roundtrip tests)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Anthropic OAuth setup-token roundtrip | AIPV-02 | Real `sk-ant-oat01-*` token requires `claude setup-token` CLI execution by user | 1) Run `claude setup-token`, copy token. 2) Open Settings → AI → Anthropic → paste token → Save. 3) Confirm card shows "Connected" + green flash. 4) Restart app. 5) Confirm credential persisted and active. |
| OpenAI live API key validation | AIPV-02 | Real OpenAI API key required | Paste key in Settings card, click Save, expect "Validating…" then "Saved" or error toast. |
| Gemini live API key validation | AIPV-02 | Real Gemini API key required | Same as OpenAI. |
| Ollama local server reachability | AIPV-03 | Local Ollama daemon required | 1) `ollama serve`. 2) Settings → AI → Ollama → base URL `http://localhost:11434` → choose model from /api/tags. 3) Save → test ping → expect "Saved". 4) Switch active to Ollama → next chat() call hits local daemon. |
| Onboarding "Connect AI" Step 2 flow | AIPV-06 | First-run onboarding UX | Wipe app_data_dir → launch app → walk Welcome → Connect AI → confirm Continue disabled until 1 provider connects, then enables. Skip path → banner appears on app shell, dismiss → banner returns next launch. |
| Active-provider switch propagation | AIPV-04 | Requires connected providers to test | Connect 2+ providers, click radio on second, immediately trigger chat() — expect request to second provider. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

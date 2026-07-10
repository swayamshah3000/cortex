# Phase 7: AI Provider Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-30
**Phase:** 07-ai-provider-foundation
**Areas discussed:** Anthropic auth UX, Connection validation on save, Onboarding placement + skip, Settings → AI tab layout, ruvector cross-cutting pivot

---

## Anthropic auth UX

### Q1: Which auth methods to support on Anthropic card?

| Option | Description | Selected |
|--------|-------------|----------|
| Setup-token only | sk-ant-oat01-* paste, Bearer + anthropic-beta header. Subscription users only. | ✓ |
| Setup-token AND API key (toggle) | Radio: Subscription or API key. Covers both user types. | |
| API key only | sk-ant-* from console.anthropic.com only. Subscription users pay twice. | |

**User's choice:** Setup-token only (learnforge default)

### Q2: How to guide users who haven't run `claude setup-token`?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline instructions + paste field | Single screen, code block + paste field + Claude CLI install link | ✓ |
| Two-step wizard | Step 1: install CLI; Step 2: run command + paste | |
| Auto-detect ~/.claude/.credentials | One-click import if found, fallback to paste | |

**User's choice:** Inline instructions + paste field

### Q3: Default model when saving Anthropic credential?

| Option | Description | Selected |
|--------|-------------|----------|
| Haiku 4.5 (fast, cheap) | claude-haiku-4-5-20251001 (learnforge default) | ✓ |
| Sonnet 4.6 (balanced) | Better extraction quality, more cost | |
| User picks at setup | No default; forces explicit choice | |

**User's choice:** Haiku 4.5

### Q4: Model-change UI after setup?

| Option | Description | Selected |
|--------|-------------|----------|
| Dropdown on provider card | Inline, applies immediately, consistent across all 4 providers | ✓ |
| Settings → AI → expanded provider config | Accordion-style, less noise | |
| Disconnect + reconnect | Model locked at credential-store time | |

**User's choice:** Dropdown on provider card

---

## Connection validation on save

### Q1: When to verify a credential works?

| Option | Description | Selected |
|--------|-------------|----------|
| Always on save (validate-or-reject) | 1-token chat, reject save on failure (learnforge pattern) | ✓ |
| Optional "Test connection" button | Save persists; user tests later | |
| Save + background validation | Save persists; card flips green/red async | |

**User's choice:** Always on save (validate-or-reject)

### Q2: Validation call shape?

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal chat (learnforge pattern) | 1-token chat per provider; 200/400 = valid | ✓ |
| List models endpoint | Cheaper but inconsistent across providers | |
| Provider-native check | Lightest per provider, more code paths | |

**User's choice:** Minimal chat (learnforge pattern)

### Q3: Save UX during validation?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline spinner on Save button | Save → Validating… → Saved/error toast | ✓ |
| Full-card overlay during validation | Greyed-out card + center spinner | |
| Toast-only feedback | Save persists immediately, toast async | |

**User's choice:** Inline spinner on Save button

### Q4: Validation timeout?

| Option | Description | Selected |
|--------|-------------|----------|
| 10s hard timeout | Strict; catches slow networks fast | |
| 30s default reqwest timeout | Allows slow networks to succeed | ✓ |
| Configurable in Settings → AI → Advanced | Overkill for v1.1 | |

**User's choice:** 30s default reqwest timeout

---

## Onboarding placement + skip

### Q1: Where does "Connect AI" fit in the wizard?

| Option | Description | Selected |
|--------|-------------|----------|
| Step 2 (before Folders) | AI commitment before any indexing | ✓ |
| Step 5 (last, after Spaces preview) | User sees value before being asked | |
| Step 5 with skip-promoted CTA | Same as #2 with persistent banner | |

**User's choice:** Step 2 — Welcome → Connect AI → Folders → Scanning → Done

### Q2: Skip behavior?

| Option | Description | Selected |
|--------|-------------|----------|
| Skip → continue + persistent banner | Dismissible banner in app shell | ✓ |
| Skip → confirm modal | Light friction to discourage skip | |
| Skip allowed only if Ollama detected | Auto-suggest local Ollama fallback | |

**User's choice:** Skip → continue onboarding + persistent banner in app

### Q3: All 4 providers at once or one-at-a-time?

| Option | Description | Selected |
|--------|-------------|----------|
| 4 provider cards in 2x2 grid | Logo + Connect → expands inline form | ✓ |
| Stacked vertical list | More breathing room, more scroll | |
| Recommended-first wizard | Anthropic front + center, others hidden | |

**User's choice:** 4 provider cards in a 2x2 grid

### Q4: Banner dismissibility?

| Option | Description | Selected |
|--------|-------------|----------|
| Dismissible (session only) | X hides for session; returns next launch | ✓ |
| Dismissible forever | Writes flag to settings; never shows again | |
| Not dismissible until connected | Banner stays until provider connected | |

**User's choice:** Dismissible (session only)

---

## Settings → AI tab layout

### Q1: Main layout for the 4 providers?

| Option | Description | Selected |
|--------|-------------|----------|
| Stacked provider cards (one per row) | 4 vertical cards, consistent w/ Watched Folders | ✓ |
| Active-provider dropdown + config below | Single-config focus | |
| Sub-tabs per provider | Horizontal sub-tabs, more clicks to see status | |

**User's choice:** Stacked provider cards (one per row)

### Q2: Active-provider selector location?

| Option | Description | Selected |
|--------|-------------|----------|
| Radio button on each provider card | Inline, applies immediately | ✓ |
| Dropdown at top of AI tab | Separates 'connect' from 'activate' | |
| TopBar global switcher | Discoverable from any page, more impl | |

**User's choice:** Radio button on each provider card

### Q3: Existing "Embedding Model" section disposition?

| Option | Description | Selected |
|--------|-------------|----------|
| Keep at top of AI tab | Embeddings + Providers on one tab w/ divider | ✓ |
| Move to Indexing tab | AI tab becomes purely LLM providers | |
| Drop the section | Defer until embedding routes through provider too | |

**User's choice:** Keep at top of AI tab (above providers)

### Q4: Provider card states?

| Option | Description | Selected |
|--------|-------------|----------|
| Collapsed by default, expandable | Compact row, click chevron for full form | ✓ |
| Always expanded | Lots of vertical scroll | |
| Two states: collapsed-connected, expanded-setup | Different states for different intents | |

**User's choice:** Collapsed by default, expandable

### Q5: Runtime error surface for provider failures?

| Option | Description | Selected |
|--------|-------------|----------|
| Global toast + inline on card | Two surfaces catch user wherever they are | ✓ |
| Global toast only | Simpler, lose context on dismiss | |
| Inline card status only | Quiet, easy to miss during background backfill | |

**User's choice:** Global toast (sonner) + inline on provider card

---

## ruvector cross-cutting pivot

User asked mid-discussion: "are we using ruvector for this project? if not, can we?" → followed by "analyse if ruvector is useful and help us to do gnn based relationships etc" → then "and get rid of some or all of LLM work" → then "dont we need to store vectors for semantic search and smart spaces".

### Q1: After analysis, how to proceed?

| Option | Description | Selected |
|--------|-------------|----------|
| Keep Phase 7 as-is, wrap up | LLM-driven v1.1 plan stands | |
| Add ruvector exploration as Phase 7 gray area | Captures the question in the record | ✓ |
| Revisit roadmap | Roadmap-level discussion, separate workflow | |
| Cut LLM work — ruvector-first v1.1 | Drop Phase 7-9, quality bar drops | |

**User's choice:** Add ruvector exploration as a Phase 7 gray area

### Q2: ruvector × Phase 7 — anything Phase 7 should touch?

| Option | Description | Selected |
|--------|-------------|----------|
| Nothing — Phase 7 is pure HTTP/credential | ruvector unchanged | |
| Note LLM provider routes also used for OpenAI embeddings | Unify cloud cred storage | |
| Plan future ruvector adoption notes for downstream phases | Captured in deferred ideas | ✓ |

**User's choice:** Plan future ruvector adoption notes for downstream phases

### Q3: Also fold in the embedding-credential unification?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — unify cloud creds | Phase 7 OpenAI cred reused for embeddings | ✓ |
| No — keep embedding cred separate | Decoupled, simpler scope | |

**User's choice:** Yes — unify cloud creds

---

## Claude's Discretion

- Exact IPC command names and signatures (locked patterns: async + spawn_blocking + serde camelCase)
- Whether to expose `chat()` IPC in Phase 7 for testability or wait for Phase 8
- Hardcoded model lists per provider (Anthropic / OpenAI / Gemini); Ollama via `/api/tags`
- Retry / backoff for transient 429 / 503 (learnforge `ai/retry.rs` available to crib)
- Backend module layout (`ai/` + `auth/` vs merge into `intelligence/`)

## Deferred Ideas

### Phase 7 follow-ups
- macOS Keychain credential storage (REQUIREMENTS OOS — v1.2 hardening)
- OAuth handshakes for OpenAI/Gemini (zeroclaw OAuth removed in OSS — API key only)
- Streaming responses (not needed by Phase 8/9)
- Cost / token-usage tracking UI
- Per-feature provider override (single active for v1.1)

### ruvector adoption for downstream phases
- Phase 10 — adopt `ruvector-cluster` / `ruvector-hyperbolic-hnsw` for sub-clustering
- Phase 11 — adopt `ruvector-gnn` for Related panel (entity-aware ranking)
- Phase 9 — possible TF-IDF + top-entity fallback for low-LLM-call mode

### v2 / future
- Force-directed knowledge graph viz (ASPAC-04) — `ruvector-graph` + GNN edges
- Chat with documents / RAG (ASPAC-05)

---
phase: 6
slug: knowledge-graph-and-native-integrations
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-29
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Filled from `06-RESEARCH.md` §"Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Frontend framework** | vitest (existing) |
| **Backend framework** | `cargo test` (existing) |
| **Config files** | `vitest.config.ts`, `src-tauri/Cargo.toml` |
| **Quick run command** | `bun run test --run` (frontend) / `cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1` (backend) |
| **Full suite command** | `bun run test --run && cargo test --manifest-path src-tauri/Cargo.toml` |
| **Estimated runtime** | ~60 seconds combined |

---

## Sampling Rate

- **After every task commit:** Run quick run command for the side touched (frontend or backend)
- **After every plan wave:** Run full suite command
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

To be populated by the planner from `06-RESEARCH.md` §"Validation Architecture" — each test fixture and assertion in that section becomes a row here once tasks are minted. Planner MUST emit a row per task with: requirement ID (KG-01..05, UX-05, PAGE-13, UX-06), test type (unit / integration / manual), automated command, and fixture file path.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | TBD | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 sets up shared fixtures used by every other wave. Planner MUST emit Wave 0 tasks covering:

- [ ] `src-tauri/tests/fixtures/ner_corpus/` — small text corpus with known PER/ORG/LOC entities (KG-01 accuracy floor)
- [ ] `src-tauri/tests/fixtures/aliases/` — paired surface forms (e.g., "John Smith" / "J. Smith"; "123 Main St" / "Main Street property") for KG-02 cosine-merge test
- [ ] `src-tauri/tests/fixtures/preview/` — 1 small PDF, 1 large PDF (> 50 MB sentinel), 1 image, 1 plain text, 1 markdown file for PAGE-13 + D-15 size-guard tests
- [ ] `client/test/setup.ts` — Tauri `invoke` mock helpers for `useEntity` / `useEntitiesByType` / `useRelatedEntities` query hooks
- [ ] `tauri-plugin-dialog` + `tauri-plugin-opener` added to `src-tauri/Cargo.toml` and `package.json` (UX-05, UX-06 — without the deps no test can pass)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Native folder picker actually opens OS dialog | UX-05 | OS dialog cannot be driven from a unit test | Run `bun tauri dev`, navigate `/watched`, click "Add Folder", confirm native dialog opens, pick a folder, confirm it appears in the watched list |
| `revealItemInDir` opens Finder/Explorer at the document's parent folder | UX-06 | OS shell side-effect | Run app, open a document, click "Reveal in Finder", confirm Finder opens at the parent folder with file selected |
| `openPath` opens the file in the OS default app | UX-06 | OS shell side-effect | Run app, open a document, click "Open in default app", confirm system default app launches with the file |
| Embedded PDF viewer renders correctly inside iframe | PAGE-13 | WebView PDF renderer behavior varies per OS | Open a PDF document in DocumentPage, confirm in-app preview renders (zoom, scroll, find should work) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (NER fixtures, alias pairs, preview fixtures, plugin deps)
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

# Phase 6: Knowledge Graph and Native Integrations - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-29
**Phase:** 06-knowledge-graph-and-native-integrations
**Areas discussed:** Entity scope & extraction depth, Entity normalization strategy, Entity click-through UX, File preview rendering

---

## Entity scope & extraction depth

### Q1 — What entity types should be first-class graph nodes?

| Option | Description | Selected |
|--------|-------------|----------|
| Promote what exists | Date, Amount, Person/Org-as-one, Email — ship KG-01 fast; defer dedicated Org+Location | |
| Add Organization + Location now | Upgrade regex to emit Org (Inc/LLC heuristics) + Location (city/state) | |
| Promote all 5 with stub regex | Minimal patterns to honor 5-type contract on day one | |

**User's choice:** Free text — "how about using LLM here, or should we not?"
**Notes:** Pivoted the question to extraction method (Q2) since the answer to entity scope depends on what extractor is available.

### Q2 — Entity extraction method?

| Option | Description | Selected |
|--------|-------------|----------|
| ONNX NER model | Local ONNX NER (e.g. dslim/bert-base-NER), ~50ms/doc, same pattern as fastembed | ✓ |
| Ollama LLM extraction | Best recall, seconds/doc — backfilling 10K docs is hours | |
| Hybrid regex + LLM on demand | Regex on hot path; LLM enrich per-doc on user action | |
| Stay regex, expand patterns | Add Org/Location regex; cheapest, lowest quality | |

**User's choice:** ONNX NER model
**Notes:** Privacy-first constraint from PROJECT.md ruled out cloud APIs; ONNX won over Ollama because the indexing hot path needs sub-second per-doc latency.

### Q3 — Backfill strategy for existing indexed docs?

| Option | Description | Selected |
|--------|-------------|----------|
| Background backfill on startup | Tokio task with Tauri event progress, UI stays responsive | ✓ |
| Manual button in Settings | Predictable cost but most users never click | |
| Lazy: re-extract on doc open | Slow first-open per doc, no mass migration | |

**User's choice:** Background backfill on startup
**Notes:** Mirrors the Phase 5 indexing event pattern so the UX is consistent.

### Q4 — Keep regex extractors alongside ONNX NER?

| Option | Description | Selected |
|--------|-------------|----------|
| Regex for structured + NER for unstructured | Date/Amount/Email regex + Person/Org/Location NER, merge + dedup | ✓ |
| NER does everything | Drop regex; bert-base-NER often misclassifies dates/currency | |
| Regex only, no NER | Revert; skip model dep | |

**User's choice:** Regex for structured + NER for unstructured
**Notes:** Bert-base-NER is weak on currency / structured dates; regex stays for what it's good at.

---

## Entity normalization strategy

### Q1 — How should entity aliases merge?

| Option | Description | Selected |
|--------|-------------|----------|
| Embedding similarity | Fastembed entity values; cluster cosine ≥ 0.85 | ✓ |
| Normalized string match | Lowercase + Levenshtein ≤ 2; misses semantic aliases | |
| User-managed only | No auto-merge; manual merge button | |
| Embedding + per-type rules | Embedding base + type-specific (surname matching, geocoding) | |

**User's choice:** Embedding similarity
**Notes:** Reuses existing fastembed infra; semantic aliases like "123 Main St" vs "Main Street" handled out of the box.

### Q2 — When should merging run?

| Option | Description | Selected |
|--------|-------------|----------|
| On backfill + new doc ingest | Once after backfill; incremental against existing canonical set | ✓ |
| Batch only — user triggers | Stale until clicked | |
| Every doc, full re-cluster | Safest but scales badly | |

**User's choice:** On backfill + new doc ingest (incremental)

### Q3 — Canonical surface form?

| Option | Description | Selected |
|--------|-------------|----------|
| Most frequent surface form | Variant appearing in most documents wins | ✓ |
| Longest form | Prefer the longest variant | |
| User-editable, default = most frequent | Auto-pick with Rename action | |

**User's choice:** Most frequent surface form

### Q4 — Bad-merge recovery?

| Option | Description | Selected |
|--------|-------------|----------|
| Manual unmerge action | Split alias on entity detail page | ✓ |
| Higher threshold, no unmerge UI | Raise threshold to ≥ 0.92; if still wrong, retag doc | |
| Don't worry for v1 | Defer to later phase | |

**User's choice:** Manual unmerge (Split alias)

---

## Entity click-through UX

### Q1 — Where does clicking an entity chip land?

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated /entities/:id route | Full page; mirrors SpaceDetailPage; bookmarkable | ✓ |
| Side panel on current page | Sheet over DocumentPage; no URL change | |
| Filter on /search page | Reuses search UI; no Related Entities concept | |
| /entities index + detail | Index browser + detail page | |

**User's choice:** Dedicated /entities/:id route

### Q2 — Sidebar entry for entities?

| Option | Description | Selected |
|--------|-------------|----------|
| Sidebar 'Entities' link → /entities index | Discoverable; mirrors Tags + Spaces | ✓ |
| Only in-doc click-through | Hidden knowledge graph | |
| Cmd+K + pinned only, no index page | Smaller surface | |

**User's choice:** Sidebar 'Entities' link → /entities index
**Notes:** Pulled the "/entities index + detail" choice from Q1 in here — both Q1 and Q2 selections together yield the full index+detail surface.

### Q3 — Related entities computation?

| Option | Description | Selected |
|--------|-------------|----------|
| Co-occurrence in same document | Entities in N ≥ 2 same docs are related | ✓ |
| Co-occurrence weighted by doc similarity | Captures topical relatedness | |
| Graph distance in DocumentGraph | Reuses Phase 3 adjacency | |

**User's choice:** Co-occurrence in same document

### Q4 — Entity ops on /entities/:id?

| Option | Description | Selected |
|--------|-------------|----------|
| Rename + manual unmerge | Two actions; matches normalization decisions | ✓ |
| Rename + merge + unmerge + hide | Full CRUD | |
| Read-only for v1 | Conflicts with prior unmerge decision | |

**User's choice:** Rename + manual unmerge

---

## File preview rendering

### Q1 — PDF rendering approach?

| Option | Description | Selected |
|--------|-------------|----------|
| Tauri asset protocol + browser-native PDF | convertFileSrc + iframe/embed; zero deps | ✓ |
| react-pdf (pdfjs-dist) | JS-side canvas; full control; +1MB bundle | |
| Native shell.open only | No inline PDF; defeats PAGE-13 | |

**User's choice:** Tauri asset protocol + browser-native PDF

### Q2 — Image / text / markdown preview scope?

| Option | Description | Selected |
|--------|-------------|----------|
| All three | Image + monospace text + react-markdown | ✓ |
| Image + text only, markdown raw | Markdown as raw text | |
| Image only | Narrowest | |

**User's choice:** All three formats

### Q3 — Size guard?

| Option | Description | Selected |
|--------|-------------|----------|
| Soft limit with 'Load anyway' | PDFs > 50MB, text > 5MB, images > 20MB show placeholder | ✓ |
| Hard limit — fall back to Open in OS | No preview above thresholds | |
| No guard — always render | Trust WebView | |

**User's choice:** Soft limit with 'Load anyway'

### Q4 — Open in OS surfaces?

| Option | Description | Selected |
|--------|-------------|----------|
| DocumentPage + Search results + Recent | Plus context menu on rows in /favorites, /spaces/:id | ✓ |
| DocumentPage only | Smaller scope | |
| Everywhere with inline icons | Visual noise | |

**User's choice:** DocumentPage + Search + Recent (plus context menus on doc rows in /favorites, /spaces/:id)

---

## Claude's Discretion

- Exact ONNX NER model file / quantization
- Entity storage schema (RuVector collection vs SQLite vs in-memory rebuilt)
- IPC command signatures (general shape agreed; details for planner)
- Co-occurrence threshold tuning for Related Entities (chose ≥ 2 as starting point)
- Sidebar icon for Entities nav item

## Deferred Ideas

- DOCX / XLSX in-app preview (Open in OS sufficient for v1)
- Manual cross-entity merge action
- Hide entity action
- Geocoding for Location entities
- Entity-driven smart-space generation
- Pinned/favorite entities in sidebar

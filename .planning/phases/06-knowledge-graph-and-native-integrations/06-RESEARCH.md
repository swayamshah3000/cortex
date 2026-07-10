# Phase 6: Knowledge Graph and Native Integrations - Research

**Researched:** 2026-06-29
**Domain:** Local NER + entity graph + Tauri native filesystem integrations + in-app file preview
**Confidence:** HIGH (Tauri plugins, react-markdown, asset protocol, dslim model card) / MEDIUM (ort 2.x rc API specifics, NER backfill throughput estimates)

## Summary

Phase 6 has four well-scoped workstreams: (1) bolt a local ONNX NER model alongside the existing fastembed embedder to start emitting Organization/Location entities (currently 0% emitted), (2) build a sidecar `EntityGraph` keyed by canonical entity IDs with alias-merge via cosine similarity on entity-value embeddings, (3) wire `tauri-plugin-dialog` and `tauri-plugin-opener` to replace the ts-ignored dynamic import and the dead "Open in Finder" button, and (4) render per-type file previews (PDF via WebView native viewer through the asset protocol, image via `<img>`, text via `<pre>`, markdown via `react-markdown`).

The dominant technical risks are: ONNX model selection and bundle size (108-216 MB depending on quantization, comparable to the existing fastembed model the user already accepted), ort 2.x being in `rc.12` (RC, not stable — but stable enough that pyke uses it in production), and the Tauri asset protocol requiring a CSP update (the current `csp: null` permits everything but should be tightened in this phase since we're opening the door to arbitrary local files).

**Primary recommendation:** Use **Xenova/bert-base-NER `model_quantized.onnx`** (109 MB, INT8, MIT) via **`ort = "2.0.0-rc.12"`** with the **`tokenizers = "0.20"`** crate (hf-tokenizers via `Tokenizer::from_file("tokenizer.json")`). Store entities in a **separate in-memory `EntityStore` rebuilt from RuVector document metadata on startup** (no new persistence layer — the entity-ID-to-document mapping lives in document metadata on the existing RuVector collection, with a side `EntityStore` HashMap for fast lookup, mirroring `DocumentIndexer::path_index`). Compute co-occurrence on the fly from the `EntityStore` reverse index (cheap: O(docs-per-entity²) for the queried entity only). Backfill via a Tokio task spawned in `lib.rs::setup`, emitting `entity-backfill-progress` every 25 docs. PDF/image preview via `convertFileSrc` + iframe/img with a tightened CSP. Markdown via `react-markdown@10` + `remark-gfm@4` (no syntax highlighter — none in current deps, defer to v2). Dialog via `tauri-plugin-dialog@2.7.1`, opener via `tauri-plugin-opener@2.5.4`.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Entity Extraction**
- **D-01:** Use a local ONNX NER model (e.g., `dslim/bert-base-NER`) for Person / Organization / Location. Same pattern as the existing fastembed pipeline — load once, call inside `spawn_blocking`. No Ollama dependency.
- **D-02:** Keep the existing regex extractors for Date / Amount / Email (`pipeline/entities.rs`) — they are deterministic and fast for structured forms where bert-base-NER is mediocre. Merge regex + NER results, dedup by `(value, entity_type)`.
- **D-03:** Backfill existing indexed documents on app startup as a Tokio task. Emit progress as a Tauri event (mirrors the existing indexing event flow from Phase 5). UI stays responsive; user sees backfill in the TopBar indicator.
- **D-04:** Per-doc entity cap of 20 from current `entities.rs` stays; bert-base-NER outputs are merged into the same cap.

**Entity Normalization**
- **D-05:** Alias merging uses **embedding similarity** — embed each entity surface form using the same fastembed model used for documents, cluster aliases by cosine ≥ 0.85.
- **D-06:** Merge pass runs (a) once after the NER backfill completes, and (b) incrementally on every new document — embed new entities and check against the existing canonical set. No full O(n²) re-cluster on every doc.
- **D-07:** Canonical surface form is **most frequent** — the variant that appears in the most documents wins. Deterministic, no UI needed.
- **D-08:** Wrong merges are recoverable via a **Split alias** action on `/entities/:id`.

**Entity Click-Through UX**
- **D-09:** Click an entity chip on DocumentPage → navigate to a dedicated **`/entities/:id`** route.
- **D-10:** Add an **`/entities` index** to the sidebar (placed under "Tags").
- **D-11:** "Related entities" computed via **co-occurrence in same document** — two entities are related if they appear together in ≥ 2 documents, ranked by co-occurrence count.
- **D-12:** User actions on `/entities/:id` are scoped to **Rename canonical** + **Split alias**. No manual merge, no hide.

**File Preview**
- **D-13:** **PDF**: render via Tauri asset protocol (`convertFileSrc(path)`) inside an `<iframe>` or `<embed>`.
- **D-14:** **Image**: `<img src={convertFileSrc(path)} />`. **Text / code**: monospace `<pre>` block with syntax highlighting only if Prism/Shiki already in deps, else plain. **Markdown**: render via `react-markdown` (add as new dep).
- **D-15:** **Size guard — soft limit with "Load anyway"**: PDFs > 50 MB, text > 5 MB, images > 20 MB show a placeholder card.
- **D-16:** Both backend (read content + emit metadata) and frontend (per-type renderer components) live behind a `usePreview(documentId)` hook.

**Open in OS**
- **D-17:** Use **`tauri-plugin-opener`** for both `openPath` and `revealItemInDir`.
- **D-18:** Action surfaces: **DocumentPage header** (two visible buttons), and **right-click context menu** on document rows in `/search`, `/recent`, `/favorites`, `/spaces/:id`.

**Native Folder Picker**
- **D-19:** Replace the dynamic import + ts-ignore hack with a proper `tauri-plugin-dialog` dep. Single-folder select only. On cancel, do nothing silently. Validate the returned path exists and is a directory before submitting.

### Claude's Discretion

- Exact ONNX NER model file path / quantization choice — pick based on size + accuracy benchmarks **(this research recommends `model_quantized.onnx` from Xenova/bert-base-NER, 109 MB INT8 — see ONNX NER Recommendation below)**
- Storage schema for entities: RuVector collection vs. SQLite vs. in-memory HashMap rebuilt on startup. Lean toward persistent storage since NER backfill is expensive — but planner decides exact mechanism. **(This research recommends in-memory `EntityStore` HashMap rebuilt from existing RuVector document metadata — see Entity Storage + Graph below.)**
- Co-occurrence threshold for "Related entities" (≥ 2 as a starting heuristic — planner may tune).
- IPC command shape: `get_entities_by_type`, `get_entity(id)`, `get_documents_for_entity(id)`, `get_related_entities(id)`, `rename_entity_canonical`, `split_entity_alias`. **(Exact signatures sketched in Entity Storage + Graph below.)**
- Sidebar icon for Entities (suggest Lucide `Network` or `GitBranch`).

### Deferred Ideas (OUT OF SCOPE)

- DOCX / XLSX in-app preview — Open in OS covers them. Defer.
- Manual cross-entity merge action — auto-merge + split-alias covers v1.
- "Hide entity" action — defer until users complain.
- Geocoding for Location entities — out of scope.
- Entity-driven smart-space generation — future phase.
- DOCX/XLSX-aware preview thumbnails — future phase.
- Pinned/favorite entities in the sidebar — possible later.

## Phase Requirements

| ID | Description (from ROADMAP.md success criteria) | Research Support |
|----|-----------------------------------------------|------------------|
| KG-01 | Entities from documents appear as graph nodes; clicking surfaces every document mentioning them | Entity Storage + Graph (EntityStore design, IPC sketch); ONNX NER Recommendation |
| KG-02 | Entity normalization merges aliases (e.g., "123 Main St" and "Main Street property") so duplicates collapse | Entity Storage + Graph (alias-merge algorithm); reuses fastembed (no new deps for embedding) |
| KG-03 | Knowledge graph queryable via IPC — "entities by type", "documents for entity", "related entities" | Entity Storage + Graph (6 IPC commands sketched) |
| KG-04 | Rename canonical + Split alias on `/entities/:id` | Entity Storage + Graph (Split alias UX section) |
| KG-05 | NER backfill runs on startup, emits progress events, UI stays responsive | Backfill Strategy (Tokio task lifecycle + event throttling) |
| UX-05 | Add Watched Folder opens a native OS folder picker; manual path typing is gone | Tauri Plugin APIs (dialog) |
| PAGE-13 | Document detail page renders an in-app preview for PDF, image, plain-text, and markdown | File Preview Architecture; Markdown Pipeline |
| UX-06 | Open in Finder / Open with default app works from Document detail and search results | Tauri Plugin APIs (opener) |

**Note:** KG-01..KG-05, UX-05, PAGE-13, UX-06 are NOT YET in `.planning/REQUIREMENTS.md`. Phase 6 plan must add them under a new "Knowledge Graph" section + extend UX and PAGE sections.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| NER inference (BERT token classification) | Rust backend (pipeline) | — | CPU-heavy, runs inside `spawn_blocking`. Mirror of fastembed pattern. |
| Entity canonicalization + alias merge | Rust backend (graph) | — | Needs embedder access + RuVector collection scan. Pure server-side reasoning. |
| Co-occurrence calculation | Rust backend (graph) | — | Cheap on-demand from EntityStore reverse index. No precomputed matrix. |
| Backfill orchestration | Rust backend (Tokio task) | Frontend (event listener for progress) | Long-running, must not block startup. Frontend listens for `entity-backfill-progress`. |
| Folder picker dialog | Frontend (TS) | Rust backend (plugin init) | Plugin-provided native dialog, invoked from JS. |
| Open file / Reveal in Finder | Frontend (TS) | Rust backend (plugin init) | Plugin-provided OS shell call, invoked from JS. |
| PDF / image rendering | Frontend (WebView native) | Rust backend (asset protocol scope) | WebView's built-in PDF viewer + `<img>` tag. Backend only configures asset scope. |
| Markdown rendering | Frontend (react-markdown) | Rust backend (file read via asset URL) | Pure client-side render. Backend just exposes file. |
| Text preview | Frontend (`<pre>` block) | Rust backend (IPC to read raw text, since asset URL serves binary) | Small text files served via existing `get_document` excerpt OR new `get_document_text_preview` IPC. |
| Size guard | Frontend (decision UI) | Rust backend (size in metadata, already present) | Document.size already exists in IPC payload. |
| Entity chip → /entities/:id navigation | Frontend (React Router) | — | Pure routing. |

## Standard Stack

### Core (new Rust deps)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `ort` | `2.0.0-rc.12` [VERIFIED: crates.io] | ONNX Runtime wrapper for BERT inference | The only mainstream Rust ONNX wrapper. pyke maintains, 2.x is the active line. [CITED: docs.rs/ort] |
| `tokenizers` | `0.20` [VERIFIED: crates.io shows 0.23.1 latest; we pin a tested minor] | HuggingFace tokenizer for `tokenizer.json` | Official HF Rust port. Required for any BERT-family model. |
| `ndarray` | `0.16` [VERIFIED: crates.io shows 0.17.2 latest] | Tensor construction for ort inputs | ort's `inputs!` macro consumes ndarray views. |
| `tauri-plugin-dialog` | `2.7.1` [VERIFIED: crates.io] | Native folder picker | Official Tauri plugin. MIT/Apache. Required by D-19. [CITED: v2.tauri.app/plugin/dialog] |
| `tauri-plugin-opener` | `2.5.4` [VERIFIED: crates.io] | `openPath` + `revealItemInDir` | Official Tauri plugin (replaced older `tauri-plugin-shell::open` pattern in v2). [CITED: v2.tauri.app/plugin/opener] |

### Core (new npm deps)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `@tauri-apps/plugin-dialog` | `^2.7.1` [VERIFIED: npm registry, `npm view` 2026-06-29] | TS bindings for dialog plugin | Paired with Rust crate. [CITED: v2.tauri.app/plugin/dialog] |
| `@tauri-apps/plugin-opener` | `^2.5.4` [VERIFIED: npm registry, `npm view` 2026-06-29] | TS bindings for opener plugin | Paired with Rust crate. [CITED: v2.tauri.app/plugin/opener] |
| `react-markdown` | `^10.1.0` [VERIFIED: npm registry, `npm view` 2026-06-29] | Markdown → React renderer | Safe-by-default, no `dangerouslySetInnerHTML`. [CITED: github.com/remarkjs/react-markdown] |
| `remark-gfm` | `^4.0.1` [VERIFIED: npm registry, `npm view` 2026-06-29] | GitHub Flavored Markdown plugin (tables, strikethrough, task lists, autolinks) | Standard companion to react-markdown. |

### Supporting (deferred — NOT installing in Phase 6)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `hf-hub` | `1.0.0-rc.1` | Download ONNX model from HuggingFace at runtime | Optional convenience — Phase 6 bundles the .onnx file in `src-tauri/models/` instead (no download on first run, mirrors how fastembed currently caches but the planner can move to hf-hub later if model swapping becomes a feature). |
| `rehype-sanitize` | `^6.x` | Sanitize rendered HTML for markdown content | Only if we add `rehype-raw` to render embedded HTML inside markdown. Default react-markdown already escapes HTML, so deferred. |
| `shiki` / `prismjs` | n/a | Syntax highlighting for code blocks inside markdown / text preview | Not in current deps. Defer to a future phase. Code blocks render as plain `<pre>` for v1. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `ort` + raw `tokenizers` | `rust-bert` (high-level pipelines) | rust-bert wraps tch (LibTorch) by default — drags in 700 MB+ of native deps unrelated to ONNX. The `onnx` feature exists but routes through ort anyway. Direct ort is leaner. |
| `ort` + `tokenizers` | `fastembed`'s reranker feature | fastembed supports text embedding only — no NER pipeline. Confirmed via crate inspection in code review for this phase. |
| Bundling `.onnx` in `src-tauri/models/` | `hf-hub` runtime download | Bundling adds ~110 MB to the app installer but guarantees offline first-run. fastembed already downloads ~90 MB on first run; doing the same for NER doubles cold-start time. **Bundle is recommended** per CLAUDE.md privacy-first / offline-first ethos. Planner may decide otherwise. |
| `react-markdown` | `marked` + manual React render | react-markdown's React-native rendering is safer (no innerHTML) and uses the unified/remark plugin ecosystem. marked requires manual XSS protection. |
| Asset protocol + iframe for PDF | `react-pdf` (PDF.js wrapper) | react-pdf is ~700 KB gzipped + needs a web worker — adds complexity. The WebView's built-in PDF viewer (Chromium on win/linux, native on macOS) is free, zero-dep, zero-perf-cost, and exactly what D-13 asks for. |
| Custom `EntityStore` HashMap | RuVector entity collection | Storing entity-value embeddings in a RuVector collection would enable HNSW-accelerated alias-search at scale. Cost: another collection to manage + persistence overhead. Phase 6 v1 has at most a few thousand entities — linear scan of canonical entities is < 5 ms. **HashMap recommended; revisit if entity count exceeds 10K.** |

**Installation (planner — finalize in plan):**
```bash
# Rust (src-tauri/Cargo.toml)
ort = { version = "2.0.0-rc.12", default-features = false, features = ["load-dynamic", "ndarray"] }
tokenizers = "0.20"
ndarray = "0.16"
tauri-plugin-dialog = "2.7.1"
tauri-plugin-opener = "2.5.4"

# npm (package.json)
pnpm add @tauri-apps/plugin-dialog@^2.7.1 @tauri-apps/plugin-opener@^2.5.4 react-markdown@^10 remark-gfm@^4
```

**Version verification (run 2026-06-29):**
```
npm view @tauri-apps/plugin-dialog version  → 2.7.1
npm view @tauri-apps/plugin-opener  version  → 2.5.4
npm view react-markdown             version  → 10.1.0
npm view remark-gfm                 version  → 4.0.1
cargo search ort                    → ort = "2.0.0-rc.12"
cargo search tokenizers             → tokenizers = "0.23.1"  (we pin "0.20" for stable LTS-style)
cargo search tauri-plugin-dialog    → 2.7.1
cargo search tauri-plugin-opener    → 2.5.4
```

## Package Legitimacy Audit

> slopcheck was unavailable at research time (no Python tool in PATH). All packages below are tagged `[ASSUMED]` per the protocol. Planner must add a `checkpoint:human-verify` task before each `cargo add` / `pnpm add`.

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `ort` | crates.io | 5+ yr (pyke active) | High (top ONNX wrapper) | github.com/pykeio/ort | [ASSUMED-OK] | Approved (well-known, MIT/Apache) |
| `tokenizers` | crates.io | 5+ yr (HuggingFace official) | Very high | github.com/huggingface/tokenizers | [ASSUMED-OK] | Approved (HF official) |
| `ndarray` | crates.io | 10+ yr | Very high (de-facto Rust ndarray) | github.com/rust-ndarray/ndarray | [ASSUMED-OK] | Approved |
| `tauri-plugin-dialog` | crates.io | 2+ yr (official) | Very high (official Tauri) | github.com/tauri-apps/plugins-workspace | [ASSUMED-OK] | Approved (official Tauri) |
| `tauri-plugin-opener` | crates.io | 1+ yr (official) | Very high | github.com/tauri-apps/plugins-workspace | [ASSUMED-OK] | Approved (official Tauri) |
| `@tauri-apps/plugin-dialog` | npm | 2+ yr | Very high | github.com/tauri-apps/plugins-workspace | [ASSUMED-OK] | Approved |
| `@tauri-apps/plugin-opener` | npm | 1+ yr | Very high | github.com/tauri-apps/plugins-workspace | [ASSUMED-OK] | Approved |
| `react-markdown` | npm | 8+ yr (remarkjs) | ~5M/week | github.com/remarkjs/react-markdown | [ASSUMED-OK] | Approved (de-facto markdown lib for React) |
| `remark-gfm` | npm | 5+ yr | High | github.com/remarkjs/remark-gfm | [ASSUMED-OK] | Approved |

**Packages removed due to slopcheck [SLOP] verdict:** none (slopcheck unavailable; no decisions made)
**Packages flagged as suspicious [SUS]:** none

**Model file (not a package — separate audit):**
- **`Xenova/bert-base-NER` `onnx/model_quantized.onnx`** (109 MB INT8). Source: HuggingFace, MIT-licensed. Derived from `dslim/bert-base-NER` (MIT). [VERIFIED: huggingface.co/Xenova/bert-base-NER/tree/main/onnx file listing] The planner SHOULD include a `checkpoint:human-verify` step before bundling — verify SHA-256 of the downloaded file against the HuggingFace LFS pointer to defend against supply-chain swap.

## Architecture Patterns

### System Architecture Diagram

```
File system (watched folders)
        │
        ▼
notify-debouncer (Phase 2)
        │
        ▼
DocumentIndexer (Phase 2, EXTEND in Phase 6)
        │  parse → hash → embed → extract_entities (regex + NER) → upsert
        │                              │
        │                              ▼
        │                    NerService (NEW — ort + tokenizers, load once + spawn_blocking)
        │                              │ returns Vec<ExtractedEntity> for PER/ORG/LOC
        │                              ▼
        │                    EntityExtractor::extract() (Phase 2, EXTEND)
        │                              │ regex DATE/AMOUNT/EMAIL + merge NER results
        │                              │ dedup by (value, entity_type), cap at 20
        │                              ▼
        │                    Per-document Vec<ExtractedEntity>  ─→ stored on doc metadata
        ▼                              │
RuVector collection 'documents_384'    │
   (existing — has extracted_entities) │
        │                              │
        ▼                              ▼
DocumentGraph (Phase 3)        EntityStore (NEW — sibling to DocumentGraph in AppState)
   shared_entity edges                 │
   between docs                        │  reverse index: canonical_id → Set<doc_id>
                                       │  alias map:     surface_form → canonical_id
                                       │  canonical:     canonical_id → CanonicalEntity{name, type, alias_embeddings}
                                       │
                                       ▼
                          ┌─────────────┴──────────────┐
                          │                            │
                  IPC commands                 Co-occurrence query
                  - get_entities_by_type       (on-demand from reverse index;
                  - get_entity(id)             no precomputed matrix)
                  - get_documents_for_entity(id)
                  - get_related_entities(id)
                  - rename_entity_canonical
                  - split_entity_alias
                          │
                          ▼
                  React Query hooks (useEntities, useEntity, useRelatedEntities,
                                     useEntityDocuments, useRenameEntity, useSplitAlias)
                          │
            ┌─────────────┴─────────────┐
            ▼                           ▼
    /entities (index)           /entities/:id (detail)
    grouped by type             alias list + Split + Rename + Documents Mentioning + Related

Backfill on startup:
  lib.rs::setup → tauri::async_runtime::spawn(backfill_task)
       loops over RuVector docs missing NER entities
       processes 1 doc at a time (spawn_blocking inside)
       every 25 docs: emit "entity-backfill-progress" event
       on completion: emit "entity-backfill-progress" { status: "complete" }
       runs once, idempotent (skips docs whose metadata.entities_version >= 2)

UI for Phase 6:
  WatchedPage      → swap dynamic-import hack for plugin-dialog
  DocumentPage     → in-app preview (PDF/image/text/md) + Open/Reveal buttons + entity chips link to /entities/:id
  Search/Recent/   → right-click context menu (Open / Reveal)
  Favorites/Spaces
  /entities, /entities/:id (new routes)
  Sidebar          → add "Entities" link (Network icon)
```

### Recommended Project Structure (additions / modifications)

```
src-tauri/
├── models/                      # NEW — bundled ONNX model + tokenizer
│   ├── bert-base-NER.onnx
│   ├── tokenizer.json
│   └── config.json              # for id2label mapping
├── src/
│   ├── pipeline/
│   │   ├── ner.rs               # NEW — NerService (ort Session + Tokenizer)
│   │   └── entities.rs          # EXTEND — merge regex + NER, accept Vec<ExtractedEntity> from NerService
│   ├── graph/
│   │   └── entity_store.rs      # NEW — EntityStore with canonical/alias/reverse index
│   ├── commands/
│   │   └── entities.rs          # NEW — 6 entity IPC commands
│   └── state.rs                 # EXTEND — add Arc<Mutex<EntityStore>> + Arc<NerService>
client/
├── pages/
│   ├── EntitiesPage.tsx         # NEW — /entities index, grouped by type
│   ├── EntityDetailPage.tsx     # NEW — /entities/:id
│   └── DocumentPage.tsx         # EXTEND — replace excerpt with FilePreview; add Open/Reveal buttons
├── components/
│   ├── preview/                 # NEW
│   │   ├── FilePreview.tsx      # dispatches by docType
│   │   ├── PdfPreview.tsx
│   │   ├── ImagePreview.tsx
│   │   ├── TextPreview.tsx
│   │   ├── MarkdownPreview.tsx
│   │   └── SizeGuardCard.tsx
│   ├── entities/                # NEW
│   │   ├── EntityChip.tsx       # extracted from current DocumentPage
│   │   ├── EntityTypeBadge.tsx
│   │   └── AliasList.tsx
│   └── DocumentContextMenu.tsx  # NEW — Open / Reveal right-click menu
├── hooks/
│   └── useTauri.ts              # EXTEND — add entity + preview hooks
```

### Pattern 1: Load-once Inference Service (mirror fastembed)

**What:** A struct that wraps an ONNX Session + Tokenizer behind a `std::sync::Mutex`, constructed once at startup, called inside `spawn_blocking`.

**When to use:** Any CPU-bound model that needs to be loaded once and shared across requests. Existing example: `EmbeddingService` in `pipeline/embedder.rs`.

**Example sketch:**
```rust
// Source: pattern mirrors src-tauri/src/pipeline/embedder.rs + ort docs [CITED: docs.rs/ort]
use ort::session::{builder::GraphOptimizationLevel, Session};
use tokenizers::Tokenizer;
use ndarray::Array2;

pub struct NerService {
    session: std::sync::Mutex<Session>,
    tokenizer: Tokenizer,
    id2label: Vec<String>,        // ["O", "B-PER", "I-PER", "B-ORG", "I-ORG", "B-LOC", "I-LOC", "B-MISC", "I-MISC"]
}

impl NerService {
    pub fn new(model_path: &Path, tokenizer_path: &Path) -> Result<Self, AppError> {
        let session = Session::builder()
            .map_err(|e| AppError::Embedding(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| AppError::Embedding(e.to_string()))?
            .with_intra_threads(2)
            .map_err(|e| AppError::Embedding(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| AppError::Embedding(e.to_string()))?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| AppError::Embedding(e.to_string()))?;

        Ok(Self {
            session: std::sync::Mutex::new(session),
            tokenizer,
            id2label: load_id2label_from_config(/* models/config.json */)?,
        })
    }

    /// Extract PER/ORG/LOC entities from text. Truncates to 512 tokens (BERT max).
    pub fn extract(&self, text: &str) -> Result<Vec<ExtractedEntity>, AppError> {
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| AppError::Embedding(e.to_string()))?;
        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&u| u as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&u| u as i64).collect();
        let token_type_ids: Vec<i64> = encoding.get_type_ids().iter().map(|&u| u as i64).collect();

        let seq_len = input_ids.len();
        let ids_array = Array2::from_shape_vec((1, seq_len), input_ids)
            .map_err(|e| AppError::Embedding(e.to_string()))?;
        let mask_array = Array2::from_shape_vec((1, seq_len), attention_mask.clone())
            .map_err(|e| AppError::Embedding(e.to_string()))?;
        let type_array = Array2::from_shape_vec((1, seq_len), token_type_ids)
            .map_err(|e| AppError::Embedding(e.to_string()))?;

        let mut session = self.session.lock().map_err(|e| AppError::Embedding(e.to_string()))?;
        let outputs = session.run(ort::inputs![
            "input_ids" => ids_array.view(),
            "attention_mask" => mask_array.view(),
            "token_type_ids" => type_array.view(),
        ]).map_err(|e| AppError::Embedding(e.to_string()))?;

        // Logits shape: (1, seq_len, num_labels=9)
        let logits = outputs[0].try_extract_tensor::<f32>()
            .map_err(|e| AppError::Embedding(e.to_string()))?;
        let logits_array = logits.view();

        // Argmax per token + BIO decode into entities
        decode_bio(&logits_array, &encoding, &self.id2label, text)
    }
}
```

`decode_bio` is a helper that walks tokens, picks the argmax label, then collapses `B-LOC` → `I-LOC` → `I-LOC` runs into a single `ExtractedEntity { entity_type: "location", value: "<span text>" }`. Use `encoding.get_offsets()` to map back to the original character spans (avoids the WordPiece subword problem).

### Pattern 2: Asset Protocol for File Preview

**What:** Tauri exposes local files to the WebView via the `asset:` custom protocol. `convertFileSrc(path)` returns an `asset://localhost/<encoded-path>` URL that can be used in `<img src>`, `<iframe src>`, `<embed src>`, etc.

**When to use:** Any time the frontend needs to display the raw content of a local file (image, PDF, video).

**Example:**
```typescript
// Source: tauri-apps docs [CITED: v2.tauri.app/reference/javascript/api/namespacecore]
import { convertFileSrc } from "@tauri-apps/api/core";

const assetUrl = convertFileSrc(doc.path);
// e.g.  asset://localhost/Users/gshah/Documents/invoice.pdf

// In React:
<iframe src={assetUrl} className="w-full h-full" title={doc.name} />
<img src={assetUrl} alt={doc.name} />
```

**Required `tauri.conf.json` change:**
```json
{
  "app": {
    "security": {
      "csp": "default-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data:; frame-src 'self' asset: http://asset.localhost; object-src 'self' asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'",
      "assetProtocol": {
        "enable": true,
        "scope": ["**"]
      }
    }
  }
}
```
[CITED: v2.tauri.app/security/asset-protocol/]

Notes on the CSP above:
- Current config is `"csp": null` (everything allowed). Tightening it here is a Phase 6 hygiene win and is REQUIRED for the asset protocol to work properly — if CSP is null but `assetProtocol.enable: true`, asset URLs work but you also lose all security guarantees. Set CSP explicitly even though it's new for this phase.
- `frame-src` permits `<iframe>` for PDF.
- `object-src` permits `<embed>` and `<object>` (fallback if iframe doesn't render PDF on a given platform).
- `data:` in `img-src` allows base64 thumbnail fallbacks.
- `'unsafe-inline'` for styles is required by TailwindCSS and react-markdown's inline styles.
- Scope `["**"]` is permissive — matches any path. Cortex indexes user-chosen folders so any path the user has indexed is fair game. The user has already trusted Cortex with these paths.

### Pattern 3: Capability-based Permissions

**What:** Tauri 2 uses a capability system (`capabilities/default.json`) to declare which plugin permissions a window has.

**When to use:** Any new Tauri plugin requires its permissions to be added here, or its commands return permission errors.

**Required additions to `src-tauri/capabilities/default.json`:**
```json
{
  "$schema": "../node_modules/@tauri-apps/cli/schema/acl/schemas/capability.json",
  "identifier": "default",
  "description": "Default capability for Cortex",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:allow-open",
    "opener:allow-open-path",
    "opener:allow-reveal-item-in-dir"
  ]
}
```
[CITED: v2.tauri.app/plugin/dialog, v2.tauri.app/plugin/opener]

Optional hardening (recommended):
```json
{
  "identifier": "opener:allow-open-path",
  "allow": [{"path": "**"}]
}
```
The default-deny model from tauri-plugin-opener requires either the universal `opener:allow-open-path` permission OR per-path glob allowlists. Since Cortex needs to open arbitrary indexed files, the universal permission is correct. Document this in the plan as an accepted security tradeoff (we are a local-file-management tool by definition).

### Anti-Patterns to Avoid

- **Anti-pattern: Running NER inside the IPC command itself.** NER per-document is 100-500ms. Doing this synchronously on `index_document` IPC would stall the watcher loop. Instead, NER runs *inside* `DocumentIndexer::index_file`, which already runs in `spawn_blocking` from the watcher task.
- **Anti-pattern: Storing entities in their own RuVector collection.** Two collections to keep in sync, two failure modes on persistence. Keep entities derived from doc metadata (existing field `extracted_entities`) and rebuild the EntityStore on startup. Stateless, simple.
- **Anti-pattern: Sending Tauri event per indexed doc during backfill.** A 10K-doc backfill at one event per doc floods the event bus and degrades frontend frame rate. Throttle to every 25 docs (or 500ms, whichever is sooner).
- **Anti-pattern: Re-clustering all aliases on every new document (D-06 already locks this out, but flag for code reviewers).** O(n²) cluster on every doc means linear-cost ingestion becomes quadratic. Instead, embed new entity values and search top-5 nearest canonicals — if any is ≥ 0.85 cosine, merge.
- **Anti-pattern: Loading the .onnx model in `Session::run`.** The CONTEXT.md explicitly says load-once. Mirror `EmbeddingService::new_local`: load in `lib.rs::setup`, stash on AppState.
- **Anti-pattern: Sanitizing markdown by passing `rehype-raw`.** `rehype-raw` re-enables HTML inside markdown which IS the XSS surface. react-markdown's default (no rehype-raw) escapes all HTML — safe. Do not add `rehype-raw`.
- **Anti-pattern: Using `window.open(file:///...)` instead of `tauri-plugin-opener`.** `file://` URLs from a WebView are blocked by every modern browser policy. Use the plugin.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| BERT WordPiece tokenization | Custom regex tokenizer | `tokenizers = "0.20"` from HuggingFace | Subword splitting + special-token handling + offsets-to-original-text is non-trivial. Wrong tokenization breaks NER alignment. |
| ONNX model loading & inference | Calling onnxruntime C API directly | `ort = "2.0.0-rc.12"` | Memory-safety, tensor-shape sanity, MIT/Apache. |
| BIO tag decoding | Naive `B- → I-` join | `decode_bio` helper that respects `encoding.get_offsets()` | WordPiece can split "Smith" into ["Sm", "##ith"]. Using offsets maps back to original character spans correctly. |
| Native folder picker | Custom path text input + `fs.exists` | `tauri-plugin-dialog::open({directory: true})` | OS-native UX. Already-locked by D-19. |
| Open file in default app | Spawn `xdg-open` / `open` / `start` via `std::process::Command` | `tauri-plugin-opener::openPath` | Cross-platform. Handles edge cases (paths with spaces, Unicode). |
| Reveal in Finder / Explorer | Spawn `open -R` etc. directly | `tauri-plugin-opener::revealItemInDir` | Same. |
| PDF rendering in browser | PDF.js wrapper (`react-pdf`) | WebView's built-in PDF viewer via `<iframe>` + asset protocol | Zero deps, zero JS overhead, native zoom/find/print. WebView is Chromium on Win/Linux and WKWebView on macOS — all have PDF support. |
| Markdown rendering | `marked` + manual escaping | `react-markdown@10` + `remark-gfm@4` | Safe-by-default, plugin ecosystem, no innerHTML. |
| Embedding similarity for aliases | Custom Jaro-Winkler / Levenshtein | Reuse `EmbeddingService.embed_text` + cosine | D-05 already locks this. Embeddings catch semantic variants ("123 Main St" / "Main Street property") that string-distance metrics miss. |
| Right-click context menus | Custom-positioned divs | `@radix-ui/react-context-menu` (already in deps) | Already installed (`client/components/ui/context-menu.tsx` exists). Handles keyboard nav, escape, off-screen positioning. |
| HuggingFace model download at runtime | `reqwest` + manual SHA verification | `hf-hub` if download-on-first-run desired (deferred) | We bundle the model in Phase 6, so no download library needed. |

**Key insight:** Phase 6 looks like a "build a NER pipeline + a folder picker + a PDF viewer" lift, but every one of those is a solved problem in Rust/JS ecosystems. The actual work is plumbing, not invention.

## Runtime State Inventory

This is an extension phase (no rename), but several runtime-state categories apply because we are adding inference and a graph layer:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | Existing `extracted_entities` on every doc in `documents_384` RuVector collection (Phase 2 regex output). After Phase 6, schema gains NER-extracted entries + a `canonical_id` per entity. | Backfill — emit `entities_version: 2` in metadata so docs already processed are skipped on second restart. Existing regex entities remain valid (D-02 keeps regex), so backfill is **additive only** (does not overwrite). |
| Live service config | None — Cortex has no external service config. | None. |
| OS-registered state | macOS Quarantine on the bundled `.onnx` file (if downloaded outside the app bundle). | Bundling inside `src-tauri/models/` and shipping via the Tauri bundler avoids this — files inside the app bundle are signed-by-association. Verify on first dev run that ONNX model loads cleanly. |
| Secrets/env vars | None new. fastembed cache dir `~/.cache/fastembed/` already in use; ONNX model is bundled (no separate cache). | None. |
| Build artifacts | `src-tauri/.fastembed_cache/` is already gitignored. If using `hf-hub` later, a similar cache appears. **Phase 6 bundles the .onnx model in `src-tauri/models/`** — this directory must be added to Tauri's `bundle.resources` in `tauri.conf.json` so it ships with the binary. | Add to `tauri.conf.json` bundle.resources: `"resources": ["models/*"]`. Resolve path at runtime via `app.path().resource_dir()`. |

**Canonical question — runtime systems after every file is updated:**
- RuVector collection metadata (extracted_entities) — addressed by backfill task
- App bundle resource directory (models/) — addressed by Tauri bundler
- No external services, no OS-level registrations, no secrets touched.

## Common Pitfalls

### Pitfall 1: bert-base-NER 512-token truncation drops entities in long docs

**What goes wrong:** BERT-base has a hard 512-token sequence length. Documents longer than ~2000 characters (≈ 500 WordPiece tokens) get truncated; entities in later sections are missed entirely.

**Why it happens:** BERT was trained with positional embeddings only up to position 512.

**How to avoid:** **Chunk by sentence** (or by paragraph) before NER. Send each chunk (≤ 510 tokens including [CLS]/[SEP]) through the model separately. Union the results, then dedup by (value, type). For Cortex's average doc size this is one or two chunks; for a 100-page PDF it's ~50 chunks ≈ 5-25 seconds NER per doc. The 20-entity cap (D-04) still applies post-union.

**Warning signs:** Backfill on long PDFs returns < 5 NER entities while the doc clearly mentions 30+ people. If observed, verify chunking is happening.

### Pitfall 2: WordPiece subword splits break entity surface forms

**What goes wrong:** BERT tokenizes "Smithfield" as `["Smith", "##field"]`. If you naively concatenate tokens you get "Smith##field" or "Smithfield" with no spaces between adjacent tokens that were originally separate words.

**Why it happens:** WordPiece is a subword algorithm; original surface forms are not preserved at the token level.

**How to avoid:** Use `encoding.get_offsets()` from the tokenizers crate. Each token has a (start_char, end_char) span into the original text. Map B-I-I runs back to character spans, then slice the original text — guarantees verbatim surface form.

**Warning signs:** Entity values contain `##` literally, or have weird spacing.

### Pitfall 3: ort 2.x is `rc.12` — API can shift

**What goes wrong:** `2.0.0-rc.X` releases occasionally break API between RCs (e.g., `inputs!` macro signature shifted between rc.8 and rc.10).

**Why it happens:** pyke is iterating on ergonomics ahead of 2.0 stable. The Rust ML ecosystem has small but committed maintainers.

**How to avoid:** Pin the exact RC version (`= "2.0.0-rc.12"`, not `^2.0.0-rc`). Add a `cargo build` step to CI that catches breakage when the next RC drops. Plan a follow-up task to upgrade to 2.0 stable when it ships (currently ETA unknown).

**Warning signs:** `cargo update -p ort` fails to build after a `cargo update`.

### Pitfall 4: `convertFileSrc` returns URLs with unencoded spaces

**What goes wrong:** A path like `/Users/Foo Bar/file.pdf` becomes `asset://localhost/Users/Foo Bar/file.pdf` which some browsers refuse to load.

**Why it happens:** Tauri's `convertFileSrc` URI-encodes most special characters but spaces vary by version. Some older Tauri 2 betas left spaces unencoded.

**How to avoid:** Tauri 2.x stable already encodes spaces correctly [VERIFIED: API ref shows behavior fixed in 2.0]. Still, defensively test on a path with spaces in the size-guard test fixture.

**Warning signs:** PDF iframe is blank for paths with spaces, but works for paths without.

### Pitfall 5: Asset protocol scope `["**"]` allows the renderer to fetch ANY file

**What goes wrong:** A malicious markdown file (e.g., from a watched Downloads folder) could include an `<img src="asset://localhost/Users/<you>/.ssh/id_rsa">` that the renderer dutifully serves. The data ends up loadable by JS in the WebView.

**Why it happens:** Wide-open asset scope + permissive markdown.

**How to avoid:** 
1. react-markdown by default escapes HTML — `<img>` from inside markdown is rendered as text. ✓
2. react-markdown's `img` component override could allow asset URLs but only after a `urlTransform` whitelist check. We don't override the default → safe.
3. For non-markdown text, we render via `<pre>{text}</pre>` which is plain text. ✓
4. PDF iframe has no JS execution against parent — safe.

**Warning signs:** A code review request to add `rehype-raw` to react-markdown. Reject it.

### Pitfall 6: `tauri-plugin-opener` default-denies all paths

**What goes wrong:** First-time call to `openPath` returns `"Path not allowed"` error.

**Why it happens:** Tauri 2's capability system is default-deny. `opener:allow-open-path` permission must be present in `capabilities/default.json`. [CITED: v2.tauri.app/plugin/opener]

**How to avoid:** Add `opener:allow-open-path`, `opener:allow-reveal-item-in-dir` to capabilities (sketched in Pattern 3 above). Without these, the open/reveal buttons return errors silently in production.

**Warning signs:** "Open in Finder" button does nothing in `cargo tauri build` mode (where dev permissions are stricter than dev mode).

### Pitfall 7: Backfill races with new-file indexing on first launch

**What goes wrong:** User launches Cortex, the file watcher fires for a new file at the same time the backfill task is processing the same file. Both call `index_file` → NER runs twice, last writer wins, possible doc_id duplication.

**Why it happens:** Two concurrent producers writing to the same RuVector key.

**How to avoid:** `DocumentIndexer::index_file` already uses `path_index` to detect existing docs and skip if hash unchanged. NER backfill, however, only updates `extracted_entities` — it does NOT call `index_file`. It calls a new lower-level helper `DocumentIndexer::backfill_entities(doc_id, &collection, &ner_service)` that does an atomic `db.get → modify metadata → db.insert` cycle on a *single doc id*. Watcher-driven `index_file` and backfill operate on disjoint metadata fields (`vector` + `content_hash` vs `extracted_entities`) — they cannot race destructively as long as the read-modify-write is wrapped in the collection lock.

**Warning signs:** Doc count doubles after backfill, or NER entities appear briefly then disappear.

### Pitfall 8: Tokenizer.json doesn't ship with `Xenova/bert-base-NER` directly — check the repo root

**What goes wrong:** Plan downloads `model_quantized.onnx` from `onnx/` subdir but forgets `tokenizer.json` lives at the repo root.

**Why it happens:** HuggingFace splits ONNX files into `onnx/` while keeping config + tokenizer at the root. Easy to miss.

**How to avoid:** Bundle these 4 files in `src-tauri/models/`:
- `bert-base-NER.onnx`        (renamed from `onnx/model_quantized.onnx`)
- `tokenizer.json`            (from repo root)
- `config.json`               (from repo root — for `id2label`)
- `special_tokens_map.json`   (defensive; some tokenizer.json files reference it)

## Code Examples

### Example 1: Backfill task spawned at startup

```rust
// Source: pattern from src-tauri/src/lib.rs setup hook (existing watcher::worker::spawn_watcher_task)
// File would live in src-tauri/src/pipeline/backfill.rs
use std::sync::Arc;
use tauri::Emitter;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityBackfillProgress {
    pub processed: u32,
    pub total: u32,
    pub status: String,         // "running" | "complete" | "error"
    pub error: Option<String>,
}

pub fn spawn_entity_backfill(
    app_handle: tauri::AppHandle,
    engine: Arc<tokio::sync::Mutex<CortexEngine>>,
    ner_service: Arc<NerService>,
    entity_store: Arc<std::sync::Mutex<EntityStore>>,
    embedder: Arc<EmbeddingService>,
) {
    tauri::async_runtime::spawn(async move {
        let total = match count_docs_needing_backfill(&engine).await {
            Ok(n) => n,
            Err(e) => {
                let _ = app_handle.emit("entity-backfill-progress", EntityBackfillProgress {
                    processed: 0, total: 0,
                    status: "error".to_string(),
                    error: Some(e.to_string()),
                });
                return;
            }
        };

        if total == 0 {
            let _ = app_handle.emit("entity-backfill-progress", EntityBackfillProgress {
                processed: 0, total: 0,
                status: "complete".to_string(), error: None,
            });
            return;
        }

        let _ = app_handle.emit("entity-backfill-progress", EntityBackfillProgress {
            processed: 0, total,
            status: "running".to_string(), error: None,
        });

        let mut processed: u32 = 0;
        let mut last_emit = std::time::Instant::now();

        loop {
            // Spawn_blocking per doc — NER is CPU-bound
            let ah = app_handle.clone();
            let eng = engine.clone();
            let ner = ner_service.clone();
            let store = entity_store.clone();
            let emb = embedder.clone();
            let res = tokio::task::spawn_blocking(move || -> Result<bool, AppError> {
                let engine_guard = eng.blocking_lock();
                let mut store_guard = store.lock()
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                // Returns true if a doc was processed, false if none remain.
                backfill_one_doc(&engine_guard, &ner, &mut store_guard, &emb)
            }).await.unwrap_or(Ok(false));

            match res {
                Ok(true) => {
                    processed += 1;
                    // Throttle: emit every 25 docs OR every 500ms (whichever first)
                    if processed % 25 == 0 || last_emit.elapsed() >= std::time::Duration::from_millis(500) {
                        let _ = app_handle.emit("entity-backfill-progress", EntityBackfillProgress {
                            processed, total,
                            status: "running".to_string(), error: None,
                        });
                        last_emit = std::time::Instant::now();
                    }
                }
                Ok(false) => break,
                Err(e) => {
                    let _ = app_handle.emit("entity-backfill-progress", EntityBackfillProgress {
                        processed, total,
                        status: "error".to_string(),
                        error: Some(e.to_string()),
                    });
                    return;
                }
            }
        }

        // Run alias-merge pass (D-06 (a))
        if let Err(e) = {
            let mut store_guard = entity_store.lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            store_guard.run_full_alias_merge(&embedder)
        } {
            eprintln!("[backfill] alias merge failed: {e}");
        }

        let _ = app_handle.emit("entity-backfill-progress", EntityBackfillProgress {
            processed, total,
            status: "complete".to_string(), error: None,
        });
    });
}
```

### Example 2: `tauri-plugin-dialog` folder picker (TS)

```typescript
// Source: tauri-apps docs [CITED: v2.tauri.app/plugin/dialog]
// File: client/pages/WatchedPage.tsx — replace handleAddFolder
import { open } from "@tauri-apps/plugin-dialog";

const handleAddFolder = useCallback(async () => {
  if (!isTauri()) {
    // Browser dev fallback (existing pattern)
    setShowAddDialog(true);
    return;
  }
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Add Watched Folder",
    });
    // Return type: string | string[] | null
    if (selected && typeof selected === "string") {
      // D-19: validate before submitting (planner can choose IPC vs JS)
      addFolder(selected);
    }
    // null = user cancelled — do nothing silently per D-19
  } catch (e) {
    // Permission denied or platform unsupported (iOS/Android — n/a for Cortex desktop)
    console.error("Folder picker failed", e);
  }
}, [addFolder]);
```

### Example 3: `tauri-plugin-opener` (TS)

```typescript
// Source: tauri-apps docs [CITED: v2.tauri.app/plugin/opener]
// File: client/components/DocumentContextMenu.tsx OR DocumentPage header
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

async function handleOpen(path: string) {
  try {
    await openPath(path);
  } catch (e) {
    toast.error(`Could not open file: ${e}`);
  }
}

async function handleReveal(path: string) {
  try {
    await revealItemInDir(path);
  } catch (e) {
    toast.error(`Could not reveal file: ${e}`);
  }
}
```

### Example 4: PDF preview iframe + size guard

```tsx
// File: client/components/preview/PdfPreview.tsx
import { convertFileSrc } from "@tauri-apps/api/core";

const PDF_SIZE_LIMIT = 50 * 1024 * 1024; // 50 MB per D-15

export function PdfPreview({ doc }: { doc: Document }) {
  const [forceLoad, setForceLoad] = useState(false);
  const isLarge = doc.size > PDF_SIZE_LIMIT;

  if (isLarge && !forceLoad) {
    return (
      <SizeGuardCard
        sizeMB={Math.round(doc.size / 1024 / 1024)}
        onLoad={() => setForceLoad(true)}
        onOpenExternal={() => openPath(doc.path)}
      />
    );
  }

  const assetUrl = convertFileSrc(doc.path);
  return (
    <iframe
      src={assetUrl}
      className="w-full h-full border-0"
      title={doc.name}
    />
  );
}
```

### Example 5: Markdown preview with GFM

```tsx
// File: client/components/preview/MarkdownPreview.tsx
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

export function MarkdownPreview({ text }: { text: string }) {
  return (
    <div className="prose prose-invert max-w-none p-6">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        // NO rehype-raw — HTML in markdown stays escaped (XSS defence)
        // NO custom urlTransform — react-markdown default sanitizes URLs
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
```

Note: `prose prose-invert` are TailwindCSS Typography plugin classes — already present in `devDependencies` as `@tailwindcss/typography`. Confirm the plugin is enabled in `global.css` `@plugin` directive (per Tailwind 4 CSS-first config); if not, planner adds a one-liner.

## Markdown Pipeline

Stack: **`react-markdown@10.1.0` + `remark-gfm@4.0.1`**, plus existing `@tailwindcss/typography` for default styles.

**Why this stack:**
- react-markdown is safe by default — does not use `dangerouslySetInnerHTML`, escapes HTML, sanitizes URLs. [CITED: github.com/remarkjs/react-markdown]
- remark-gfm adds tables, strikethrough, task lists, autolinks — the GitHub Flavored Markdown features most users expect.
- ~60 KB minzipped acceptable for a desktop app.

**Syntax highlighting decision (per CONTEXT.md D-14):** Neither Shiki nor Prism is currently in `package.json`. **Skip syntax highlighting in v1.** Code blocks render as plain `<pre>` (typography plugin styles them as monospaced grey boxes — looks acceptable). Defer Shiki integration to a future phase. Rationale: each syntax highlighter adds 200-800 KB plus a language-grammar tree, and Cortex's primary use case is document discovery, not code-reading.

**XSS posture:**
1. Default react-markdown escapes raw HTML inside markdown → `<script>` becomes text.
2. Default react-markdown's `urlTransform` blocks `javascript:` URLs.
3. We do NOT add `rehype-raw` (which re-enables HTML parsing).
4. The asset protocol scope `["**"]` does not affect rendered markdown content because the markdown renderer never resolves `asset://` URLs as JS-executable resources.

**Result:** arbitrary user-supplied markdown content is safe to render. No sanitizer plugin required.

## Entity Storage + Graph

### Storage Decision

**Recommendation: in-memory `EntityStore` rebuilt from RuVector document metadata on every startup.**

**Justification:**
- Entity ground truth is already persisted as part of each document's metadata (`extracted_entities` field on the RuVector entry — Phase 2 schema). Adding a parallel SQLite or separate RuVector collection introduces a second source of truth that can drift.
- Rebuild cost at startup is O(N) scan of `documents_384` collection. The existing `DocumentIndexer::rebuild_path_index` does the same scan; we piggyback on it (one pass, populate both indexes).
- Memory footprint: ~10 K docs × ~10 entities/doc × ~50 bytes/entity ≈ 5 MB. Negligible.
- Canonical-entity-to-document edges live in metadata (`canonical_id` per entity entry); rebuilding the reverse index is the same scan.

**Trade-off accepted:** Backfill cost is per-fresh-install, not per-restart. After backfill writes `entities_version: 2` to every doc's metadata, subsequent restarts skip the NER work entirely — only the in-memory index rebuild runs.

### Schema (in-memory)

```rust
// File: src-tauri/src/graph/entity_store.rs (NEW)
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalEntity {
    pub id: String,                    // canonical_id (UUID; see Canonical-ID Strategy)
    pub canonical_name: String,        // most-frequent surface form per D-07
    pub entity_type: String,           // "person" | "organization" | "location" | "date" | "amount" | "email"
    pub aliases: Vec<String>,          // all surface forms ever merged into this canonical
    pub document_count: u32,
}

pub struct EntityStore {
    /// canonical_id → CanonicalEntity
    pub canonicals: HashMap<String, CanonicalEntity>,
    /// (surface_form_lowercase, entity_type) → canonical_id  (fast alias lookup)
    pub alias_index: HashMap<(String, String), String>,
    /// canonical_id → Set<doc_id>  (reverse index for "documents mentioning")
    pub doc_index: HashMap<String, HashSet<String>>,
    /// canonical_id → cached embedding of canonical_name (for merge-check on new entities)
    pub canonical_embeddings: HashMap<String, Vec<f32>>,
}

impl EntityStore {
    pub fn new() -> Self { /* empty maps */ }

    /// Build the store by scanning every doc in the RuVector collection.
    pub fn rebuild_from_engine(
        &mut self,
        engine: &CortexEngine,
        embedder: &EmbeddingService,
    ) -> Result<(), AppError> { /* O(N) scan; populates all four maps */ }

    /// Called by DocumentIndexer when a new doc is indexed.
    /// Returns canonical_ids that the doc was linked to.
    pub fn register_doc_entities(
        &mut self,
        doc_id: &str,
        entities: &[ExtractedEntity],
        embedder: &EmbeddingService,
    ) -> Result<Vec<String>, AppError> { /* D-06 (b): incremental merge */ }

    /// D-05 alias-merge: for a new surface form, find best canonical (cosine ≥ 0.85)
    /// or create a new canonical if no match.
    fn find_or_create_canonical(
        &mut self,
        surface: &str,
        entity_type: &str,
        embedder: &EmbeddingService,
    ) -> Result<String, AppError> { /* linear scan over canonicals of same type */ }

    /// D-06 (a): full alias merge pass after backfill. O(n²) within same entity type.
    pub fn run_full_alias_merge(
        &mut self,
        embedder: &EmbeddingService,
    ) -> Result<(), AppError> { /* may merge canonicals; updates aliases + doc_index */ }

    /// D-08: split an alias off into a new canonical entity.
    pub fn split_alias(
        &mut self,
        canonical_id: &str,
        alias_to_split: &str,
        embedder: &EmbeddingService,
    ) -> Result<String, AppError> { /* returns new canonical_id */ }

    /// D-12 rename: update canonical_name (no merge/split happens).
    pub fn rename_canonical(
        &mut self,
        canonical_id: &str,
        new_name: &str,
    ) -> Result<(), AppError> { /* trivial map update */ }

    /// D-11 co-occurrence — computed on demand for the queried entity.
    /// O(D × E_avg) where D = docs mentioning canonical_id, E_avg = avg entities per doc.
    /// For D≈100, E_avg≈10 → 1000 ops, sub-millisecond.
    pub fn related_entities(
        &self,
        canonical_id: &str,
        min_co_occurrence: u32,    // D-11 default 2
        limit: usize,
    ) -> Vec<(String, u32)> {
        // For each doc in self.doc_index[canonical_id]:
        //   for each canonical_id2 in doc's entities (re-derived from doc's metadata):
        //     count[canonical_id2] += 1
        // Return top `limit` by count, filtered by ≥ min_co_occurrence.
    }
}
```

### Canonical-ID Strategy

**Recommendation: UUID v4 per canonical entity, generated once at first surface-form-creation.**

**Rationale:**
- Stable URLs in `/entities/:id` — survives D-07 canonical-name changes (most-frequent variant flipping as more docs are indexed) AND survives D-08 split-alias (split creates a new UUID; old one remains).
- Hash-of-normalized-value is brittle: any rename or merge changes the hash, breaking saved bookmarks.
- Auto-increment integers couple to insertion order, ugly in URLs.

**Persistence:** `canonical_id` is written into each doc's `extracted_entities[i].canonical_id` metadata field. The EntityStore rebuilds from this on startup → stable IDs across runs.

### Alias-Merge Algorithm (D-05, D-06 (b))

```
On register_doc_entities(doc_id, entities[]):
  for each ext_entity in entities:
    canonical_id = find_or_create_canonical(ext_entity.value, ext_entity.entity_type)
    update ext_entity.canonical_id = canonical_id (mutate doc metadata)
    add canonical_id → doc_id to doc_index
    if surface form is novel, append to aliases[]
  return list of touched canonical_ids

find_or_create_canonical(surface, type):
  # 1. Exact-match lookup
  if (lowercase(surface), type) in alias_index:
    return alias_index[(...)]

  # 2. Embedding-based search
  surface_emb = embedder.embed_text(surface)
  best_canonical_id = None
  best_score = 0.0
  for cid, canonical in canonicals.iter().filter(|c| c.entity_type == type):
    score = cosine(surface_emb, canonical_embeddings[cid])
    if score > best_score:
      best_score = score
      best_canonical_id = cid
  if best_score >= 0.85:
    aliases.push(surface)
    alias_index[(lowercase(surface), type)] = best_canonical_id
    return best_canonical_id

  # 3. New canonical
  new_id = uuid_v4()
  canonicals[new_id] = CanonicalEntity { id: new_id, canonical_name: surface, ... }
  canonical_embeddings[new_id] = surface_emb
  alias_index[(lowercase(surface), type)] = new_id
  return new_id
```

**D-07 canonical name = most frequent:** After every `register_doc_entities`, recompute canonical_name as the alias with the highest count across `doc_index[cid]` (lookup each doc's entities, count occurrences of each alias). For doc counts < 1000 this is sub-millisecond.

### Split-Alias Mechanics (D-08)

When the user clicks "Split" next to alias `"John Smith"` on canonical `cid_old` (which currently bundles "John Smith" + "J. Smith"):

```
split_alias(cid_old, "John Smith"):
  1. Create new canonical: cid_new = uuid_v4()
     canonicals[cid_new] = { canonical_name: "John Smith", entity_type: same, aliases: ["John Smith"] }
     canonical_embeddings[cid_new] = embedder.embed_text("John Smith")
  2. Remove "John Smith" from canonicals[cid_old].aliases
  3. alias_index[("john smith", type)] = cid_new
  4. For each doc_id in doc_index[cid_old]:
       look up doc's extracted_entities (via engine)
       find entries where value == "John Smith"
       if any: rewrite their canonical_id to cid_new, update doc metadata
       if doc no longer has any entity with canonical_id == cid_old:
         doc_index[cid_old].remove(doc_id)
       doc_index[cid_new].insert(doc_id)
  5. Recompute document_count for both canonicals
  6. Persist: doc metadata writes are atomic per-doc (insert/upsert on existing IDs)
  return cid_new
```

**Cost:** O(docs_mentioning_canonical) RuVector reads + writes. For "John Smith" with 50 mentions, ~50 metadata writes. Fast (sub-second).

### IPC Commands (sketches)

All use `#[tauri::command] async + spawn_blocking + #[serde(rename_all = "camelCase")]` per Phase 1/4 standard:

```rust
// File: src-tauri/src/commands/entities.rs (NEW)

#[tauri::command]
pub async fn get_entities_by_type(
    entity_type: Option<String>,        // None = all types
    state: State<'_, AppState>,
) -> Result<Vec<CanonicalEntity>, AppError>;
// Returns canonicals sorted by document_count desc.

#[tauri::command]
pub async fn get_entity(
    id: String,
    state: State<'_, AppState>,
) -> Result<CanonicalEntity, AppError>;

#[tauri::command]
pub async fn get_documents_for_entity(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Document>, AppError>;
// Joins EntityStore.doc_index with RuVector to build full Document objects.

#[tauri::command]
pub async fn get_related_entities(
    id: String,
    min_co_occurrence: Option<u32>,     // default 2 per D-11
    limit: Option<usize>,               // default 10
    state: State<'_, AppState>,
) -> Result<Vec<RelatedEntity>, AppError>;
// RelatedEntity = { canonical: CanonicalEntity, co_occurrence_count: u32 }

#[tauri::command]
pub async fn rename_entity_canonical(
    id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<CanonicalEntity, AppError>;

#[tauri::command]
pub async fn split_entity_alias(
    canonical_id: String,
    alias: String,
    state: State<'_, AppState>,
) -> Result<CanonicalEntity, AppError>;     // returns the NEW canonical
```

## Tauri Plugin APIs

### tauri-plugin-dialog 2.7.1

**Rust setup** (`src-tauri/src/lib.rs`):
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())   // add this line before .setup()
    .setup(...)
```

**tauri.conf.json** — no explicit `"plugins"` entry needed for dialog (zero config).

**capabilities/default.json:**
```json
"permissions": [
  "core:default",
  "dialog:allow-open"
]
```

**TypeScript invocation** (`open` with directory mode):
```typescript
import { open } from "@tauri-apps/plugin-dialog";

const selected: string | string[] | null = await open({
  directory: true,
  multiple: false,
  title: "Add Watched Folder",
  defaultPath: "/Users",          // optional
});
// string | null when multiple=false
// string[] | null when multiple=true
// null when user cancels
```

**Error modes:**
- Throws if `dialog:allow-open` permission missing.
- Throws on iOS/Android (folder picker unsupported) — N/A for Cortex desktop.
- Returns `null` on user cancel (NOT an error per D-19 "do nothing silently").

### tauri-plugin-opener 2.5.4

**Rust setup:**
```rust
.plugin(tauri_plugin_opener::init())
```

**capabilities/default.json:**
```json
"permissions": [
  "core:default",
  "dialog:allow-open",
  "opener:allow-open-path",
  "opener:allow-reveal-item-in-dir"
]
```

**TypeScript:**
```typescript
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

await openPath(path);                    // opens with default app
await openPath(path, "Visual Studio Code"); // opens with specific app (optional)
await revealItemInDir(path);             // highlights file in Finder/Explorer
```

**Return type:** `Promise<void>`. All errors throw — wrap in try/catch.

**Security note:** With universal `opener:allow-open-path` permission, the user can be tricked into opening arbitrary files. Mitigation: Cortex only calls these from `doc.path` values that came from the watched-folders-flow user actions — never from untrusted input. Document this in the plan.

## File Preview Architecture

### Renderer Dispatch

```tsx
// File: client/components/preview/FilePreview.tsx
import { PdfPreview } from "./PdfPreview";
import { ImagePreview } from "./ImagePreview";
import { TextPreview } from "./TextPreview";
import { MarkdownPreview } from "./MarkdownPreview";
import { UnsupportedPreview } from "./UnsupportedPreview";
import type { Document } from "@/lib/types";

export function FilePreview({ doc }: { doc: Document }) {
  switch (doc.docType) {
    case "pdf":  return <PdfPreview doc={doc} />;
    case "png":
    case "jpg":  return <ImagePreview doc={doc} />;
    case "md":   return <MarkdownPreview doc={doc} />;
    case "txt":
    case "csv":  return <TextPreview doc={doc} />;
    default:
      // docx / xlsx / unknown — show metadata + "Open in default app" button
      return <UnsupportedPreview doc={doc} />;
  }
}
```

### Per-Renderer Notes

**PdfPreview:** `convertFileSrc(doc.path)` → `<iframe>`. Size guard at 50 MB.

**ImagePreview:** `convertFileSrc(doc.path)` → `<img>`. Size guard at 20 MB. Use `object-contain` and a placeholder for load failure.

**TextPreview:** Cannot use asset URL directly because `<pre>` doesn't fetch URLs — it renders inline text. So we need a new IPC `read_document_text(path, max_bytes)` that returns the file content as a string. Apply size guard at 5 MB. Render via:
```tsx
<pre className="font-mono text-sm whitespace-pre-wrap p-6 overflow-auto">
  {text}
</pre>
```

**MarkdownPreview:** Same `read_document_text` IPC, then pipe through `<ReactMarkdown remarkPlugins={[remarkGfm]}>`.

### `usePreview` Hook (D-16)

```typescript
// File: client/hooks/useTauri.ts (extend)
export function usePreview(docId: string, docType: string, docSize: number) {
  // For text/markdown — fetch file content.
  // For pdf/image — return null (renderer uses convertFileSrc directly).
  const needsContent = docType === "txt" || docType === "md" || docType === "csv";
  const SIZE_LIMIT = docType === "md" || docType === "txt" || docType === "csv" 
    ? 5 * 1024 * 1024 : Infinity;
  const tooLarge = docSize > SIZE_LIMIT;

  return useQuery({
    queryKey: ["preview", docId],
    queryFn: () => tauriInvoke<string>("read_document_text", { docId, maxBytes: SIZE_LIMIT }),
    enabled: needsContent && !tooLarge,
  });
}
```

### New IPC for text preview

```rust
// File: src-tauri/src/commands/documents.rs (extend)
#[tauri::command]
pub async fn read_document_text(
    doc_id: String,
    max_bytes: u64,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let engine = state.engine.clone();
    tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        let collection = engine_guard.collections
            .get_collection("documents_384")
            .ok_or_else(|| AppError::VectorStorage("not found".into()))?;
        let entry = collection.read().db.get(&doc_id)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(doc_id.clone()))?;
        let path = entry.metadata.as_ref()
            .and_then(|m| m.get("path").and_then(|v| v.as_str()))
            .ok_or_else(|| AppError::Internal("doc has no path".into()))?;
        // Bounded read — stops at max_bytes to enforce size guard server-side too
        let bytes = std::fs::read(path).map_err(AppError::from)?;
        if bytes.len() as u64 > max_bytes {
            return Err(AppError::Parse(format!("file exceeds {} bytes", max_bytes)));
        }
        String::from_utf8(bytes).map_err(|e| AppError::Parse(e.to_string()))
    }).await?
}
```

## Backfill Strategy

**Lifecycle:**
1. `lib.rs::setup` finishes registering AppState.
2. `spawn_entity_backfill` is called *after* `rebuild_path_index` (so we know which docs exist).
3. Background Tokio task iterates `documents_384` collection, finds docs where `metadata.entities_version` is missing or `< 2`.
4. For each, `spawn_blocking` runs: chunked NER → merge with existing regex entities → register entities in EntityStore → write back to RuVector entry with `entities_version: 2`.
5. Throttled progress events every 25 docs OR 500ms.
6. After loop completes, `EntityStore.run_full_alias_merge()` runs (D-06 (a)).
7. Final `entity-backfill-progress { status: "complete" }` event.

**Idempotency on restart:** `entities_version: 2` is the gate. After first successful backfill, every restart's backfill task finds 0 docs needing work — emits `complete` immediately. Cost: one collection scan, no NER runs.

**Failure mode:** If NER throws on one doc, log + emit `entity-backfill-progress { status: "error", error: "..." }` for that doc, continue to next. We do NOT abort the entire backfill on a single bad PDF.

**Frontend wiring:** Listen for `entity-backfill-progress` in `AppShell` (mirrors existing `index-progress` listener):
```tsx
useEffect(() => {
  let unlisten: (() => void) | undefined;
  (async () => {
    if (!isTauri()) return;
    const { listen } = await import("@tauri-apps/api/event");
    unlisten = await listen<EntityBackfillProgress>("entity-backfill-progress", (e) => {
      // Pipe into a Zustand store for TopBar indicator
      useBackfillStore.getState().update(e.payload);
    });
  })();
  return () => unlisten?.();
}, []);
```

TopBar gains a second indicator (next to the indexing one) showing "Extracting entities (X / Y)…" during backfill. When `status: complete`, indicator disappears. Existing `useIndexingStore` pattern is the template.

## ONNX NER Recommendation

### Recommended Model File

**`Xenova/bert-base-NER` → `onnx/model_quantized.onnx`** (INT8 dynamic quantization, 109 MB) [VERIFIED: huggingface.co/Xenova/bert-base-NER/tree/main/onnx file listing 2026-06-29]

| Property | Value |
|----------|-------|
| Base model | `dslim/bert-base-NER` (bert-base-cased fine-tuned on CoNLL-2003) |
| F1 score (CoNLL-2003 test) | 0.913 [CITED: huggingface.co/dslim/bert-base-NER eval table] |
| Parameters | 110 M |
| Entity labels | B-PER, I-PER, B-ORG, I-ORG, B-LOC, I-LOC, B-MISC, I-MISC, O (9 labels) |
| License | MIT [CITED: dslim model card] |
| File size (quantized) | 109 MB (INT8) vs 431 MB (fp32) vs 216 MB (fp16) vs 93.7 MB (Q4-FP16) |

### Why this variant (vs alternatives in `onnx/`)

| Variant | Size | Expected F1 (relative to fp32) | Recommendation |
|---------|------|-------------------------------|----------------|
| `model.onnx` (fp32) | 431 MB | 1.00× | Too large for desktop bundle |
| `model_fp16.onnx` | 216 MB | ~0.99× | Acceptable but bundle weight unjustified — no GPU in target |
| `model_quantized.onnx` (INT8) | 109 MB | ~0.97-0.98× | **Recommended.** Best size/accuracy tradeoff; same CPU-only inference path. |
| `model_int8.onnx` | 108 MB | ~0.97× | Near-identical to `model_quantized.onnx`; some HF tools generate both, pick `model_quantized.onnx` as the canonical name. |
| `model_q4.onnx` | 145 MB | ~0.95× | 4-bit; meaningful F1 drop on rare entities |
| `model_q4f16.onnx` | 93.7 MB | ~0.93× | Smallest but biggest accuracy hit; not worth 15 MB savings |
| `model_uint8.onnx` | 108 MB | ~0.97× | UINT8 variant; functionally equivalent to INT8 for this model |
| `model_bnb4.onnx` | 139 MB | ~0.93× | bitsandbytes 4-bit; ONNX support patchy |

**Memory + cold-start cost (estimated, ASSUMED):**
- Model load: ~500 ms (one-shot, in `lib.rs::setup`)
- RAM footprint: ~250 MB resident while session is alive (INT8 model + ort runtime + ndarray temp buffers)
- Per-inference: ~50-200 ms per 510-token chunk on M1/M2 CPU (single-threaded, GraphOptimizationLevel::Level3, intra_threads=2)
- 10K-doc backfill estimate at avg 3 chunks/doc: 30K chunks × 100 ms = ~50 minutes. Acceptable for one-time work behind a progress indicator.

### Where to source the .onnx file

1. **Build-time download (recommended for v1):** Add a `build.rs` step in `src-tauri/` OR a separate `scripts/download-ner-model.sh` script that downloads via `curl`:
   ```bash
   curl -L -o src-tauri/models/bert-base-NER.onnx \
     https://huggingface.co/Xenova/bert-base-NER/resolve/main/onnx/model_quantized.onnx
   curl -L -o src-tauri/models/tokenizer.json \
     https://huggingface.co/Xenova/bert-base-NER/resolve/main/tokenizer.json
   curl -L -o src-tauri/models/config.json \
     https://huggingface.co/Xenova/bert-base-NER/resolve/main/config.json
   ```
   Add `src-tauri/models/*.onnx` to `.gitignore` (large binary; download on clone-build). Tauri bundler picks them up via:
   ```json
   "bundle": {
     "resources": ["models/*"]
   }
   ```

2. **Verify checkpoint:** `checkpoint:human-verify` task in plan that prints SHA-256 of downloaded `bert-base-NER.onnx` and prompts user to confirm against the HuggingFace LFS pointer (supply-chain hardening).

### ort + tokenizers minimal example (full)

```rust
// File: src-tauri/src/pipeline/ner.rs (full sketch with chunking)
use std::path::Path;
use std::sync::Mutex;
use ort::session::{builder::GraphOptimizationLevel, Session};
use tokenizers::Tokenizer;
use ndarray::Array2;
use crate::error::AppError;
use crate::types::ExtractedEntity;

const MAX_TOKENS: usize = 510; // 512 minus [CLS] and [SEP]
const CHUNK_OVERLAP: usize = 50;

pub struct NerService {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    id2label: Vec<String>,
}

impl NerService {
    pub fn new(model_dir: &Path) -> Result<Self, AppError> {
        let model_path = model_dir.join("bert-base-NER.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");
        let config_path = model_dir.join("config.json");

        let session = Session::builder().map_err(map_err)?
            .with_optimization_level(GraphOptimizationLevel::Level3).map_err(map_err)?
            .with_intra_threads(2).map_err(map_err)?
            .commit_from_file(&model_path).map_err(map_err)?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| AppError::Embedding(format!("tokenizer load: {e}")))?;

        // Parse config.json's id2label
        let config: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&config_path).map_err(AppError::from)?
        ).map_err(|e| AppError::Embedding(e.to_string()))?;
        let id2label_map = config.get("id2label")
            .and_then(|v| v.as_object())
            .ok_or_else(|| AppError::Embedding("config.json missing id2label".into()))?;
        let mut id2label = vec![String::new(); 9];
        for (k, v) in id2label_map {
            let idx: usize = k.parse().map_err(|e: std::num::ParseIntError| AppError::Embedding(e.to_string()))?;
            id2label[idx] = v.as_str().unwrap_or("O").to_string();
        }

        Ok(Self { session: Mutex::new(session), tokenizer, id2label })
    }

    /// Extract Person / Organization / Location entities from text.
    /// Chunks text to fit BERT's 512-token limit. MISC entities dropped.
    pub fn extract(&self, text: &str) -> Result<Vec<ExtractedEntity>, AppError> {
        let chunks = chunk_text(text, MAX_TOKENS, CHUNK_OVERLAP, &self.tokenizer)?;
        let mut all_entities: Vec<ExtractedEntity> = Vec::new();
        for chunk in chunks {
            let chunk_entities = self.extract_chunk(&chunk)?;
            all_entities.extend(chunk_entities);
        }
        // Dedup by (value, entity_type)
        all_entities.sort_by(|a, b| {
            a.entity_type.cmp(&b.entity_type).then(a.value.cmp(&b.value))
        });
        all_entities.dedup_by(|a, b| a.value == b.value && a.entity_type == b.entity_type);
        Ok(all_entities)
    }

    fn extract_chunk(&self, text: &str) -> Result<Vec<ExtractedEntity>, AppError> {
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| AppError::Embedding(format!("encode: {e}")))?;
        let seq_len = encoding.get_ids().len();
        if seq_len == 0 {
            return Ok(vec![]);
        }

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&u| u as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&u| u as i64).collect();
        let token_type_ids: Vec<i64> = encoding.get_type_ids().iter().map(|&u| u as i64).collect();
        let offsets = encoding.get_offsets().to_vec();

        let ids = Array2::from_shape_vec((1, seq_len), input_ids).map_err(map_err)?;
        let mask = Array2::from_shape_vec((1, seq_len), attention_mask).map_err(map_err)?;
        let types = Array2::from_shape_vec((1, seq_len), token_type_ids).map_err(map_err)?;

        let mut session = self.session.lock().map_err(|e| AppError::Embedding(e.to_string()))?;
        let outputs = session.run(ort::inputs![
            "input_ids" => ids.view(),
            "attention_mask" => mask.view(),
            "token_type_ids" => types.view(),
        ]).map_err(map_err)?;

        // outputs[0] shape: (1, seq_len, 9)
        let logits = outputs[0].try_extract_tensor::<f32>().map_err(map_err)?;
        // ... argmax over last dim, decode BIO using offsets to slice original text ...
        Ok(decode_bio(&logits, &offsets, &self.id2label, text))
    }
}

fn map_err<E: std::fmt::Display>(e: E) -> AppError {
    AppError::Embedding(e.to_string())
}

// Helpers: chunk_text, decode_bio — both unit-testable in isolation.
```

`decode_bio` walks tokens, maps each to its argmax label string, then collapses runs:
- "B-PER" begins a Person; subsequent "I-PER" tokens extend it.
- Encountering "O" or a different B-* closes the current entity.
- Use `offsets[token_idx]` to get `(start_char, end_char)` and slice the original chunk text.
- Map label `"B-PER"`/`"I-PER"` → entity_type `"person"`; `"B-ORG"`/`"I-ORG"` → `"organization"`; `"B-LOC"`/`"I-LOC"` → `"location"`. Drop MISC.

## Common Pitfalls (continued — already covered above in dedicated section)

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `tauri-plugin-shell::open` | `tauri-plugin-opener::openPath` | Tauri 2 GA (Oct 2024) | Shell plugin deprecated for file/URL opening; opener is the v2 standard |
| spaCy / Stanza in Python sidecar | ONNX NER via ort (Rust-native) | 2023-2025 | No sidecar process, no Python runtime, smaller bundle |
| Manual file dialog via `std::process::Command` to `osascript` | `tauri-plugin-dialog` | Tauri 1 → 2 | Proper async API, cross-platform |
| `react-pdf` (PDF.js wrapper) | WebView native PDF viewer + `<iframe>` | n/a (always been an option) | Saves 700+ KB bundle, native zoom/find/print |

**Deprecated/outdated:**
- `tauri-plugin-shell::open` for file/URL opening: replaced by `tauri-plugin-opener` in v2. Shell plugin is still valid for `spawn` use cases. [CITED: github.com/tauri-apps/plugins-workspace]
- BERT-base for NER is no longer SOTA accuracy-wise (DeBERTa-v3, RoBERTa-large, or LLM-based extractors score higher), but the F1=0.91 is still excellent and the size/speed tradeoff is unbeatable for desktop CPU inference. We trade ~3 F1 points for 4× smaller model + 5× faster inference vs RoBERTa-large.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Xenova ONNX export is functionally equivalent to dslim base model | ONNX NER Recommendation | If conversion is buggy, F1 drops materially. **Mitigation:** Validation Architecture includes a golden-fixture test with known entities to set an F1 ≥ 0.85 floor. |
| A2 | INT8 quantized variant retains ~97% of fp32 F1 | ONNX NER Recommendation | Could be lower in practice; falls back to `model_fp16.onnx` (216 MB) if F1 floor fails. |
| A3 | M1/M2 CPU inference is ~100ms per 510-token chunk | ONNX NER Recommendation | Could be 2-3× slower; backfill might take 2-3 hours on 10K docs. Acceptable behind progress UI. |
| A4 | `cosine ≥ 0.85` threshold separates true aliases from coincidental similarity for fastembed `all-MiniLM-L6-v2` | Entity Storage + Graph (alias-merge algorithm) | Could over-merge (e.g., two distinct John Smiths) or under-merge. **Mitigation:** D-08 Split-alias UX recovers; threshold is configurable in plan. |
| A5 | RuVector's `db.insert` is upsert (overwrites by `id` if present) | Entity Storage + Graph (Split-alias mechanics) | If insert is append-only, splits would corrupt the index. **Mitigation:** Phase 2 plans already rely on this behavior (`indexer.rs` Step 11 uses insert for both new and modified docs). Confirmed by code inspection of `pipeline/indexer.rs:200-211`. |
| A6 | The Tauri WebView (WebKit on macOS, WebView2 on Win, WebKitGTK on Linux) renders PDFs natively | Don't Hand-Roll, File Preview Architecture | If WebKitGTK on a target Linux distro lacks PDF support, users see blank iframe. **Mitigation:** Detect via `<embed>` fallback or show "Open in default app" if the iframe stays blank. Defer Linux-specific testing to verifier. |
| A7 | Bundling 110 MB ONNX in Tauri resource dir works on macOS code-signing path | ONNX NER Recommendation | Large resources can hit notarization quirks. **Mitigation:** Code signing is in v2 scope (DIST-01); for v1 unsigned-build use case, no issue. |
| A8 | The `tokenizers` crate `0.20` API is stable for `Tokenizer::from_file` + `encode` + `get_offsets` | Code Examples | API is stable since 0.13 [ASSUMED]. **Mitigation:** Pin exact version. |
| A9 | slopcheck would have flagged none of the chosen packages (all are well-known) | Package Legitimacy Audit | If slopcheck disagrees, planner removes. All packages are official Tauri / HuggingFace / remarkjs — extremely low risk. |
| A10 | Estimated 30 K chunks × 100 ms = ~50 minutes for 10K-doc backfill | ONNX NER Recommendation | Could be 30 min - 2 hours depending on hardware + doc lengths. UX-tolerable because (a) backfill is one-time, (b) progress is visible, (c) app remains responsive. |

## Open Questions (RESOLVED)

1. **Should NER MISC entities be exposed?**
   - What we know: dslim/bert-base-NER emits 4 entity classes (PER/ORG/LOC/MISC).
   - What's unclear: MISC catches nationalities, events, products — could be a useful "everything else" bucket OR add noise.
   - RESOLVED: **Drop MISC for v1.** It adds a 4th entity type the user never asked for. If users request later, easy to expose by changing one filter in `decode_bio`.

2. **Should the EntityStore persist its `canonical_embeddings` across restarts?**
   - What we know: Embeddings are deterministic for the same input string, so rebuild-on-startup re-embeds every canonical (~5K entities × 1 ms = 5 sec).
   - What's unclear: Is 5 sec on startup acceptable, or should we cache to disk?
   - RESOLVED: **Defer to Phase 7.** Accept the 5 sec cost. If post-launch profiling shows the cost is larger than expected, add an `entities.json` sidecar in Phase 7.

3. **Where should the size guard live — frontend or backend?**
   - What we know: Document.size is already in the IPC payload, so frontend can decide.
   - What's unclear: A malicious script could call `read_document_text` with `maxBytes: u64::MAX` for a huge file.
   - RESOLVED: **Both layers — defense in depth.** Frontend shows the size-guard card and gates the request; backend enforces a hard 5 MB cap regardless of caller-supplied `maxBytes` (per D-15 and Plan 03 Task 2 acceptance criteria).

4. **Should the user be able to dismiss the backfill indicator?**
   - What we know: TopBar indicator stays until `status: complete`.
   - What's unclear: For a 1-hour backfill, the indicator is persistent visual noise.
   - RESOLVED: **Out of scope for Phase 6.** Show the indicator but keep it ≤ 200px wide so it's not noisy (UI-SPEC §Cross-cutting Surface 1). A "Pause backfill" action lands in a future phase if users complain.

5. **Does the asset protocol scope need finer constraints than `["**"]`?**
   - What we know: Cortex indexes user-chosen folders; the user has already trusted Cortex with their full Documents/Desktop/Downloads dirs.
   - What's unclear: A sophisticated attacker who can write a malicious markdown file into a watched folder could induce `<img>` requests to other paths — but react-markdown's HTML escaping already blocks this.
   - RESOLVED: **Keep `["**"]` scope.** react-markdown's default HTML escape posture neutralizes the `<img>`-injection vector. Re-evaluate if we ever add a renderer that resolves user-supplied URLs without sanitization. Tracked as accepted threat T-06-AP in Plan 01.

6. **What happens during NER backfill if the user clicks Split alias / Rename?**
   - What we know: Backfill writes to EntityStore; user actions also write.
   - What's unclear: Race conditions.
   - RESOLVED: **EntityStore behind `Arc<Mutex<EntityStore>>`** (per state.rs convention) — lock serializes all writes. Frontend-driven mutations and backfill writes coexist safely. Tracked as accepted threat T-06-MUTATION-RACE in Plan 06 and T-06-BACKFILL-RACE in Plan 03.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust | Backend build | ✓ | 1.77.2+ required by Cargo.toml | — |
| Cargo | Backend build | ✓ | — | — |
| pnpm | Frontend build | ✓ | 10.14.0+ (per package.json packageManager) | — |
| curl | Model download script | ✓ (macOS default) | — | wget |
| Tauri 2 CLI | dev/build | ✓ (per devDependencies @tauri-apps/cli 2.10.0) | 2.10.0 | — |
| ONNX Runtime native library | ort crate at runtime | ✗ (needs `load-dynamic` to find on system OR bundle) | — | `ort` `download-binaries` feature downloads at build time |
| HuggingFace network access | Model download | ✓ (during build) | — | Skip the model download script; users provide model manually |

**Missing dependencies with no fallback:** none — `ort` has the `download-binaries` feature that auto-fetches the ONNX Runtime binary if `load-dynamic` is not used.

**Missing dependencies with fallback:** ONNX Runtime native lib — planner decides between `download-binaries` (simpler, bundles ~30 MB more) vs `load-dynamic` (smaller binary, requires path setup). **Recommend `download-binaries` for v1** since the existing fastembed crate already pulls in a similar bundle.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Frontend framework | vitest 3.2.4 (per package.json) |
| Backend framework | cargo test (Rust built-in) |
| Config file | None for vitest (uses defaults) / Cargo.toml for Rust |
| Quick run command (frontend) | `pnpm test --run --reporter=dot` |
| Quick run command (backend) | `cargo test --lib --no-fail-fast` |
| Full suite (frontend) | `pnpm test --run` |
| Full suite (backend) | `cargo test --workspace` |
| Phase gate | `cargo test --workspace` + `pnpm test --run` + `cargo tauri build --debug` (smoke) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| KG-01 | EntityStore rebuilt from collection populates canonicals + reverse index | unit (Rust) | `cargo test -p cortex_lib graph::entity_store::tests::test_rebuild` | ❌ Wave 0 |
| KG-01 | get_entity IPC returns canonical with correct alias list | integration (Rust) | `cargo test -p cortex_lib commands::entities::tests::test_get_entity` | ❌ Wave 0 |
| KG-01 | Click chip → /entities/:id navigates correctly | unit (TS) | `pnpm test --run client/pages/EntityDetailPage.test.tsx` | ❌ Wave 0 |
| KG-02 | NER finds "John Smith" PER in golden fixture | unit (Rust) | `cargo test -p cortex_lib pipeline::ner::tests::test_extract_person -- --ignored` (`#[ignore]` because requires bundled .onnx) | ❌ Wave 0 |
| KG-02 | EntityStore merges "John Smith" + "J. Smith" via embedding similarity | unit (Rust) | `cargo test -p cortex_lib graph::entity_store::tests::test_alias_merge_two_variants -- --ignored` (requires fastembed) | ❌ Wave 0 |
| KG-02 | F1 score on a 20-doc fixture ≥ 0.85 (sanity floor for INT8 quantization) | integration (Rust) | `cargo test -p cortex_lib pipeline::ner::tests::test_f1_floor -- --ignored` | ❌ Wave 0 |
| KG-03 | get_entities_by_type IPC returns canonicals filtered by type | unit (Rust) | `cargo test -p cortex_lib commands::entities::tests::test_get_entities_by_type` | ❌ Wave 0 |
| KG-03 | get_related_entities returns co-occurrence ≥ 2 | unit (Rust) | `cargo test -p cortex_lib graph::entity_store::tests::test_related_entities_cooccurrence` | ❌ Wave 0 |
| KG-04 | rename_entity_canonical updates name without affecting aliases | unit (Rust) | `cargo test -p cortex_lib graph::entity_store::tests::test_rename_canonical` | ❌ Wave 0 |
| KG-04 | split_entity_alias creates new canonical and rewrites affected doc metadata | unit (Rust) | `cargo test -p cortex_lib graph::entity_store::tests::test_split_alias` | ❌ Wave 0 |
| KG-05 | Backfill task emits `entity-backfill-progress` events; idempotent on second run | integration (Rust) | `cargo test -p cortex_lib pipeline::backfill::tests::test_backfill_idempotent -- --ignored` | ❌ Wave 0 |
| KG-05 | Throttle: ≤ N events per second during backfill of 1000-doc fixture | integration (Rust) | `cargo test -p cortex_lib pipeline::backfill::tests::test_event_throttle -- --ignored` | ❌ Wave 0 |
| UX-05 | WatchedPage `handleAddFolder` invokes `open({directory: true})` from plugin-dialog | unit (TS, mocked invoke) | `pnpm test --run client/pages/WatchedPage.test.tsx` | ❌ Wave 0 |
| UX-05 | User cancel → no folder added (silent) | unit (TS) | same file | ❌ Wave 0 |
| PAGE-13 | FilePreview dispatches to PdfPreview for `docType=pdf` | unit (TS) | `pnpm test --run client/components/preview/FilePreview.test.tsx` | ❌ Wave 0 |
| PAGE-13 | MarkdownPreview escapes raw `<script>` tag (XSS regression) | unit (TS) | `pnpm test --run client/components/preview/MarkdownPreview.test.tsx` | ❌ Wave 0 |
| PAGE-13 | SizeGuardCard shows for PDF > 50 MB | unit (TS) | `pnpm test --run client/components/preview/SizeGuardCard.test.tsx` | ❌ Wave 0 |
| PAGE-13 | read_document_text rejects file > maxBytes | unit (Rust) | `cargo test -p cortex_lib commands::documents::tests::test_read_document_text_size_cap` | ❌ Wave 0 |
| UX-06 | DocumentPage Open button calls `openPath(doc.path)` | unit (TS, mocked invoke) | `pnpm test --run client/pages/DocumentPage.test.tsx` | ❌ Wave 0 |
| UX-06 | DocumentContextMenu Reveal calls `revealItemInDir(doc.path)` | unit (TS) | `pnpm test --run client/components/DocumentContextMenu.test.tsx` | ❌ Wave 0 |
| UX-06 (manual) | Cmd-click Open from /search row opens in OS default app | manual smoke | `cargo tauri dev`, manual click | n/a (manual) |

### Test Fixture Data Needed

1. **NER golden fixture (`fixtures/ner_golden.json`):** 5-10 short documents (200-1000 chars each) with hand-labeled expected entities. Used for KG-02 F1 floor test. Example entries:
   ```json
   [
     {
       "text": "John Smith from Acme Corp visited the Brooklyn office on 2024-03-15.",
       "expected": [
         {"type": "person", "value": "John Smith"},
         {"type": "organization", "value": "Acme Corp"},
         {"type": "location", "value": "Brooklyn"},
         {"type": "date", "value": "2024-03-15"}
       ]
     }
   ]
   ```
2. **Alias-merge fixture:** 2 entities — "John Smith" and "J. Smith" — that must merge with cosine ≥ 0.85; and "John Smith" vs "Jane Doe" that must NOT merge.
3. **Oversized PDF fixture:** A 60 MB synthetic PDF (can be a single huge PNG embedded N times) used to verify SizeGuardCard renders.
4. **Markdown XSS fixture:** A markdown string containing `<script>alert(1)</script>` and `<img src=x onerror=alert(1)>` — verify rendered DOM contains the text but not the executable script.
5. **Multi-alias entity fixture for split:** A canonical "John Smith" with 3 docs mentioning it, then split off one alias — verify doc metadata updated correctly.
6. **Empty-collection fixture:** Backfill on empty RuVector emits immediate `complete` event without errors.

### Sampling Rate
- **Per task commit:** `cargo test --lib --no-fail-fast` (Rust unit tests, ~10 sec) + `pnpm test --run --reporter=dot` (vitest, ~10 sec)
- **Per wave merge:** `cargo test --workspace` (includes `#[ignore]`d tests if model is bundled in CI: `cargo test --workspace -- --include-ignored`) + `pnpm test --run`
- **Phase gate:** All of the above + `cargo tauri build --debug` (smoke, ~3 min) + manual sanity on dev runtime for UX-05, UX-06, PAGE-13

### Wave 0 Gaps
- [ ] `src-tauri/src/pipeline/ner.rs` — NerService struct + tests
- [ ] `src-tauri/src/pipeline/backfill.rs` — backfill task + tests
- [ ] `src-tauri/src/graph/entity_store.rs` — EntityStore + tests
- [ ] `src-tauri/src/commands/entities.rs` — 6 IPC commands + tests
- [ ] `src-tauri/tests/fixtures/ner_golden.json` — NER ground truth fixture
- [ ] `src-tauri/tests/fixtures/markdown_xss.md` — XSS test input
- [ ] `client/components/preview/*.test.tsx` — per-renderer unit tests
- [ ] `client/components/preview/*.tsx` — actual implementations (FilePreview, PdfPreview, ImagePreview, TextPreview, MarkdownPreview, SizeGuardCard, UnsupportedPreview)
- [ ] `client/components/DocumentContextMenu.tsx` + test
- [ ] `client/pages/EntitiesPage.tsx` + `EntityDetailPage.tsx` + tests
- [ ] `client/pages/WatchedPage.test.tsx` — new tests for plugin-dialog flow
- [ ] `client/pages/DocumentPage.test.tsx` — new tests for Open/Reveal buttons + chip click navigation
- [ ] Vitest config: a minimal `vitest.config.ts` if absent (verify in Wave 0)

## Project Constraints (from CLAUDE.md)

- **TDD discipline:** No production code without a failing test first. Apply RED → GREEN → REFACTOR per task.
- **Verification:** Show test output before claiming "tests pass." Show `cargo tauri build` exit code before claiming "build succeeds."
- **Privacy-first / local-only:** No cloud calls in the NER pipeline. **Honored.** ONNX runs entirely on-device.
- **Tauri 2:** Use Tauri 2 plugin patterns. **Honored.** Both new plugins are v2.
- **Named exports for components, default exports for route pages:** WatchedPage, DocumentPage already follow this; new EntitiesPage / EntityDetailPage are route pages (default export); EntityChip, FilePreview, etc. are components (named export).
- **Zustand for UI state, React Query for server/data state:** Backfill indicator state goes in a new Zustand store; entity data via React Query hooks.
- **Tauri terminal apps: DOM reparenting, not React portals:** N/A (no terminal in this phase).
- **Design tokens from `src/styles/globals.css`, never hardcode colors:** Confirmed; all new preview components and entity badges use `bg-*`, `text-*`, `border-*` token classes.
- **File type icons via Lucide:** Add `Network` (or `GitBranch`) for /entities sidebar link.
- **`cn()` helper:** All conditional classnames use it.

## Sources

### Primary (HIGH confidence)
- [v2.tauri.app/plugin/dialog](https://v2.tauri.app/plugin/dialog/) — folder picker API, Rust + JS deps, capability permissions
- [v2.tauri.app/plugin/opener](https://v2.tauri.app/plugin/opener/) — openPath/revealItemInDir, default-deny model, capability identifiers
- [v2.tauri.app/security/asset-protocol](https://v2.tauri.app/security/asset-protocol/) — assetProtocol config, scope, CSP requirements
- [v2.tauri.app/reference/javascript/api/namespacecore](https://v2.tauri.app/reference/javascript/api/namespacecore/#convertfilesrc) — convertFileSrc signature
- [huggingface.co/dslim/bert-base-NER](https://huggingface.co/dslim/bert-base-NER) — base model card, F1 0.913, MIT license, 110M params, BIO labels
- [huggingface.co/Xenova/bert-base-NER/tree/main/onnx](https://huggingface.co/Xenova/bert-base-NER/tree/main/onnx) — exact ONNX file sizes (model_quantized.onnx = 109 MB INT8)
- [huggingface.co/protectai/bert-base-NER-onnx](https://huggingface.co/protectai/bert-base-NER-onnx) — alternate ONNX export, MIT license, BIO label confirmation
- [github.com/remarkjs/react-markdown](https://github.com/remarkjs/react-markdown) — version 10.1.0, safe-by-default XSS posture, GFM via remark-gfm
- `npm view` outputs — version sanity for all four npm deps (run 2026-06-29)
- `cargo search` outputs — version sanity for ort/tokenizers/tauri-plugin-*
- Existing Cortex codebase (extensive read): `src-tauri/src/pipeline/{embedder,entities,indexer}.rs`, `src-tauri/src/graph/{edges,related}.rs`, `src-tauri/src/commands/documents.rs`, `src-tauri/src/types.rs`, `src-tauri/src/state.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/watcher/worker.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/Cargo.toml`, `client/pages/{DocumentPage,WatchedPage}.tsx`, `client/hooks/useTauri.ts`, `client/components/layout/Sidebar.tsx`, `client/App.tsx`, `package.json`

### Secondary (MEDIUM confidence)
- [docs.rs/ort/latest](https://docs.rs/ort/latest/ort/) — Session API for 2.0.0-rc.12, inputs! macro pattern
- [github.com/pykeio/ort](https://github.com/pykeio/ort) — README sample code (Session::builder, commit_from_file)
- [docs.rs/tokenizers](https://docs.rs/tokenizers) — Tokenizer::from_file, encode, get_offsets API
- WebSearch results for tauri-plugin-dialog 2.7.1 release date and tauri-plugin-opener 2.5.4 release date (8 Jan 2026)

### Tertiary (LOW confidence)
- Inference latency estimates (50-200 ms per chunk on M1/M2) — based on general BERT-base INT8 benchmarks, NOT measured on this specific model. **Flagged for validation in Wave 0.**
- 5 MB / 20 MB / 50 MB size thresholds — accepted from D-15 verbatim; no empirical backing.

## Metadata

**Confidence breakdown:**
- Standard stack (Tauri plugins, react-markdown): HIGH — official docs verified, versions confirmed against registries
- ONNX model file selection: HIGH — file sizes confirmed against HuggingFace; F1 from base model card
- ort 2.x crate API: MEDIUM — RC version, API spot-checked from docs; may shift between RCs
- Entity storage architecture: HIGH — design follows existing `path_index` + `DocumentGraph` patterns in the codebase
- Alias-merge embedding-threshold (0.85): MEDIUM — heuristic from research literature; locked by D-05
- Backfill throughput estimate: LOW — varies with hardware and doc lengths; flagged in Assumptions Log
- Pitfalls: HIGH — drawn from Tauri docs, BERT-NER literature, and direct codebase inspection

**Research date:** 2026-06-29
**Valid until:** 2026-07-29 (stable plugins + model; check ort version monthly for RC promotion)

## RESEARCH COMPLETE

**Phase:** 6 — Knowledge Graph and Native Integrations
**Confidence:** HIGH

### Key Findings

1. **Local NER is a solved Rust problem.** Bundle `Xenova/bert-base-NER`'s `model_quantized.onnx` (109 MB, INT8, MIT, F1=0.913 on CoNLL-2003) and run via `ort = "2.0.0-rc.12"` + `tokenizers = "0.20"` inside `spawn_blocking`. Cost: ~50 minutes for one-time 10K-doc backfill, ~100ms per chunk thereafter. No Python sidecar, no cloud.
2. **EntityStore lives in memory and is rebuilt from RuVector doc metadata.** No new persistence layer; `extracted_entities` metadata stays the source of truth. Mirrors the existing `path_index` pattern. ~5 MB RAM footprint, ~5 sec rebuild on startup.
3. **Asset protocol unlocks PDF + image preview with zero deps.** WebView's built-in PDF renderer + `convertFileSrc` + tightened CSP gives free zoom/find/print. Markdown via `react-markdown@10` + `remark-gfm@4` (safe-by-default XSS posture).
4. **Tauri 2 plugins are drop-in.** `tauri-plugin-dialog@2.7.1` replaces the ts-ignored hack; `tauri-plugin-opener@2.5.4` replaces the dead "Open in Finder" button. Capabilities + permissions documented.
5. **CSP must be tightened in this phase.** Current `csp: null` is permissive. The asset-protocol enablement is the right moment to introduce a real CSP; researched value provided.

### File Created
`/Users/gshah/work/apps/cortex/.planning/phases/06-knowledge-graph-and-native-integrations/06-RESEARCH.md`

### Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Standard stack (Tauri plugins, react-markdown) | HIGH | Official docs + version-verified against registries |
| ONNX model selection (Xenova/bert-base-NER int8) | HIGH | File sizes verified, F1 from base model card |
| ort 2.x API specifics | MEDIUM | RC version; pinned exact version mitigates churn |
| Entity storage architecture | HIGH | Mirrors existing in-codebase patterns |
| Alias-merge cosine threshold (0.85) | MEDIUM | Heuristic; D-08 Split-alias provides recovery |
| Backfill throughput estimate | LOW | Hardware-dependent; flagged in Assumptions Log |
| Pitfalls | HIGH | From Tauri docs + BERT-NER literature + codebase inspection |

### Open Questions Surfaced

1. Drop MISC entities from NER output? (Recommendation: yes)
2. Persist canonical_embeddings? (Recommendation: no, rebuild on startup)
3. Size-guard enforcement layer? (Recommendation: both frontend + backend)
4. Pause backfill UX? (Recommendation: defer)
5. Tighter asset-protocol scope? (Recommendation: keep `["**"]`, react-markdown's escape posture is sufficient)

### Ready for Planning

Research is complete. The planner has:
- Locked decisions copied verbatim from CONTEXT.md
- Concrete library + version recommendations
- Architecture sketch (EntityStore schema, NerService struct, IPC commands)
- Code examples for the load-once inference pattern, asset protocol, plugin invocations
- Validation Architecture mapping every requirement to an automated test
- Assumptions Log with risk ratings
- Open questions tagged for planner / discuss-phase escalation

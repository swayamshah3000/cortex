# Project Research Summary

**Project:** Cortex — Self-Organizing Document Intelligence
**Domain:** Tauri 2 desktop app with Rust vector backend, ONNX embeddings, GNN clustering
**Researched:** 2026-02-27
**Confidence:** HIGH (stack + architecture); MEDIUM-HIGH (features + pitfalls)

## Executive Summary

Cortex is a local-first document intelligence desktop app built on Tauri 2 with a Rust backend that embeds RuVector (a custom self-learning vector database) for semantic search and GNN-driven auto-organization. The React 19 frontend already exists with mock data; the remaining work is entirely the Rust backend — from Tauri scaffolding through document parsing, ONNX embedding generation, vector indexing, GNN clustering, and IPC command wiring. The recommended approach is to build in strict dependency order (foundation → storage engine → document pipeline → background subsystems → intelligence layer → IPC commands → frontend integration), with the frontend remaining functional on mock data throughout until the backend is ready to replace it.

The core product bet — "Find anything. Organize nothing." — is validated by the competitive landscape. DEVONthink is the closest competitor, and its most-praised features (See Also, Classify) are manual suggestion workflows that Cortex makes fully automatic. The Windows Recall backlash confirms that the market is actively seeking a trusted local-first AI document tool. The key technical differentiators (semantic search via HNSW, GNN auto-clustering for Smart Spaces, SONA self-learning, graph relationship visualization) have no equivalent in any consumer desktop tool today, and all are implementable via the existing RuVector crate ecosystem at the identified path dependency location.

The principal risks cluster around three areas: async runtime misuse (blocking Tokio with CPU-bound ONNX inference or GNN clustering), IPC serialization correctness (type boundaries crossing the Rust-JS bridge), and pipeline design (GNN clustering must be decoupled from the per-document indexing loop from day one — retrofitting this is a multi-day refactor). These are not theoretical risks; each has documented real-world failures in the Tauri GitHub issue tracker. The mitigation is architectural: establish `spawn_blocking` patterns, a typed `AppError` enum, and a decoupled GNN scheduling system before writing any substantive pipeline code.

## Key Findings

### Recommended Stack

The Rust backend stack is well-established with high-confidence version decisions. The frontend is already decided (React 19, TypeScript, Vite, TailwindCSS 4, Zustand, React Query, React Router v7) and must not be changed. The backend builds on Tauri 2.10.2 as the desktop shell, with `fastembed 5` as the embedding engine (preferred over raw `ort` for its bundled model management), `notify 8.2.0` for cross-platform file watching, and `pdf-extract 0.10.0` / `docx-rust` / `calamine 0.33.0` for document parsing. All RuVector crates are consumed as local path dependencies from `/Users/gshah/work/apps/experiments/ruvector/`. Version compatibility is non-trivial — `fastembed 5` pins its own `ort` version, `tauri-build` must match Tauri minor version exactly, and `reqwest 0.12` is required for `tokio 1.x` compatibility.

**Core technologies:**
- `tauri 2.10.2`: Desktop shell + IPC bridge — latest stable, smallest binary footprint, native WebView
- `fastembed 5`: ONNX embedding engine — bundles model management, tokenizer, batching (3-4x less code than raw `ort`)
- `ruvector-core` (path dep): HNSW vector storage + indexing — powers all search and clustering
- `ruvector-gnn` (path dep): GNN clustering → Smart Spaces — the defining intelligence feature
- `notify 8.2.0`: Cross-platform file watching — use Tokio `mpsc` bridge pattern, not `async-watcher`
- `pdf-extract 0.10.0` + `docx-rust` + `calamine 0.33.0`: Multi-format text extraction (avoid `docx-rs` — writer-only)
- `leptess 0.14.0` (optional, `ocr` feature flag): OCR for images — requires system Tesseract, gate as opt-in
- `ollama-rs` (optional, `ollama` feature flag): LLM space naming — degrade gracefully if Ollama unavailable
- `tokio 1.41`: Async runtime — must match RuVector workspace pin; use `parking_lot::Mutex` for non-async state
- `thiserror 2.0` + `anyhow 1.0`: Typed error definitions + Tauri command error conversion

### Expected Features

The feature dependency chain is strict: Watched Folder Monitoring enables File Detection → Document Parsing → Embedding Generation → Vector Storage → both Semantic Search AND GNN Clustering → Smart Spaces. Nothing intelligence-related works without the full indexing pipeline first.

**Must have — table stakes (users assume these exist):**
- Watched folder setup and file change monitoring — without this, nothing indexes
- Document parsing: PDF, DOCX, TXT, MD (90% of real-world use; spreadsheets and OCR can wait)
- Local ONNX embedding generation (all-MiniLM-L6-v2, 384-dim) — local-first from day one
- Semantic search with natural language queries — the #1 differentiator, non-negotiable for launch
- Search result excerpts with highlighting — users must see WHY results matched
- Background indexing with progress indicator — without this, users assume the app is broken
- Document preview (PDF, DOCX) — macOS Quick Look sets this expectation universally
- Recent documents, Favorites, basic metadata filtering — standard file manager table stakes
- Onboarding wizard (4-step) — reduces cold-start abandonment

**Should have — competitive differentiators:**
- GNN clustering → Smart Spaces (auto-generated, fully automatic) — defining product feature
- Space view with document list — users need to see what's in each space
- Auto-generated tags from content — complements spaces
- Entity extraction (dates, amounts, people, organizations) — enables precision filtering ("invoices over $500")
- Related documents panel (graph edges) — analogous to DEVONthink's most-praised "See Also" feature
- Space network graph visualization — compelling demo feature, no competitor has this
- SONA self-learning (click-through tuning) — requires established search interactions first
- OCR opt-in per folder, spreadsheet indexing, command palette (Cmd+K)

**Defer to v2+:**
- Sub-space hierarchy (hyperbolic HNSW clustering) — complex; flat spaces must be validated first
- Domain expansion (transfer learning for new spaces) — advanced; needs mature cluster graph
- Optional cloud embeddings (OpenAI), LLM space naming via Ollama — nice-to-have quality upgrades
- Full analytics/Insights page, keyboard shortcuts — polish features after PMF

**Anti-features — never build:**
- Auto-moving files on disk — trust-destroying, breaks external tool references; use virtual spaces only
- AI chat / RAG chatbot — separate product surface, out of scope
- Cloud sync of the document index — breaks local-first guarantee
- Mandatory cloud embeddings — breaks local-first trust; strictly opt-in

### Architecture Approach

The system has four clearly separated layers: (1) React WebView frontend communicating over Tauri IPC via `invoke()` and `listen()`; (2) Tauri command handlers organized by domain (documents, spaces, search, folders, analytics, settings) that share a single `AppState` struct; (3) a RuVector engine wrapper layer (`engine/`) that insulates command handlers from RuVector API changes; and (4) background tasks (file watcher, indexer pipeline, GNN scheduler) that run as `tauri::async_runtime::spawn()` tasks communicating via `tokio::mpsc` channels. The frontend uses dual-mode React Query hooks that fall back to mock data when the Tauri runtime is absent, allowing UI development to proceed independently of backend completion.

**Major components:**
1. `AppState` — singleton holding `Arc<Mutex<T>>` handles to all subsystems; initialized in `tauri::Builder::setup()`
2. `commands/` — domain-split IPC handlers (documents, spaces, search, folders, analytics, settings); receive `tauri::State<AppState>`
3. `engine/` — thin wrappers around RuVector crates (vector_store, gnn_cluster, graph_engine, filter, attention, learning, collections)
4. `background/` — file watcher (`notify-rs` + Tokio bridge) and indexing pipeline orchestrator
5. `pipeline/` — pure-function document processors (parser, embedder, entity extractor); decoupled from storage
6. `storage/` — settings persistence via Tauri store plugin
7. Frontend hooks (`src/hooks/`) — dual-mode React Query hooks with mock fallback; event-driven invalidation via centralized `useTauriEventInvalidation`

### Critical Pitfalls

1. **Blocking Tokio runtime with CPU-bound work** — ONNX inference, GNN clustering, and PDF parsing must all use `tokio::task::spawn_blocking()`. Never call these directly inside `async` Tauri commands. Failure causes UI freezes of 1-5 seconds per document. Establish the `spawn_blocking` boundary in Phase 1 before writing any document processing code.

2. **GNN clustering per-document in the indexing pipeline** — GNN re-clustering must be a separate background Tokio task on a debounced timer (trigger after 10-second quiet period or every 30 seconds if new documents arrived), not a step called after every document index. Implementing it inline causes UI freezes, space thrashing, and 50-100 seconds of blocking during bulk imports. This is an architectural decision that cannot be retrofitted cheaply.

3. **IPC serialization failures with complex Rust types** — Define a typed `AppError` enum implementing `serde::Serialize` before writing any commands. Return `Result<T, AppError>` from all commands, never `Result<T, String>` or raw `anyhow::Error`. Missing `Serialize` on nested types causes frontend Promises to hang indefinitely (Tauri issue #10327).

4. **Embedding dimension mismatch on model switching** — The HNSW index is fixed-dimension at creation. If the user switches from local ONNX (384-dim) to OpenAI API (1536-dim), every existing vector becomes incompatible. Design separate named collections per model from day one, and surface a re-index warning modal in Settings before any switch.

5. **Tauri event listener memory leak** — Every `listen()` call must have a corresponding `unlisten()` in `useEffect` cleanup. Failure causes memory to reach 1.1GB+ during sustained indexing (Tauri issue #12724). Establish a `useTauriEvent` hook with built-in cleanup before any event-driven UI component.

6. **notify-rs dropping events at scale** — At 1,500+ files, the notify-rs watcher drops ~16% of modification events silently. Build a polling fallback (periodic full-directory hash scan) alongside the event watcher from the start, not as a retrofit.

7. **pdf-extract panicking on malformed PDFs** — Wrap all parse calls in `std::panic::catch_unwind`. Malformed or encrypted PDFs will otherwise crash the indexer background task.

## Implications for Roadmap

The architecture has seven clear dependency layers (Foundation → Storage Engine → Document Pipeline → Background Subsystems → Intelligence Layer → IPC Commands → Frontend Integration). These map naturally to phases.

### Phase 1: Tauri Foundation and Contracts
**Rationale:** Nothing can be built without Tauri scaffolding and the type contracts that cross the IPC boundary. Both the `AppError` enum (Pitfall #3) and the `spawn_blocking` pattern (Pitfall #1) must be established here before any pipeline code is written.
**Delivers:** `src-tauri/` directory, `Cargo.toml` with all path deps and feature flags, `AppState` stub, typed `AppError`, Tauri IPC boilerplate compiled and running, dual-mode frontend hooks with mock fallback still active.
**Addresses:** Onboarding wizard prerequisites; settings persistence; the compile check that all RuVector path dependencies resolve correctly.
**Avoids:** Pitfalls #1 (blocking runtime) and #3 (IPC serialization) — both are pattern-level decisions locked in here.
**Research flag:** SKIP — Tauri 2 official docs are comprehensive; patterns are standard and well-documented.

### Phase 2: Document Ingestion Pipeline
**Rationale:** The indexing pipeline (parse → embed → store) is the foundation of all intelligence features. Semantic search and Smart Spaces both depend on populated vector storage. This phase validates the core local-first promise before building any search or clustering UI.
**Delivers:** Multi-format document parser (PDF, DOCX, TXT, MD), fastembed ONNX embedding generation (all-MiniLM-L6-v2, 384-dim), RuVector core vector storage with HNSW indexing, content hash change detection, entity extraction (regex-based Phase 1).
**Uses:** `pdf-extract`, `docx-rust`, `fastembed 5`, `ruvector-core`, `regex`, `sha2`.
**Avoids:** Pitfall #4 (embedding dimension mismatch) — multi-model named collection schema designed here; Pitfall #7 (malformed PDF panics) — `catch_unwind` wrapping established here.
**Research flag:** NEEDS RESEARCH — fastembed API integration details, RuVector core insert/search API surface, and entity extraction chunking strategy need phase-level research before implementation.

### Phase 3: File Watching and Background Indexing
**Rationale:** The ingestion pipeline from Phase 2 needs to be triggered automatically by file system events. The background subsystems must be architecturally separate from IPC commands to avoid blocking. This phase wires notify-rs → indexing pipeline with proper Tokio channel separation.
**Delivers:** notify-rs file watcher with Tokio `mpsc` bridge, debounce (300ms), polling fallback (periodic hash scan), `WatcherCmd`/`IndexCmd` message types, background indexer task orchestrator, `indexing-progress` events emitted to frontend, scan-on-startup for previously watched folders.
**Avoids:** Pitfall #5 (notify-rs event loss) — polling fallback built alongside event watcher; Pitfall #6 (watcher state lost on restart) — persisted folder list restored on launch.
**Research flag:** SKIP — notify-rs patterns are well-documented; Tokio mpsc channel pattern is standard.

### Phase 4: IPC Command Layer and Frontend Wiring
**Rationale:** With a working backend pipeline, the IPC command surface can be built and the frontend dual-mode hooks can be flipped from mock data to live Tauri invocations. This phase also establishes the React Query invalidation architecture.
**Delivers:** All Tauri commands (documents, spaces, search, folders, analytics, settings), centralized `useTauriEventInvalidation` hook, React Query `staleTime` tuning (`refetchOnWindowFocus: false`), `QUERY_KEYS` constants, end-to-end: drop file → search finds it.
**Avoids:** Pitfall #7 (React Query stale data) — invalidation hook registered in `AppShell` before any data-displaying page; Pitfall #6 (event listener memory leak) — `useTauriEvent` hook with cleanup established here.
**Research flag:** SKIP — Tauri IPC command patterns are fully documented; React Query invalidation is standard.

### Phase 5: GNN Clustering and Smart Spaces
**Rationale:** Smart Spaces are the product's defining feature, but they require a populated vector index (Phase 2) and a properly decoupled background scheduler (critical to avoid Pitfall #2). This phase must not be attempted until the indexing pipeline from Phase 2-3 is stable.
**Delivers:** GNN re-clustering background scheduler (debounced, not per-document), incremental cluster assignment for new documents, full re-cluster on schedule or user request, Smart Space CRUD commands, `space-updated` events batched per clustering run (not per document), `ClusteringState` enum exposed to frontend.
**Avoids:** Pitfall #2 (GNN per-document) — scheduling separation is the entire point of this phase's design.
**Research flag:** NEEDS RESEARCH — ruvector-gnn API surface (cluster assignment, incremental vs. full re-cluster options, EWC configuration) needs phase-level research before implementation. RuVector is still at version 2.0.5 and the GNN API is not externally documented.

### Phase 6: Search Intelligence and Graph Layer
**Rationale:** With documents indexed and Smart Spaces established, the search can be enhanced with attention re-ranking, metadata filtering, graph-based related document discovery, and the SONA self-learning engine. These features require an established corpus to generate learning signals.
**Delivers:** Hybrid search (metadata pre-filter via ruvector-filter + HNSW vector search + attention re-ranking), related documents graph (ruvector-graph edges), search result excerpts and highlighting, "why this space" explanation surface, SONA self-learning wired to search command.
**Uses:** `ruvector-filter`, `ruvector-attention`, `ruvector-graph`, `sona`.
**Research flag:** NEEDS RESEARCH — ruvector-attention re-ranking API, ruvector-graph edge management, and SONA LearningSignal integration all need phase-level research. These are the most complex crates in the RuVector ecosystem.

### Phase 7: Distribution and Polish
**Rationale:** Production distribution requires macOS entitlements, code signing, notarization, auto-update, and verified binary size. These are blockers for any external user testing.
**Delivers:** macOS entitlements (JIT allowances for WebView), code signing and notarization pipeline, auto-updater (tauri-plugin-updater), binary size verification (<50MB target), ONNX model bundled correctly in production build (not pointing to dev absolute path), Ollama graceful fallback for space naming.
**Avoids:** "Looks Done But Isn't" checklist items: ONNX model path in production build, macOS entitlements for WebView JIT, file watcher restart on app relaunch, stale vector cleanup on document deletion.
**Research flag:** SKIP — macOS notarization process is documented; Tauri app size optimization docs are comprehensive.

### Phase Ordering Rationale

- Phases 1-3 establish the complete data flow before any feature UI. Nothing in Phase 5+ works without Phases 2 and 3 shipping first.
- Phase 4 can overlap with Phase 3 partially — frontend hooks can be wired to IPC commands as soon as any command exists, even before the full pipeline is working.
- Phase 5 (GNN) must not start until Phase 2 (embeddings + vector storage) is stable. GNN operates on stored embeddings; running it on an empty or partial index produces meaningless clusters.
- Phase 6 (SONA self-learning) requires Phase 4's search command to be complete before it can record learning signals. There is no value in SONA until real users are executing real searches.
- Phase 7 should be scheduled once Phase 5 is functional — distribution validation unblocks the first external user testing cycle.

### Research Flags

Phases needing `/gsd:research-phase` during planning:
- **Phase 2:** fastembed API integration details, RuVector core insert/query API surface, embedding chunking strategy for long documents
- **Phase 5:** ruvector-gnn API (incremental vs full re-cluster, EWC config, cluster assignment thresholds)
- **Phase 6:** ruvector-attention re-ranking API, ruvector-graph edge types, SONA LearningSignal integration

Phases with standard patterns (skip research-phase):
- **Phase 1:** Tauri 2 official docs are authoritative; IPC patterns are standard Rust
- **Phase 3:** notify-rs + Tokio channel pattern is well-documented
- **Phase 4:** React Query + Tauri IPC invalidation pattern is standard
- **Phase 7:** Tauri distribution/signing process is documented in official guides

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified against official sources or local crate inspection. Tauri 2.10.2, fastembed 5, notify 8.2.0, pdf-extract 0.10 all confirmed. `ollama-rs` version unconfirmed — verify on crates.io before Cargo.toml. |
| Features | MEDIUM-HIGH | Table stakes (HIGH) verified against multiple competitor analyses and documented user expectations. Differentiator value (MEDIUM) based on market analysis; RuVector capability claims rely on local source inspection. |
| Architecture | HIGH | Tauri 2 IPC patterns verified via official docs. RuVector crate structure verified by direct source inspection. The dual-mode hook pattern and AppState design are established Tauri community patterns. |
| Pitfalls | MEDIUM-HIGH | IPC/async pitfalls (HIGH) — documented with specific Tauri GitHub issue numbers (#10327, #12724). RuVector-specific pitfalls (MEDIUM) — inferred from general HNSW and GNN literature; RuVector is custom with thin community data. |

**Overall confidence:** HIGH for the Tauri backend implementation path. MEDIUM for RuVector internal API details (gnn, attention, sona) which will need phase-level research during planning.

### Gaps to Address

- **ruvector-gnn incremental clustering API:** The ARCHITECTURE.md references incremental cluster assignment but the specific ruvector-gnn API surface for this is not confirmed from source inspection. Must research before Phase 5 planning.
- **ruvector-attention re-ranking call signature:** 46 attention mechanisms are referenced but the selection API and configuration surface are not documented externally. Must research before Phase 6 planning.
- **SONA LearningSignal API:** SONA's LoRA adaptation is referenced architecturally but the Rust API surface (what constitutes a LearningSignal, how trajectory data is recorded) is not confirmed. Must research before Phase 6 planning.
- **RuVector workspace exclusions:** ARCHITECTURE.md notes that some RuVector crates may be excluded from the workspace — this needs verification during Phase 1 Cargo.toml setup to prevent build failures.
- **pdf-extract panic safety:** The `catch_unwind` pattern for malformed PDFs needs to be validated against the pdf-extract 0.10.0 API — some crates have internal FFI boundaries where `catch_unwind` does not propagate correctly.
- **ollama-rs version:** Unconfirmed on crates.io as of research date. Verify the latest stable version before adding to Cargo.toml.

## Sources

### Primary (HIGH confidence)
- Tauri 2 official docs (v2.tauri.app) — IPC, state management, project structure, commands, events
- Tauri GitHub issues #10327, #12724, #12388 — IPC Promise hang, memory leak, event subscription gaps
- notify-rs GitHub issue #412 — large-scale file watching event loss at 1,500+ files
- RuVector local source (`/Users/gshah/work/apps/experiments/ruvector/`) — crate structure, Cargo.toml, workspace deps
- fastembed-rs GitHub — v5 API, all-MiniLM-L6-v2 support
- crates.io — pdf-extract 0.10.0, calamine 0.33.0, notify 8.2.0, tauri-plugin-dialog 2.4.2

### Secondary (MEDIUM confidence)
- Community Tauri async patterns — rfdonnelly.github.io, sneakycrow.dev
- Competitor feature analysis — DEVONthink official docs, independent reviews (Elephas)
- USENIX ATC '25 — GNN system pitfalls (peer-reviewed, general GNN patterns)
- Qdrant production vector search — HNSW at scale patterns
- Vector dimension mismatch analysis — multiple corroborating sources

### Tertiary (LOW confidence)
- Vendor blogs (iomovo, DEVONtechnologies) — useful for user pain point framing, treat with skepticism
- Competitor marketing (Fenn, TheDrive.ai) — useful only for Spotlight user pain points, not product decisions

---
*Research completed: 2026-02-27*
*Ready for roadmap: yes*

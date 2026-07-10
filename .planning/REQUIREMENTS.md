# Requirements: Cortex

**Defined:** 2026-02-27
**Core Value:** Documents sort themselves into meaningful spaces through AI-powered clustering, and users find anything with natural language search — all running locally.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Tauri Foundation

- [x] **TAURI-01**: Tauri 2 shell wraps existing React frontend with WebView
- [x] **TAURI-02**: Express server removed, replaced by Tauri IPC command stubs
- [x] **TAURI-03**: AppError enum with serde::Serialize for all IPC error handling
- [x] **TAURI-04**: spawn_blocking pattern established for all CPU-bound operations
- [x] **TAURI-05**: Dual-mode frontend hooks (mock data in dev, Tauri invoke in production)
- [x] **TAURI-06**: AppState struct with Arc<CortexEngine> and channel senders

### Document Pipeline

- [x] **DPIP-01**: PDF text extraction via pdf-extract/lopdf
- [x] **DPIP-02**: DOCX parsing via docx-rust
- [x] **DPIP-03**: Plain text and Markdown direct read
- [x] **DPIP-04**: Spreadsheet indexing (XLSX, CSV) via calamine
- [x] **DPIP-05**: OCR for images via tesseract bindings (opt-in per folder)
- [x] **DPIP-06**: Local ONNX embedding generation (all-MiniLM-L6-v2, 384-dim) via fastembed
- [x] **DPIP-07**: Optional API embedding (OpenAI text-embedding-3-small, 1536-dim)
- [x] **DPIP-08**: Content hash computation for change detection
- [x] **DPIP-09**: Entity extraction: dates, amounts, people, organizations, locations

### Vector Storage

- [x] **VSTOR-01**: RuVector core integration with HNSW indexing
- [x] **VSTOR-02**: Multi-collection support (separate indices per embedding dimension)
- [x] **VSTOR-03**: Metadata filtering (type, date range, space, tags) before vector search
- [x] **VSTOR-04**: Hybrid queries: structured filters + semantic similarity

### File Watching

- [x] **FWAT-01**: Watched folder monitoring via notify-rs with debounce (300ms)
- [x] **FWAT-02**: Polling fallback for event-dropped scenarios (notify-rs limitation)
- [x] **FWAT-03**: File type toggles per watched folder
- [x] **FWAT-04**: Exclusion patterns (node_modules, .git, hidden files)
- [x] **FWAT-05**: Background indexing as Tokio task with progress events emitted to frontend
- [x] **FWAT-06**: Re-index on document modification (content hash comparison)

### Search

- [x] **SRCH-01**: Semantic search with natural language queries via HNSW nearest neighbor
- [x] **SRCH-02**: Search result highlighting with matched excerpts
- [x] **SRCH-03**: Metadata filters (type, date, space) applied pre-search
- [x] **SRCH-04**: Entity-filtered search ("invoices over $500") using extracted entities
- [x] **SRCH-05**: Incremental search-as-you-type with 150ms debounce
- [x] **SRCH-06**: GNN attention re-ranking of search results (ruvector-attention)

### Smart Spaces

- [x] **SPAC-01**: GNN clustering auto-discovers document groups as Smart Spaces
- [x] **SPAC-02**: GNN clustering runs as decoupled background job (not per-document)
- [x] **SPAC-03**: Space naming via rule-based approach (most frequent entity type + noun)
- [x] **SPAC-04**: Space centroid vectors for similarity comparison
- [x] **SPAC-05**: Related documents discovery via graph edges (ruvector-graph)
- [x] **SPAC-06**: User can move document between spaces manually
- [x] **SPAC-07**: Domain expansion: new spaces bootstrap from existing knowledge (ruvector-domain-expansion)

### Intelligence

- [x] **INTL-01**: SONA self-learning: search queries generate learning signals
- [x] **INTL-02**: Click-through data tunes search ranking over time
- [x] **INTL-03**: Graph edges connect documents by content similarity, shared space, shared tags, shared entities
- [x] **INTL-04**: Space network graph data from ruvector-graph for visualization

### Frontend Pages

- [x] **PAGE-01**: Dashboard with real stats (sparklines, recent docs, top spaces, activity feed)
- [x] **PAGE-02**: Smart Spaces grid (auto-organized, grid/list toggle)
- [x] **PAGE-03**: Space detail (sub-spaces, document list, related spaces)
- [x] **PAGE-04**: Search page (split-pane: results + preview panel, filters)
- [x] **PAGE-05**: Recent documents timeline (Today/Yesterday/This Week)
- [x] **PAGE-06**: Favorites page (starred documents with sort)
- [x] **PAGE-07**: Tag cloud page (auto-generated + user-created tags)
- [x] **PAGE-08**: Watched folders management (add/remove/pause, file type toggles, exclusions)
- [x] **PAGE-09**: Insights/analytics (donut chart, area chart, bar chart, space network graph)
- [x] **PAGE-10**: Settings: General, Indexing, AI & Models, Privacy, Storage, About
- [x] **PAGE-11**: Document detail: preview (65%) + metadata sidebar (35%)
- [x] **PAGE-12**: Onboarding wizard (Welcome, Select Folders, Scanning, Spaces Ready)
- [x] **PAGE-13**: Document detail in-app preview for PDF, image, plain-text, and markdown files (no 200-char excerpt)

### UX

- [x] **UX-01**: Command palette (Cmd+K) for search and navigation
- [x] **UX-02**: Keyboard shortcuts (Cmd+1/2/3, Cmd+,, Cmd+D, Cmd+\)
- [x] **UX-03**: System tray with background indexing indicator
- [x] **UX-04**: Background indexing progress in TopBar
- [x] **UX-05**: Add Watched Folder opens a native OS folder picker (no manual path typing)
- [x] **UX-06**: Open in Finder / Open with default app from Document detail AND search results

### Knowledge Graph

- [x] **KG-01**: Entities extracted from documents appear as graph nodes; clicking surfaces every document mentioning them
- [x] **KG-02**: Entity normalization merges aliases (e.g., "123 Main St" / "Main Street property") via embedding similarity
- [x] **KG-03**: Knowledge graph queryable via IPC — entities by type, documents for entity, related entities
- [x] **KG-04**: Rename canonical name + Split alias actions on /entities/:id
- [x] **KG-05**: NER backfill runs on startup, emits progress events, UI stays responsive

## v1.1 Requirements

UAT against v1.0 surfaced two fundamental gaps: bert-base-NER produced ~80% false-positive entities on personal corpora (Plan 06-02 trained on CoNLL-03 news), and the heuristic `name_space()` clustering produced collisions ("Work" x 4) and "Space N - Related" labels instead of meaningful categories. v1.1 replaces both with LLM-driven extraction and labeling, cloud-default with Ollama fallback, porting the proven provider/auth pattern from `/Users/gshah/work/apps/learnforge/src-tauri/src/{ai,auth}/`.

### AI Provider Foundation

- [x] **AIPV-01**: User can connect Anthropic (Claude) via OAuth setup-token OR API key, with provider showing in Settings → AI tab as authenticated
- [x] **AIPV-02**: User can connect OpenAI via subscription token OR API key, with provider showing in Settings → AI tab as authenticated
- [x] **AIPV-03**: User can connect Google Gemini via API key OR OAuth, with provider showing in Settings → AI tab as authenticated
- [x] **AIPV-04**: User can configure Ollama as a fallback provider with base URL + model selection in Settings → AI tab
- [x] **AIPV-05**: User can pick the active AI provider from the configured list, and switch active provider at any time
- [x] **AIPV-06**: First-run onboarding adds a "Connect AI" step that prompts for at least one provider; user can skip and configure later via Settings
- [x] **AIPV-07**: Credentials persist across app restarts in `app_data_dir/credentials.json` and are removed by an explicit "Disconnect" action
- [x] **AIPV-08**: Provider failures surface human-readable error toasts (invalid token, rate limit, no network) with a clear next step

### LLM Entity Extraction

- [x] **LLME-01**: User-visible entities on /document/:id are extracted by the active AI provider (cloud or Ollama), not bert-base-NER
- [x] **LLME-02**: Entity types cover person, organization, location, date, amount, email, and topic uniformly across any document domain (tax, recipes, medical, code, letters)
- [x] **LLME-03**: Per-document entity cap of 20 still applies; extraction is idempotent (re-extract same doc returns stable entities)
- [x] **LLME-04**: Entity extraction failure on a single document does not block the rest of the index (graceful fallback to regex-only on error)
- [x] **LLME-05**: Backfill of existing documents to the new extractor is triggered on Settings change; TopBar BackfillIndicator surfaces progress and ETA
- [x] **LLME-06**: bert-base-NER + ort + tokenizers Rust deps are removed from `src-tauri/Cargo.toml`; `src-tauri/models/bert-base-NER.onnx` and tokenizer files are removed from disk

### LLM Space Labeling

- [x] **LLML-01**: Each Smart Space gets a 2-4 word category label generated by the active AI provider from the cluster's top-N document titles + entity summary
- [x] **LLML-02**: Each Space gets a 1-sentence description shown on hover / in detail view
- [x] **LLML-03**: Labels are cached by cluster-membership fingerprint; re-labeling only fires when membership shifts > 20%
- [x] **LLML-04**: Two clusters never receive identical labels — disambiguation falls back to suffix or LLM re-prompt
- [x] **LLML-05**: Label generation runs in background after recluster; spaces show "Generating label…" placeholder until ready

### Hierarchical Spaces

- [x] **HSPC-01**: Top-level Spaces auto-split into sub-spaces when a cluster exceeds 50 documents, matching `mockSpaces` shape (Property → Tax → Insurance)
- [x] **HSPC-02**: Sub-spaces are themselves LLM-labeled and clickable, navigating to /spaces/:id with parent breadcrumb
- [x] **HSPC-03**: Sub-clustering uses HDBSCAN or recursive k-means on intra-cluster vectors; unclustered docs surface in a "Misc" sub-space
- [x] **HSPC-04**: Sidebar shows top 5 Spaces with sub-counts; clicking a Space expands sub-spaces inline

### Entity-Driven Exploration

- [x] **ENEX-01**: Clicking any entity chip filters the current view (search, recent, space detail) to documents mentioning that entity
- [x] **ENEX-02**: User can save the current search query as a virtual Space; saved Spaces appear in Sidebar alongside auto-clustered Spaces
- [x] **ENEX-03**: /document/:id includes a "Related" panel showing top-5 documents by entity overlap + cosine similarity
- [x] **ENEX-04**: Saved search Spaces re-evaluate on Sidebar render so newly indexed matching docs appear without manual refresh

### Ontology / Relation Extraction

- [x] **ONTO-01**: Pass 3 relation extractor runs after Pass 2 in backfill; produces `Vec<Triple>` from doc text + Pass 2 entities using the active AI provider; docs advance from entities_version 3.0 → 3.5 on success
- [x] **ONTO-02**: `TripleStore` persists triples with subject_id, predicate, object_id, doc_ids provenance, and user_added flag to `app_data_dir/triples.json`; forward index `(subject_id, predicate) → HashSet<object_id>` and reverse index `(predicate, object_id) → HashSet<subject_id>` support O(1) directional queries; auto-inverse writes for directional pairs (owns/owned_by, etc.) and symmetric predicates (married_to, partner_of)
- [x] **ONTO-03**: `/entity/:class/:value` shows a "Relations" section listing outgoing + incoming triples with target entity chips clickable to their own entity pages
- [x] **ONTO-04**: `/ownership/:personId` renders assets grouped by AssetType (Property / Vehicle / Investment / Business / Financial); Sidebar "Owned by me" quick link navigates to the top-doc-count Person entity's ownership page
- [x] **ONTO-05**: `add_manual_triple` and `delete_triple` IPCs support user overrides; manual triples carry `user_added=true` and are preserved when the LLM re-runs; delete removes both the primary triple and any auto-inverse partner

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Advanced Spaces

- **ASPAC-01**: Sub-space hierarchy via hyperbolic HNSW clustering
- **ASPAC-02**: ~~LLM-based space naming via Ollama (upgrade from rule-based)~~ — pulled into v1.1 as LLML-01..05
- **ASPAC-03**: Suggest space renames via notification when content shifts
- **ASPAC-04**: Force-directed knowledge graph visualization on /insights (deferred from v1.1)
- **ASPAC-05**: Chat with documents (RAG) (deferred from v1.1)
- **ASPAC-06**: macOS Keychain credential storage (v1.2 hardening, plaintext JSON acceptable for v1.1)

### Distribution

- **DIST-01**: macOS code signing and notarization
- **DIST-02**: Windows MSI installer
- **DIST-03**: Linux AppImage/deb packages
- **DIST-04**: Auto-updater via Tauri plugin

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Auto-move files on disk | Smart Spaces are virtual views only. Moving files breaks symlinks, scripts, references. Trust-destroying if AI is wrong. |
| Cloud sync / multi-device | Breaks local-first guarantee. Complex conflict resolution. Not the product. |
| Real-time collaboration | Single-user desktop app. Adds auth, sync, server complexity. |
| AI chat / Q&A over documents | Separate product surface. RAG chatbot requires LLM integration, hallucination mitigation. |
| Document version history | Git-for-docs is unsolved UX. Storage explosion. Track modified_at instead. |
| Email / IMAP indexing | Massive scope expansion. OAuth flows, protocol complexity. Export to PDF instead. |
| Browser history indexing | Privacy risk. Recall backlash validates users don't want this. ~/Downloads covers most cases. |
| Mandatory cloud embeddings | Breaks local-first trust. Local ONNX always default. Cloud strictly opt-in. |
| Mobile app | Desktop-first via Tauri. Mobile is a separate product. |
| Web deployment | Tauri desktop only. Existing Netlify config is legacy from prototype. |
| Cloud-only operation (no Ollama fallback) | Ollama is a first-class alternative for offline / privacy-strict users. Cloud-default but never cloud-only. |
| Force-directed knowledge graph viz in v1.1 | Cool but low daily utility. Defer until clustering quality is proven. |
| Chat with documents (RAG) in v1.1 | Separate product surface. Bring after Spaces + entities are solid. |
| macOS Keychain credential storage in v1.1 | Plaintext JSON in `app_data_dir` acceptable for v1.1 ship; Keychain is v1.2 hardening. |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| TAURI-01 | Phase 1 | Complete |
| TAURI-02 | Phase 1 | Complete |
| TAURI-03 | Phase 1 | Complete |
| TAURI-04 | Phase 1 | Complete |
| TAURI-05 | Phase 1 | Complete |
| TAURI-06 | Phase 1 | Complete |
| VSTOR-01 | Phase 1 | Complete |
| VSTOR-02 | Phase 1 | Complete |
| VSTOR-03 | Phase 1 | Complete |
| VSTOR-04 | Phase 1 | Complete |
| DPIP-01 | Phase 2 | Complete |
| DPIP-02 | Phase 2 | Complete |
| DPIP-03 | Phase 2 | Complete |
| DPIP-04 | Phase 2 | Complete |
| DPIP-05 | Phase 2 | Complete |
| DPIP-06 | Phase 2 | Complete |
| DPIP-07 | Phase 2 | Complete |
| DPIP-08 | Phase 2 | Complete |
| DPIP-09 | Phase 2 | Complete |
| FWAT-01 | Phase 2 | Complete |
| FWAT-02 | Phase 2 | Complete |
| FWAT-03 | Phase 2 | Complete |
| FWAT-04 | Phase 2 | Complete |
| FWAT-05 | Phase 5 | Complete |
| FWAT-06 | Phase 5 | Complete |
| SRCH-01 | Phase 3 | Complete |
| SRCH-02 | Phase 3 | Complete |
| SRCH-03 | Phase 3 | Complete |
| SRCH-04 | Phase 3 | Complete |
| SRCH-05 | Phase 3 | Complete |
| SRCH-06 | Phase 3 | Complete |
| SPAC-01 | Phase 3 | Complete |
| SPAC-02 | Phase 3 | Complete |
| SPAC-03 | Phase 3 | Complete |
| SPAC-04 | Phase 3 | Complete |
| SPAC-05 | Phase 3 | Complete |
| SPAC-06 | Phase 3 | Complete |
| SPAC-07 | Phase 3 | Complete |
| INTL-01 | Phase 3 | Complete |
| INTL-02 | Phase 5 | Complete |
| INTL-03 | Phase 3 | Complete |
| INTL-04 | Phase 3 | Complete |
| PAGE-01 | Phase 4 | Complete |
| PAGE-02 | Phase 4 | Complete |
| PAGE-03 | Phase 4 | Complete |
| PAGE-04 | Phase 4 | Complete |
| PAGE-05 | Phase 4 | Complete |
| PAGE-06 | Phase 5 | Complete |
| PAGE-07 | Phase 4 | Complete |
| PAGE-08 | Phase 5 | Complete |
| PAGE-09 | Phase 4 | Complete |
| PAGE-10 | Phase 5 | Complete |
| PAGE-11 | Phase 5 | Complete |
| PAGE-12 | Phase 5 | Complete |
| UX-01 | Phase 4 | Complete |
| UX-02 | Phase 4 | Complete |
| UX-03 | Phase 4 | Complete |
| UX-04 | Phase 5 | Complete |
| UX-05 | Phase 6 | Complete |
| UX-06 | Phase 6 | Complete |
| PAGE-13 | Phase 6 | Complete |
| KG-01 | Phase 6 | Complete |
| KG-02 | Phase 6 | Complete |
| KG-03 | Phase 6 | Complete |
| KG-04 | Phase 6 | Complete |
| KG-05 | Phase 6 | Complete |
| AIPV-01 | Phase 7 | Complete |
| AIPV-02 | Phase 7 | Complete |
| AIPV-03 | Phase 7 | Complete |
| AIPV-04 | Phase 7 | Complete |
| AIPV-05 | Phase 7 | Complete |
| AIPV-06 | Phase 7 | Complete |
| AIPV-07 | Phase 7 | Complete |
| AIPV-08 | Phase 7 | Complete |
| LLME-01 | Phase 8 | Complete |
| LLME-02 | Phase 8 | Complete |
| LLME-03 | Phase 8 | Complete |
| LLME-04 | Phase 8 | Complete |
| LLME-05 | Phase 8 | Complete |
| LLME-06 | Phase 8 | Complete |
| LLML-01 | Phase 9 | Complete |
| LLML-02 | Phase 9 | Complete |
| LLML-03 | Phase 9 | Complete |
| LLML-04 | Phase 9 | Complete |
| LLML-05 | Phase 9 | Complete |
| HSPC-01 | Phase 10 | Complete |
| HSPC-02 | Phase 10 | Complete |
| HSPC-03 | Phase 10 | Complete |
| HSPC-04 | Phase 10 | Complete |
| ENEX-01 | Phase 11 | Complete |
| ENEX-02 | Phase 11 | Complete |
| ENEX-03 | Phase 11 | Complete |
| ENEX-04 | Phase 11 | Complete |

**Coverage:**

- v1 requirements: 66 total (58 original + 8 Phase 6 Knowledge Graph requirements)
- v1.1 requirements: 27 total (AIPV-01..08, LLME-01..06, LLML-01..05, HSPC-01..04, ENEX-01..04)
- v1 mapped to phases: 66
- v1.1 mapped to phases: 27
- Unmapped: 0

---
*Requirements defined: 2026-02-27*
*Last updated: 2026-06-30 after v1.1 roadmap creation (traceability populated for Phases 7-11)*

# Roadmap: Cortex

## Overview

Cortex starts as an existing React frontend with mock data and transforms into a fully working Tauri 2 desktop app that indexes documents locally, auto-organizes them into AI-discovered Smart Spaces, and lets users find anything with natural language search. Four phases deliver this: stand up the Tauri shell and type contracts (Phase 1), build the full document ingestion pipeline with file watching (Phase 2), wire GNN clustering and semantic search intelligence (Phase 3), then flip the frontend from mock data to live backend and ship the complete UI (Phase 4).

v1.1 continues from Phase 7. It replaces the v1.0 heuristic clustering and bert-base-NER stack with LLM-driven entity extraction and Space labeling, introduces hierarchical sub-spaces, and adds entity-driven exploration — delivering production-quality Smart Spaces on diverse personal corpora.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Tauri Foundation** - Tauri 2 shell, type contracts, vector storage, and spawn_blocking patterns established before any pipeline code (completed 2026-02-27)
- [x] **Phase 2: Document Pipeline and File Watching** - Full ingestion loop: parse, embed, hash, extract entities, watch folders, index in background (completed 2026-02-28)
- [x] **Phase 3: Search Intelligence and Smart Spaces** - Semantic search, GNN clustering, graph edges, SONA self-learning, attention re-ranking (completed 2026-02-28)
- [x] **Phase 4: Frontend Integration and UX** - All 12 pages wired to live backend, command palette, onboarding, system tray, keyboard shortcuts (completed 2026-02-28)
- [x] **Phase 5: Integration Fixes and Gap Closure** - Fix 6 integration breaks: IPC arg mismatches, event wiring, settings persistence, onboarding layout, path_index rebuild (completed 2026-03-13)
- [x] **Phase 6: Knowledge Graph and Native Integrations** - Promote entities to first-class graph nodes, add native folder picker, in-app file preview (PDF/image/text), and Open in OS (completed 2026-06-29)
- [x] **Phase 7: AI Provider Foundation** - Pluggable AI backend (Anthropic, OpenAI, Gemini, Ollama) with credential storage, provider selection UI, and onboarding step (gap closure in progress 2026-07-02 — Codex/Gemini OAuth PKCE) (completed 2026-07-02)
- [ ] **Phase 8: LLM Entity Extraction** - Replace bert-base-NER with active AI provider extraction; backfill existing index; remove ONNX model and deps
- [ ] **Phase 9: LLM Space Labeling** - Replace heuristic name_space() with LLM-generated labels and descriptions; fingerprint-based cache
- [ ] **Phase 10: Hierarchical Spaces** - Sub-clustering for large spaces; LLM-labeled sub-spaces; sidebar drill-down
- [ ] **Phase 11: Entity-Driven Exploration** - Entity chip filtering, saved searches as virtual Spaces, Related panel on document detail
- [ ] **Phase 11.5: Ontology / Relation Extraction** - Cross-document knowledge graph via Pass 3 relation extractor, TripleStore, Entity Relations panel, and /ownership page
- [ ] **Phase 11.6: Adaptive Ontology + Corpus-Seeded Bootstrap** - Ontology (predicates + entity subtypes) grows from the user's corpus via bootstrap, adaptive predicate discovery, entity normalization, frequency-weighted ranking, and a consolidation loop
- [ ] **Phase 11.7: Chat with Your Docs (RAG)** - ChatGPT-style streaming chat interface with cited answers backed by top-K retrieval + chunk reranking + Tauri event streaming

## Phase Details

### Phase 1: Tauri Foundation

**Goal**: The Tauri 2 desktop app compiles and runs with all type contracts, IPC plumbing, and vector storage foundations in place — the safe architectural base every subsequent phase builds on.
**Depends on**: Nothing (first phase)
**Requirements**: TAURI-01, TAURI-02, TAURI-03, TAURI-04, TAURI-05, TAURI-06, VSTOR-01, VSTOR-02, VSTOR-03, VSTOR-04
**Success Criteria** (what must be TRUE):

  1. The app launches as a Tauri 2 desktop window showing the React frontend (Express server is gone)
  2. All IPC commands return typed AppError values — no raw strings or panics crossing the bridge
  3. CPU-bound operations use spawn_blocking — dev tools confirm no Tokio runtime blocking during heavy calls
  4. RuVector core is initialized with multi-collection support and metadata filtering ready to receive documents
  5. Frontend hooks operate in mock-data mode by default, switch to Tauri invoke when runtime is present

**Plans**: 6 plans

  - Plan 01 (Wave 1): Backend deps, Tauri plugin wiring, capabilities + CSP + asset protocol, ONNX model bundle, frontend deps install (KG-01, KG-05, UX-05, UX-06, PAGE-13 — foundational)
  - Plan 02 (Wave 2): NerService (ort + bert-base-NER) + entities.rs extension (email type fix, dedup-by-pair, NER merge) + types.rs + indexer hook + AppState/lib.rs wiring (KG-01, KG-02)
  - Plan 03 (Wave 3): EntityStore (alias merge, split, related, rename) + 6 entity IPC commands + read_document_text + Tokio backfill task with throttled progress events + Wave 0 fixtures (KG-01..KG-05, PAGE-13)
  - Plan 04 (Wave 2): Frontend types mirror + native folder picker on WatchedPage + DocumentContextMenu + DocumentRow extraction + context-menu wiring on search/recent/favorites/spaces-detail (UX-05, UX-06)
  - Plan 05 (Wave 3): 7 file preview components (FilePreview/PdfPreview/ImagePreview/TextPreview/MarkdownPreview/SizeGuardCard/UnsupportedPreview) + usePreview hook + DocumentPage header buttons + entity-chip-as-Link (PAGE-13, UX-06)
  - Plan 06 (Wave 4): Entity UI (9 components) + EntitiesPage + EntityDetailPage + 7 React Query hooks + BackfillIndicator + useBackfillProgress + Sidebar Entities link + REQUIREMENTS.md update + end-to-end UX checkpoint (KG-01, KG-03, KG-04, KG-05)

### Phase 2: Document Pipeline and File Watching

**Goal**: Documents in watched folders are automatically discovered, parsed, embedded, and indexed — the complete data flow from file on disk to searchable vector in RuVector.
**Depends on**: Phase 1
**Requirements**: DPIP-01, DPIP-02, DPIP-03, DPIP-04, DPIP-05, DPIP-06, DPIP-07, DPIP-08, DPIP-09, FWAT-01, FWAT-02, FWAT-03, FWAT-04, FWAT-05, FWAT-06
**Success Criteria** (what must be TRUE):

  1. User drops a folder of PDFs, DOCXs, and text files — all are indexed within seconds without any manual action
  2. Background indexing progress appears in the UI as files are processed, with no UI freezes
  3. A modified document is re-indexed automatically (old vector replaced); unchanged files are skipped via content hash
  4. Dates, amounts, people, organizations, and locations are extracted and stored as document metadata
  5. Folder exclusions (node_modules, .git, hidden files) and per-folder file-type toggles work as configured

**Plans**: TBD

### Phase 3: Search Intelligence and Smart Spaces

**Goal**: Users can find any indexed document with natural language search, and the system automatically discovers meaningful Smart Spaces through GNN clustering — the defining intelligence of Cortex.
**Depends on**: Phase 2
**Requirements**: SRCH-01, SRCH-02, SRCH-03, SRCH-04, SRCH-05, SRCH-06, SPAC-01, SPAC-02, SPAC-03, SPAC-04, SPAC-05, SPAC-06, SPAC-07, INTL-01, INTL-02, INTL-03, INTL-04
**Success Criteria** (what must be TRUE):

  1. User types a natural language query and gets ranked results with highlighted excerpts showing why each matched
  2. Metadata filters (type, date range, space) narrow search results before vector lookup
  3. Smart Spaces appear automatically after indexing — documents cluster into named groups (Property, Work, Medical) without user configuration
  4. User can move a document to a different space manually; moving one document does not trigger full re-cluster
  5. Related documents panel shows graph-connected documents for any open document
  6. Space network graph data is available for visualization (relationships between spaces)

**Plans**: TBD

### Phase 4: Frontend Integration and UX

**Goal**: Every page in the app shows live data from the Rust backend — mock data is gone, all 12 routes are functional, and the complete UX (onboarding, command palette, keyboard shortcuts, system tray) is operational.
**Depends on**: Phase 3
**Requirements**: PAGE-01, PAGE-02, PAGE-03, PAGE-04, PAGE-05, PAGE-06, PAGE-07, PAGE-08, PAGE-09, PAGE-10, PAGE-11, PAGE-12, UX-01, UX-02, UX-03, UX-04
**Success Criteria** (what must be TRUE):

  1. New user completes the 4-step onboarding wizard, selects a folder, watches the scanning progress, and lands on a populated Spaces view
  2. All 12 pages display live backend data — no mock data visible in any route
  3. Cmd+K command palette opens from any page and navigates to any route or executes any search
  4. Watched folders management page shows real folder status and lets user pause/remove/reconfigure folders
  5. Insights page renders donut chart, area chart, and space network graph from real indexed data

**Plans**:

  - Plan 01 (Wave 1): Type alignment — serde camelCase on Rust, align TS types, update mock data
  - Plan 02 (Wave 2): Dashboard + layout wiring — Index page, Sidebar, TopBar use live hooks; add missing backend commands
  - Plan 03 (Wave 2): Core pages — Spaces grid, Space detail, Search (split-pane), Document detail
  - Plan 04 (Wave 2): Secondary pages — Recent (timeline), Favorites, Tags (cloud+list), Watched Folders (management)
  - Plan 05 (Wave 2): Analytics + Settings — Insights (4 chart types, network graph), Settings (6 tabs)
  - Plan 06 (Wave 3): UX polish — Onboarding wizard, Cmd+K command palette, keyboard shortcuts, indexing indicator, final cleanup

### Phase 5: Integration Fixes and Gap Closure

**Goal**: Fix all 6 integration breaks so every IPC command works correctly, events flow from backend to frontend, settings persist, and onboarding renders fullscreen.
**Depends on**: Phase 4
**Requirements**: INTL-02, FWAT-05, FWAT-06, PAGE-06, PAGE-08, PAGE-10, PAGE-11, PAGE-12, UX-04
**Gap Closure**: Closes 6 integration breaks from v1.0-MILESTONE-AUDIT.md
**Success Criteria** (what must be TRUE):

  1. toggle_favorite and record_search_click IPC commands succeed at runtime (no arg mismatch errors)
  2. TopBar indexing indicator activates during background indexing (event listener wired)
  3. WatchedPage scan progress updates correctly (field names and status strings match)
  4. Previously-indexed documents are not re-embedded on app restart (path_index rebuilt)
  5. Settings persist across app restarts (JSON file in app_data_dir)
  6. Onboarding wizard renders fullscreen without Sidebar/TopBar

**Plans**:

  - Plan 01: Rust backend fixes — IPC param names, IndexProgress serde camelCase, path_index rebuild
  - Plan 02: Frontend + settings wiring — event listener, settings persistence, onboarding route, WatchedPage fixes

### Phase 6: Knowledge Graph and Native Integrations

**Goal**: Cortex moves from "doc auto-organizer" to "knowledge-graph-backed personal brain" — entities (Property, Person, Organization, Amount, Date) become first-class graph nodes that users can click to see every related document, the native folder picker replaces the manual path text input, and any indexed file can be previewed in-app or opened in the OS default application.
**Depends on**: Phase 5
**Requirements**: KG-01, KG-02, KG-03, KG-04, KG-05, UX-05, PAGE-13, UX-06
**Success Criteria** (what must be TRUE):

  1. Entities extracted from documents appear as graph nodes; clicking an entity surfaces every document mentioning it
  2. Entity normalization merges aliases (e.g., "123 Main St" and "Main Street property") so duplicates collapse
  3. Add Watched Folder opens a native OS folder picker; manual path typing is gone
  4. Document detail page renders an in-app preview for PDF, image, plain-text, and markdown files (not just a 200-char excerpt)
  5. Open in Finder / Open with default app works from Document detail and search results
  6. Knowledge graph is queryable via IPC — frontend can request "entities by type", "documents for entity", "related entities"

**Plans**: 7 plans

  - Plan 01 (Wave 1): Backend deps, Tauri plugin wiring (dialog/opener/fs), capabilities + CSP + asset protocol, ONNX model bundle, frontend deps install (KG-01, KG-05, UX-05, UX-06, PAGE-13 — foundational)
  - Plan 02 (Wave 2): NerService (ort + bert-base-NER) + entities.rs extension (email type fix, dedup-by-pair, NER merge) + types.rs + indexer hook + AppState/lib.rs wiring (KG-01, KG-02)
  - Plan 03 (Wave 3): EntityStore (alias merge, split, related, rename) + 6 entity IPC commands + read_document_text + Tokio backfill task with throttled progress events + Wave 0 fixtures + F1-floor test (KG-01..KG-05, PAGE-13)
  - Plan 04 (Wave 2): Frontend types mirror + native folder picker on WatchedPage (with D-19 client-side directory validation) + DocumentContextMenu + DocumentRow extraction + context-menu wiring on search/recent/favorites/spaces-detail (UX-05, UX-06)
  - Plan 05 (Wave 3): 7 file preview components (FilePreview/PdfPreview/ImagePreview/TextPreview/MarkdownPreview/SizeGuardCard/UnsupportedPreview) + usePreview hook + DocumentPage header buttons + entity-chip-as-Link (PAGE-13, UX-06)
  - Plan 06 (Wave 4): 5 entity components (EntityChip/EntityTypeBadge/EntityCard/EntityTypeFilterBar/RelatedEntityChip) + EntitiesPage + 5 React Query read hooks + Sidebar Entities link + App.tsx /entities route + DocumentPage chip swap + mock-data (KG-01, KG-03, PAGE-13)
  - Plan 07 (Wave 5): EntityDetailPage + 4 entity components (EntityDetailHeader/AliasChipList/AliasChip/SplitAliasDialog) + 2 mutation hooks (rename + split) + BackfillIndicator + useBackfillProgress + useBackfillStore + AppShell mount + App.tsx /entities/:id route + REQUIREMENTS.md update + end-to-end UX checkpoint (KG-01, KG-04, KG-05)

### Phase 7: AI Provider Foundation

**Goal**: Users can authenticate with at least one AI provider (Anthropic, OpenAI, Gemini, or Ollama) and Cortex routes all LLM calls through a single pluggable backend — the prerequisite for every LLM-driven feature in v1.1.
**Depends on**: Phase 6
**Requirements**: AIPV-01, AIPV-02, AIPV-03, AIPV-04, AIPV-05, AIPV-06, AIPV-07, AIPV-08
**Success Criteria** (what must be TRUE):

  1. User opens Settings → AI tab and sees all four providers (Anthropic, OpenAI, Gemini, Ollama) with their authentication status
  2. User can connect Anthropic via OAuth setup-token or API key; the provider card shows "Connected" after a successful credential save
  3. User can set Ollama as the active provider by entering a base URL and model name; Cortex routes a test ping through Ollama and reports success or a human-readable error
  4. User can switch the active provider from a dropdown; any subsequent LLM calls immediately use the newly selected provider
  5. Credentials survive an app restart (persisted in app_data_dir/credentials.json) and are removed cleanly by clicking "Disconnect"
  6. First-run onboarding includes a "Connect AI" step before completing; user can skip it and configure later from Settings
  7. **(Gap closure D-22..D-25)** User can sign into OpenAI via their ChatGPT/Codex subscription (OAuth PKCE) instead of pasting an API key; Gemini offers a symmetric "Sign in with Google" path (subject to 07-OAUTH-RESEARCH.md's Option A/B/C decision); access + refresh tokens are stored and refreshed transparently so long sessions survive weeks

**UI hint**: yes

**Plans**: 10 plans (6 original + 4 gap closure)

  - Plan 01 (Wave 1): Backend deps + auth/ module port (AuthState, CredentialStore, OAuth flow, save_setup_token, 23 unit tests) — AIPV-01, 02, 03, 04, 07, 08
  - Plan 02 (Wave 2): ai/ module port (anthropic_chat, openai_chat, gemini_chat, ollama_chat, ai_request router, retry/backoff, 13+ unit tests) — AIPV-01, 02, 03, 04, 08
  - Plan 03 (Wave 3): IPC wiring (commands/ai.rs with 8 commands, lib.rs manage() + invoke_handler) — AIPV-01..05, 07, 08
  - Plan 04 (Wave 4): Frontend foundation — TS types, 6 React Query hooks in useTauri.ts, useAiBannerStore (no persist), mock-data, stores.test.ts — AIPV-05, 06, 07, 08
  - Plan 05 (Wave 5): Settings AI tab — ProviderCard + AiProvidersSection + D-20 embedding unification, ProviderCard.test.tsx, end-to-end UX checkpoint — AIPV-01..05, 07, 08
  - Plan 06 (Wave 5, parallel to 05): Onboarding Step 2 (ConnectAiStep 2x2 grid) + AiNoProviderBanner + AppShell mount, OnboardingPage.test.tsx, first-run UX checkpoint — AIPV-06
  - **Plan 07 (Wave 6, gap closure): Research spike — codex CLI OAuth endpoints + Google Gemini OAuth endpoints. Writes 07-OAUTH-RESEARCH.md. Blocks downstream on absent public references — AIPV-01, 02, 03, 07**
  - **Plan 08 (Wave 7, gap closure): auth/pkce.rs shared PKCE flow module + auth/loopback.rs listener + ProviderCredential struct extension (refresh_token, expires_at) + ai_request refresh preflight + 401 retry — AIPV-01, 02, 03, 07**
  - **Plan 09 (Wave 8, gap closure): start_openai_oauth IPC command + optional start_gemini_oauth (Option A/B) + Codex chat routing + AI_PROVIDERS allow-list update — AIPV-02, 03, 07**
  - **Plan 10 (Wave 9, gap closure): Two-mode OpenAI card (Sign in with ChatGPT / Use API key instead) + Gemini card (Option A/B mirror) + useStartOpenAiOAuth hook + UI-SPEC amendment + human verification checkpoint — AIPV-02, 03, 07**

### Phase 8: LLM Entity Extraction

**Goal**: Every document's entities are extracted by the active AI provider, not bert-base-NER — the quality bar is mockEntities diversity (person, org, location, date, amount, email, topic) across any document domain.
**Depends on**: Phase 7
**Requirements**: LLME-01, LLME-02, LLME-03, LLME-04, LLME-05, LLME-06
**Success Criteria** (what must be TRUE):

  1. Opening /document/:id on a tax PDF shows entities including amounts, dates, and organizations extracted by the active LLM — not a bert-NER model
  2. Entity extraction works across unlike document types (a recipe returns ingredient/person, a medical note returns person/organization/date) without any config change
  3. Re-extracting the same document returns the same entity set (idempotent); a single document failure logs a warning and the rest of the index continues unaffected
  4. User triggers backfill from Settings → AI after switching provider; TopBar shows "Backfilling entities — X of Y done (ETA Zs)" and remains usable during backfill
  5. bert-base-NER.onnx, tokenizer files, ort, and tokenizers crates are absent from the repo; `cargo check` succeeds without them

**UI hint**: yes

**Plans**: 10 plans

  - Plan 01 (Wave 1): Types schema extension (ExtractedEntity + ExtractedEntities + Settings + normalize_tag) — LLME-01, 02, 05
  - Plan 02 (Wave 2): Pass1PatternExtractor (regex + Aadhaar/IBAN/credit-card/PAN/SSN/NINO/SIN/VIN/GSTIN validators) + cargo add gate (dateparser/iban_validate/verhoeff/luhn) — LLME-02, 03
  - Plan 03 (Wave 2): Pass2LlmRefiner (REFINE_PROMPT + JSON fence-strip + semaphore + chunking) — LLME-01, 02, 03
  - Plan 04 (Wave 2): Frontend types + hooks + mock-data (useExtractionSettings, useUpdateExtractionSettings, useTriggerEntityBackfill) — LLME-01, 05
  - Plan 05 (Wave 3): TwoPassExtractor facade + merge policy + 3 IPC commands + AppState wiring — LLME-01, 03, 04, 05
  - Plan 06 (Wave 4): Backfill async rewire (float version gate fix, ETA calculator, fallbacks counter, provider-disconnect handling) — LLME-04, 05
  - Plan 07 (Wave 4): Settings UI — ExtractionSettings + BackfillIndicator copy + AiNoProviderBanner extension + provider-switch/completion toasts — LLME-01, 05
  - Plan 08 (Wave 5): Frontend entity display — TopicChip + TagChip + ConfidenceExpander + EntityChip 8-class icon map + DocumentPage sidebar — LLME-01, 02
  - Plan 09 (Wave 5): TopicFilterBar + get_topics IPC + SearchPage/TagsPage wiring — LLME-01, 02
  - Plan 10 (Wave 6): Indexer + lib.rs rewire + BERT/ort/tokenizers/ndarray deletion + empirical prompt validation checkpoint — LLME-01, 02, 06

### Phase 9: LLM Space Labeling

**Goal**: Every Smart Space has a 2-4 word label and a one-sentence description generated by the active AI provider — output quality approximates mockSpaces names on a real personal corpus, with no collisions. Cluster boundaries themselves upgrade from hand-rolled k-means to `ruvector-cluster`; new-cluster bootstrapping uses `ruvector-domain-expansion` to seed labels from existing knowledge.
**Depends on**: Phase 8
**Requirements**: LLML-01, LLML-02, LLML-03, LLML-04, LLML-05
**Success Criteria** (what must be TRUE):

  1. After indexing a mixed personal corpus, each Space shows a meaningful 2-4 word label (e.g., "Property Tax Records", "Kids School Docs") rather than "Space 1 — Related" or duplicate "Work" labels
  2. Hovering a Space card or opening /spaces/:id shows a one-sentence description of what the space contains
  3. Triggering recluster with an unchanged corpus does not call the LLM for spaces whose membership has shifted less than 20%; unchanged labels persist from cache
  4. No two Spaces share an identical label after labeling completes; if a collision is detected, the second space is re-prompted or suffixed automatically
  5. Spaces display a "Generating label..." placeholder while LLM labeling runs in the background; the app remains fully navigable during this time
  6. Clustering runs through `ruvector-cluster` (hand-rolled k-means in `spaces/clustering.rs` removed); recluster time for 10K docs ≤ current baseline within ±20%
  7. New spaces detected on recluster bootstrap their label via `ruvector-domain-expansion` transfer from existing spaces before falling back to a full LLM call

**UI hint**: yes

**Plans**: 8 plans

  - Plan 01 (Wave 1): Space type extension — Rust Space struct + TS Space interface + mockSpaces exercising new fields (LLML-01, LLML-02, LLML-05 — foundational types)
  - Plan 02 (Wave 1): Fingerprint + label cache modules — spaces/fingerprint.rs (SHA-256 + Jaccard) + spaces/label_cache.rs (space_labels.json sidecar, keyed by space_id per pitfall #4) (LLML-03)
  - Plan 03 (Wave 2): LLM Labeler + Domain Expansion Bootstrap — spaces/llm_labeler.rs with SPACE_LABEL_PROMPT (6 few-shot exemplars), label_cluster, collision resolver, try_bootstrap_from_nearest (pure cosine sim, replaces ruvector-domain-expansion per 09-RESEARCH.md), canonical entity hint, SpaceLabelingProgress event struct (LLML-01, LLML-02, LLML-04, SC7)
  - Plan 04 (Wave 3): SpaceManager integration + IPC commands — recluster loop wired to cache + labeler + collision retry + progress events + user_locked skip; 4 new IPC commands (get_space_labels, rename_space_label, clear_space_override, trigger_relabel); naming.rs marked as fallback per pitfall #2 (LLML-01, LLML-03, LLML-04, LLML-05, SC6 deviation note)
  - Plan 05 (Wave 2): Frontend types + hooks + progress store — useSpaceLabels, useRenameSpace, useClearSpaceOverride, useRelabelSpace, useSpaceLabelingProgress event listener, useSpaceLabelingStore (Zustand, non-persisted) (LLML-05)
  - Plan 06 (Wave 4): SpaceCard extraction + Phase 9 UI states — SpaceCard shimmer/tooltip/lock/entity-hint, EntityHintChip, SpaceLabelingIndicator, SpacesPage wiring, AppShell mount of useSpaceLabelingProgress (LLML-02 tooltip, LLML-05 shimmer, D-15, D-17)
  - Plan 07 (Wave 4): SpaceDetailPage inline label edit + description + regenerate — view/edit header states, Save/Cancel edit/Clear override buttons, Regenerate label ghost button, full description prose or entity-hint italic fallback, description shimmer (LLML-02 detail, D-15)
  - Plan 08 (Wave 5): End-to-end UX checkpoint — human verification of LLML-01..05 + D-15/D-17 on a real personal corpus, SC6 deviation acknowledgement

### Phase 10: Hierarchical Spaces

**Goal**: Large Smart Spaces automatically split into navigable sub-spaces, matching the mockSpaces hierarchy shape (e.g., Property → Tax → Insurance) — users can drill into sub-categories from both the sidebar and /spaces/:id. Underlying index becomes hierarchy-aware via `ruvector-hyperbolic-hnsw` so sub-space search stays logarithmic at depth.
**Depends on**: Phase 9
**Requirements**: HSPC-01, HSPC-02, HSPC-03, HSPC-04
**Success Criteria** (what must be TRUE):

  1. A Space containing more than 50 documents automatically shows sub-spaces on /spaces/:id; a Space under 50 docs shows no sub-spaces
  2. Sub-spaces have LLM-generated labels and are clickable, navigating to /spaces/:id with a parent breadcrumb showing the hierarchy
  3. Documents that do not fit any sub-cluster appear in a "Misc" sub-space within the parent; no documents are silently dropped
  4. Sidebar shows the top 5 Spaces with sub-counts (e.g., "Property (3)"); clicking a Space expands its sub-spaces inline without a page navigation
  5. Sub-space search uses `ruvector-hyperbolic-hnsw` for hierarchy-aware retrieval; navigating parent → child → grand-child returns results in ≤ 2× the time of a flat top-level search

**UI hint**: yes

**Plans**: 9 plans

  - Plan 01 (Wave 1): Rust types — Space struct (depth, sub_space_ids) + SpaceLabelEntry (parent_id, depth) with backward-compat serde defaults (HSPC-01..04)
  - Plan 02 (Wave 1): Frontend types + useSidebarStore expandedSpaceIds Set (no persist per D-13) (HSPC-01, HSPC-04)
  - Plan 03 (Wave 2): spaces/subspace_detector.rs — detect() + build_misc_space() + SUB_SPACE_THRESHOLD=50, MIN_SUB_CLUSTER_SIZE=3 (HSPC-01, HSPC-03)
  - Plan 04 (Wave 2): llm_labeler::label_sub_cluster — parent-context prompt reusing Phase 9 pipeline (HSPC-02)
  - Plan 05 (Wave 3): SpaceManager::recluster extension — sub-space pass, cache persistence with parent_id + depth, Misc bucket, D-08 parent-shift invalidation (HSPC-01..03)
  - Plan 06 (Wave 3): ruvector-hyperbolic-hnsw integration — AppState hyp_index + rebuild_hyp_index + SC5 perf gate (HSPC-01, HSPC-02, SC5)
  - Plan 07 (Wave 4): Sidebar refactor — top 5 filter, chevron expand, shadcn Collapsible sub-list, no Framer Motion (HSPC-04)
  - Plan 08 (Wave 4): SpaceDetailPage refactor — shadcn Breadcrumb + ParentContextBanner + extracted SubSpaceCard w/ isMisc dashed variant + flat filter (HSPC-02, HSPC-03)
  - Plan 09 (Wave 5): End-to-end UX checkpoint on real corpus — HSPC-01..04 + SC5 verification, SUMMARY records

Plans:
- [ ] 10-01-PLAN.md — Rust Space + SpaceLabelEntry hierarchy fields
- [ ] 10-02-PLAN.md — Frontend Space type + useSidebarStore expandedSpaceIds
- [ ] 10-03-PLAN.md — spaces/subspace_detector.rs module (detect + Misc)
- [ ] 10-04-PLAN.md — llm_labeler::label_sub_cluster parent-context variant
- [ ] 10-05-PLAN.md — SpaceManager::recluster sub-space pass orchestration
- [ ] 10-06-PLAN.md — ruvector-hyperbolic-hnsw secondary index + SC5 perf gate
- [ ] 10-07-PLAN.md — Sidebar chevron expand + top-5 + sub-count
- [ ] 10-08-PLAN.md — SpaceDetailPage breadcrumb + ParentContextBanner + SubSpaceCard
- [ ] 10-09-PLAN.md — End-to-end UX checkpoint on real corpus

### Phase 11: Entity-Driven Exploration

**Goal**: Users can navigate the corpus through entities — filtering views by entity chip, saving searches as persistent virtual Spaces, and seeing related documents on any document detail page.
**Depends on**: Phase 8
**Requirements**: ENEX-01, ENEX-02, ENEX-03, ENEX-04
**Success Criteria** (what must be TRUE):

  1. Clicking any entity chip on a document, search result, or space detail filters the current view to documents that mention that entity — no page reload required
  2. User can save the current search query as a named virtual Space from the search page; the saved Space appears in the Sidebar immediately alongside auto-clustered Spaces
  3. /document/:id shows a "Related" panel with up to 5 documents ranked by entity overlap and cosine similarity
  4. Saved search Spaces refresh their document count on Sidebar render, so newly indexed matching documents appear without the user manually triggering a refresh

**UI hint**: yes

**Plans**: 9 plans

  - Plan 01 (Wave 1): Types foundation — Rust `SearchFilters.entity_filters` + `EntityClassFilter/SavedSearch/SavedSearchFilters/RelatedDocScored/EntityPageData/RelatedEntityRef`; TS mirrors; queryKeys entries (ENEX-01..04 foundational)
  - Plan 02 (Wave 1): `SavedSearchStore` module mirroring `SpaceLabelCache` — JSON sidecar + 6 unit tests (ENEX-02, ENEX-04)
  - Plan 03 (Wave 2): `apply_entity_class_filters` in `search/filters.rs` + `execute_query` wiring for URL-driven entity filters (ENEX-01)
  - Plan 04 (Wave 2): 4 saved-search IPCs (save/delete/get/counts) + `AppState.saved_search_store` + `lib.rs` registration (ENEX-02, ENEX-04)
  - Plan 05 (Wave 1): EntityChip dual-navigation refactor — left-click filter, right-click entity page + `isActive` accent styling (ENEX-01)
  - Plan 06 (Wave 3): `get_related_docs_scored` (0.6*cosine + 0.4*jaccard) + `get_entity_page_data` IPCs + `lib.rs` registration (ENEX-01, ENEX-03)
  - Plan 07 (Wave 4): 6 React Query hooks + SearchPage URL-param entity filter + `EntityFilterBar`/`EntityFilterPill`/`SaveSearchDialog` + `ScoreBadge` extraction (ENEX-01, ENEX-02)
  - Plan 08 (Wave 4): `EntityDetailPage11` at `/entity/:class/:value` — header + aliases + paginated docs + co-occurring entities + empty/error states (ENEX-01)
  - Plan 09 (Wave 5): Sidebar "Saved Searches" section + DocumentPage Related panel switch to scored variant + end-to-end human-verify checkpoint (ENEX-02, ENEX-03, ENEX-04)

Plans:
- [ ] 11-01-PLAN.md — Rust + TS type foundation (SearchFilters.entity_filters, SavedSearch, EntityClassFilter, RelatedDocScored, EntityPageData) + queryKeys entries
- [ ] 11-02-PLAN.md — SavedSearchStore JSON-sidecar module (mirrors SpaceLabelCache)
- [ ] 11-03-PLAN.md — apply_entity_class_filters backend + execute_query wiring
- [ ] 11-04-PLAN.md — 4 saved-search IPCs + AppState + lib.rs registration
- [ ] 11-05-PLAN.md — EntityChip dual-navigation refactor (left=filter, right=entity page)
- [ ] 11-06-PLAN.md — get_related_docs_scored + get_entity_page_data IPCs
- [ ] 11-07-PLAN.md — 6 hooks + SearchPage URL filter + SaveSearchDialog + ScoreBadge extraction
- [ ] 11-08-PLAN.md — EntityDetailPage11 new route /entity/:class/:value
- [ ] 11-09-PLAN.md — Sidebar Saved Searches + DocumentPage Related panel + human-verify checkpoint

### Phase 11.5: Ontology / Relation Extraction

**Goal**: Users can navigate a real cross-document knowledge graph — every entity page shows the relations it participates in (owns, located_in, married_to, ...) and a dedicated /ownership page rolls up all assets for the primary Person entity, backed by an LLM-driven Pass 3 relation extractor writing to a persisted TripleStore.
**Depends on**: Phase 11
**Requirements**: ONTO-01, ONTO-02, ONTO-03, ONTO-04, ONTO-05
**Success Criteria** (what must be TRUE):

  1. After backfill, docs progress from entities_version=3.0 to 3.5; a persisted `triples.json` sidecar contains subject_id/predicate/object_id/doc_ids/user_added tuples per D-11
  2. Every entity detail page (/entity/:class/:value) shows a Relations section listing outgoing + incoming triples with target entity chips clickable to their own entity pages
  3. /ownership/:personId groups assets by AssetType (Property / Vehicle / Investment / Business / Financial) — empty sections hidden per D-14
  4. Auto-inverse writes: storing (A, owns, B) automatically writes (B, owned_by, A) so queries in either direction return results; symmetric predicates (married_to, partner_of) write both directions per D-03
  5. User-added triples (add_manual_triple IPC) carry user_added=true and are preserved across LLM re-runs per D-12; user can delete any triple via UI and the deletion also removes the auto-inverse partner

**UI hint**: yes

**Plans**: 8 plans

  - Plan 01 (Wave 1): Types foundation — Rust `Triple`/`TripleWithEntities`/`RelationsPageData`/`OwnershipPageData`/`AssetType`/`PredicateSubjectPair`/`PredicateObjectPair` + `PREDICATE_VOCABULARY` (21 items) + `PASS3_TARGET_VERSION=3.5` + TS mirrors + queryKeys entries (ONTO-01..05 foundational)
  - Plan 02 (Wave 2): `TripleStore` module — JSON sidecar mirror of `SavedSearchStore` w/ forward + reverse HashMap indices, auto-inverse + symmetric writes, user_added preservation on upsert (ONTO-02, ONTO-05)
  - Plan 03 (Wave 2): `Pass3RelationExtractor` module — mirror of `Pass2LlmRefiner` w/ `EXTRACT_RELATIONS_PROMPT` + 21-predicate lock + fence-strip + retry + semaphore (ONTO-01)
  - Plan 04 (Wave 3): Extend `backfill_one_doc_async` to call Pass 3 after Pass 2, upsert to TripleStore w/ `cleanup_doc` + `upsert_from_doc`, bump `PASS3_TARGET_VERSION` gate; extend `TwoPassExtractor` w/ `pass3()` accessor + model propagation (ONTO-01, ONTO-02)
  - Plan 05 (Wave 4): 7 IPC commands (`get_entity_relations`, `get_all_owned_by`, `get_all_related_to`, `get_subjects_by_predicate_object`, `get_objects_by_subject_predicate`, `add_manual_triple`, `delete_triple`) + `AppState.triple_store` + `lib.rs` handler registration + backfill call-site update (ONTO-02..05)
  - Plan 06 (Wave 5): 7 React Query hooks (`useEntityRelations`, `useAllOwnedBy`, `useAllRelatedTo`, `useTriplesBySubjectPredicate`, `useTriplesByPredicateObject`, `useAddManualTriple`, `useDeleteTriple`) w/ mutation invalidations (ONTO-03..05)
  - Plan 07 (Wave 6): `EntityRelationsPanel` component (extends `/entity/:class/:value`) + `/ownership/:personId` route (`OwnershipPage`) w/ 6 asset-type sections + `OwnershipAssetSection` + `DeleteTripleButton` w/ AlertDialog confirmation (ONTO-03, ONTO-04, ONTO-05)
  - Plan 08 (Wave 7): Sidebar "Owned by me" quick link + `useTopPersonId` helper + end-to-end human-verify checkpoint against real corpus (ONTO-04)

Plans:
- [x] 11.5-01-PLAN.md — Rust + TS type foundation (Triple, RelationsPageData, OwnershipPageData, AssetType, PREDICATE_VOCABULARY, PASS3_TARGET_VERSION) + queryKeys entries
- [x] 11.5-02-PLAN.md — TripleStore JSON-sidecar module w/ forward + reverse indices + auto-inverse + symmetric writes
- [x] 11.5-03-PLAN.md — Pass3RelationExtractor module mirroring Pass2LlmRefiner (prompt, fence-strip, retry, semaphore)
- [x] 11.5-04-PLAN.md — Backfill wiring: Pass 3 stage + PASS3_TARGET_VERSION gate + TripleStore persistence
- [x] 11.5-05-PLAN.md — 7 relation IPC commands + AppState.triple_store + lib.rs registration + trigger_entity_backfill caller update
- [x] 11.5-06-PLAN.md — 7 React Query hooks (3 reads + 2 exploratory + 2 mutations)
- [x] 11.5-07-PLAN.md — EntityRelationsPanel + /ownership/:personId route + OwnershipAssetSection + DeleteTripleButton
- [x] 11.5-08-PLAN.md — Sidebar "Owned by me" link + useTopPersonId + end-to-end human-verify checkpoint (checkpoint deferred to real-corpus UAT)

### Phase 11.6: Adaptive Ontology + Corpus-Seeded Bootstrap

**Goal**: Cortex's ontology (predicates + entity classes + subtypes) grows from the user's corpus instead of being frozen at the Phase 11.5 21-predicate + 8-class seed — corpus-seeded bootstrap on first run, adaptive predicate discovery from Pass 3, entity normalization (canonical_short_name), frequency-weighted entity ranking, and a background consolidation loop with user-approved diffs in Settings.
**Depends on**: Phase 11.5 (Ontology / Relation Extraction)
**Requirements**: ONTO-01, ONTO-02
**Success Criteria** (what must be TRUE):

  1. `SEED_PREDICATES` (Phase 11.5's frozen 21-predicate list) is never mutated; new predicates from bootstrap/adaptive discovery land in separate `corpus_seed`/`adaptive_predicates`/`pending_predicates` collections
  2. `app_data_dir/ontology.json` persists the full ontology store schema and survives corrupt/partial-file reads via `#[serde(default)]` on every field
  3. `canonical_short_name` is populated for verbose entities post-normalization and displayed in the UI in place of raw values where present

**UI hint**: yes

**Plans**: 9 plans

  - Plan 01 (Wave 1): Rust + TS type foundation — `SEED_PREDICATES` alias, `Predicate`/`EntitySubclass`/`BootstrapSeed`/`ConsolidationKind`/`ConsolidationSuggestion`/`PendingConsolidation`/`OntologyStoreSchema`/`PromoteResult`/`PromotionSource` structs, `canonical_short_name` on `ExtractedEntity`/`CanonicalEntity`, TS mirrors + queryKeys entries (ONTO-01, ONTO-02)

Plans:
- [x] 11.6-01-PLAN.md — Rust + TS type foundation (SEED_PREDICATES, Predicate, EntitySubclass, BootstrapSeed, OntologyStoreSchema, canonical_short_name) + queryKeys entries
- [x] 11.6-02-PLAN.md — OntologyStore JSON sidecar (load/save, forward+reverse indices, effective_predicates merge, promote_pending min-support gate, apply_bootstrap/register_manual_predicate/rename_predicate/merge_predicates/reset_to_seed/apply_consolidation)
- [x] 11.6-03-PLAN.md — Entity normalizer (pipeline/entity_normalizer.rs): rule-based canonical_short_name (corporate-suffix strip for Organization, hyphen-unit segment for Location); wired into TwoPassExtractor.extract_full (all 3 return paths) and EntityStore.register_doc_entities (mode-across-aliases aggregation)
- [x] 11.6-04-PLAN.md — Corpus-seeded ontology bootstrap (pipeline/ontology_bootstrap.rs): BOOTSTRAP_PROMPT + OntologyBootstrapper.bootstrap() mirroring Pass 2/3 LLM plumbing, parse_bootstrap_json + validate_bootstrap (seed-dup/name/cap/entity-class guards); wired into backfill.rs pass2_success_count → fires exactly once at BOOTSTRAP_MIN_DOCS (30); AppState.ontology_store + auth_state added (Rule 3 deviation, unblocks Plan 06's Settings > Ontology IPC work)

### Phase 11.7: Chat with Your Docs (RAG)

**Goal**: Users can ask natural-language questions about their indexed corpus in a ChatGPT-style streaming chat interface and receive cited answers where each citation is a clickable chip that navigates to the source document with the exact chunk highlighted.
**Depends on**: Phase 7 (AI providers), Phase 8 (LLM entity extraction — provides quality metadata for retrieval)
**Requirements**: RAGCH-01, RAGCH-02, RAGCH-03, RAGCH-04, RAGCH-05, RAGCH-06, RAGCH-07, RAGCH-08
**Success Criteria** (what must be TRUE):

  1. Navigating to /chat opens a fresh session; typing a question and pressing Enter streams the assistant response token-by-token via Tauri events (chat-stream-token / chat-stream-complete)
  2. Retrieval pipeline runs: query embedded via MiniLM → HNSW top-8 docs → per-doc 500-char/50-overlap chunks → top-3 per doc → rerank to top-12 → RAG prompt (RAG_SYSTEM_PROMPT + numbered docs) → ai_request_stream
  3. Below cosine floor 0.35, the assistant responds "I couldn't find anything relevant in your library." WITHOUT calling the LLM
  4. Inline citation chips [1] [2] appear in the answer; clicking one navigates to /document/{docId}?highlight={chunkStart}-{chunkEnd} and DocumentPage scrolls to + highlights that range
  5. Sidebar has a Chat link + drawer of the 5 most-recent sessions; sessions persist across app restart via app_data_dir/chat_sessions.json
  6. Streaming works for all four providers (Anthropic SSE, OpenAI Chat Completions SSE, OpenAI Codex Responses SSE, Ollama NDJSON, Gemini streamGenerateContent SSE); Ollama non-streaming fallback emits a single final chunk

**UI hint**: yes

**Plans**: 7 plans

  - Plan 01 (Wave 1): Shared type contracts — Rust ChatSession/ChatMessage/Citation/StreamChunk + TS mirrors + queryKeys.chatSessions (RAGCH-01, RAGCH-05 foundational)
  - Plan 02 (Wave 1): ChatSessionStore module — JSON sidecar mirror of SavedSearchStore w/ create_session, append_message, rename_session, delete, list, get (RAGCH-06)
  - Plan 03 (Wave 1): DocumentPage ?highlight=start-end query param + FilePreview text/markdown scroll-and-mark (RAGCH-07)
  - Plan 04 (Wave 1): ai_request_stream + StreamChunk + per-provider streaming (Anthropic/OpenAI/Codex/Gemini SSE + Ollama NDJSON w/ fallback) (RAGCH-03, RAGCH-08)
  - Plan 05 (Wave 2): ChatEngine RAG pipeline (embed + HNSW + chunk + rerank + prompt + stream) + 4 IPC commands (start_chat, list/delete/rename_chat_session) + AppState wiring (RAGCH-01, RAGCH-02, RAGCH-04, RAGCH-05)
  - Plan 06 (Wave 3): ChatPage + 4 chat components (CitationChip, ChatInput, ChatMessageBubble, ChatMessageList) + useChatStream + 4 React Query hooks + /chat routes + human-verify checkpoint (RAGCH-02, RAGCH-03, RAGCH-04)
  - Plan 07 (Wave 3): Sidebar Chat link + recent-sessions drawer w/ delete confirmation + human-verify checkpoint (RAGCH-05, RAGCH-06)

Plans:
- [ ] 11.7-01-PLAN.md — Shared Rust + TS types + queryKeys
- [x] 11.7-02-PLAN.md — ChatSessionStore JSON sidecar
- [x] 11.7-03-PLAN.md — DocumentPage highlight query param + FilePreview scroll-and-mark
- [x] 11.7-04-PLAN.md — ai_request_stream + StreamChunk + per-provider streaming
- [x] 11.7-05-PLAN.md — ChatEngine + 4 IPC commands + AppState wiring
- [ ] 11.7-06-PLAN.md — ChatPage + streaming UI + citation chips + hooks + /chat routes
- [ ] 11.7-07-PLAN.md — Sidebar Chat link + recent-sessions drawer

### Phase 12: GNN Clustering Swap (ruvector-gnn)

**Goal**: Replace hand-rolled k-means in `spaces/clustering.rs` with `ruvector-gnn`. Message-passing over the doc-doc HNSW graph produces semantically-coherent clusters even when docs share no vocabulary — a receipt and its cover email land together via mutual neighbor overlap, not text similarity alone.
**Depends on**: Phase 9
**Requirements**: TBD (GNNC-01..GNNC-05 to be specified in discuss-phase)
**Success Criteria** (what must be TRUE):

  1. `ruvector-gnn` is a Cargo.toml dependency; k-means code in `spaces/clustering.rs` is deleted (no dead-code compat layer)
  2. Recluster on a canonical 500-doc benchmark corpus produces clusters whose internal cosine coherence ≥ current k-means baseline within ±5%
  3. GNN cluster labels (post-Phase 9 LLM naming) show measurable diversity improvement on the same corpus vs k-means-derived labels — labeling collision rate < 10%
  4. Recluster wall-clock time for 10K docs ≤ 2× current baseline; documented in SUMMARY.md if slower

**UI hint**: no

### Phase 13: Cypher Entity Graph (ruvector-graph)

**Goal**: Replace hand-rolled `graph/edges.rs` with `ruvector-graph`'s Cypher engine. Enables queries currently impossible: "docs mentioning Person X AND dated 2025", "spaces containing entity Y", "entities co-occurring with Z". Feeds Phase 11 Entity-Driven Exploration with real graph traversal.
**Depends on**: Phase 8, Phase 11
**Requirements**: TBD (GRPH-01..GRPH-06 to be specified in discuss-phase)
**Success Criteria** (what must be TRUE):

  1. `ruvector-graph` is a Cargo.toml dependency; entity nodes + document nodes + MENTIONS edges are built during indexing
  2. A Tauri IPC command `graph_query(cypher: String)` accepts a Cypher query and returns matching doc IDs or entity IDs
  3. Frontend entity chip click on a document detail page issues a Cypher query for co-mentioned entities and renders them as related-chip results
  4. Entity graph survives an app restart (persisted alongside HNSW vectors)
  5. Query performance: single-hop entity → docs query on 10K-doc corpus completes in < 100 ms

**UI hint**: yes

### Phase 14: SONA Feedback Loop Close

**Goal**: Read `ruvector-sona` trajectories back into ranking. Every search + click improves future results. Currently write-only — `SearchLearner` records but nothing consumes the signal. Ship eval harness alongside (MRR / NDCG) so lift is measurable, not vibes.
**Depends on**: Phase 3
**Requirements**: TBD (SONA-01..SONA-05 to be specified in discuss-phase)
**Success Criteria** (what must be TRUE):

  1. `ruvector-sona` engine's ranking-influence hook is wired into `search/query.rs` — trajectories change future result ordering
  2. Eval harness runs offline (recorded queries + judgments) and reports MRR + NDCG@10 baseline vs SONA-influenced
  3. On a benchmark of ≥ 100 recorded queries with click judgments, SONA-influenced ranking shows non-negative MRR delta vs baseline
  4. User can view a "Search learning" dashboard in Settings showing trajectory count + click-through rate over time
  5. User can reset the SONA learning state from Settings (privacy control)

**UI hint**: yes

### Phase 15: Visual Intelligence (rupixel)

**Goal**: Integrate [rupixel](https://github.com/ruvnet/rupixel) for image and screenshot understanding. Real thumbnails replace hex placeholders; screenshots become semantically searchable by content; image OCR gets preprocessing so tesseract accuracy improves.
**Depends on**: Phase 7 (for LLM description of screenshots)
**Requirements**: TBD (VIZI-01..VIZI-06 to be specified in discuss-phase)
**Success Criteria** (what must be TRUE):

  1. `rupixel` is a Cargo.toml dependency
  2. Document cards show real thumbnails: PDF first-page render for pdf, native image thumbnail for png/jpg — hex placeholder path only used as fallback on parse failure
  3. png/jpg documents pass through rupixel preprocessing (deskew, denoise, contrast) before OCR; OCR text quality measurably improves on a benchmark image set
  4. Screenshot indexing: rupixel detects text regions + UI element hierarchy → LLM generates description → description becomes searchable via existing HNSW pipeline
  5. A user search like "screenshot of Slack conversation about deploy" returns the actual screenshot from ~/Desktop even when the filename is `Screen Shot 2025-08-14 at 10.42.11 AM.png`
  6. Thumbnail rendering is background-async and cached; document card render is not blocked by rupixel work

**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14 → 15

Phase 14 (SONA feedback loop) and Phase 15 (rupixel visual intelligence) declare only their historical minimum dependency — they can be pulled forward if desired without breaking anything.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Tauri Foundation | 5/5 | Complete   | 2026-02-27 |
| 2. Document Pipeline and File Watching | 5/5 | Complete    | 2026-02-28 |
| 3. Search Intelligence and Smart Spaces | 5/5 | Complete    | 2026-02-28 |
| 4. Frontend Integration and UX | 6/6 | Complete | 2026-02-28 |
| 5. Integration Fixes and Gap Closure | 2/2 | Complete   | 2026-03-13 |
| 6. Knowledge Graph and Native Integrations | 7/7 | Complete   | 2026-06-29 |
| 7. AI Provider Foundation | 10/10 | Complete   | 2026-07-02 |
| 8. LLM Entity Extraction | 10/10 | Complete | 2026-07-03 |
| 9. LLM Space Labeling | 8/8 | Complete | 2026-07-05 |
| 10. Hierarchical Spaces | 9/9 | Complete | 2026-07-05 |
| 11. Entity-Driven Exploration | 9/9 | Complete | 2026-07-08 |
| 11.6 Adaptive Ontology + Corpus-Seeded Bootstrap | 4/9 | In Progress | - |
| 12. GNN Clustering Swap (ruvector-gnn) | 0/4 | Skipped (deviation) | 2026-07-08 |
| 13. Cypher Entity Graph (ruvector-graph) | 0/5 | Skipped (deviation — value delivered by Phase 11) | 2026-07-08 |
| 14. SONA Feedback Loop Close | 0/5 | Skipped (deviation — v1.2 candidate) | 2026-07-08 |
| 15. Visual Intelligence (rupixel) | 0/? | Skipped (deviation — v1.2 candidate) | 2026-07-08 |

---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: milestone
status: completed
stopped_at: Completed 11.7-03/05 + 11.6-04; ChatPage UI in-progress
last_updated: "2026-07-10T04:22:00Z"
last_activity: 2026-07-09
progress:
  total_phases: 18
  completed_phases: 7
  total_plans: 48
  completed_plans: 85
  percent: 39
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-06-30)

**Core value:** Documents sort themselves into meaningful spaces through AI-powered clustering; users find anything with natural language search -- all running locally.
**Current focus:** v1.1 milestone shipped; Phase 11.5 (Ontology / Relation Extraction) underway — Plan 08 (Sidebar quick link) complete; all 8 plans code-complete pending real-corpus UAT.

## Current Position

Phase: 11 complete; 12-15 skipped w/ documented deviations; 11.5 code-complete (Plan 08/8 complete; end-to-end human-verify checkpoint deferred to real-corpus UAT)
Status: v1.1 shipped; Phase 11.5 all plans complete pending real-corpus UAT sign-off
Last activity: 2026-07-09

## v1.1 Milestone Summary

**Shipped (Phases 8-11):**

- Phase 8 LLM Entity Extraction — two-pass engine (Pass 1 deterministic patterns + Pass 2 LLM refinement), 8 seed classes, free-form topic+tags, LLM-optional, BERT stack removed
- Phase 9 LLM Space Labeling — LlmSpaceLabeler + SpaceLabelCache w/ SHA-256 fingerprint (20% Jaccard shift gate), user-locked labels, collision retry, Settings edit/regenerate
- Phase 10 Hierarchical Spaces — recursive k-means sub-clustering (50-doc threshold, min-3, Misc rollup), ruvector-hyperbolic-hnsw secondary index, Sidebar chevron expand, breadcrumb navigation, parent context banner
- Phase 11 Entity-Driven Exploration — dual-nav EntityChip (left=filter search, right=entity page), URL-param entity filters w/ AND semantics, saved searches sidecar, 0.6*cosine+0.4*Jaccard related docs, entity detail page w/ co-occurring entities, batched Sidebar counts

**Skipped w/ documented deviations (v1.2 candidates):**

- Phase 12 GNN Clustering — ruvector-gnn is HNSW re-ranking, not clustering (audit finding); k-means retained
- Phase 13 Cypher Graph — 80% of value already delivered by Phase 11 get_entity_page_data + get_related_docs_scored; multi-hop Cypher deferred
- Phase 14 SONA Feedback Loop — dashboard + eval + ranking hook is a full v1.2 release on its own
- Phase 15 rupixel Visual Intelligence — CLIP pipeline is a v1.2 differentiator release

**Test coverage:**

- Rust: 470 lib tests passing, 21 ignored (all long-running benchmarks)
- Frontend: 340 tests passing, 0 failures
- Both build clean: `cargo check` + `npx tsc --noEmit`

## Roadmap Evolution

- 2026-06-25: Phase 6 added -- Knowledge Graph and Native Integrations. Extends v1.0 before milestone close. Covers entity-as-node graph (promote regex entities to first-class nodes with click-through), tauri-plugin-dialog folder picker, in-app file preview (PDF/image/text), and Open in OS via tauri-plugin-opener.
- 2026-06-30: v1.1 roadmap defined -- Phases 7-11 covering AI provider foundation, LLM entity extraction, LLM space labeling, hierarchical spaces, and entity-driven exploration.

## Performance Metrics

**Velocity:**

- Total plans completed: 36 (v1.0: 30 across 6 phases)
- Total execution time: ~2 hours (v1.0)

**By Phase:**

| Phase | Plans | Status |
|-------|-------|--------|
| 01-tauri-foundation | 5/5 | Complete |
| 02-document-pipeline-and-file-watching | 5/5 | Complete |
| 03-search-intelligence-and-smart-spaces | 5/5 | Complete |
| 04-frontend-integration-and-ux | 6/6 | Complete |
| 05-integration-fixes-and-gap-closure | 2/2 | Complete |
| 06-knowledge-graph-and-native-integrations | 7/7 | Complete |
| 07-ai-provider-foundation | 0/? | Not started |
| 08-llm-entity-extraction | 2/10 | In Progress |
| 09-llm-space-labeling | 0/? | Not started |
| 10-hierarchical-spaces | 0/? | Not started |
| 11-entity-driven-exploration | 0/? | Not started |

**Test Counts:**
| Phase | Tests Added | Total |
|-------|-------------|-------|
| Phase 1 | ~30 | 30 |
| Phase 2 | ~42 | 72 |
| Phase 3 | ~40 | 112 |
| Phase 04 P05 | 242s | 2 tasks | 3 files |
| Phase 06 P02 | 120 | 3 tasks | 11 files |
| Phase 08 P03 | 307s | 2 tasks | 2 files |
| Phase 08 P09 | 11 | 2 tasks | 10 files |
| Phase 09 P06 | 659 | 3 tasks | 7 files |
| Phase 10-hierarchical-spaces P04 | 173 | 1 tasks | 1 files |
| Phase 10-hierarchical-spaces P03 | 3 | 1 tasks | 2 files |
| Phase 10-hierarchical-spaces P10-05 | 261 | 1 tasks | 1 files |
| Phase 10-hierarchical-spaces P06 | 25 | 2 tasks | 6 files |
| Phase 10 P07 | 5 | 1 tasks | 2 files |
| Phase 11 P03 | 25 | 2 tasks | 3 files |
| Phase 11-entity-driven-exploration P09 | 11 | 2 tasks | 4 files |
| Phase 11.5-ontology-relations P01 | 12min | 3 tasks | 3 files |
| Phase 11.5-ontology-relations P02 | 12min | 1 tasks | 2 files |
| Phase 11.5-ontology-relations P03 | 20min | 2 tasks | 2 files |
| Phase 11.5-ontology-relations P04 | 35min | 2 tasks | 6 files |
| Phase 11.5-ontology-relations P05 | 25min | 1 tasks | 3 files |
| Phase 11.5-ontology-relations P06 | 12min | 1 tasks | 1 files |
| Phase 11.5-ontology-relations P07 | 25min | 3 tasks | 7 files |
| Phase 11.5-ontology-relations P08 | 12min | 2 tasks | 2 files |
| Phase 11.8 P02 | 12min | 2 tasks | 1 files |
| Phase 11.8-ruvector-sweep P03 | 12min | 2 tasks | 1 files |
| Phase 11.8-ruvector-sweep P01 | 2min | 2 tasks | 1 files |
| Phase 11.6-adaptive-ontology P01 | 55min | 2 tasks | 12 files |
| Phase 11.6-adaptive-ontology P02 | 35min | 2 tasks | 3 files |
| Phase 11.6-adaptive-ontology P03 | 25min | 2 tasks | 4 files |
| Phase 11.7-rag-chat P04 | 55min | 5 tasks | 5 files |
| Phase 11.8 P05 | 25min | 2 tasks | 4 files |
| Phase 11.7-rag-chat P03 | 40min | 2 tasks | 6 files |
| Phase 11.7-rag-chat P05 | 50min | 4 tasks | 5 files |
| Phase 11.6-adaptive-ontology P04 | 30min | 2 tasks | 6 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Roadmap]: 4 phases chosen over research's 7 -- quick depth compresses pipeline (Phase 2) and background watching into one phase; IPC wiring collapses into frontend phase
- [Roadmap]: VSTOR requirements moved to Phase 1 (not Phase 2) -- vector storage must be initialized before pipeline code runs
- [Roadmap]: PAGE + UX requirements unified in Phase 4 -- all frontend wiring happens together after intelligence layer is complete
- [01-01]: Manually scaffolded src-tauri/ instead of using interactive tauri init (non-interactive environment)
- [01-01]: Icons generated with RGBA color type (required by Tauri generate_context! macro validation)
- [01-01]: Vite outDir changed from dist/spa to dist/ to match tauri.conf.json frontendDist
- [01-02]: vite.config.ts cleaned to remove express plugin (was importing deleted server/)
- [01-02]: AppError uses thiserror derives + serde tagged JSON (#[serde(tag="kind", content="message")]) for frontend discriminated union pattern
- [01-02]: tokio::sync::Mutex chosen over std::sync::Mutex -- Tauri command handlers are async, state crosses .await points
- [01-02]: CortexEngine intentionally left as empty placeholder -- RuVector fields deferred to Plan 04 as designed
- [Phase 01-03]: spawn_blocking wraps all IPC command bodies to establish async-safe CPU-bound pattern for Phase 2 real implementation
- [Phase 01-03]: 20 IPC commands: 16 from CLAUDE.md + get_watched_folders, get_tags, toggle_favorite, get_activity_feed for frontend-implied operations
- [Phase 01-04]: Path from src-tauri/ to ruvector is ../../experiments/ruvector (not ../../../) -- cortex and experiments are siblings under apps/
- [Phase 01-04]: tauri::Manager trait must be in scope for setup hook to call app.path() and app.manage()
- [Phase 01-04]: CollectionManager creates directories itself; AlreadyExists on collection creation ignored for idempotent restarts
- [Phase 01]: TailwindCSS 4 CSS-first config: theme tokens migrated to @theme {} in global.css, eliminating tailwind.config.ts and postcss.config.js
- [Phase 01]: isTauri() uses window.__TAURI__ for Tauri 2 runtime detection; tauriInvoke() pattern enables zero-config dual-mode operation
- [Phase 01]: Types use ISO string dates for Rust serde compatibility; React Query queryKeys factory enables precise cache invalidation
- [Phase 02-01]: docx-rust 0.1 used (0.2 not on crates.io); Body.text() used instead of manual paragraph traversal
- [Phase 02-01]: AppError::Embedding added in Plan 01 to prevent both plans 01 and 02 modifying error.rs
- [Phase 02-02]: std::sync::Mutex for fastembed model -- embed() is sync, called inside spawn_blocking; avoids async lock in sync context
- [Phase 02-02]: Integration tests (fastembed model download) marked #[ignore] for CI; fast unit tests cover truncation and regex logic
- [Phase 02]: notify_debouncer_mini::notify::RecursiveMode used (not top-level notify crate) to avoid dependency conflict
- [Phase 02]: DebouncedEventKind matched with wildcard _ -- enum is non-exhaustive in notify-debouncer-mini 0.4
- [Phase 03-01]: Used manual k-means instead of ruvector-gnn (training framework, not clustering lib)
- [Phase 03-01]: Entity filter parsing supports "before:DATE", "after:DATE", "from:PERSON" in query text
- [Phase 03-03]: In-memory adjacency list graph instead of ruvector-graph (full Cypher DB -- overkill for v1)
- [Phase 03-04]: Package name is ruvector-sona (not sona); ruvector-attention default-features=false to avoid simd
- [Phase 03-04]: Reranker blends 0.7*cosine + 0.3*attention -- conservative blend for v1
- [Phase 03-05]: ActivityLog capped at 200 items; Domain expansion uses 0.6 similarity threshold for bootstrap
- [Phase 03-05]: Search-as-you-type: backend handles via min query length; frontend adds 150ms debounce in Phase 4
- [Phase 04-01]: serde rename_all=camelCase on all 16 IPC structs; Rust types.rs is source of truth, TS types.ts mirrors exactly
- [Phase 04-01]: ActivityItem uses #[serde(rename="type")] on activity_type field to avoid Rust keyword collision
- [Phase 04-01]: TopQuery as separate struct for structured search analytics (query + count)
- [Phase 04-01]: Space subSpaces/sampleFiles required arrays (not optional) matching Rust Vec
- [Phase 04-04]: Added useRecentDocuments/useFavoriteDocuments hooks inline (missing from Plan 02)
- [Phase 04-04]: Tag cloud font size scales 14px-32px based on document count range
- [Phase 04-04]: Tauri dialog import uses ts-ignore for optional plugin-dialog dependency
- [Phase 04-04]: Pause/Resume buttons shown but disabled (backend support not yet available)
- [Phase 04-05]: SVG circular layout for space network graph (react-force-graph not in deps)
- [Phase 04-05]: Local state with dirty detection for settings form pattern; sonner toast on save
- [Phase 04]: resolveIcon utility maps Lucide icon name strings to components with FileText fallback
- [Phase 04]: 150ms debounce on search via custom useDebouncedValue hook; split-pane layout for Search and Document pages
- [Phase 04-05]: SVG circular layout for space network graph instead of react-force-graph (not in deps)
- [Phase 04-06]: Zustand installed with persist middleware for onboarding localStorage state
- [Phase 04-06]: cmdk library used for command palette (already in package.json)
- [Phase 04-06]: System tray (UX-03) deferred -- TopBar indexing indicator provides equivalent UX
- [Phase 04-06]: Sidebar collapsed state migrated from useState to Zustand store for cross-component sync
- [Phase 05-01]: Settings path derived from registry_path.parent() to avoid adding dirs crate dependency
- [Phase 05-01]: scan-complete status was in commands/folders.rs trigger_scan (not worker.rs as plan stated); fixed in correct location
- [Phase 05-01]: rebuild_path_index placed after engine_arc declaration (blocking_lock safe in sync setup closure)
- [Phase 05-02]: isTauri() import from @/lib/tauri (not @/lib/utils) -- Tauri helpers live in dedicated tauri.ts module
- [Phase 05-02]: Tauri event payload (filePath/status) mapped to Zustand store shape (currentFile/isIndexing) in AppShell listener
- [Phase 05-02]: /onboarding route placed BEFORE AppShell Route group -- React Router matches first non-layout route, so fullscreen pages must precede layout wrapper
- [v1.1 Roadmap]: AI provider foundation (Phase 7) must precede all LLM feature phases -- single ai_request abstraction gates LLME, LLML, HSPC labeling
- [v1.1 Roadmap]: ENEX (Phase 11) depends only on Phase 8 (real entities), not Phase 9/10 -- entity filtering is independent of space labeling quality
- [v1.1 Roadmap]: Credentials stored as plaintext JSON in app_data_dir for v1.1; macOS Keychain deferred to v1.2 (ASPAC-06)
- [v1.1 Roadmap]: bert-base-NER removal is a hard requirement (LLME-06) -- cargo check must succeed without ort/tokenizers after Phase 8
- [08-02]: iban_validate package lib name is `iban` -- use `iban::Iban` not `iban_validate::Iban`
- [08-02]: Date values normalized to YYYY-MM-DD (not RFC-3339) for dedup stability -- dateparser injects current time for date-only strings (Pitfall 3)
- [08-02]: Credit card: Luhn + (BIN prefix OR context word) required -- pure Luhn has ~10% false positive rate (D-03)
- [08-02]: GSTIN test vectors computed from Mod-36 algorithm -- plan's "22AAAAA0000A1Z5" example was incorrect; verified vectors: 27AABCU9603R1ZN, 29GGGGG9999G1ZY
- [11.5-01]: PREDICATE_VOCABULARY has 21 tokens per PLAN.md's literal enumeration
- [11.5-02]: TripleStore insert_one upsert lookup is a linear scan (matches SavedSearchStore precedent) — triple count is corpus-scoped/user-bounded (T-11.5-06 accepted)
- [11.5-02]: Auto-inverse/symmetric partner writes propagate the primary triple's user_added flag so manual triples produce manual partners too
- [11.5-03]: Pass3RelationExtractor mirrors Pass2LlmRefiner's struct shape + lifecycle exactly (semaphore=8, provider-absent → Ok(None)); reuses strip_json_fences from pass2_llm_refiner directly instead of duplicating fence-strip logic
- [11.5-03]: extract() drops triples where either endpoint's ExtractedEntity.canonical_id is None, rather than erroring — tolerates partial Pass 2 entity registration
- [11.5-04]: AppState.triple_store added now (not deferred to Plan 05) because the spawn_entity_backfill signature change broke 3 call sites (lib.rs boot-time backfill, commands/entities.rs, commands/folders.rs::trigger_scan), not just the 1 the plan cited — crate must compile after this plan
- [11.5-04]: collect_backfill_candidates gate moved from TWO_PASS_TARGET_VERSION (3.0) to PASS3_TARGET_VERSION (3.5); 2 pre-existing tests asserting the old v3.0-excluded behavior were corrected to match
- [11.5-05]: Task 1 (AppState wiring) and Task 3 (backfill call site) were already complete from 11.5-04's deviation — verified via file reads before editing, only Task 2 (7 IPC commands in new commands/relations.rs) required implementation
- [11.5-05]: classify_asset_type separates pure logic (asset_type_from_signal, unit-testable) from engine-dependent doc-topic/subclass lookup (classify_asset_type wrapper) per the plan's testability guidance
- [11.5-06]: All 7 hooks implemented verbatim per plan body — types (Plan 01) and IPC command signatures (Plan 05) were already finalized, no architectural deviation needed
- [Phase 11.5-07]: EntityRelationsPanel returns null on error/zero-relations to keep outer page loading/error state authoritative
- [11.5-08]: Owned by me link placed below Saved Searches, above Bottom nav — personal-state cluster stays together (D-19 planner discretion)
- [11.8-02]: ruvector-attention audited NO-GO for search re-rank — compute() (all 46 mechanisms) returns a blended value vector [dim], never per-candidate relevance scores; no rerank()/score() API exists in the crate. Plan 07 skipped; existing 0.9*cosine + 0.1*recency formula stands.
- [11.8-03]: ruvector-solver audited NO-GO for entity PageRank — crate only implements seed-relative Personalized PageRank (Forward Push/Backward Push/Hybrid Random Walk); QueryType exposes only PageRankSingle/PageRankPairwise, no global whole-graph variant. Plan 08 skipped; Sidebar/Ontology panel keep doc_count DESC.
- [Phase 11.8-01]: ruvllm audited GO — LlmBackend::generate/generate_stream_v2 map directly onto ai_request(); target features inference-metal+gguf-mmap+parallel+accelerate+async-runtime; default model Qwen2.5-7B-Instruct Q4_K_M (fallback Llama 3.2 3B Instruct)
- [11.7-02]: ChatSessionStore mirrors SavedSearchStore exactly; create_session/append_message/rename_session take caller-supplied id+now_iso for testability
- [11.7-02]: Plan 01's Chat type contracts (ChatSession/ChatMessage/ChatRole/Citation) were added to types.rs as a Rule 3 prerequisite fix since Plan 01 hadn't executed yet when Plan 02 started; `pub mod chat;` added to lib.rs despite plan's no-lib.rs-edit instruction, required for the plan's own cargo test verify command to compile
- [Phase 11.6-adaptive-ontology]: [11.6-01]: SEED_PREDICATES is a compile-time alias of PREDICATE_VOCABULARY (never mutates it, D-03); adding canonical_short_name to ExtractedEntity+CanonicalEntity required fixing 26 downstream struct-literal construction sites (Rule 3)
- [Phase 11.6-02]: Added AppError::Invalid(String) variant (Rule 3) — required by rename_predicate/register_manual_predicate; OntologyStore mirrors SavedSearchStore/TripleStore JSON-sidecar pattern, apply_consolidation returns TripleRewriteInstruction rather than mutating TripleStore directly
- [Phase 11.6-03]: entity_normalizer splits Location values on hyphen only (not spaces) so 'Riverside Complex P705' stays full per CONTEXT.md; corporate-suffix stripping picks the single longest matching suffix (not chained repeated stripping); used std::sync::OnceLock (not once_cell) since crate rust-version=1.77.2 predates LazyLock
- [Phase 11.6-04]: OntologyBootstrapper mirrors Pass2LlmRefiner/Pass3RelationExtractor LLM plumbing exactly (temperature=0.2, not Pass3's 0.0 — D-02 modest creativity); AppState.ontology_store + auth_state fields added now (not deferred to Plan 06) because spawn_entity_backfill's new ontology_store+auth params broke 3 existing call sites (lib.rs, commands/folders.rs, commands/entities.rs) — crate must compile for this plan's own cargo test verify command, and Plan 06 needed this wiring anyway for its Settings > Ontology IPC commands
- [Phase ?]: Chose async-stream (stream! macro) over tokio-stream+mpsc for streaming SSE/NDJSON parsing (11.7-04)
- [Phase ?]: No 401 retry for streaming (SSE cannot be transparently replayed); documented in fn doc comments (11.7-04)
- [Phase ?]: [11.8-05]: Wired ruvector-hyperbolic-hnsw into search_documents_impl -- parent-scoped search routes through hyp_search (validation-only) when hyp_index populated, always narrows via space_descendant_candidates (exact membership); SC5 perf gate re-verified passing (5480ms flat vs 0ms hyperbolic)
- [Phase ?]: [11.7-03]: FilePreview.highlightRange threaded from DocumentPage's ?highlight=start-end query param; TextPreview splits+marks the exact range with span slicing, MarkdownPreview marks the whole rendered block since raw markdown source can't be split without corrupting syntax; scrollIntoView calls guarded against missing implementation in edge runtimes (jsdom)
- [11.7-05]: ChatEngine::answer wraps the entire retrieval segment (embed query, HNSW top-8, per-doc chunk+embed+score, rerank top-12) in a single tokio::task::spawn_blocking so std::sync::Mutex guards (engine, entity_store) never cross an .await; citation doc_title resolved from metadata["title"] (fallback "Unknown", matching build_document_from_metadata) rather than the raw doc_id
- [11.7-05]: start_chat persists the user ChatMessage and generates assistant_message_id BEFORE tokio::spawn'ing ChatEngine::answer (not awaited) so the frontend can attach its chat-stream-* event listener using known ids before the first token arrives; query length capped at 4000 chars (T-11.7-12)

### Pending Todos

None yet for v1.1.

### Blockers/Concerns

- Phase 7 source: port ai/ + auth/ modules from /Users/gshah/work/apps/learnforge/src-tauri/src/ -- verify module API surface before planning
- Phase 8: backfill ETA calculation requires knowing per-document LLM latency; cloud vs Ollama will have very different throughput
- 11.6-03: shared working-directory commit df90c0d unintentionally included 2 sibling-agent files (ai/openai.rs, ai/stream.rs, Plan 11.7-04) staged concurrently by another agent — verified non-destructive (cargo test --lib ai::stream 8/8 pass), documented in 11.6-adaptive-ontology/deferred-items.md, no functional impact on 11.6-03's own deliverables

## Session Continuity

Last session: 2026-07-10T04:22:00Z
Stopped at: Merged 11.6-04 + 11.7-03/05 to main; ChatPage UI next
Resume file: None
Resume action: Build ChatPage client + wire useChatStream hook; then merge 11.6-06/05 + 11.8-06.

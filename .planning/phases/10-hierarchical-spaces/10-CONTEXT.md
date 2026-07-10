# Phase 10: Hierarchical Spaces - Context

**Gathered:** 2026-07-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 10 delivers automatic sub-space discovery for large Smart Spaces (>50 docs), matching the mockSpaces hierarchy shape (Property → Tax → Insurance). Users can drill into sub-categories from both the sidebar (inline expand w/o navigation) and `/spaces/:id` (breadcrumb navigation). Adds `ruvector-hyperbolic-hnsw` as a secondary hierarchy-aware index when the crate proves suitable (planner audits first — following the Phase 9 lesson on ruvector-cluster/domain-expansion misfits).

### What Phase 10 delivers

1. **Sub-space detection** — when a top-level Space contains > 50 documents, `spaces/subspace_detector.rs` runs recursive k-means on intra-cluster vectors (k = sqrt(n/2)); sub-clusters ≥ 3 docs become sub-spaces; smaller clusters roll up to a synthetic "Misc" sub-space (per HSPC-03).
2. **Sub-space LLM labeling** — reuses Phase 9 `LlmSpaceLabeler` with parent-context in prompt ("Sub-space of {parent_label}. Cluster docs: [...]. Return 2-4 word sub-label distinct from parent."). Persists to same `space_labels.json` cache.
3. **Space data model extension** — `Space.parent_id: Option<String>` + `Space.depth: u8` fields. Flat storage; depth pointer walks tree. Max depth = 2 (Parent → Sub).
4. **`ruvector-hyperbolic-hnsw` secondary index** — hierarchy-aware search when query context includes a parent Space (searching within `/spaces/:id`). Silent fallback to flat HNSW on init/lookup error. **Planner MUST audit local crate first** (`/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-hyperbolic-hnsw/`) to confirm it delivers hierarchy-aware document search (not distributed DB primitives like ruvector-cluster/domain-expansion turned out to be).
5. **Sidebar inline expand** — click chevron toggles sub-space list beneath top-level Space (no page nav). Format `"Property (3)"` next to name.
6. **Breadcrumb on `/spaces/:id`** — `Spaces / Property / Tax` — parent link back to `/spaces/parent_id`, current name plain text.
7. **SpaceDetailPage sub-space grid** — reuses SpaceDetailPage w/ parent context banner + single SpaceCard grid for sub-spaces (identical to top-level layout).

### Out of scope

- Depth ≥ 3 (Property → Tax → Assessment Year). Deferred until users request.
- User-editable sub-space taxonomy (deferred — LLM labels only, user can rename via existing Phase 9 rename hook).
- Search UX overhauls (Phase 11 owns entity-driven exploration; Phase 12+ owns clustering swap).
- Sub-space thumbnails or custom icons (v1.2).
- Move-doc-between-sub-spaces manual UI (deferred — automatic clustering only).

</domain>

<decisions>
## Implementation Decisions

### Sub-Space Detection & Creation (Area 1)

- **D-01: Threshold = 50 documents** per parent Space (HSPC-01). Configurable in code as `const SUB_SPACE_THRESHOLD: usize = 50;` for easy future tuning.
- **D-02: Sub-clustering algorithm = recursive k-means** on intra-cluster vectors. `k = sqrt(n / 2).max(2)`. Rationale: HDBSCAN needs enough density which sub-clusters typically lack (parent already trimmed). Recursive k-means is deterministic + works on small vector sets.
- **D-03: Max hierarchy depth = 2 (Parent → Sub).** Prevents runaway nesting. Field `Space.depth: u8` gates recursion.
- **D-04: Min docs per sub-space = 3.** Sub-clusters below threshold roll up to a synthetic `"Misc"` sub-space per parent (HSPC-03). No document is silently dropped.

### Sub-Space Labeling & Persistence (Area 2)

- **D-05: Reuse Phase 9 `LlmSpaceLabeler`** with an extended prompt variant. Sub-space labeling prompt = base label prompt + parent-context prefix: `"You are labeling a sub-space of the '{parent_label}' Space. Return a 2-4 word label distinct from '{parent_label}'. Cluster documents: [...]"`. Reuses existing collision retry + domain-expansion bootstrap paths.
- **D-06: Sub-spaces cached in same `space_labels.json`.** Keyed by `space_id` (unique across parent/sub). Cache entries carry `parent_id: Option<String>` and `depth: u8` fields (new). Same fingerprint / 20% membership shift logic applies (LLML-03).
- **D-07: Space data model.** Extend `types.rs::Space`:
  - `parent_id: Option<String>` — None for top-level, Some(uuid) for sub-spaces
  - `depth: u8` — 0 for top-level, 1 for sub-space
  - `sub_space_ids: Vec<String>` — computed at build time for top-level spaces; empty for sub-spaces
- **D-08: Recluster invalidates sub-spaces.** When a parent Space's membership shifts > 20% (Phase 9 fingerprint gate), drop ALL its sub-spaces from cache + recompute. Simpler than incremental; sub-space labels are cheap when domain-expansion bootstraps them.

### ruvector-hyperbolic-hnsw Adoption (Area 3)

- **D-09: Planner audits `ruvector-hyperbolic-hnsw` FIRST** — read `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-hyperbolic-hnsw/` README + lib.rs. Confirm the crate delivers hierarchy-aware document search (not distributed DB primitives). Following Phase 9 lesson: two ruvector crates (`ruvector-cluster`, `ruvector-domain-expansion`) were misnamed — cluster is a gossip protocol, domain-expansion is Thompson Sampling. If hyperbolic-hnsw is misfit, document as deviation + ship flat HNSW.
- **D-10: If usable — dual-index pattern.** Add hyperbolic-hnsw as a SECONDARY index over parent Space centroid tree; keep flat HNSW for top-level search. Selects hyperbolic path only when query context includes a parent Space (`SearchFilters.parent_space_id.is_some()`).
- **D-11: Silent fallback.** On hyperbolic init or lookup error → log warning + fall back to flat HNSW filtered by parent Space membership. Never leave user with empty results.
- **D-12: Perf gate (SC5).** Integration test measures parent→child→grandchild query time vs flat top-level query time; asserts ≤ 2× flat baseline on a 10K-doc corpus.

### UI — Sidebar Expand + Breadcrumb + Detail (Area 4)

- **D-13: Sidebar interaction — inline expand.** Click chevron on top-level Space entry → expands sub-space list beneath without page navigation. Chevron rotates (0° → 90°). Persists expanded state in Zustand `useSidebarStore`.
- **D-14: Sub-count format = `"Property (3)"`.** Text child count next to name, `text-xs text-text-tertiary` styling.
- **D-15: Breadcrumb on `/spaces/:id`.** Format `Spaces / Property / Tax`. `Spaces` links to `/spaces`; parent name links to `/spaces/{parent_id}`; current name plain text. Uses shadcn Breadcrumb primitive.
- **D-16: Sub-space detail page = SpaceDetailPage reused.** Adds parent context banner ("Sub-space of Property → return to parent") above breadcrumb. Sub-space grid renders as top-level SpaceCard grid (identical component). Editing / rename / regenerate all flow through existing Phase 9 hooks with `depth = 1` awareness.
- **D-17: Sidebar top-5 selection.** Top 5 top-level Spaces by document count (per HSPC-04). Sub-spaces of expanded parents render beneath their parent within the top-5 slot.

### Claude's Discretion (Planner-owned)

- Exact IPC command signatures: `get_sub_spaces(parent_id)`, `expand_space(space_id)`, `get_hierarchy()` (planner finalizes).
- Rust module layout: `spaces/subspace_detector.rs` + extended `spaces/manager.rs` recluster.
- ruvector-hyperbolic-hnsw exact API surface — planner reads local crate first.
- Whether to expose sub-spaces on `useSpaces()` hook (top-level filter) or add separate `useSubSpaces(parent_id)` hook. Recommend: extend `Space` type; single hook returns full flat list, frontend filters by `parent_id`.
- Sidebar animation — Framer Motion vs CSS transitions for chevron rotate + list expand. shadcn Collapsible primitive is available.
- SpaceCard `sub_count` display — inline text vs badge. UI-SPEC will finalize.
- Empty sub-space handling — parent w/o sub-spaces (< 50 docs) shows no sub-space section on SpaceDetailPage. UI-SPEC finalizes copy.
- Perf test corpus source — planner picks (may seed with mock 10K docs or use sample of user's ~/private).

</decisions>

<canonical_refs>
## Canonical References

### Project specs
- `.planning/ROADMAP.md` §"Phase 10: Hierarchical Spaces" — goal + HSPC-01..04 + SC1-5
- `.planning/REQUIREMENTS.md` §"Hierarchical Spaces" — HSPC-01..04 full text
- `.planning/phases/09-llm-space-labeling/09-CONTEXT.md` — Phase 9 LlmSpaceLabeler + SpaceLabelCache surface (Phase 10 reuses)
- `.planning/phases/08-llm-entity-extraction/08-CONTEXT.md` — Phase 8 entity classes drive canonical_entity_hint on sub-spaces

### Existing Cortex code
- `src-tauri/src/spaces/clustering.rs` — k-means (recursion target for sub-clustering)
- `src-tauri/src/spaces/manager.rs` — Phase 9 async recluster (extend for sub-space pass)
- `src-tauri/src/spaces/llm_labeler.rs` — reuse for sub-space labeling
- `src-tauri/src/spaces/label_cache.rs` — extend SpaceLabelEntry with parent_id + depth
- `src-tauri/src/types.rs::Space` — extend
- `src-tauri/src/commands/spaces.rs` — extend for sub-space IPCs
- `client/pages/SpaceDetailPage.tsx` — extend for sub-space grid + breadcrumb + parent banner
- `client/components/Sidebar.tsx` (or wherever spaces list lives) — extend for chevron expand
- `client/components/spaces/SpaceCard.tsx` — reuse for sub-space rendering

### RuVector crates (local)
- `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-hyperbolic-hnsw/` — planner AUDITS FIRST

### Patterns to mirror
- Phase 9 `SpaceLabelCache` sidecar pattern
- Phase 9 recluster orchestration (async, tokio Mutex for space_manager)
- Phase 4 React Query hook factory
- Phase 8/9 UI-SPEC design system (shadcn, 4px grid, indigo accent)
- shadcn `Breadcrumb` + `Collapsible` primitives

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `spaces/llm_labeler.rs::label_batch()` — reuse for sub-space batch labeling
- `spaces/label_cache.rs` — extend, don't fork
- `spaces/clustering.rs::cluster_documents()` — recursive call from subspace_detector
- SpaceCard component from Phase 9
- SpaceLabelingIndicator from Phase 9 (badge shows during sub-space labeling too)
- Phase 9 useSpaceLabels / useRelabelSpace hooks
- shadcn Breadcrumb + Collapsible + Chevron primitives
- `sonner` toast

### Established Patterns
- IPC: `#[tauri::command] async + serde camelCase`
- App state: `Arc<tokio::sync::Mutex<T>>` for cross-await mutation
- Persistence: JSON sidecars in `app_data_dir/`
- Tauri event-driven progress
- Zustand for UI state (sidebar expanded map)
- React Query for server data

### Integration Points
- `src-tauri/src/lib.rs` — no new state (reuse space_label_cache from Phase 9)
- `commands/spaces.rs` — add sub-space commands
- `spaces/mod.rs` — add `subspace_detector` module
- `client/components/Sidebar.tsx` — add chevron expand
- `client/pages/SpaceDetailPage.tsx` — add sub-space grid + breadcrumb
- `client/hooks/useTauri.ts` — extend Space typing or add hook

</code_context>

<specifics>
## Specific Ideas

- **mockSpaces hierarchy shape is the quality bar** — Property → Tax → Insurance. Sub-space labels must be corpus-derived, not template-driven.
- **~/private sample folders for planner spike** — `~/private/docs/` has property multi-level structure (AlphaComplex, GV7, SF4, Downtown) which naturally clusters into sub-spaces.
- **Universal seed applies to sub-space taxonomy too** — no hardcoded sub-space names. LLM emerges labels from cluster content.
- **Domain-expansion bootstrap works for sub-spaces** — cheap labels when a new sub-space matches an existing labeled sub-space via cosine similarity (Phase 9's `try_bootstrap_from_nearest` reused with sub-space-only nearest-neighbor search).
- **ruvector-hyperbolic-hnsw audit is a real gate.** Phase 9 taught us — ruvector crate names don't always match ML expectations. Planner reads lib.rs before adopting.
- **"Misc" sub-space is per parent** — each parent that has small sub-clusters gets its own Misc. No global Misc pool.

</specifics>

<deferred>
## Deferred Ideas

### Phase 10 follow-ups (v1.2 candidates)
- **Depth ≥ 3** — Property → Tax → Assessment Year. Deferred until users request.
- **Manual doc-move between sub-spaces** — automatic clustering only in v1.1.
- **Sub-space thumbnails / custom icons** — v1.2 polish.
- **Sidebar drag-to-reorder Spaces** — v1.2.
- **Sub-space search perf tuning** — hyperbolic-hnsw parameter sweep for optimal depth-vs-time.

### Downstream phase dependencies
- **Phase 11 (Entity-Driven Exploration)** consumes sub-space breadcrumb pattern for entity-detail navigation.
- **Phase 12 (GNN Clustering Swap)** replaces k-means (top-level + recursive sub-clustering). GNN clusters may naturally form hierarchy without explicit recursive pass.
- **Phase 13 (Cypher Entity Graph)** — sub-space membership becomes graph node label alongside `canonical_entity_hint`.

### v2 / future
- **User-editable sub-space taxonomy** — rename / merge / split at sub-space level in Settings.
- **Cross-parent sub-space merge** — if two parent Spaces have similar sub-spaces (e.g., "Insurance" under both Property and Vehicle), suggest merging.
- **Sub-space templates** — user-defined "always split Property into Tax / Insurance / Utilities" rules.

</deferred>

---

*Phase: 10-hierarchical-spaces*
*Context gathered: 2026-07-08*

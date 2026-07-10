# Phase 11: Entity-Driven Exploration - Context

**Gathered:** 2026-07-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 11 unlocks navigation of the corpus through entities:

1. **Clickable entity chips filter the view** — any chip on a document, search result, or space detail page navigates to `/search?entity={class}:{value}` and filters results. Multi-entity AND semantics via repeated `?entity=` params.
2. **Saved search Spaces** — user saves the current search query as a named virtual Space. Persists to `app_data_dir/saved_searches.json`. Appears in the Sidebar under a "Saved Searches" section (separate from Smart Spaces). Doc count re-evaluates on every Sidebar render (ENEX-04).
3. **Related documents panel on `/document/:id`** — top-5 related docs ranked by `0.6 × cosine + 0.4 × entity_overlap_jaccard`.
4. **Entity detail page** at `/entity/:class/:value` — shows entity header + aliases + all docs mentioning the entity + top co-occurring entities.

Reuses Phase 8 `ExtractedEntity` (class + subclass + value + confidence) and Phase 9 `Space.canonical_entity_hint` (drives entity navigation from Space cards).

### Out of scope

- Full Cypher graph query engine (Phase 13 owns via `ruvector-graph`)
- Real-time entity co-occurrence dashboard (v1.2 knowledge-graph viz)
- Entity type auto-linking (e.g., "Alex" == "Alex Doe" fuzzy match) — Phase 11 uses exact `{class}:{value}` match; NEL deferred
- Entity relationship editor (user-created relations) — v2
- Sharing saved searches across devices — v2

</domain>

<decisions>
## Implementation Decisions

### Entity Chip Filter Behavior (Area 1)

- **D-01: Filter scope = URL params on `/search`.** Format: `?entity=Person:Alex%20Shah`. Repeated params for multi-entity: `?entity=A&entity=B`. Routing-driven → shareable + back-button works.
- **D-02: Chip click navigates cross-page to `/search?entity=X`.** Even from `/document/:id` or `/spaces/:id` — one unified filter surface. Applies to all EntityChip components.
- **D-03: Chip payload = `{class}:{value}`.** Precise; disambiguates same-value entities across classes (e.g., `Person:AlphaComplex` vs `Location:AlphaComplex`).
- **D-04: Multi-entity = additive AND.** Multiple `?entity=` params AND together. UI shows active filters as removable pill chips atop `/search` results. Removing a pill drops its URL param.

### Saved Search Spaces (Area 2)

- **D-05: Storage = `app_data_dir/saved_searches.json` sidecar.** Mirrors Phase 5/8/9 JSON persistence pattern. Load on startup into AppState; save on any mutation.
- **D-06: Data shape:**
  ```json
  {
    "saved_searches": [
      { "id": "ss-uuid", "name": "Property Tax 2024", "query": "property tax 2024",
        "filters": { "entities": ["Location:AlphaComplex"], "topic": "property" },
        "created_at": "2026-07-08T10:00Z", "doc_count_cache": 12 }
    ]
  }
  ```
  `doc_count_cache` is a hint for immediate render; actual count re-evaluates on Sidebar mount (ENEX-04).
- **D-07: Sidebar section = separate "Saved Searches" header** below "Smart Spaces". Each saved search row: bookmark icon + name + `({count})`. Click navigates to `/search` with saved filters applied.
- **D-08: Refresh strategy (ENEX-04).** Sidebar `useSavedSearches()` hook fires a lightweight count query per saved search on mount (batched). React Query cache 30s TTL for count. Newly indexed docs → next Sidebar mount shows updated count.
- **D-09: Save UX.** `/search` header shows "Save this search" button; opens sonner-integrated modal with name input + save action.

### Related Documents Panel (Area 3)

- **D-10: Ranking = `0.6 × cosine + 0.4 × entity_overlap_jaccard`.** Cosine dominates (semantic core); entity overlap distinguishes near-tie neighbors.
- **D-11: Entity overlap = shared `{class}:{value}` set / union.** Precise; ignores topic + tags (those signal broader themes, not doc-doc relatedness).
- **D-12: Top 5 results** per ENEX-03. Fewer if fewer valid neighbors (min score threshold 0.3).
- **D-13: Compute on demand at doc-detail render.** React Query cache 5min TTL keyed by `doc_id`. No persisted related-doc index — RuVector HNSW query fast enough (< 20ms on 10k corpus).
- **D-14: UI = Related section below entity chips on DocumentPage sidebar.** Each row: doc title + relevance score badge + snippet.

### Entity Detail Page + Canonical Nav (Area 4)

- **D-15: Route = `/entity/:class/:value`.** URL-encoded value. E.g. `/entity/Person/Alex%20Shah`.
- **D-16: Page structure:**
  - Header: class icon + value + alias count + total doc count
  - Aliases section: canonical + alt forms (from Phase 6 entity_store alias index)
  - Documents section: all docs mentioning this entity (paginated 20/page)
  - Co-occurring entities: top 10 entities that appear in the same docs
- **D-17: Nav triggers.** Every EntityChip has two actions:
  - Click → nav to `/search?entity={class}:{value}` (filter search)
  - Right-click / long-press → nav to `/entity/{class}/{value}` (dedicated entity page)
  Also from Space detail `canonical_entity_hint` → link to entity page.
- **D-18: Empty state.** `/entity/{class}/{value}` when zero docs → `"No documents mention {class}:{value}. Try syncing your folders or connecting more provider data."` w/ links to Watched / Settings.

### Claude's Discretion (Planner-owned)

- Exact IPC names: `get_saved_searches`, `save_search`, `delete_saved_search`, `get_entity_page_data(class, value)`, `get_related_docs(doc_id, top_n)` (planner finalizes).
- Rust module layout: `saved_searches/store.rs` + `saved_searches/commands.rs` or extend existing `commands/`. Planner picks.
- React Query key strategy for saved-search counts (per-search key vs batched map).
- Modal component library for "Save this search" (shadcn Dialog vs sonner action).
- Entity-page pagination UX (numbered pager vs "Load more").
- Score display format on Related panel (percentage vs fraction vs stars).
- Multi-entity chip filter clear-all action.

</decisions>

<canonical_refs>
## Canonical References

### Project specs
- `.planning/ROADMAP.md` §"Phase 11: Entity-Driven Exploration" — ENEX-01..04, SC1-4
- `.planning/REQUIREMENTS.md` §"Entity-Driven Exploration" — ENEX-01..04 full text
- `.planning/phases/08-llm-entity-extraction/08-CONTEXT.md` — ExtractedEntity class/subclass/value/confidence (drives filter payload)
- `.planning/phases/09-llm-space-labeling/09-CONTEXT.md` — Space.canonical_entity_hint (entity-nav from spaces)
- `.planning/phases/06-knowledge-graph-and-native-integrations/06-CONTEXT.md` — entity_store alias index (aliases on entity page)

### Existing Cortex code
- `src-tauri/src/graph/entity_store.rs` — CanonicalEntity + alias index (reuse for entity page)
- `src-tauri/src/commands/entities.rs` — Phase 6 entity IPC (extend for entity-page + related-docs)
- `src-tauri/src/search/query.rs` — search filter pipeline (extend for entity URL params)
- `src-tauri/src/commands/spaces.rs` — Phase 9 space IPCs (reuse pattern for saved-search IPCs)
- `client/pages/SearchPage.tsx` — extend for URL-param entity filter + save button
- `client/pages/DocumentPage.tsx` — extend Related panel
- `client/pages/EntityDetailPage.tsx` — new route
- `client/components/entities/EntityChip.tsx` — add click + right-click nav
- `client/components/layout/Sidebar.tsx` — add Saved Searches section

### Patterns to mirror
- Phase 9 SpaceLabelCache JSON sidecar → saved_searches.json
- Phase 4 React Query hook factory
- Phase 8 IPC convention (`#[tauri::command] async + serde camelCase`)
- Phase 9 recluster async orchestration (if saved-search count query touches space_manager)
- shadcn Dialog primitive for Save modal

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `EntityChip.tsx` from Phase 8 — extend onClick
- `CanonicalEntity` + alias index from Phase 6
- `search::query::execute_query` — extend filter pipeline
- `HNSW.search()` for cosine neighbors on Related panel
- Phase 9 SpaceLabelCache JSON pattern
- Phase 4 React Query queryKeys factory
- shadcn Dialog + Command primitives
- sonner toast
- Existing Sidebar section pattern from Phase 9 (Smart Spaces list)

### Established Patterns
- IPC: `#[tauri::command] async + serde camelCase`
- App state: `Arc<tokio::sync::Mutex<T>>`
- Persistence: JSON sidecars in `app_data_dir/`
- React Query for server data
- Zustand for UI state (potentially `useSavedSearchesStore` for optimistic updates)

### Integration Points
- `src-tauri/src/lib.rs` — register saved_searches state + IPC commands
- `commands/mod.rs` — add saved_searches module (or extend spaces)
- `search/query.rs` — accept `Vec<EntityFilter>` in SearchFilters
- `client/hooks/useTauri.ts` — add saved-search hooks + related-docs hook + entity-page hook
- `client/components/layout/Sidebar.tsx` — Saved Searches section
- `client/pages/SearchPage.tsx` — URL param parsing + entity filter pills + Save modal
- `client/pages/DocumentPage.tsx` — Related section
- `client/pages/EntityDetailPage.tsx` — new file
- Router: add `/entity/:class/:value` route

</code_context>

<specifics>
## Specific Ideas

- **URL params over global state** — shareable filters, back-button works, no state sync bugs.
- **`{class}:{value}` payload disambiguates** — `Person:AlphaComplex` vs `Location:AlphaComplex` won't collide.
- **Saved-search count re-eval on Sidebar mount** = correct ENEX-04 semantics (auto-refresh on doc index).
- **Related = 0.6 cosine + 0.4 entity Jaccard** — balances semantic core with entity precision.
- **Entity page reuses Phase 6 alias index** for canonical/alias display.
- **Right-click on chip → entity page; left-click → filter search** — two nav modes on single chip.
- **Canonical entity hint (Phase 9) links to entity page** — Space cards become entity nav springboards.

</specifics>

<deferred>
## Deferred Ideas

### Phase 11 follow-ups (v1.2)
- **Full Cypher query engine** — Phase 13 delivers via ruvector-graph.
- **NEL fuzzy matching** — resolve "Alex" ≈ "Alex Doe" as same entity. Currently exact match only.
- **Entity relationship editor** (user-created "X is spouse of Y" relations) — v2.
- **Share saved searches across devices** — cloud sync, v2.
- **Entity page graph viz** — force-directed network of co-occurring entities. v2 (ASPAC-04).
- **Related-doc pre-index** — persisted top-N related per doc, updated on index. Currently on-demand.

### Downstream phase dependencies
- **Phase 13 (Cypher Entity Graph)** — extends `/entity/:class/:value` page with Cypher-queried relations. Phase 11's dedicated page is the surface Phase 13 enriches.
- **Phase 14 (SONA Feedback Loop)** — entity chip clicks + saved search views become SONA training signals for personalized ranking.

### v2 / future
- **Journey view** — cross-doc timeline attached to entity node (Property → tax docs → receipts → renovation invoices → insurance → sale deed). Requires ruvector-graph traversal.
- **Multi-user annotations** on entity page.
- **Entity-driven notifications** ("new doc mentioning Aadhaar detected").

</deferred>

---

*Phase: 11-entity-driven-exploration*
*Context gathered: 2026-07-08*

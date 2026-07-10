# Phase 11: Entity-Driven Exploration - Research

**Researched:** 2026-07-09
**Domain:** Entity navigation, saved-search persistence, hybrid re-ranking, React Query key strategy
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Entity Chip Filter Behavior (Area 1)**
- D-01: Filter scope = URL params on `/search`. Format: `?entity=Person:Alex%20Shah`. Repeated params for multi-entity: `?entity=A&entity=B`.
- D-02: Chip click navigates cross-page to `/search?entity=X` from any page.
- D-03: Chip payload = `{class}:{value}`. Disambiguates same-value entities across classes.
- D-04: Multi-entity = additive AND. Multiple `?entity=` params AND together. Active filters shown as removable pills.

**Saved Search Spaces (Area 2)**
- D-05: Storage = `app_data_dir/saved_searches.json` sidecar.
- D-06: Data shape: `{ "saved_searches": [{ "id": "ss-uuid", "name": "...", "query": "...", "filters": { "entities": ["Location:AlphaComplex"], "topic": "property" }, "created_at": "...", "doc_count_cache": 12 }] }`.
- D-07: Sidebar section = separate "Saved Searches" header below "Smart Spaces". Each row: bookmark icon + name + `({count})`.
- D-08: Refresh strategy (ENEX-04): `useSavedSearches()` hook fires a count query per saved search on Sidebar mount (batched). React Query cache 30s TTL.
- D-09: Save UX: `/search` header shows "Save this search" button; opens sonner-integrated modal with name input.

**Related Documents Panel (Area 3)**
- D-10: Ranking = `0.6 × cosine + 0.4 × entity_overlap_jaccard`.
- D-11: Entity overlap = shared `{class}:{value}` set / union.
- D-12: Top 5 results per ENEX-03. Min score threshold 0.3.
- D-13: Compute on demand. React Query cache 5min TTL keyed by `doc_id`. No persisted related-doc index.
- D-14: UI = Related section below entity chips on DocumentPage sidebar.

**Entity Detail Page + Canonical Nav (Area 4)**
- D-15: Route = `/entity/:class/:value`. URL-encoded value.
- D-16: Page structure: Header → Aliases → Documents (paginated 20/page) → Co-occurring entities (top 10).
- D-17: EntityChip has two actions: click → `/search?entity={class}:{value}`; right-click/long-press → `/entity/{class}/{value}`.
- D-18: Empty state when zero docs.

### Claude's Discretion
- Exact IPC names (planner finalizes)
- Rust module layout for saved_searches (store.rs + commands.rs or extend existing commands/)
- React Query key strategy for saved-search counts (per-search key vs batched map)
- Modal component library for "Save this search" (shadcn Dialog vs sonner action)
- Entity-page pagination UX (numbered pager vs "Load more")
- Score display format on Related panel (percentage vs fraction vs stars)
- Multi-entity chip filter clear-all action

### Deferred Ideas (OUT OF SCOPE)
- Full Cypher query engine (Phase 13)
- Real-time entity co-occurrence dashboard (v1.2)
- Entity type auto-linking / NEL fuzzy match (Phase 11 uses exact `{class}:{value}`)
- Entity relationship editor (v2)
- Sharing saved searches across devices (v2)
- Entity page graph viz (v2 / ASPAC-04)
- Related-doc pre-index (currently on-demand)
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ENEX-01 | Clicking any entity chip filters the current view to documents mentioning that entity — no page reload | URL param design (D-01..D-04), SearchFilters extension with `entity_filters: Vec<EntityFilter>` field, `apply_entity_class_filter` in filters.rs |
| ENEX-02 | User can save current search query as virtual Space; appears in Sidebar immediately | SavedSearch sidecar schema (D-05..D-09), `save_search` / `get_saved_searches` IPCs, `useSavedSearches` hook + Sidebar section |
| ENEX-03 | /document/:id shows "Related" panel with top-5 by entity overlap + cosine similarity | New `get_related_docs_scored(doc_id, top_n)` IPC replacing graph-edge-based related; Jaccard + cosine hybrid ranking |
| ENEX-04 | Saved search Spaces refresh doc count on Sidebar render without manual trigger | `get_saved_search_counts(ids: Vec<String>)` batched IPC; React Query 30s TTL; Sidebar `useSavedSearches()` on-mount strategy |
</phase_requirements>

---

## Summary

Phase 11 adds four tightly coupled but architecturally distinct capabilities on top of the existing entity extraction (Phase 8) and space labeling (Phase 9/10) stack. The research findings cluster around three Rust concerns and two frontend concerns.

**Rust side.** (1) SearchFilters in `types.rs` gains a new `entity_filters` field (`Vec<EntityClassFilter>`) so the existing `apply_metadata_filters` pipeline can pre-narrow candidates by `{class}:{value}` before the HNSW search — no separate filter path needed. (2) A new `saved_searches` module (mirroring `spaces/label_cache.rs`) owns sidecar JSON persistence with `Arc<tokio::sync::Mutex<SavedSearchStore>>` in `AppState`. (3) A new `get_related_docs_scored` IPC replaces the Phase 3 graph-edge-only `get_related_documents` for this page: it queries HNSW for cosine neighbors, then re-ranks using Jaccard over the `{class}:{value}` entity sets derived from `EntityStore.doc_index`.

**Frontend side.** (1) `EntityChip` gains left-click-navigate-to-search and right-click/long-press-navigate-to-entity-page, replacing the current Phase 6 `/entities/:canonicalId` single-click behavior. (2) Saved-search count queries should use a single batched IPC (`get_saved_search_counts`) rather than N parallel per-search queries, with the React Query key structured as `["saved-searches", "counts", sortedIds]` so React Query can cache the batch result for 30 s while still invalidating on any mutation.

The key architectural risk is write ordering in `saved_searches.json` under concurrent Sidebar renders (batched count query) plus mutations (save/delete). The mitigation pattern is identical to `SpaceLabelCache`: a single `Arc<Mutex<SavedSearchStore>>` in `AppState`, with all reads and writes going through `tokio::task::spawn_blocking`.

**Primary recommendation:** Mirror `spaces/label_cache.rs` exactly for saved_searches persistence; extend `SearchFilters` with a new optional `entity_filters` field; implement a new `get_related_docs_scored` IPC using HNSW cosine + EntityStore Jaccard; add `/entity/:class/:value` route driven by existing `CanonicalEntity` + `doc_index` data.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Entity chip click (filter nav) | Browser / Client | — | URL param write + React Router navigate; no backend needed |
| Entity chip right-click (entity page nav) | Browser / Client | — | URL navigate only; onContextMenu handler |
| SearchFilters entity pre-filter | API / Backend (Rust) | — | Intersection of entity doc_index sets before HNSW search |
| Saved search persistence | API / Backend (Rust) | — | JSON sidecar in `app_data_dir`; must survive restart |
| Saved search doc counts | API / Backend (Rust) | Browser/Client (cache) | Backend executes query; React Query caches 30s |
| Related doc hybrid ranking | API / Backend (Rust) | — | HNSW cosine + EntityStore Jaccard; owned by backend |
| Entity detail page data | API / Backend (Rust) | — | `CanonicalEntity` + `doc_index` + co-occurrence aggregation |
| Sidebar saved searches section | Browser / Client | — | React component consuming hook; pure UI |
| Entity chip dual navigation | Browser / Client | — | Left click vs right click event handling in React |
| `/entity/:class/:value` route | Browser / Client | API (data) | New React route; data from `get_entity_page_data` IPC |

---

## Standard Stack

### Core (all already installed)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `@tanstack/react-query` | 5.84.2 (installed) [VERIFIED: package.json] | Server-state caching, saved-search counts TTL | Already used for all Tauri IPC queries |
| `react-router-dom` | 6.30.1 (installed) [VERIFIED: package.json] | URL param parsing for entity filters, new `/entity/:class/:value` route | Already routing framework |
| `zustand` | 5.0.11 (installed) [VERIFIED: package.json] | Optimistic UI state for saved searches (pending save indicator) | Already used for sidebar, command palette |
| `sonner` | 1.7.4 (installed) [VERIFIED: package.json] | "Save this search" toast notification + modal integration | Already in `AppShell`; D-09 specifies sonner |
| `@radix-ui/react-dialog` | 1.1.14 (installed) [VERIFIED: package.json] | "Save search" modal (shadcn Dialog primitive) | Already present in ui/ |

### Supporting (Rust — already in Cargo.toml)
| Crate | Purpose | Notes |
|-------|---------|-------|
| `uuid` | Generate `ss-{uuid}` saved search IDs | Already used in `entity_store.rs` |
| `serde` + `serde_json` | Serialize `SavedSearchStore` to JSON sidecar | Already used throughout |
| `tokio` | Async Mutex for `Arc<Mutex<SavedSearchStore>>` | Already in `AppState` |
| `chrono` or `std::time` | `created_at` ISO 8601 timestamp | Existing code uses `std::time::SystemTime`; no new dep needed |

**No new packages required.** Phase 11 reuses everything installed by Phases 8/9/10.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Batched count IPC | N per-search count queries | Batched is one network crossing vs N; React Query can't merge N queries automatically |
| shadcn Dialog for save modal | sonner toast action | Dialog gives full name-input form; toast is too small for text input |
| `{class}:{value}` URL param | `canonical_id` URL param | Class:value is human-readable, shareable, and survives entity store rebuilds; canonical_id is opaque UUID |

---

## Package Legitimacy Audit

> Phase 11 installs no new packages. All listed libraries are already present in `package.json` (verified) or `Cargo.toml`.

| Package | Registry | Status | Disposition |
|---------|----------|--------|-------------|
| `@tanstack/react-query` | npm | Already installed v5.84.2 [VERIFIED: package.json] | Approved — no install needed |
| `sonner` | npm | Already installed v1.7.4 [VERIFIED: package.json] | Approved — no install needed |
| `zustand` | npm | Already installed v5.0.11 [VERIFIED: package.json] | Approved — no install needed |
| `uuid` (Rust) | crates.io | Already in use via entity_store.rs [VERIFIED: codebase] | Approved — no install needed |

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
User clicks EntityChip
        │
        ├─── left click ──────────────────► navigate("/search?entity=Person:Alex Doe")
        │                                          │
        │                                          ▼
        │                              SearchPage reads useSearchParams()
        │                              → parses entity[] params
        │                              → passes EntityClassFilter[] into SearchFilters
        │                              → search_documents IPC ──────► Rust backend
        │                                                                    │
        │                              ┌─────────────────────────────────────┤
        │                              │  apply_entity_class_filter:          │
        │                              │  EntityStore.doc_index[canonical_id] │
        │                              │  → candidate set intersection        │
        │                              │  → HNSW search on survivors          │
        │                              └──────────────────────────────────────┘
        │
        └─── right click ─────────────► navigate("/entity/Person/Alex%20Shah")
                                                   │
                                                   ▼
                                        EntityDetailPage
                                        get_entity_page_data IPC
                                              │
                                    ┌─────────┴──────────────┐
                                    │  EntityStore             │
                                    │  CanonicalEntity         │
                                    │  doc_index[canonical_id] │
                                    │  co-occurrence agg       │
                                    └──────────────────────────┘

User saves search
        │
        ▼
save_search IPC ─► SavedSearchStore ─► saved_searches.json
        │
        ▼
Sidebar renders
        │
        ▼
get_saved_search_counts(ids) IPC ─► executes per-search count query
        │                           (same as search_documents but returns count only)
        ▼
useSavedSearches() (React Query, 30s TTL) ─► displays name + (count)

/document/:id loads
        │
        ▼
get_related_docs_scored(doc_id, 5) IPC
        │
        ├─► HNSW search: k=20 cosine neighbors from doc embedding
        │       scored as cosine_sim
        │
        └─► EntityStore lookup: doc's {class}:{value} set
                for each neighbor: Jaccard({class}:{value} sets)
                final_score = 0.6 * cosine + 0.4 * jaccard
                filter score >= 0.3, sort desc, truncate at top_n
```

### Recommended Project Structure

```
src-tauri/src/
├── saved_searches/
│   ├── mod.rs           # pub mod store; pub mod commands;
│   ├── store.rs         # SavedSearch struct, SavedSearchStore (load/save/CRUD)
│   └── commands.rs      # save_search, delete_saved_search, get_saved_searches,
│                        #   get_saved_search_counts, get_entity_page_data,
│                        #   get_related_docs_scored
├── search/
│   └── filters.rs       # extend EntityFilter → add EntityClassFilter for {class}:{value}
├── types.rs             # add EntityClassFilter, SearchFilters.entity_filters, SavedSearch,
│                        #   RelatedDocScored, EntityPageData types
└── state.rs             # add saved_search_store: Arc<Mutex<SavedSearchStore>>

client/
├── pages/
│   ├── SearchPage.tsx       # add URL param parsing, entity pills, save modal
│   ├── DocumentPage.tsx     # add Related panel
│   └── EntityDetailPage11.tsx  # NEW route /entity/:class/:value (Phase 11 variant)
├── components/
│   ├── entities/
│   │   └── EntityChip.tsx       # add onContextMenu for entity-page nav
│   ├── layout/
│   │   └── Sidebar.tsx          # add Saved Searches section below Smart Spaces
│   └── search/
│       └── SaveSearchModal.tsx  # NEW shadcn Dialog + name input
├── hooks/
│   └── useTauri.ts             # add useSavedSearches, useSaveSearch,
│                               #   useDeleteSavedSearch, useRelatedDocsScored,
│                               #   useEntityPage, useSavedSearchCounts
└── lib/
    └── types.ts                # add SavedSearch, RelatedDocScored, EntityPageData,
                                #   EntityClassFilter (extend SearchFilters)
```

### Pattern 1: SavedSearchStore — Mirror SpaceLabelCache

**What:** A JSON sidecar at `app_data_dir/saved_searches.json` managed by `Arc<tokio::sync::Mutex<SavedSearchStore>>`.
**When to use:** All CRUD on saved searches; count re-evaluation on Sidebar mount.

```rust
// Source: mirror of src-tauri/src/spaces/label_cache.rs (VERIFIED: codebase)

// types.rs additions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSearch {
    pub id: String,
    pub name: String,
    pub query: String,
    pub filters: SavedSearchFilters,  // entities: Vec<String>, topic: Option<String>
    pub created_at: String,           // ISO 8601
    pub doc_count_cache: u32,         // hint for immediate render; re-evaluated on Sidebar mount
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SavedSearchFilters {
    #[serde(default)]
    pub entities: Vec<String>,        // ["Person:Alex Doe", "Location:AlphaComplex"]
    #[serde(default)]
    pub topic: Option<String>,
    // mirrors SearchFilters scalar fields that can be saved
    #[serde(default)]
    pub doc_type: Option<String>,
    #[serde(default)]
    pub space_id: Option<String>,
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

// saved_searches/store.rs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedSearchStore {
    pub saved_searches: Vec<SavedSearch>,
}

impl SavedSearchStore {
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join("saved_searches.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, app_data_dir: &Path) -> std::io::Result<()> {
        let path = app_data_dir.join("saved_searches.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }
}
```

### Pattern 2: SearchFilters Extension for EntityClassFilter

**What:** Add `entity_filters: Option<Vec<EntityClassFilter>>` to the existing `SearchFilters` struct. Use `#[serde(default)]` to maintain backward compatibility with all existing callers.
**When to use:** Every `search_documents` call that carries `?entity=` URL params.

```rust
// Source: extends src-tauri/src/types.rs SearchFilters (VERIFIED: codebase)

/// New struct for Phase 11 entity-class URL param filters (D-01..D-03).
/// Format in URL: `?entity=Person:Alex%20Shah&entity=Location:AlphaComplex`
/// Backend parse: split on first ':' → class + value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityClassFilter {
    pub class: String,   // e.g. "Person", "Location"
    pub value: String,   // e.g. "Alex Doe", "AlphaComplex"
}

// Updated SearchFilters (add to existing struct)
// Use #[serde(default)] so Phase 6/8/9/10 callers passing no entity_filters still work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilters {
    pub doc_type: Option<String>,
    pub space_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub tags: Option<Vec<String>>,
    // Phase 11 addition
    #[serde(default)]
    pub entity_filters: Option<Vec<EntityClassFilter>>,
}
```

**Backend filter logic in `filters.rs`:**

```rust
// Source: [ASSUMED] — pattern derived from existing apply_metadata_filters (VERIFIED: codebase)
//         EntityStore.doc_index is the canonical data source

/// Apply entity-class filters: for each EntityClassFilter, look up canonical_id from
/// alias_index, then intersect doc sets. AND semantics across multiple filters.
pub fn apply_entity_class_filters(
    entity_filters: &[EntityClassFilter],
    entity_store: &EntityStore,
) -> Option<HashSet<String>> {
    if entity_filters.is_empty() {
        return None;  // no filter → no narrowing
    }

    let mut result: Option<HashSet<String>> = None;

    for ef in entity_filters {
        let key = (ef.value.to_lowercase(), ef.class.clone());
        let canonical_id = match entity_store.alias_index.get(&key) {
            Some(id) => id.clone(),
            None => return Some(HashSet::new()),  // no docs match → empty candidate set
        };
        let doc_set: HashSet<String> = entity_store
            .doc_index
            .get(&canonical_id)
            .cloned()
            .unwrap_or_default();

        result = Some(match result {
            None => doc_set,
            Some(existing) => existing.intersection(&doc_set).cloned().collect(),
        });
    }

    result
}
```

### Pattern 3: Related Docs Hybrid Ranking (HNSW + Jaccard)

**What:** `get_related_docs_scored` fetches HNSW k=20 neighbors (cosine), retrieves entity sets for each neighbor from `EntityStore`, computes Jaccard on `{class}:{value}` pairs, combines scores.
**When to use:** `/document/:id` Related panel render (on-demand, React Query 5min TTL, D-13).

```rust
// Source: [ASSUMED] — derived from existing HNSW search pattern in query.rs (VERIFIED: codebase)
//         and EntityStore.doc_index pattern (VERIFIED: codebase)

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedDocScored {
    pub document: Document,
    pub score: f64,           // 0.6 * cosine + 0.4 * jaccard
    pub cosine_score: f64,
    pub jaccard_score: f64,
}

// In commands/saved_searches/commands.rs (or extend commands/documents.rs)
#[tauri::command]
pub async fn get_related_docs_scored(
    doc_id: String,
    top_n: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<RelatedDocScored>, AppError> {
    let engine = state.engine.clone();
    let embedding_service = state.embedding_service.clone();
    let entity_store = state.entity_store.clone();
    let n = top_n.unwrap_or(5);

    let results = tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        let store_guard = entity_store.lock().map_err(|e| AppError::Internal(e.to_string()))?;

        // 1. Fetch embedding of target doc
        let collection_arc = engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| AppError::VectorStorage("documents_384 not found".to_string()))?;

        let target_entry = {
            let col = collection_arc.read();
            col.db.get(&doc_id).map_err(|e| AppError::VectorStorage(e.to_string()))?
        };
        let target_entry = target_entry.ok_or_else(|| AppError::NotFound(doc_id.clone()))?;
        let target_vec = target_entry.vector.clone();

        // 2. Build target entity set: canonical_id → {class}:{value}
        let target_entity_set: HashSet<String> = {
            if let Some(meta) = &target_entry.metadata {
                if let Some(arr) = meta.get("extracted_entities").and_then(|v| v.as_array()) {
                    arr.iter()
                        .filter_map(|e| {
                            let class = e.get("class").and_then(|v| v.as_str()).unwrap_or("");
                            let value = e.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            if class.is_empty() || value.is_empty() { None }
                            else { Some(format!("{}:{}", class, value)) }
                        })
                        .collect()
                } else { HashSet::new() }
            } else { HashSet::new() }
        };

        // 3. HNSW k=20 search
        let search_query = ruvector_core::types::SearchQuery {
            vector: target_vec,
            k: 20,
            filter: None,
            ef_search: None,
        };
        let raw = {
            let col = collection_arc.read();
            col.db.search(search_query).map_err(|e| AppError::VectorStorage(e.to_string()))?
        };

        // 4. Hybrid re-rank
        let mut scored: Vec<RelatedDocScored> = raw
            .into_iter()
            .filter(|r| r.id != doc_id)
            .filter_map(|r| {
                let cosine = (1.0 - r.score as f64).max(0.0).min(1.0);
                let meta = r.metadata.as_ref()?;

                // Build neighbor entity set
                let neighbor_entities: HashSet<String> = meta
                    .get("extracted_entities")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().filter_map(|e| {
                            let class = e.get("class").and_then(|v| v.as_str()).unwrap_or("");
                            let value = e.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            if class.is_empty() || value.is_empty() { None }
                            else { Some(format!("{}:{}", class, value)) }
                        }).collect()
                    })
                    .unwrap_or_default();

                // Jaccard over {class}:{value} sets
                let intersection = target_entity_set.intersection(&neighbor_entities).count();
                let union = target_entity_set.union(&neighbor_entities).count();
                let jaccard = if union == 0 { 0.0 } else { intersection as f64 / union as f64 };

                let score = 0.6 * cosine + 0.4 * jaccard;
                if score < 0.3 { return None; }

                let doc = build_document_from_metadata(&r.id, meta);
                Some(RelatedDocScored { document: doc, score, cosine_score: cosine, jaccard_score: jaccard })
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(n);
        Ok::<Vec<RelatedDocScored>, AppError>(scored)
    }).await??;

    Ok(results)
}
```

### Pattern 4: Entity Detail Page IPC

**What:** `get_entity_page_data(class, value)` aggregates `CanonicalEntity` + all doc refs + co-occurring entities in one call (avoids N round trips from the frontend).
**When to use:** `/entity/:class/:value` route load.

```rust
// Source: [ASSUMED] — derived from EntityStore.related_entities pattern (VERIFIED: codebase)

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityPageData {
    pub canonical: CanonicalEntity,         // id, canonical_name, entity_type, aliases, document_count
    pub documents: Vec<Document>,           // paginated; backend returns page N
    pub total_document_count: u32,
    pub co_occurring_entities: Vec<RelatedEntity>,  // top 10, reuse existing RelatedEntity type
}

#[tauri::command]
pub async fn get_entity_page_data(
    class: String,       // "Person", "Location", etc.
    value: String,       // "Alex Doe" (URL-decoded by Tauri before reaching Rust)
    page: Option<u32>,   // 0-indexed; 20 docs/page
    state: State<'_, AppState>,
) -> Result<EntityPageData, AppError>;
```

**Co-occurrence algorithm (Phase 11 approximation — no Cypher engine yet):**
For each doc in `doc_index[canonical_id]`, read its `extracted_entities` metadata, accumulate `{class}:{value}` counts for all OTHER entities. Return top-10 by count. This is the same O(|docs| × |entities_per_doc|) traversal already done by `EntityStore::related_entities` — just adapted to use `{class}:{value}` keys instead of canonical_ids.

### Pattern 5: React Query Key Strategy for Saved Search Counts

**What:** Single batched query replaces N parallel per-search queries.
**Rationale:** D-08 says "batched". N = O(saved searches count). React Query has no built-in query merger, so the batch must happen at the IPC level.

```typescript
// Source: [ASSUMED] — derived from Phase 4 queryKeys factory pattern (VERIFIED: codebase)

// In useTauri.ts — new keys
export const queryKeys = {
  // ... existing keys ...
  savedSearches: ["saved-searches"] as const,
  // Batch: sorted ids ensures same cache entry regardless of fetch order
  savedSearchCounts: (ids: string[]) =>
    ["saved-searches", "counts", [...ids].sort().join(",")] as const,
  entityPage: (cls: string, value: string) =>
    ["entity-page", cls, value] as const,
  relatedDocsScored: (docId: string) =>
    ["documents", docId, "related-scored"] as const,
};

// Hook: batched count fetch on Sidebar mount
export function useSavedSearchCounts(ids: string[]) {
  return useQuery({
    queryKey: queryKeys.savedSearchCounts(ids),
    queryFn: () =>
      tauriInvoke<Record<string, number>>(
        "get_saved_search_counts",
        { ids },
        () => Object.fromEntries(ids.map((id) => [id, 0]))
      ),
    staleTime: 30_000,   // 30s TTL per D-08
    enabled: ids.length > 0,
  });
}

// Hook: save a search
export function useSaveSearch() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (req: { name: string; query: string; filters: SavedSearchFilters }) =>
      tauriInvoke<SavedSearch>("save_search", req, async () => ({
        id: `ss-mock-${Date.now()}`,
        name: req.name,
        query: req.query,
        filters: req.filters,
        createdAt: new Date().toISOString(),
        docCountCache: 0,
      })),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: queryKeys.savedSearches });
      // Also invalidate counts since a new search was added
      qc.invalidateQueries({ queryKey: ["saved-searches", "counts"] });
    },
  });
}
```

### Pattern 6: EntityChip Dual-Navigation

**What:** Replace current single `<Link to={href}>` with a div that routes left click to search and right click to entity page.
**When to use:** All EntityChip renders after Phase 11.

```typescript
// Source: [ASSUMED] — derived from current EntityChip.tsx (VERIFIED: codebase) + D-17

import { useNavigate } from "react-router-dom";

export function EntityChip({ entity }: EntityChipProps) {
  const navigate = useNavigate();

  const resolvedClass = entity.class ?? mapLegacyEntityTypeToClass(entity.entityType);

  // Left click → filter search (D-02)
  const handleClick = (e: React.MouseEvent) => {
    e.preventDefault();
    const param = `${resolvedClass ?? entity.entityType}:${entity.value}`;
    navigate(`/search?entity=${encodeURIComponent(param)}`);
  };

  // Right click → entity detail page (D-17)
  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    const cls = resolvedClass ?? entity.entityType;
    navigate(`/entity/${encodeURIComponent(cls)}/${encodeURIComponent(entity.value)}`);
  };

  return (
    <button
      onClick={handleClick}
      onContextMenu={handleContextMenu}
      aria-label={`Filter by ${entity.value}`}
      className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full border border-border-secondary bg-bg-tertiary hover:bg-accent-subtle transition-colors"
    >
      {icon}
      <span className="text-sm text-text-primary truncate max-w-[160px]">{entity.value}</span>
      {showSubclassBadge && <Badge>{entity.subclass}</Badge>}
    </button>
  );
}
```

### Pattern 7: URL Param Parsing in SearchPage

**What:** Read `entity` params from URL and build `EntityClassFilter[]` for the backend filter.

```typescript
// Source: [ASSUMED] — derived from React Router v6 useSearchParams (VERIFIED: installed package)

import { useSearchParams } from "react-router-dom";

function useEntityFilters() {
  const [searchParams, setSearchParams] = useSearchParams();
  const rawEntities = searchParams.getAll("entity"); // ["Person:Alex Doe", "Location:AlphaComplex"]

  const entityFilters = rawEntities
    .map((raw) => {
      const colonIdx = raw.indexOf(":");
      if (colonIdx === -1) return null;
      return {
        class: raw.slice(0, colonIdx),
        value: raw.slice(colonIdx + 1),
      };
    })
    .filter(Boolean) as { class: string; value: string }[];

  const removeEntity = (chip: { class: string; value: string }) => {
    const next = rawEntities.filter(
      (r) => r !== `${chip.class}:${chip.value}`
    );
    setSearchParams((prev) => {
      prev.delete("entity");
      next.forEach((v) => prev.append("entity", v));
      return prev;
    });
  };

  const clearAll = () => setSearchParams((prev) => { prev.delete("entity"); return prev; });

  return { entityFilters, removeEntity, clearAll };
}
```

### Anti-Patterns to Avoid

- **Storing entity filters in Zustand:** They must be URL params (D-01) — Zustand state would break back-button and shareability.
- **Using canonical_id in URL:** `entity.canonicalId` is a UUID opaque to users; `{class}:{value}` is human-readable, shareable, and stable across entity store rebuilds.
- **N parallel count queries:** `useQuery(queryKeys.savedSearchCount(id))` called N times in a loop causes N IPC round trips. Use the batched `get_saved_search_counts(ids)` command instead.
- **Holding the `entity_store` std::sync::Mutex across an await:** `EntityStore` uses `std::sync::Mutex` (not `tokio::sync::Mutex`). Acquiring it inside a `spawn_blocking` closure and then trying to await is a deadlock. Always read entity_store inside `spawn_blocking`, never across an `.await` boundary.
- **Skipping the score threshold:** Including results with score < 0.3 will surface loosely related docs on the Related panel. D-12 mandates the 0.3 floor.
- **Writing `saved_searches.json` without holding the Mutex:** All reads and writes must go through the `Arc<Mutex<SavedSearchStore>>` in AppState to prevent torn writes under concurrent Sidebar renders.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Entity-class filter doc set intersection | Custom loop over all docs | `EntityStore.alias_index` → `doc_index` intersection | Already O(1) lookup + HashSet intersect; re-scanning all docs is O(N) |
| Cosine similarity | New cosine function | `entity_store.rs::cosine()` (already exported) | Tested, handles zero-vector edge case |
| JSON sidecar persistence | Custom file format | `serde_json::to_string_pretty` + `std::fs::write` | Identical to `SpaceLabelCache::save()` — tested, atomic overwrite |
| "Save this search" modal | Custom overlay | `shadcn Dialog` (`@radix-ui/react-dialog`, already installed) | Accessible, keyboard-handled, matches existing UI patterns |
| Toast notifications | Custom toast | `sonner` (already installed, already in AppShell) | Already wired; D-09 specifies sonner |
| Pagination | Custom pager | `shadcn Pagination` component (already in `ui/pagination.tsx`) | Already present in ui/ directory |
| UUID generation | Time-based IDs | `uuid::Uuid::new_v4().to_string()` | Already used in `entity_store.rs` |

**Key insight:** Phase 11 is primarily a wiring phase — nearly every primitive it needs (cosine, entity index, HNSW search, JSON sidecars, React Query, shadcn Dialog) is already present and tested. The core new work is the Jaccard re-ranking computation and the saved-search persistence module.

---

## Runtime State Inventory

> Phase 11 adds saved_searches.json sidecar. No renaming/migration — purely additive.

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | `saved_searches.json` — does not exist yet; created on first `save_search` call | None (new file; `SavedSearchStore::load` returns empty default when absent) |
| Live service config | None | None |
| OS-registered state | None | None |
| Secrets/env vars | None | None |
| Build artifacts | None — no new binary artifacts; no new model files | None |

**Nothing found requiring migration.** `SavedSearchStore::load()` follows the `SpaceLabelCache::load()` pattern: returns `Default::default()` (empty list) when file is absent, never panics.

---

## Common Pitfalls

### Pitfall 1: EntityStore std::sync::Mutex Deadlock in Async Context

**What goes wrong:** `get_related_docs_scored` acquires `entity_store.lock()` (std::sync::Mutex) and then tries to call an async function or await. The Tokio runtime detects the blocking mutex held across an await boundary and can deadlock.
**Why it happens:** `EntityStore` uses `std::sync::Mutex` (see `state.rs`), not `tokio::sync::Mutex`. All existing entity commands use `spawn_blocking`.
**How to avoid:** Wrap the ENTIRE related-docs computation inside `tokio::task::spawn_blocking(move || { ... })`. Acquire the mutex, do all work, release before returning.
**Warning signs:** Tauri command hangs indefinitely on first call to `get_related_docs_scored`.

### Pitfall 2: entity_filters Field Breaks Existing IPC Callers

**What goes wrong:** Adding `entity_filters: Vec<EntityClassFilter>` (not Option) to `SearchFilters` causes every existing `search_documents` call to fail deserialization when the frontend sends `null` or the field is absent.
**Why it happens:** Serde requires all non-Option fields unless `#[serde(default)]` is applied.
**How to avoid:** `entity_filters: Option<Vec<EntityClassFilter>>` with `#[serde(default)]`. The existing `filters.rs::apply_metadata_filters` checks `if filters.entity_filters.is_none() { return Ok(None); }` pattern mirrors existing fields.
**Warning signs:** All existing search queries return 500 after Phase 11 SearchFilters change.

### Pitfall 3: URL Entity Param Encoding Mismatch

**What goes wrong:** Frontend encodes `?entity=Person:Alex+Shah` (space → +) but backend decodes as "Alex+Shah" (literal plus). `alias_index` lookup fails.
**Why it happens:** `encodeURIComponent` encodes space as `%20`; Tauri URL decoding may not translate `+` → space.
**How to avoid:** Use `encodeURIComponent` (not `encodeURI`) consistently. Backend receives URL-decoded values directly from Tauri IPC (Tauri deserializes the param map before passing to the command). The frontend sends entity filters as part of the `SearchFilters` JSON object, not as raw URL query strings to the Rust backend — the URL params are parsed client-side and converted to the `EntityClassFilter[]` array in the IPC payload.
**Warning signs:** `alias_index` lookup returns `None` for entities that clearly exist; entity filter returns 0 results.

### Pitfall 4: Saved Search Count Staleness on Fast Mutations

**What goes wrong:** User saves a search, Sidebar re-renders immediately with `doc_count_cache: 0` before the React Query 30s TTL expires and the count query re-fires.
**Why it happens:** The batched count query has a 30s stale time; newly saved search appears but shows count 0.
**How to avoid:** On `useSaveSearch` success, invalidate `["saved-searches", "counts"]` queries via `queryClient.invalidateQueries`. This forces an immediate re-fetch. The `doc_count_cache` field in the JSON is the fallback for initial render before the count comes back.
**Warning signs:** Sidebar shows "(0)" next to newly saved search even when there are matching documents.

### Pitfall 5: co-occurring Entities Counting by Entity Instance vs Canonical

**What goes wrong:** If `entity_page_data` co-occurrence counting uses `extracted_entities.value` (raw string) instead of `canonical_id`, different surface forms of the same canonical entity ("Acme Corp" vs "Acme") count as two different co-occurring entities.
**Why it happens:** The `get_entity_page_data` IPC must read the `canonicalId` field from entity metadata, not the raw `value`, to get canonical-level co-occurrence.
**How to avoid:** Follow the same pattern as `EntityStore::related_entities` — use `canonical_id` from the stored metadata. Fall back to `{class}:{value}` key only when `canonical_id` is absent (legacy Phase 6 docs).
**Warning signs:** Entity page shows 5 "co-occurring" entries for what is clearly one entity with multiple aliases.

### Pitfall 6: React Query Cache Key Collision for Saved Search Counts

**What goes wrong:** If the count query key is `["saved-searches", "counts"]` without including the IDs, two Sidebar instances with different saved searches share the same cache entry.
**Why it happens:** React Query deduplication is based on the exact key object.
**How to avoid:** Include the sorted, comma-joined ID list in the key: `["saved-searches", "counts", [...ids].sort().join(",")]`. The sort ensures the key is stable regardless of order.
**Warning signs:** Sidebar shows wrong counts when two windows are open with different saved search sets.

### Pitfall 7: Right-Click Context Menu Navigation Blocked by Browser

**What goes wrong:** `onContextMenu` on a div does not fire on iOS Safari or some Tauri WebView configurations; long-press may not register as a right-click on touch screens.
**Why it happens:** D-17 specifies right-click/long-press but Tauri WebView on macOS uses WKWebView which handles context menus differently than Chrome.
**How to avoid:** Implement right-click navigation as an additional `onContextMenu` handler that calls `e.preventDefault()` and then `navigate()`. For long-press, add `onTouchStart` / `onTouchEnd` with a 500ms timeout. Keep left-click as primary interaction since that's ENEX-01's core requirement.
**Warning signs:** Long-press on entity chip opens native browser context menu instead of navigating.

---

## Code Examples

### IPC: get_saved_search_counts (batched)

```rust
// Source: [ASSUMED] — derived from existing IPC patterns in commands/entities.rs (VERIFIED: codebase)

/// Returns a map of saved_search_id → document count.
/// Executes a lightweight search query (no embedding) per saved search:
/// just applies metadata + entity_class filters against the collection index.
#[tauri::command]
pub async fn get_saved_search_counts(
    ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<HashMap<String, u32>, AppError> {
    let saved_search_store = state.saved_search_store.clone();
    let entity_store = state.entity_store.clone();
    let engine = state.engine.clone();

    let result = tokio::task::spawn_blocking(move || {
        let store = saved_search_store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let entity_guard = entity_store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let engine_guard = engine.blocking_lock();

        let mut counts = HashMap::new();
        for id in &ids {
            let Some(ss) = store.saved_searches.iter().find(|s| &s.id == id) else {
                counts.insert(id.clone(), 0u32);
                continue;
            };
            // Count matching docs using the entity_class filter intersection
            let entity_filters: Vec<EntityClassFilter> = ss
                .filters
                .entities
                .iter()
                .filter_map(|e| {
                    let idx = e.find(':')?;
                    Some(EntityClassFilter {
                        class: e[..idx].to_string(),
                        value: e[idx + 1..].to_string(),
                    })
                })
                .collect();
            let count = count_matching_docs(&entity_filters, &entity_guard, &engine_guard)?;
            counts.insert(id.clone(), count);
        }
        Ok::<HashMap<String, u32>, AppError>(counts)
    }).await??;
    Ok(result)
}
```

### Frontend: Sidebar Saved Searches Section

```typescript
// Source: [ASSUMED] — derived from existing Sidebar.tsx Smart Spaces section (VERIFIED: codebase)

import { Bookmark } from "lucide-react";
import { useSavedSearches, useSavedSearchCounts } from "@/hooks/useTauri";

function SavedSearchesSection() {
  const { data: savedSearches } = useSavedSearches();
  const ids = savedSearches?.map((s) => s.id) ?? [];
  const { data: counts } = useSavedSearchCounts(ids);

  if (!savedSearches?.length) return null;

  return (
    <div className="px-3 py-2">
      <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-1">
        Saved Searches
      </p>
      {savedSearches.map((ss) => (
        <Link
          key={ss.id}
          to={buildSavedSearchUrl(ss)}
          className="flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-bg-tertiary text-text-secondary hover:text-text-primary transition-colors"
        >
          <Bookmark size={14} className="shrink-0" />
          <span className="text-sm truncate flex-1">{ss.name}</span>
          <span className="text-xs text-text-tertiary">
            ({counts?.[ss.id] ?? ss.docCountCache})
          </span>
        </Link>
      ))}
    </div>
  );
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| EntityChip links to `/entities/:canonicalId` (Phase 6) | EntityChip left-click → `/search?entity=Class:value`, right-click → `/entity/Class/value` | Phase 11 | Two navigation modes; filter search is primary |
| `get_related_documents` uses DocumentGraph edges (Phase 3) | `get_related_docs_scored` uses HNSW cosine + EntityStore Jaccard | Phase 11 | Entity-aware relatedness; more relevant for personal corpora |
| SearchFilters has no entity field | SearchFilters gains `entity_filters: Option<Vec<EntityClassFilter>>` | Phase 11 | Enables URL-param-driven entity filtering |
| No virtual saved spaces | `saved_searches.json` sidecar + Sidebar section | Phase 11 | Persistent user-curated search views |

**Deprecated/outdated:**
- `EntityChip` single-click-to-entity-page: replaced by left-click-to-search + right-click-to-entity-page.
- `get_related_documents` (Phase 3 graph-edge-based): still exists for backward compat but Related panel on DocumentPage uses the new `get_related_docs_scored` IPC.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `get_related_docs_scored` is a new IPC; the existing `get_related_documents` stays registered for backward compat | Architecture Patterns | If planner consolidates to one command, the signature conflict must be resolved |
| A2 | Co-occurrence counting in `get_entity_page_data` uses `canonicalId` from stored metadata (falls back to `{class}:{value}` for legacy docs) | Pattern 4 | If canonical_id is absent on many docs (Phase 6 vintage), co-occurrence will be under-counted |
| A3 | Saved-search count re-evaluation does NOT re-embed the query text; it only applies metadata/entity filters against the collection index | Pattern: get_saved_search_counts | If full semantic search is required for counts, latency on Sidebar render will be unacceptable |
| A4 | The `/entity/:class/:value` route is a NEW route separate from `/entities/:id` (which is Phase 6's route by canonical_id) | Architecture Patterns | If the planner reuses `/entities/:id`, the URL format decision (class+value vs UUID) must be resolved in CONTEXT.md |
| A5 | `SavedSearchFilters.entities` stores `"{class}:{value}"` strings, not canonical_ids, matching the URL param format | Pattern 1 | If stored as canonical_ids, saved searches become stale when entity store rebuilds assign new UUIDs |
| A6 | `entity_class_filter` pre-narrows the HNSW candidate set (intersection of doc sets from alias_index) before vector search; it does NOT post-filter results | Pattern 2 | If done post-filter, large corpora with many entity-matching docs would cause the HNSW to over-fetch |

---

## Open Questions

1. **Saved search count algorithm depth**
   - What we know: D-08 says "count query per saved search"; D-06 shows `doc_count_cache` as a hint.
   - What's unclear: Does "count query" mean full semantic search (embed query → HNSW → filter) or metadata-only filter (no embedding)?
   - Recommendation: Metadata-only + entity filter count (no embedding) for Sidebar performance. Full semantic count would require a model inference call per saved search on every Sidebar render.

2. **EntityDetailPage route conflict with Phase 6 `/entities/:id`**
   - What we know: Phase 6 ships `/entities/:id` (UUID-keyed); Phase 11 adds `/entity/:class/:value` (human-readable).
   - What's unclear: Whether the Phase 11 entity detail page replaces or supplements the Phase 6 page.
   - Recommendation: Keep both routes for now; Phase 11's `/entity/:class/:value` is the new surface; Phase 13 (Cypher graph) will enrich it further.

3. **Multi-entity filter source-of-truth for saved searches**
   - What we know: D-06 `filters.entities` stores `["Person:Alex Doe"]`; D-01 URL format is `?entity=Person:Alex Doe`.
   - What's unclear: When a saved search is loaded, does clicking it navigate to `/search?entity=...` with params rebuilt from the stored filter, or does it POST to a search API with the filters directly?
   - Recommendation: Navigate to URL with query params reconstructed from stored `filters` object. This preserves shareability and back-button behavior.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| vitest | Frontend tests | Yes | 3.2.4 [VERIFIED: package.json] | — |
| @testing-library/react | Frontend tests | Yes | 16.3.2 [VERIFIED: package.json] | — |
| cargo (Rust) | Backend tests | Yes | installed [VERIFIED: existing phases compile] | — |
| sonner | Save modal toast | Yes | 1.7.4 [VERIFIED: package.json] | — |
| @radix-ui/react-dialog | Save modal | Yes | 1.1.14 [VERIFIED: package.json] | — |

**Missing dependencies with no fallback:** none
**Missing dependencies with fallback:** none

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Frontend framework | Vitest 3.2.4 + @testing-library/react 16.3.2 |
| Backend framework | Rust built-in `#[test]` + `#[tokio::test]` |
| Config file | `vite.config.ts` (test section, jsdom environment) [VERIFIED: codebase] |
| Quick run command | `bun test --run` |
| Full suite command | `bun test --run` + `cargo test --manifest-path src-tauri/Cargo.toml` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ENEX-01 | Entity chip left-click navigates to `/search?entity=Class:value` | unit | `bun test --run client/components/entities/EntityChip.test.tsx` | Partially — EntityChip.test.tsx exists; needs new test cases |
| ENEX-01 | SearchFilters entity_class_filter pre-narrows candidate set | unit (Rust) | `cargo test -p cortex-lib -- filters::tests` | No — Wave 0 |
| ENEX-02 | save_search IPC persists to saved_searches.json + returns SavedSearch | unit (Rust) | `cargo test -p cortex-lib -- saved_searches::tests` | No — Wave 0 |
| ENEX-02 | Sidebar renders Saved Searches section with count | unit | `bun test --run client/components/layout/Sidebar.test.tsx` | Partially — Sidebar.test.tsx exists; needs saved-searches assertions |
| ENEX-03 | get_related_docs_scored returns top-5 with score >= 0.3 | unit (Rust) | `cargo test -p cortex-lib -- saved_searches::commands::tests::test_related_scoring` | No — Wave 0 |
| ENEX-03 | Hybrid score formula: 0.6*cosine + 0.4*jaccard | unit (Rust) | `cargo test -p cortex-lib -- saved_searches::commands::tests::test_score_formula` | No — Wave 0 |
| ENEX-04 | useSavedSearchCounts returns map keyed by search id | unit | `bun test --run client/hooks/useTauri.test.ts` | Partially — useTauri.test.ts exists; needs saved-search count hook test |

### Sampling Rate
- **Per task commit:** `bun test --run`
- **Per wave merge:** `bun test --run` + `cargo test --manifest-path src-tauri/Cargo.toml`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src-tauri/src/saved_searches/store.rs` — unit tests for `SavedSearchStore::load/save/roundtrip/malformed-json` (mirrors label_cache.rs test suite)
- [ ] `src-tauri/src/saved_searches/commands.rs` — unit tests for `get_saved_search_counts` zero-filter case + entity filter intersection
- [ ] `src-tauri/tests/` or inline — `get_related_docs_scored` score formula: `test_score_formula_pure_cosine`, `test_score_formula_pure_jaccard`, `test_threshold_filter`
- [ ] `client/components/entities/EntityChip.test.tsx` — extend with left-click-navigates-to-search + right-click-navigates-to-entity tests
- [ ] `client/components/layout/Sidebar.test.tsx` — extend with saved-searches section rendering test
- [ ] `client/hooks/useTauri.test.ts` — extend with `useSavedSearches`, `useSaveSearch`, `useSavedSearchCounts` mock tests

*(If no gaps: "None — existing test infrastructure covers all phase requirements")*

---

## Security Domain

> `security_enforcement` is not set to false in config.json — section included.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Saved searches are local-only; no auth required |
| V3 Session Management | No | Desktop app; no sessions |
| V4 Access Control | No | Single-user local desktop |
| V5 Input Validation | Yes | `entity` URL params and `saved_search.name` are user input; validate before use |
| V6 Cryptography | No | `saved_searches.json` is plaintext (same as `settings.json` — per REQUIREMENTS.md Out of Scope: Keychain v1.2) |

### Known Threat Patterns for Entity Filter + Saved Searches

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed `{class}:{value}` param causes alias_index panic | Tampering | `split_at_first_colon` returns `None` on missing colon; handled gracefully with empty result, not panic |
| Saved search name injection in sidebar rendering | Tampering | React JSX escapes string interpolation by default; no `dangerouslySetInnerHTML` |
| `doc_count_cache` overflow (u32 max) | Denial of service | Rust u32::MAX = ~4B docs; not a realistic threat for personal corpus |
| Concurrent writes to `saved_searches.json` from multiple windows | Tampering / Corruption | Mitigated by `Arc<Mutex<SavedSearchStore>>` — single writer at a time |

---

## Sources

### Primary (HIGH confidence)
- `src-tauri/src/spaces/label_cache.rs` — canonical pattern for JSON sidecar persistence [VERIFIED: codebase]
- `src-tauri/src/graph/entity_store.rs` — `EntityStore`, `alias_index`, `doc_index`, `related_entities`, `cosine()` [VERIFIED: codebase]
- `src-tauri/src/search/filters.rs` — `apply_metadata_filters`, `EntityFilter`, existing filter pipeline [VERIFIED: codebase]
- `src-tauri/src/search/query.rs` — HNSW search pattern, `build_document_from_metadata` [VERIFIED: codebase]
- `src-tauri/src/types.rs` — `SearchFilters`, `ExtractedEntity`, `Space`, `CanonicalEntity` [VERIFIED: codebase]
- `src-tauri/src/state.rs` — `AppState` structure, mutex conventions [VERIFIED: codebase]
- `src-tauri/src/commands/entities.rs` — `spawn_blocking` + entity_store IPC pattern [VERIFIED: codebase]
- `src-tauri/src/commands/documents.rs` — existing `get_related_documents` (graph-edge-based) [VERIFIED: codebase]
- `src-tauri/src/lib.rs` — IPC registration pattern + AppState construction [VERIFIED: codebase]
- `client/hooks/useTauri.ts` — `queryKeys` factory, existing hook patterns [VERIFIED: codebase]
- `client/lib/types.ts` — frontend type definitions, `SearchFilters`, `Space` [VERIFIED: codebase]
- `client/components/entities/EntityChip.tsx` — current EntityChip implementation [VERIFIED: codebase]
- `client/components/layout/Sidebar.tsx` — Sidebar structure, Smart Spaces section [VERIFIED: codebase]
- `client/App.tsx` — existing routes, AppShell wrapper [VERIFIED: codebase]
- `package.json` — installed npm packages [VERIFIED: codebase]
- `.planning/phases/11-entity-driven-exploration/11-CONTEXT.md` — all locked decisions D-01..D-18 [VERIFIED: planning doc]
- `.planning/REQUIREMENTS.md` — ENEX-01..04 requirements [VERIFIED: planning doc]

### Secondary (MEDIUM confidence)
- React Router v6 `useSearchParams` docs — `getAll()` for repeated params [CITED: reactrouter.com/en/main/hooks/use-search-params]

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Saved-search persistence (sidecar schema, mutex pattern): HIGH — exact mirror of SpaceLabelCache which is verified in codebase
- SearchFilters extension: HIGH — additive change to existing struct; backward-compat pattern established by Phases 8/9/10
- Hybrid ranking (cosine + Jaccard): HIGH for algorithm; MEDIUM for exact implementation — HNSW search pattern verified, Jaccard computation derived
- React Query key strategy: HIGH — follows established queryKeys factory; batched IPC pattern is idiomatic
- EntityChip dual navigation: HIGH — current implementation verified; modification is well-scoped
- Entity page IPC: MEDIUM — derives from existing `related_entities` + `get_documents_for_entity` patterns; one-call aggregation is new but straightforward

**Research date:** 2026-07-09
**Valid until:** 2026-08-09 (stable codebase)

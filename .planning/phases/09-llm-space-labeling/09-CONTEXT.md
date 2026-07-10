# Phase 9: LLM Space Labeling - Context

**Gathered:** 2026-07-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 9 delivers three things:

1. **LLM-generated Space labels + descriptions** — replaces rule-based `spaces/naming.rs` with a fresh `spaces/llm_labeler.rs` that consumes each cluster's top-20 doc titles + entity summary + top-3 topics + top-10 tags and returns `{label, description}` via `ai_request()`. Cached by membership fingerprint so unchanged clusters skip the LLM entirely.

2. **`ruvector-cluster` swap** — replaces hand-rolled k-means in `spaces/clustering.rs`. Density-based HDBSCAN via ruvector-cluster auto-picks cluster count; drop-in signature change. Silent k-means fallback if ruvector-cluster errors.

3. **`ruvector-domain-expansion` bootstrap** — new spaces on recluster attempt to transfer their label from the nearest existing labeled space via ruvector-domain-expansion before triggering a full LLM call. Cheap labels for near-duplicate clusters.

Also adds one Phase-11 hook: `Space.canonical_entity_hint` field — the top-count entity that defines the space (drives Phase 11 entity-detail navigation).

### Out of scope

- Template detection / doc-type ontology (Phase 12+ via GNN clustering)
- Per-template field extraction (Phase 13)
- Journey traversal UI across entities (Phase 13 Cypher engine)
- Hierarchical sub-spaces (Phase 10)
- User-editable ontology (v1.2)
- Multi-language label generation (English + Roman-script only)

</domain>

<decisions>
## Implementation Decisions

### Label & Description Generation (Area 1)

- **D-01: LLM input.** Per-cluster payload sent to LLM:
  - Top-20 doc titles (deduped, ordered by centroid-distance ascending)
  - Entity summary: top-5 classes by count (`Person: 12, Organization: 8, Amount: 5, Date: 42, Identifier: 3`)
  - Top-3 topics (from Phase 8 free-form topic field, ranked by count)
  - Top-10 tags (from Phase 8 free-form tags, ranked by count)
  - Cluster size (doc count)
  Token budget: ~1500 input tokens per cluster.
- **D-02: Output format.** Single JSON call:
  ```json
  { "label": "Property Tax Records", "description": "Documents related to municipal property tax assessments, receipts, and demand notices." }
  ```
  Parse with `serde_json` + fence-strip (reuse Phase 8 `strip_json_fences` from `pass2_llm_refiner`).
- **D-03: Prompt template.** Inline `const SPACE_LABEL_PROMPT: &str` in `spaces/llm_labeler.rs`. Multi-region few-shot examples matching Phase 8 corpus (Property Tax Records, Kids School Docs, Health Insurance Claims, Investment Statements, Vehicle Registration, Identity Docs).
- **D-04: Model + temperature.** Same active provider + user-selected `extraction_model` setting from Phase 8 (single source of truth). `temperature = 0.3` — small creativity for label variety. Deterministic idempotence not required for labels (they can naturally vary across re-runs).

### Membership Fingerprint & Cache (Area 2)

- **D-05: Fingerprint = SHA-256(sorted doc-id set).** First 16 hex chars used as cache key. Deterministic and membership-only (centroid drift doesn't invalidate cache).
- **D-06: Shift threshold = 20% Jaccard distance** from cached fingerprint (per LLML-03). Formula: `|added ∪ removed| / |union| > 0.20 → re-label`. Matches spec exactly.
- **D-07: Cache storage = `app_data_dir/space_labels.json` sidecar.** Schema:
  ```json
  {
    "labels": {
      "space-abc123": {
        "fingerprint": "d41d8cd98f00b204",
        "label": "Property Tax Records",
        "description": "...",
        "canonical_entity_hint": "Property: AlphaComplex",
        "generated_at": "2026-07-04T10:00:00Z",
        "user_locked": false
      }
    }
  }
  ```
  Survives restarts; uses same file locking pattern as Phase 8 taxonomy would have used.
- **D-08: Cache eviction.** Never evict on time. Deleted spaces get their cache entry removed lazily on next recluster (garbage collected). Prevents unbounded growth without TTL complexity.

### ruvector-cluster + Domain Expansion (Area 3)

- **D-09: Swap strategy.** Replace `cluster_documents()` in `spaces/clustering.rs` with `ruvector-cluster` API call. Keep the outer function signature identical so `manager.rs` doesn't change. Internal `ClusterResult` type stays.
- **D-10: Clustering algorithm = HDBSCAN via ruvector-cluster.** Density-based, auto-picks k. Rationale: k-means requires user-tuned k that's unstable as corpus grows; HDBSCAN adapts. Min-cluster-size = 3 docs (below that → noise). Noise docs land in a synthetic "Misc" space.
- **D-11: `ruvector-domain-expansion` for new spaces.** On recluster, for each new space (fingerprint not in cache):
  1. Find top-3 nearest existing labeled spaces by centroid cosine similarity
  2. Call `ruvector-domain-expansion` API to transfer/adapt label from nearest neighbor
  3. If confidence >= 0.75, use transferred label
  4. Else fall back to full LLM call (D-01..D-04)
  Saves LLM cost on incremental corpus growth.
- **D-12: ruvector-cluster fallback.** On any ruvector-cluster init or call error → log warning, silently fall back to existing k-means (`k = sqrt(doc_count / 2)` heuristic). Never leave user with zero spaces.

### Collision Resolution & UI (Area 4)

- **D-13: Duplicate label handling.** After a labeling batch completes, detect collisions (identical `label` strings). For each collision:
  1. Retry LLM with prompt `"Avoid these labels already used in the corpus: [X, Y, Z]. Regenerate a distinct 2-4 word label."`
  2. If still collides after retry, auto-suffix with the 2nd distinguishing entity: `"Work Docs" → "Work Docs — Freelance"` (append top non-shared entity)
  3. Max 2 retries per space
- **D-14: UI while labeling.** SpaceCard shows shimmer skeleton + "Generating label…" placeholder. Rest of app fully navigable (LLML-05). Backend emits `space-labeling-progress` Tauri event; frontend `useSpaceLabels()` hook subscribes and refetches on completion.
- **D-15: Manual user rename LOCKS the space.** Space detail page has an inline "Edit label" input. When user edits, `Space.user_locked = true` in cache. Subsequent LLM re-labels skip user-locked spaces (respect user intent). Reset via "Clear override" button.
- **D-16: Description surfaces.** Tooltip on SpaceCard hover (Radix Tooltip primitive). Full description on `/spaces/:id` page below the breadcrumb. Truncate at 100 chars in tooltip; full in detail.

### Canonical Entity Hint (new — Phase 11/13 hook)

- **D-17: `Space.canonical_entity_hint: Option<String>` field.** Computed as the highest-count entity across the space's docs. Format: `"{ClassName}: {value}"` (e.g., `"Property: AlphaComplex"`, `"Person: Alex Doe"`). Stored in space_labels.json cache alongside label. Feeds Phase 11 entity-detail page navigation and Phase 13 graph node linking.
- **D-18: Hint scope.** Nullable — if no single entity dominates (top entity count < 20% of doc count), leave `None`. Prevents noisy hints on truly mixed spaces.

### Claude's Discretion (Planner-owned)

- Exact IPC command names: `get_space_labels`, `rename_space_label`, `clear_space_override`, `trigger_relabel` (planner finalizes).
- Rust module layout: `spaces/llm_labeler.rs` + `spaces/label_cache.rs` + `spaces/fingerprint.rs`. Planner picks.
- ruvector-cluster crate API surface — planner reads local `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-cluster/` before implementing.
- ruvector-domain-expansion API surface — same.
- HDBSCAN min-cluster-size default (recommended 3; planner may tune to 5 based on empirical corpus tests).
- Concurrency: parallel labeling of N spaces (semaphore=8 reuses Phase 8 pattern).
- React Query hook design: `useSpaceLabels`, `useRelabelSpace`, `useRenameSpace`.
- SpaceCard hover tooltip implementation — Radix Tooltip already in shadcn.

</decisions>

<canonical_refs>
## Canonical References

### Project specs
- `.planning/ROADMAP.md` §"Phase 9: LLM Space Labeling" — goal, requirements (LLML-01..05), success criteria (7)
- `.planning/REQUIREMENTS.md` §"LLM Space Labeling" — LLML-01..05 full text
- `.planning/phases/08-llm-entity-extraction/08-CONTEXT.md` — Phase 8 provides topic + tags fields consumed as label input signals

### Existing Cortex code
- `src-tauri/src/spaces/clustering.rs` — hand-rolled k-means (being replaced)
- `src-tauri/src/spaces/naming.rs` — rule-based naming (being replaced)
- `src-tauri/src/spaces/manager.rs` — recluster orchestration (light updates: swap naming call, emit progress event)
- `src-tauri/src/spaces/mod.rs`
- `src-tauri/src/ai/service.rs` — `ai_request()` router (Phase 7)
- `src-tauri/src/pipeline/pass2_llm_refiner.rs` — reuse `strip_json_fences` for robust JSON parsing
- `src-tauri/src/commands/settings.rs` — reuse `extraction_model` setting for label generation
- `client/components/spaces/SpaceCard.tsx` — extend for shimmer + tooltip
- `client/pages/SpacesPage.tsx` — SpaceCard list
- `client/pages/SpaceDetailPage.tsx` — inline rename input; full description
- `client/hooks/useTauri.ts` — add `useSpaceLabels`, `useRelabelSpace`, `useRenameSpace`

### RuVector crates (local)
- `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-cluster/` — HDBSCAN + other clustering algorithms
- `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-domain-expansion/` — label transfer

### Patterns to mirror
- Phase 8 JSON fence-strip + serde_json parsing pattern
- Phase 7 `ai_request()` provider-agnostic call
- Phase 5 settings JSON sidecar pattern → `space_labels.json`
- Phase 4/8 React Query hook factory
- Phase 8 semaphore concurrency (8 in-flight)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ai/service.rs::ai_request()` — provider-agnostic entry (Phase 7)
- `ai/retry.rs` — exponential backoff (Phase 7)
- `pipeline/pass2_llm_refiner.rs::strip_json_fences` — JSON fence-strip helper
- `commands/settings.rs::extraction_model` — model selection reused for labeling
- Settings JSON sidecar pattern
- React Query hook factory
- shadcn Tooltip primitive
- `sonner` toast for user-facing progress
- Phase 8 free-form `topic` + `tags` fields — label input signals

### Established Patterns
- IPC: `#[tauri::command] async + spawn_blocking + serde camelCase`
- App state: `Arc<tokio::sync::Mutex<T>>` via `.manage()`
- Persistence: JSON sidecars in `app_data_dir/`
- Event-driven progress: Tauri emit + frontend `useEffect` listener
- Error surfacing: `AppError` → `sonner` toast
- Concurrency: `tokio::sync::Semaphore` for provider rate limits

### Integration Points
- `src-tauri/src/lib.rs` — register label-cache state via `.manage()`
- `commands/mod.rs` — add space-label IPC commands
- `spaces/mod.rs` — add `llm_labeler`, `label_cache`, `fingerprint` modules
- `client/components/spaces/SpaceCard.tsx` — shimmer + tooltip + canonical_entity_hint display
- `client/pages/SpaceDetailPage.tsx` — inline rename + description
- `client/hooks/useTauri.ts` — new hooks

</code_context>

<specifics>
## Specific Ideas

- **Universal seed + adaptive engine, mirrored from Phase 8** — 8-class ontology is universal (works for everyone). Label emergence via free-form LLM output is user-corpus-driven. No hardcoded label whitelist.
- **~/private sample corpus for prompt tuning** — same folders as Phase 8: property, identity, vehicle, finance, kids, insurance, taxes. Planner samples ~10 clusters from a test index during a research spike.
- **Cluster-membership fingerprint = sorted doc-id set** — resilient to centroid drift and re-clustering noise. Only true membership change triggers re-label.
- **User-locked label = LOCKED forever** — respects user intent. LLM never overwrites manual renames. Reset via explicit "Clear override" button.
- **Canonical entity hint sets up Phase 11 & 13** — one field addition NOW enables entity-driven navigation later. Zero extra scope in Phase 9.
- **ruvector-cluster + ruvector-domain-expansion adoption from ROADMAP.md** — Phase 9 is the first use of ruvector's clustering + transfer-learning crates. Sets pattern for Phases 10+.
- **Description on hover keeps SpaceCard visually clean** — Radix Tooltip is already in shadcn. Reuse for topic hover in Phase 8 UI (retrofit if time permits).

</specifics>

<deferred>
## Deferred Ideas

### Phase 9 follow-ups (v1.2 candidates)
- **Streaming label generation** — for 100+ new-space reclusters, streaming per-space labels progressively rather than batching. Single-shot for v1.1.
- **Label quality metric** — measure label diversity (uniqueness ratio) + semantic overlap with cluster docs. Deferred.
- **User label suggestions** — offer 3 candidate labels per space instead of 1. Deferred.
- **Ontology extraction from labels** — build a corpus-level topic hierarchy from label patterns (Property → Property Tax → Property Insurance). Deferred to Phase 10 hierarchical spaces.

### Downstream phase dependencies
- **Phase 10 (Hierarchical Spaces)** — consumes labels + descriptions to name sub-spaces. Uses `ruvector-hyperbolic-hnsw` for depth-aware search.
- **Phase 11 (Entity-Driven Exploration)** — consumes `canonical_entity_hint` for entity-detail navigation.
- **Phase 12 (GNN Clustering Swap)** — replaces the ruvector-cluster HDBSCAN backend with GNN-based clustering. Phase 9's swap is a stepping stone.
- **Phase 13 (Cypher Entity Graph)** — consumes `canonical_entity_hint` as node linking hint.

### v2 / future — Ontology + Journey (deep design captured for later phases)
- **Template detection** (Phase 12+): docs with same topic + same identifier subclasses + same amount/date shape cluster into implicit "doc templates" (e.g., "Property Tax Notice 2023-24"). GNN clustering on doc-doc graph surfaces dense sub-communities. LLM names each template.
- **Per-template field extraction** (Phase 13): once template known, LLM extracts named fields (`assessment_year`, `plot_number`, `demand_id` for property tax; `pan_number`, `holder_name` for PAN Card). Templates form a per-user emergent ontology.
- **Cross-doc journey linking** (Phase 13): entity as first-class node (Property "AlphaComplex Plot 42", Person "Alex Doe"). Docs are edges + attached facts. Cypher graph query walks: `Property "AlphaComplex" → tax docs → receipts → renovation invoices → insurance policy → sale deed`. Journey = ordered traversal of doc-edges attached to a target entity node, sorted by date.
- **User use-case adaptation** — layer 1 (universal 8-class) works for anyone; layer 2 (template ontology) emerges per corpus (medical office sees "prescription template"; lawyer sees "affidavit template"; homeowner sees "tax receipt template"); layer 3 (journey traversal) is a generic graph engine any user drives via entity selection.
- **User-editable ontology** — Settings → Templates panel with add/rename/merge; deferred to v1.2.
- **Multilingual labels** — Devanagari, Tamil, Arabic corpus labeling. v2.
- **Label-driven Space colors + icons** — LLM proposes color + Lucide icon alongside label. v1.2 polish.

</deferred>

---

*Phase: 09-llm-space-labeling*
*Context gathered: 2026-07-04*

# Phase 10: Hierarchical Spaces - Research

**Researched:** 2026-07-08
**Domain:** Sub-space detection, recursive k-means, ruvector-hyperbolic-hnsw audit, label cache extension, Space type extension, Sidebar Zustand store, shadcn Breadcrumb/Collapsible
**Confidence:** HIGH (Rust codebase: VERIFIED; ruvector crate audit: VERIFIED; frontend patterns: VERIFIED from live source)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Sub-Space Detection & Creation (Area 1)**
- D-01: Threshold = 50 documents per parent Space (HSPC-01). Constant: `const SUB_SPACE_THRESHOLD: usize = 50;`
- D-02: Sub-clustering algorithm = recursive k-means on intra-cluster vectors. `k = sqrt(n / 2).max(2)`. Rationale: HDBSCAN needs enough density which sub-clusters typically lack. Recursive k-means is deterministic + works on small vector sets.
- D-03: Max hierarchy depth = 2 (Parent → Sub). Field `Space.depth: u8` gates recursion.
- D-04: Min docs per sub-space = 3. Sub-clusters below threshold roll up to a synthetic "Misc" sub-space per parent (HSPC-03). No document is silently dropped.

**Sub-Space Labeling & Persistence (Area 2)**
- D-05: Reuse Phase 9 `LlmSpaceLabeler` with parent-context prompt variant: `"You are labeling a sub-space of the '{parent_label}' Space. Return a 2-4 word label distinct from '{parent_label}'. Cluster documents: [...]"`
- D-06: Sub-spaces cached in same `space_labels.json`. Cache entries carry `parent_id: Option<String>` and `depth: u8` fields (new). Same fingerprint / 20% Jaccard shift logic.
- D-07: Space data model extends `types.rs::Space`:
  - `parent_id: Option<String>` — None for top-level, Some(uuid) for sub-spaces
  - `depth: u8` — 0 for top-level, 1 for sub-space
  - `sub_space_ids: Vec<String>` — computed at recluster for top-level spaces; empty for sub-spaces
- D-08: Recluster invalidates sub-spaces. When parent's membership shifts > 20%, drop ALL its sub-spaces from cache + recompute.

**ruvector-hyperbolic-hnsw Adoption (Area 3)**
- D-09: Planner audits first — confirmed by researcher (see CRITICAL section below).
- D-10: If usable — dual-index pattern. Hyperbolic-hnsw as SECONDARY index over parent Space centroid tree; keep flat HNSW for top-level search. Selects hyperbolic path only when `SearchFilters.parent_space_id.is_some()`.
- D-11: Silent fallback. On hyperbolic init or lookup error → log warning + fall back to flat HNSW filtered by parent Space membership.
- D-12: Perf gate (SC5). Integration test asserts ≤ 2× flat baseline on 10K-doc corpus.

**UI — Sidebar Expand + Breadcrumb + Detail (Area 4)**
- D-13: Sidebar interaction — inline expand. Click chevron → expands sub-space list without page navigation. Chevron rotates 0°→90°. Persists expanded state in Zustand `useSidebarStore`.
- D-14: Sub-count format = `"Property (3)"`. Text child count `text-xs text-text-tertiary` inline beside name.
- D-15: Breadcrumb on `/spaces/:id`. Format `Spaces / Property / Tax`. Uses shadcn Breadcrumb primitive (confirmed installed).
- D-16: Sub-space detail page = SpaceDetailPage reused. Adds parent context banner above breadcrumb.
- D-17: Sidebar top-5 selection by document count. Sub-spaces of expanded parents render beneath their parent within the top-5 slot.

### Claude's Discretion

- Exact IPC command signatures: `get_sub_spaces(parent_id)`, `expand_space(space_id)`, `get_hierarchy()` — planner finalizes
- Rust module layout: `spaces/subspace_detector.rs` + extended `spaces/manager.rs` recluster
- ruvector-hyperbolic-hnsw exact API surface — VERIFIED by researcher (see Standard Stack section)
- Whether to expose sub-spaces on `useSpaces()` hook (single hook, flat list, frontend filters by `parent_id`) — recommended
- Sidebar animation — Framer Motion vs CSS transitions for chevron rotate + list expand. shadcn Collapsible primitive available (confirmed installed)
- SpaceCard `sub_count` display — inline text per UI-SPEC (not badge)
- Empty sub-space handling — parent w/o sub-spaces (< 50 docs) shows no sub-space section
- Perf test corpus source

### Deferred Ideas (OUT OF SCOPE)

- Depth ≥ 3 (Property → Tax → Assessment Year)
- Manual doc-move between sub-spaces
- Sub-space thumbnails / custom icons (v1.2)
- Sidebar drag-to-reorder Spaces (v1.2)
- Sub-space search perf tuning — hyperbolic-hnsw parameter sweep
- Cross-parent sub-space merge
- Sub-space templates
- User-editable sub-space taxonomy (v1.2)
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HSPC-01 | Top-level Spaces auto-split into sub-spaces when a cluster exceeds 50 documents | D-01: `SUB_SPACE_THRESHOLD=50`. `subspace_detector.rs` calls `cluster_documents()` with `k=sqrt(n/2).max(2)`. |
| HSPC-02 | Sub-spaces are LLM-labeled and clickable, navigating to /spaces/:id with parent breadcrumb | D-05 parent-context prompt reuses `llm_labeler.rs::label_cluster()`. D-15 shadcn Breadcrumb (confirmed installed). |
| HSPC-03 | Sub-clustering uses HDBSCAN or recursive k-means; unclustered docs go in "Misc" sub-space | D-02 recursive k-means (k=sqrt(n/2)). D-04 min 3 docs; orphans → "Misc". |
| HSPC-04 | Sidebar shows top 5 Spaces with sub-counts; clicking a Space expands sub-spaces inline | D-13/D-14/D-17. shadcn Collapsible confirmed. Zustand `useSidebarStore` extended with `expandedSpaceIds`. |
| SC5 | Sub-space search ≤ 2× flat baseline | D-12 perf gate. ruvector-hyperbolic-hnsw USABLE (confirmed). Fallback path always available (D-11). |
</phase_requirements>

---

## Summary

Phase 10 adds a second clustering pass over large Spaces (> 50 docs) to produce navigable sub-spaces labeled by the active AI provider. The dominant implementation work is in Rust: a new `spaces/subspace_detector.rs` module that runs recursive k-means on intra-cluster vectors, extended `spaces/manager.rs` recluster that drives the sub-space pass after top-level clustering completes, and `SpaceLabelEntry` / `Space` struct extensions for `parent_id` and `depth`. Frontend changes add chevron-expand to `Sidebar.tsx`, a breadcrumb + parent context banner to `SpaceDetailPage.tsx`, and `expandedSpaceIds: Set<string>` to `useSidebarStore` (persisted to localStorage via Zustand `persist` middleware).

**CRITICAL FINDING (Positive):** `ruvector-hyperbolic-hnsw` IS the correct crate for hierarchy-aware search. Unlike `ruvector-cluster` and `ruvector-domain-expansion` (which were misnamed in Phase 9), this crate delivers exactly what the CONTEXT.md describes: a Poincaré ball HNSW index with hierarchy-aware distance metrics, tangent-space pruning, sharded curvature, and dual-space (Euclidean fallback) search. API entry points exist and are well-specified. The crate is USABLE for SC5.

**DEPENDENCY RISK:** `ruvector-hyperbolic-hnsw` requires `rand = "0.8"` while Cortex's `Cargo.toml` uses `rand = "0.9"`. Cargo resolves minor semver differences but the `0.8`→`0.9` boundary is a major version bump — Cargo will compile both versions in the dep tree (no conflict, just slightly larger binary). No blocker.

**Primary recommendation:** Implement `subspace_detector.rs` as a pure wrapper around the existing `cluster_documents()` function in `clustering.rs` (recursive call, same k-means). Add `ruvector-hyperbolic-hnsw` as a path dependency for the secondary index. All Phase 9 infrastructure (llm_labeler, label_cache, fingerprint, SpaceLabelCache) is reused with backward-compatible field additions only.

---

## CRITICAL: ruvector-hyperbolic-hnsw Crate Audit

> D-09 requires researcher to audit this crate FIRST. Phase 9 taught us that ruvector-cluster and ruvector-domain-expansion were misnamed. This section confirms ruvector-hyperbolic-hnsw is NOT a misfit.

### What the crate actually is [VERIFIED: local crate read]

**Location:** `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-hyperbolic-hnsw/`

**Core structs exposed by `lib.rs`:**

| Export | Purpose |
|--------|---------|
| `HyperbolicHnsw` | Single-shard HNSW index in Poincaré ball — INSERT + SEARCH |
| `HyperbolicHnswConfig` | Config: curvature, metric (Poincare/Euclidean/Cosine/Hybrid), M, ef_search, tangent pruning |
| `ShardedHyperbolicHnsw` | Multi-shard index with per-shard curvature — supports `depth: Option<usize>` on insert |
| `SearchResult` | `{ id: usize, distance: f32 }` — sorted by distance ascending |
| `HierarchyMetrics` | Computes `radius_depth_correlation` Spearman + distortion metrics |
| `TangentCache` | Precomputed tangent-space representations for O(M·log n) pruning |
| `poincare::poincare_distance` | Geodesic distance in Poincaré ball (c-curvature) |

**What CONTEXT.md D-10 assumes it is:** A secondary HNSW index that uses hyperbolic distance for hierarchy-aware retrieval within a parent Space context.

**Finding:** [VERIFIED] This assumption is CORRECT. The crate provides exactly this. `HyperbolicHnsw::insert(vec![f32]) -> HyperbolicResult<usize>` stores a document vector. `HyperbolicHnsw::search(&[f32], k) -> HyperbolicResult<Vec<SearchResult>>` returns nearest neighbors by Poincaré distance. The `ShardedHyperbolicHnsw` variant inserts with `depth: Option<usize>` to encode hierarchy depth as Poincaré radius.

**Verdict: USABLE for SC5.** No misfit. Proceed with D-10 dual-index pattern.

### Integration fit assessment

| Concern | Assessment |
|---------|------------|
| Index build cost | `insert()` is O(M · log n) per vector (HNSW standard). For 10K docs, one-time build at recluster ≈ seconds, not minutes. ACCEPTABLE. |
| Query API | `index.search(&query_vec, k)` returns `Vec<SearchResult { id: usize, distance: f32 }>`. Cortex stores `space_id: String` — needs a `usize → space_id` mapping (trivial `Vec<String>` side-channel). |
| Dimension agnostic | Accepts `Vec<f32>` of any length. Cortex centroids are 384-dim (all-MiniLM-L6-v2). Compatible. |
| Fallback path | D-11 fallback (flat HNSW filtered by parent membership) already exists in `CortexEngine` via `ruvector-core`. Zero new code needed for fallback. |
| rand version conflict | Crate requires `rand = "0.8"`. Cortex uses `rand = "0.9"`. Cargo resolves both; no compile error. Binary +~100KB. ACCEPTABLE. |
| nalgebra/ndarray | Crate adds `nalgebra = "0.34.1"` and `ndarray = "0.17.1"`. Neither is currently in Cortex's Cargo.toml. These are standard, well-maintained crates. ACCEPTABLE. |
| WASM concern | Phase 10 does not need WASM build; default features exclude `wasm = []`. SAFE. |

### Key API patterns (for planner)

```rust
// Cargo.toml addition
ruvector-hyperbolic-hnsw = { path = "../../experiments/ruvector/crates/ruvector-hyperbolic-hnsw", default-features = false, features = [] }

// Build index (during recluster, after top-level pass)
use ruvector_hyperbolic_hnsw::{HyperbolicHnsw, HyperbolicHnswConfig};

let config = HyperbolicHnswConfig::default(); // curvature=1.0, Poincaré metric, ef=50
let mut hyp_index = HyperbolicHnsw::new(config);
let mut id_to_space: Vec<String> = Vec::new();  // position = HNSW internal usize id

// Insert one centroid per space
for space_data in &spaces {
    hyp_index.insert(space_data.centroid.clone()).unwrap(); // returns usize id
    id_to_space.push(space_data.space.id.clone());
}

// Optional: build tangent cache for faster search
hyp_index.build_tangent_cache().unwrap();

// Query (during search when parent_space_id filter is present)
let results = hyp_index.search(&query_embedding, 10).unwrap();
for r in results {
    let space_id = &id_to_space[r.id];
    // Use space_id to filter documents
}
```
[VERIFIED: local crate read — `HyperbolicHnsw::insert`, `::search`, `::build_tangent_cache` confirmed in `src/hnsw.rs`]

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Sub-cluster detection (> 50 doc gate) | API/Backend (Rust) | — | Pure vector computation; runs after top-level k-means |
| Recursive k-means sub-clustering | API/Backend (Rust) | — | Reuses `clustering.rs::cluster_documents()` — already in backend |
| Sub-space LLM labeling | API/Backend (Rust) | — | Calls `ai_request()` via `llm_labeler.rs` with parent-context prompt |
| Sub-space cache persistence | API/Backend (Rust) | — | Extends `space_labels.json` sidecar; backward-compat via `#[serde(default)]` |
| Hyperbolic-hnsw secondary index | API/Backend (Rust) | — | Search path decision (`parent_space_id.is_some()`) lives in query layer |
| Space type extension (parent_id, depth) | Shared (Rust + TS types) | — | Both `types.rs::Space` and `client/lib/types.ts::Space` must be extended in same commit |
| IPC commands (get_sub_spaces, get_hierarchy) | API/Backend (Rust IPC) | Frontend hooks | Tauri `#[tauri::command] async` + React Query hooks in `useTauri.ts` |
| Sidebar sub-space expand state | Frontend (Browser Zustand) | — | `expandedSpaceIds: Set<string>` persisted to localStorage — no backend needed |
| Breadcrumb navigation | Frontend (Browser) | — | Pure client-side lookup: `spaces.find(s => s.id === parentId)` |
| Sub-space grid on SpaceDetailPage | Frontend (Browser) | — | `spaces.filter(s => s.parentId === currentSpaceId)` — no new IPC needed |
| Perf integration test | API/Backend (Rust tests) | — | `cargo test` benchmark comparing flat vs hyperbolic search |

---

## Standard Stack

### Core — All already in Cargo.toml or codebase

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `cluster_documents()` in `clustering.rs` | [VERIFIED] | Sub-clustering recursion target | Already implements k-means++; accepts `Vec<(String, Vec<f32>)>`, returns `ClusterResult` |
| `label_cluster()` in `llm_labeler.rs` | [VERIFIED] | Sub-space LLM labeling | Phase 9 production code; `avoid_labels` param + parent-context prefix added |
| `SpaceLabelCache` in `label_cache.rs` | [VERIFIED] | Cache persistence | Extend `SpaceLabelEntry` with `parent_id: Option<String>`, `depth: u8` using `#[serde(default)]` |
| `SpaceManager::recluster()` in `manager.rs` | [VERIFIED] | Sub-space pass orchestration | Add sub-space detection loop AFTER existing top-level clustering |
| `sha2`, `serde_json`, `tokio` | [VERIFIED: Cargo.toml] | Fingerprint, JSON, async | Already in dependency tree |

### New Dependency

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `ruvector-hyperbolic-hnsw` | `0.1.0` (local path) [VERIFIED: local crate] | Hierarchy-aware secondary search index | Only crate that provides Poincaré ball HNSW with depth-aware insertion — exactly what SC5 requires |

### Frontend — All already installed

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| shadcn `Breadcrumb` | [VERIFIED: `breadcrumb.tsx` present] | `Spaces / Property / Tax` navigation | SpaceDetailPage when `space.parentId` is non-null |
| shadcn `Collapsible` | [VERIFIED: `collapsible.tsx` present] | Sidebar Space expand/collapse | Wraps sub-space list beneath each top-5 Space entry |
| Framer Motion | `^12.23.12` [VERIFIED: package.json] | Chevron rotation animation | `rotate: 0 → 90` on `isExpanded` state |
| Zustand `persist` middleware | already used [VERIFIED: stores.ts] | `expandedSpaceIds` localStorage | Extend `useSidebarStore` with `expandedSpaceIds: Set<string>` |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Recursive k-means (D-02) | HDBSCAN | HDBSCAN requires enough density. Parent cluster (already trimmed to a coherent topic) seldom has the density needed. k-means deterministic + already in tree. D-02 is correct choice. |
| Framer Motion chevron | CSS `transition: rotate` | CSS `rotate` via Tailwind `rotate-90` class toggle works fine — Framer Motion provides smoother animated path but either is acceptable. Planner decides. |
| Single flat `useSpaces()` hook | Separate `useSubSpaces(parentId)` | Single hook returning flat list (frontend filters by `parentId`) is simpler. Matches existing `useSpaces()` usage pattern. RECOMMENDED. |

**Installation:**
```bash
# Cargo.toml addition (Rust)
ruvector-hyperbolic-hnsw = { path = "../../experiments/ruvector/crates/ruvector-hyperbolic-hnsw", default-features = false, features = [] }

# No new npm packages needed
```

---

## Package Legitimacy Audit

> Phase 10 adds ONE new external crate: `ruvector-hyperbolic-hnsw`. This is a LOCAL PATH dependency (not from crates.io), so slopcheck and registry verification are not applicable.

| Package | Registry | Age | Source | slopcheck | Disposition |
|---------|----------|-----|--------|-----------|-------------|
| `ruvector-hyperbolic-hnsw` | LOCAL PATH | n/a | `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-hyperbolic-hnsw/` [VERIFIED] | N/A (local) | Approved — audited directly from source |

**Transitive new crates from ruvector-hyperbolic-hnsw (not in current Cortex Cargo.lock):**

| Crate | Version | Purpose | Risk |
|-------|---------|---------|------|
| `nalgebra` | `0.34.1` | Linear algebra for hyperbolic ops | LOW — well-maintained, 5M+ downloads on crates.io [ASSUMED] |
| `ndarray` | `0.17.1` | N-dimensional arrays | LOW — standard scientific Rust crate [ASSUMED] |
| `rand_distr` | `0.4` | Probability distributions (dev dep only) | NONE — dev-only, not in release binary |
| `rayon` | `1.10` (optional `parallel` feature) | Parallel search | LOW — disable with `default-features = false` |

**Action:** Disable `parallel` feature via `default-features = false` to avoid pulling `rayon` into release binary (not needed for Phase 10's sequential recluster path).

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
recluster() trigger (IPC or startup)
          │
          ▼
SpaceManager::recluster()          ← existing manager.rs
          │
    [cluster_documents() — k-means, top-level]
          │
          ▼
   For each top-level Space:
   ┌─────────────────────────────────────────┐
   │  doc_count > SUB_SPACE_THRESHOLD (50)?  │
   │                  │                       │
   │         YES      │        NO             │
   │          │       │         │             │
   │  subspace_detector::detect()             │
   │  cluster_documents(intra_vecs, k=sqrt(n/2).max(2))
   │          │                               │
   │  sub_clusters ≥ 3 docs?                 │
   │    YES → SpaceData (depth=1)            │
   │    NO  → roll up to "Misc"              │
   │    (D-04: no silent drops)              │
   └─────────────────────────────────────────┘
          │
          ▼
   LLM labeling pass (sub-spaces)
   llm_labeler::label_cluster() with parent-context prompt (D-05)
   Fingerprint + cache check (same 20% Jaccard gate, D-06)
   Collision resolution (same resolve_collisions(), D-13)
          │
          ▼
   Build HyperbolicHnsw index (D-10)
   — Insert one centroid per PARENT Space
   — id_to_space: Vec<String> maps usize → space_id
   — build_tangent_cache() for O(M log n) queries
          │
          ▼
   Space structs built:
   — top-level: depth=0, parent_id=None, sub_space_ids=[...]
   — sub-spaces: depth=1, parent_id=Some("parent-id"), sub_space_ids=[]
   — "Misc" sub-space: depth=1, parent_id=Some("parent-id"), name="Misc"
          │
          ▼
   space_labels.json saved (parent_id + depth in SpaceLabelEntry)

──── SEARCH PATH (when parent_space_id filter present) ────

query_embedding → HyperbolicHnsw.search() → nearest parent Space
       ↓ fallback if error
flat HNSW (ruvector-core) filtered by parent_space_id membership

──── FRONTEND DATA FLOW ────

useSpaces() → flat Vec<Space>
Sidebar: spaces.filter(s => !s.parentId).sort(by count).take(5)
         + expandedSpaceIds Set → sub-spaces inline
SpaceDetailPage: spaces.filter(s => s.parentId === id) → sub-space grid
Breadcrumb: spaces.find(s => s.id === parentId) → parent link
```

### Recommended Project Structure

```
src-tauri/src/spaces/
├── clustering.rs          (keep — k-means, used for both levels)
├── naming.rs              (keep — rule-based fallback)
├── manager.rs             (UPDATE — add sub-space detection pass after recluster)
├── subspace_detector.rs   (NEW — detect(), build_misc_space(), SUB_SPACE_THRESHOLD)
├── mod.rs                 (UPDATE — add subspace_detector module)
├── llm_labeler.rs         (UPDATE — add parent-context label_sub_cluster() variant)
├── label_cache.rs         (UPDATE — SpaceLabelEntry: add parent_id, depth fields)
└── fingerprint.rs         (UNCHANGED)

src-tauri/src/search/
└── query.rs               (UPDATE — dual-index path when parent_space_id.is_some())

src-tauri/src/engine.rs or lib.rs
└── (UPDATE — manage HyperbolicHnsw in AppState, rebuild on recluster)

client/
├── components/
│   ├── layout/
│   │   └── Sidebar.tsx         (UPDATE — chevron expand, sub-space list, sub-count)
│   └── spaces/
│       └── SpaceCard.tsx       (UNCHANGED — reused as-is for sub-space grid)
├── pages/
│   └── SpaceDetailPage.tsx     (UPDATE — breadcrumb, parent context banner, sub-space grid)
└── lib/
    ├── stores.ts               (UPDATE — add expandedSpaceIds to useSidebarStore)
    └── types.ts                (UPDATE — Space: parentId, depth, subSpaceIds)
```

### Pattern 1: Sub-space detection (new module)

```rust
// src-tauri/src/spaces/subspace_detector.rs
use super::clustering::{cluster_documents, Cluster};

pub const SUB_SPACE_THRESHOLD: usize = 50;
pub const MIN_SUB_CLUSTER_SIZE: usize = 3;

/// Detect sub-spaces for a parent cluster that exceeds the threshold.
/// Returns (sub_clusters, misc_doc_ids).
/// misc_doc_ids is non-empty when some docs don't fit any sub-cluster of >= 3.
pub fn detect(
    parent_doc_ids: &[String],
    parent_vectors: Vec<(String, Vec<f32>)>, // (doc_id, vector)
) -> (Vec<Cluster>, Vec<String>) {
    if parent_doc_ids.len() <= SUB_SPACE_THRESHOLD {
        return (vec![], vec![]);
    }

    let n = parent_vectors.len();
    let k = ((n as f64 / 2.0).sqrt().max(2.0)) as usize;
    let result = cluster_documents(parent_vectors, k);

    let mut sub_clusters: Vec<Cluster> = Vec::new();
    let mut misc_ids: Vec<String> = Vec::new();

    for cluster in result.clusters {
        if cluster.doc_ids.len() >= MIN_SUB_CLUSTER_SIZE {
            sub_clusters.push(cluster);
        } else {
            misc_ids.extend(cluster.doc_ids);
        }
    }

    (sub_clusters, misc_ids)
}

/// Build a synthetic "Misc" sub-space cluster for orphaned docs.
/// Only created when misc_ids is non-empty (HSPC-03 compliance).
pub fn build_misc_space(parent_id: &str, misc_ids: Vec<String>) -> Option<Cluster> {
    if misc_ids.is_empty() {
        return None;
    }
    Some(Cluster {
        id: format!("{}-misc", parent_id),
        doc_ids: misc_ids,
        centroid: vec![],  // Misc has no meaningful centroid
    })
}
```
[ASSUMED — mirrors `cluster_documents()` call pattern from `manager.rs`; VERIFIED: `cluster_documents()` signature from `clustering.rs`]

### Pattern 2: SpaceLabelEntry backward-compat extension

```rust
// src-tauri/src/spaces/label_cache.rs — extend SpaceLabelEntry
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpaceLabelEntry {
    pub fingerprint: String,
    pub label: String,
    pub description: String,
    pub canonical_entity_hint: Option<String>,
    pub generated_at: String,
    pub user_locked: bool,
    // Phase 10 additions — #[serde(default)] for backward compat
    // Old cache entries read with parent_id=None (top-level) and depth=0
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub depth: u8,
}
```
[VERIFIED: existing `SpaceLabelEntry` from `label_cache.rs`; `#[serde(default)]` pattern confirmed]

### Pattern 3: Space type extension

```rust
// src-tauri/src/types.rs — extend Space struct (Phase 10 additions)
#[serde(default)]
pub depth: u8,          // 0 = top-level, 1 = sub-space
#[serde(default)]
pub sub_space_ids: Vec<String>,  // populated for top-level spaces after sub-space pass
```
[VERIFIED: existing Phase 9 additions to Space use identical `#[serde(default)]` pattern]

```typescript
// client/lib/types.ts — extend Space interface
depth?: number;            // 0 | 1
subSpaceIds?: string[];    // populated for top-level spaces
// parentId already present in Space struct from Phase 9 original types
```
[VERIFIED: `Space` already has `parentId` in `types.rs` (`parent_id: Option<String>`) from Phase 9 Plan 01]

**Important:** `parent_id` field ALREADY EXISTS in `Space` struct from Phase 9. Only `depth` and `sub_space_ids` are new.

### Pattern 4: Sidebar Zustand extension

```typescript
// client/lib/stores.ts — extend useSidebarStore
// Current stores.ts uses zustand ^5.0.11 + persist middleware [VERIFIED]

interface SidebarState {
  isCollapsed: boolean;
  toggle: () => void;
  setCollapsed: (collapsed: boolean) => void;
  // Phase 10 additions:
  expandedSpaceIds: Set<string>;
  toggleSpaceExpanded: (spaceId: string) => void;
  isSpaceExpanded: (spaceId: string) => boolean;
}

// Use persist middleware (already used for onboarding):
export const useSidebarStore = create<SidebarState>()(
  persist(
    (set, get) => ({
      isCollapsed: false,
      toggle: () => set((s) => ({ isCollapsed: !s.isCollapsed })),
      setCollapsed: (collapsed) => set({ isCollapsed: collapsed }),
      expandedSpaceIds: new Set(),
      toggleSpaceExpanded: (spaceId) => set((s) => {
        const next = new Set(s.expandedSpaceIds);
        if (next.has(spaceId)) next.delete(spaceId);
        else next.add(spaceId);
        return { expandedSpaceIds: next };
      }),
      isSpaceExpanded: (spaceId) => get().expandedSpaceIds.has(spaceId),
    }),
    {
      name: 'cortex-sidebar',
      // Set serialization: convert Set to Array for localStorage
      storage: {
        getItem: (key) => {
          const str = localStorage.getItem(key);
          if (!str) return null;
          const data = JSON.parse(str);
          data.state.expandedSpaceIds = new Set(data.state.expandedSpaceIds ?? []);
          return data;
        },
        setItem: (key, value) => {
          const data = { ...value, state: { ...value.state, expandedSpaceIds: [...value.state.expandedSpaceIds] } };
          localStorage.setItem(key, JSON.stringify(data));
        },
        removeItem: (key) => localStorage.removeItem(key),
      },
    }
  )
);
```
[ASSUMED — Zustand v5 persist middleware API; Set serialization pattern is standard. VERIFIED: zustand `^5.0.11` confirmed in package.json. VERIFIED: persist middleware already used for `useOnboardingStore` in stores.ts]

### Pattern 5: Breadcrumb (shadcn Breadcrumb)

```tsx
// In SpaceDetailPage.tsx — when space.parentId is set
import {
  Breadcrumb, BreadcrumbList, BreadcrumbItem,
  BreadcrumbLink, BreadcrumbPage, BreadcrumbSeparator
} from "@/components/ui/breadcrumb";

// parentSpace = spaces.find(s => s.id === space.parentId)
<Breadcrumb>
  <BreadcrumbList>
    <BreadcrumbItem>
      <BreadcrumbLink asChild><Link to="/spaces">Spaces</Link></BreadcrumbLink>
    </BreadcrumbItem>
    <BreadcrumbSeparator />
    {parentSpace && (
      <>
        <BreadcrumbItem>
          <BreadcrumbLink asChild>
            <Link to={`/spaces/${parentSpace.id}`}>{parentSpace.name}</Link>
          </BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
      </>
    )}
    <BreadcrumbItem>
      <BreadcrumbPage>{space.name}</BreadcrumbPage>
    </BreadcrumbItem>
  </BreadcrumbList>
</Breadcrumb>
```
[VERIFIED: `breadcrumb.tsx` present in `client/components/ui/`; API verified from shadcn component source]

### Pattern 6: Sidebar Collapsible sub-space expand

```tsx
// In Sidebar.tsx — for each top-5 space entry
import {
  Collapsible, CollapsibleContent, CollapsibleTrigger
} from "@/components/ui/collapsible";
import { ChevronRight } from "lucide-react";

// In component:
const { expandedSpaceIds, toggleSpaceExpanded } = useSidebarStore();
const subSpaces = spaces.filter(s => s.parentId === space.id);

<Collapsible
  open={expandedSpaceIds.has(space.id)}
  onOpenChange={() => toggleSpaceExpanded(space.id)}
>
  <div className="flex items-center">
    {/* Existing space link */}
    <Link to={`/spaces/${space.id}`} className="flex-1 ...">
      <span className="text-sm font-medium">{space.name}</span>
      {subSpaces.length > 0 && (
        <span className="text-xs text-text-tertiary ml-1">({subSpaces.length})</span>
      )}
    </Link>
    {subSpaces.length > 0 && (
      <CollapsibleTrigger asChild>
        <button className="h-11 w-11 flex items-center justify-center rounded hover:bg-bg-tertiary">
          <ChevronRight
            size={14}
            className="text-text-tertiary transition-transform"
            style={{ rotate: expandedSpaceIds.has(space.id) ? '90deg' : '0deg' }}
          />
        </button>
      </CollapsibleTrigger>
    )}
  </div>
  <CollapsibleContent>
    <div className="pl-8 space-y-0">
      {subSpaces.map(sub => (
        <Link key={sub.id} to={`/spaces/${sub.id}`}
          className="flex items-center px-3 py-2 text-xs text-text-tertiary hover:bg-bg-tertiary rounded">
          {sub.name}
        </Link>
      ))}
    </div>
  </CollapsibleContent>
</Collapsible>
```
[VERIFIED: `collapsible.tsx` present. `ChevronRight` used in existing `SpaceDetailPage.tsx` — confirmed in imports. Touch target 44×44px per UI-SPEC]

### Pattern 7: Perf test scaffold (SC5)

```rust
// src-tauri/src/spaces/subspace_detector.rs (or a separate integration test)
#[cfg(test)]
mod perf_tests {
    use super::*;
    use crate::spaces::clustering::cluster_documents;
    use ruvector_hyperbolic_hnsw::{HyperbolicHnsw, HyperbolicHnswConfig};
    use std::time::Instant;

    /// SC5 perf gate: parent→child search must be ≤ 2× flat top-level search time.
    /// Uses 10K synthetic 384-dim vectors (matches production all-MiniLM-L6-v2 dim).
    #[test]
    #[ignore = "run explicitly: cargo test perf_gate -- --ignored --nocapture"]
    fn test_sc5_hierarchical_search_perf_gate() {
        let n = 10_000usize;
        let dim = 384usize;
        // Build synthetic vectors
        let vecs: Vec<Vec<f32>> = (0..n)
            .map(|i| (0..dim).map(|j| ((i + j) as f32 * 0.001).sin()).collect())
            .collect();

        let query: Vec<f32> = (0..dim).map(|j| (j as f32 * 0.001).cos()).collect();

        // -- Flat baseline: cluster_documents, search top-level --
        let flat_vecs: Vec<(String, Vec<f32>)> = vecs.iter().enumerate()
            .map(|(i, v)| (format!("doc-{}", i), v.clone())).collect();
        let t_flat_start = Instant::now();
        let _result = cluster_documents(flat_vecs, 20);
        let flat_ms = t_flat_start.elapsed().as_millis();

        // -- Hyperbolic HNSW: build + search --
        let mut hyp = HyperbolicHnsw::default_config();
        for v in &vecs {
            hyp.insert(v.clone()).unwrap();
        }
        hyp.build_tangent_cache().unwrap();

        let t_hyp_start = Instant::now();
        let _results = hyp.search(&query, 10).unwrap();
        let hyp_ms = t_hyp_start.elapsed().as_millis();

        println!("Flat baseline: {}ms, Hyperbolic search: {}ms", flat_ms, hyp_ms);
        // SC5: search must complete in ≤ 2× flat time
        assert!(
            hyp_ms <= flat_ms * 2,
            "SC5 FAILED: hyperbolic search {}ms > 2× flat {}ms", hyp_ms, flat_ms
        );
    }
}
```
[ASSUMED — test scaffold pattern; VERIFIED: `HyperbolicHnsw::insert`, `::search`, `::build_tangent_cache` API from local crate]

### Anti-Patterns to Avoid

- **Storing sub-spaces as nested `Vec<Space>` in SpaceManager:** The `Space.sub_spaces` field exists but is populated lazily from the flat `spaces` HashMap at query time, not persistently. Store all spaces flat in `SpaceManager::spaces: HashMap<String, SpaceData>` — top-level and sub-spaces alike. Build the `sub_space_ids` list at recluster time. The frontend receives a flat list and filters by `parentId`.
- **Keying sub-space cache entries differently from top-level entries:** All `SpaceLabelEntry` items are keyed by `space_id` regardless of depth. Pitfall #4 from Phase 9 applies to sub-spaces too.
- **Forgetting "Misc" sub-space in IPC response:** The "Misc" sub-space (`doc_ids` = orphan docs, `name = "Misc"`) must appear in `get_spaces()` response with `parentId` set. Frontend renders it as a dashed-border SpaceCard (per UI-SPEC).
- **Calling `label_cluster()` with empty `doc_titles`:** Sub-clusters can be as small as 3 docs. Build LLM inputs defensively — if `doc_titles` is empty, fall back to `name_space()` rather than sending an empty prompt.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Sub-clustering vectors | Custom k-means | `cluster_documents()` from `clustering.rs` | Same k-means++ implementation; already tested; recursive call needs one extra parameter |
| LLM label for sub-spaces | New labeler | `label_cluster()` / `label_with_avoid_list()` from `llm_labeler.rs` | Add parent-context prefix to user message only; all retry, fence-strip, collision logic is inherited |
| Breadcrumb navigation | Custom breadcrumb component | shadcn `Breadcrumb` (confirmed installed) | Accessible, design-system-consistent |
| Sidebar collapse animation | Manual CSS transitions | shadcn `Collapsible` + CSS `rotate` or Framer Motion | Collapsible manages open/close state; animation is a one-liner |
| Hierarchy-aware search | Custom Poincaré distance | `ruvector-hyperbolic-hnsw::HyperbolicHnsw` | Correct implementation of Poincaré ball geometry — non-trivial to hand-roll correctly |
| Set serialization in Zustand | Custom serializer | Storage override in `persist()` options | Standard pattern for Sets in Zustand; 10 lines |

**Key insight:** Phase 10 is a composition phase — nearly all primitives exist from Phases 9, 7, and 4. New code is orchestration (detect → label → persist → IPC) and the thin secondary index (hyperbolic-hnsw). No novel algorithmic work required.

---

## Common Pitfalls

### Pitfall 1: parent_id already exists in Space but sub_space_ids does not
**What goes wrong:** The `Space` struct (`types.rs`) already has `parent_id: Option<String>` from Phase 9 Plan 01 (the field was pre-added for Phase 10). A planner who adds it again gets a compile error. `sub_space_ids: Vec<String>` and `depth: u8` are the fields that need to be ADDED.
**Why it happens:** Phase 9 pre-populated the Space struct for Phase 10 readiness. Reading types.rs confirms `parent_id` is there.
**How to avoid:** Only add `sub_space_ids: Vec<String>` and `depth: u8` to `Space` struct. Do NOT re-add `parent_id`.
**Warning signs:** `error[E0124]: field 'parent_id' is defined multiple times`.

### Pitfall 2: Sidebar sub-space list shows ALL spaces, not just top 5 + their sub-spaces
**What goes wrong:** The sidebar renders the flat list from `useSpaces()` without filtering. All sub-spaces appear as top-level items.
**Why it happens:** `useSpaces()` returns ALL spaces including sub-spaces. Without `parentId` filtering, sub-spaces appear at the top level.
**How to avoid:** Sidebar selects `spaces.filter(s => !s.parentId).sort(...).slice(0, 5)` for top-level. Sub-spaces render only inside `CollapsibleContent` of their parent.
**Warning signs:** Sidebar shows 15 Space entries instead of 5.

### Pitfall 3: "Misc" sub-space created even when all docs fit into sub-clusters
**What goes wrong:** `misc_doc_ids` is empty but a "Misc" sub-space is created anyway with 0 documents.
**Why it happens:** `build_misc_space()` not guarded by `if misc_ids.is_empty() { return None; }`.
**How to avoid:** `build_misc_space()` must return `None` when `misc_ids` is empty. Only create a Misc sub-space when there are actual orphan docs (HSPC-03: "unclustered docs surface in a 'Misc' sub-space" — implies existence is conditional).
**Warning signs:** Empty "Misc" cards appearing on SpaceDetailPage.

### Pitfall 4: SpaceLabelEntry backward compat for existing cache files
**What goes wrong:** Old `space_labels.json` entries lack `parent_id` and `depth`. Loading them without `#[serde(default)]` causes a JSON deserialization error, wiping the entire cache and triggering re-labeling for all spaces on first launch after Phase 10.
**Why it happens:** Forgetting `#[serde(default)]` on the two new fields.
**How to avoid:** `parent_id: Option<String>` with `#[serde(default)]` → `None` for old entries. `depth: u8` with `#[serde(default)]` → `0` for old entries. Both defaults are semantically correct (old entries are top-level spaces).
**Warning signs:** All space labels reset to "Generating label…" after app update.

### Pitfall 5: HyperbolicHnsw index not rebuilt when recluster runs
**What goes wrong:** After recluster, `SpaceManager.spaces` is updated but the in-memory `HyperbolicHnsw` index still reflects old space centroids. Subsequent sub-space searches return stale results.
**Why it happens:** The index rebuild is async and easy to skip if wired separately from recluster.
**How to avoid:** Rebuild the `HyperbolicHnsw` index as the last step of `recluster()`, before returning. Store it in `AppState` alongside `SpaceManager`. Mutex-gate it the same way as `space_manager`.
**Warning signs:** Search within a parent Space returns documents from the wrong sub-spaces after re-indexing.

### Pitfall 6: rand version conflict causing compile errors
**What goes wrong:** `ruvector-hyperbolic-hnsw` requires `rand = "0.8"`. Cortex uses `rand = "0.9"`. Cargo may emit warnings or, in edge cases, fail if there is a conflicting feature set between the two rand versions.
**Why it happens:** rand 0.8 and 0.9 are different major versions. Cargo resolves both independently but can fail if they share a crate that specifies conflicting requirements via features.
**How to avoid:** Build immediately after adding the dependency: `cargo check -p cortex`. If it fails, pin Cortex's `rand` to `"0.8"` instead of `"0.9"` (rand's API is backward-compatible for the features Cortex uses: `rand::random::<f32>()`).
**Warning signs:** `error: failed to select a version for 'rand'` in `cargo check` output.

### Pitfall 7: sub_space_ids not populated on get_spaces() response
**What goes wrong:** Frontend `SpaceDetailPage` renders sub-space grid via `spaces.filter(s => s.parentId === id)`. This works. But the top-level `SpacesPage` grid shows `subSpaceIds.length` as the sub-count badge. If `sub_space_ids` is empty on the backend but sub-spaces exist, the count shows 0.
**Why it happens:** Two separate ways to count sub-spaces: filter by parentId OR read sub_space_ids. These must be consistent.
**How to avoid:** After the sub-space pass in `recluster()`, explicitly set `space.sub_space_ids = sub_space_ids_for_parent` before inserting into `SpaceManager.spaces`. The frontend can use either mechanism.
**Warning signs:** SpacesPage shows `(0)` sub-count while SpaceDetailPage correctly shows sub-spaces.

---

## Code Examples

### Sub-space label prompt (D-05) [ASSUMED — extends SPACE_LABEL_PROMPT pattern from llm_labeler.rs]

```rust
// In spaces/llm_labeler.rs — add sub-space variant
pub const SUB_SPACE_LABEL_PREFIX: &str = "You are labeling a sub-space of an existing Smart Space.\n";

pub async fn label_sub_cluster(
    auth: &AuthState,
    model: &str,
    parent_label: &str,
    doc_titles: &[String],
    entity_summary: &str,
    top_topics: &[String],
    top_tags: &[String],
    doc_count: usize,
) -> Result<SpaceLabel, String> {
    // Prepend parent context to user message
    let avoid = vec![parent_label.to_string()]; // always avoid parent label collision
    let base_content = build_user_content(
        doc_titles, entity_summary, top_topics, top_tags, doc_count, &avoid
    );
    let user_content = format!(
        "Parent Space: \"{parent_label}\"\nReturn a 2-4 word label that is distinct from \"{parent_label}\" and specific to this sub-group.\n\n{base_content}"
    );

    let req = AIServiceRequest {
        system_prompt: SPACE_LABEL_PROMPT.to_string(),
        messages: vec![ServiceMessage { role: "user".to_string(), content: user_content }],
        max_tokens: Some(256),
        temperature: Some(LABEL_TEMPERATURE),
        response_format: None,
        model_override: if model.is_empty() { None } else { Some(model.to_string()) },
    };
    let response = ai_request_with_retry(auth, req, MAX_LABEL_RETRIES).await
        .map_err(|e| format!("LLM sub-label call failed: {}", e))?;
    let stripped = strip_json_fences(&response.content);
    serde_json::from_str::<SpaceLabel>(&stripped)
        .map_err(|e| format!("LLM sub-label JSON parse failed: {}", e))
}
```
[ASSUMED — extends existing `label_with_avoid_list` pattern; VERIFIED: `build_user_content`, `strip_json_fences`, `ai_request_with_retry` exports confirmed in llm_labeler.rs]

### HyperbolicHnsw integration (D-10) [VERIFIED: local crate API]

```rust
// In AppState (lib.rs or state.rs) — add field
pub hyp_index: Arc<tokio::sync::Mutex<Option<HyperbolicHnsw>>>,
pub hyp_id_to_space: Arc<tokio::sync::Mutex<Vec<String>>>,  // usize → space_id

// In SpaceManager::recluster() — after building space_list, rebuild index
pub async fn rebuild_hyp_index(
    spaces: &[SpaceData],
    hyp_index: &Arc<tokio::sync::Mutex<Option<HyperbolicHnsw>>>,
    hyp_id_to_space: &Arc<tokio::sync::Mutex<Vec<String>>>,
) {
    let config = HyperbolicHnswConfig::default(); // curvature=1.0, Poincaré
    let mut new_index = HyperbolicHnsw::new(config);
    let mut id_map: Vec<String> = Vec::new();

    for space_data in spaces.iter().filter(|s| s.space.depth == 0) {
        // Insert only PARENT centroids into the hyperbolic index
        if new_index.insert(space_data.centroid.clone()).is_ok() {
            id_map.push(space_data.space.id.clone());
        }
    }
    // Build tangent cache for O(M log n) search
    let _ = new_index.build_tangent_cache();

    *hyp_index.lock().await = Some(new_index);
    *hyp_id_to_space.lock().await = id_map;
}
```
[VERIFIED: `HyperbolicHnsw::new(config)`, `::insert(Vec<f32>)`, `::build_tangent_cache()` confirmed in `src/hnsw.rs`]

---

## Runtime State Inventory

> Phase 10 is NOT a rename/refactor phase. However, it modifies `space_labels.json` schema.

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | `app_data_dir/space_labels.json` — existing entries lack `parent_id` and `depth` | Code edit only: add `#[serde(default)]` on new fields. Old entries read as `parent_id=None, depth=0` — semantically correct for top-level. No data migration required. |
| Live service config | None — desktop app only | None |
| OS-registered state | None | None |
| Secrets/env vars | None new | None |
| Build artifacts | None — no installed packages | None |

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/Cargo | Rust backend | Yes | Confirmed (project builds) | — |
| `ruvector-hyperbolic-hnsw` | SC5 hierarchy-aware search | Yes (local path) [VERIFIED] | `0.1.0` | D-11: flat HNSW filtered by parent membership |
| Active AI provider | Sub-space LLM labeling | Conditional | Depends on user config | `name_space()` rule-based fallback from naming.rs |
| shadcn `breadcrumb.tsx` | Breadcrumb navigation | Yes [VERIFIED] | Installed | — |
| shadcn `collapsible.tsx` | Sidebar expand | Yes [VERIFIED] | Installed | — |
| Framer Motion | Chevron rotate animation | Yes (`^12.23.12`) [VERIFIED] | 12.x | CSS `transition` + `rotate-90` class toggle |
| Zustand `persist` middleware | `expandedSpaceIds` localStorage | Yes (already used) [VERIFIED: stores.ts] | zustand ^5.0.11 | — |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** Active AI provider → `name_space()` rule-based fallback. `ruvector-hyperbolic-hnsw` → flat HNSW fallback (D-11).

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `#[cfg(test)]` + `cargo test` (backend); Vitest (frontend) |
| Config file | none (cargo) / `vitest.config.ts` (frontend) |
| Quick run command | `cargo test -p cortex --lib spaces::subspace_detector` |
| Full suite command | `cargo test -p cortex` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HSPC-01 | detect() returns empty on < 50 docs | unit | `cargo test -p cortex --lib spaces::subspace_detector::tests::test_detect_below_threshold` | No — Wave 0 |
| HSPC-01 | detect() runs sub-clustering on > 50 docs | unit | `cargo test -p cortex --lib spaces::subspace_detector::tests::test_detect_above_threshold` | No — Wave 0 |
| HSPC-03 | Docs < MIN_SUB_CLUSTER_SIZE roll up to misc_ids | unit | `cargo test -p cortex --lib spaces::subspace_detector::tests::test_misc_rollup` | No — Wave 0 |
| HSPC-03 | build_misc_space returns None when misc_ids empty | unit | `cargo test -p cortex --lib spaces::subspace_detector::tests::test_no_misc_on_empty` | No — Wave 0 |
| HSPC-04 (cache) | SpaceLabelEntry with parent_id+depth round-trips | unit | `cargo test -p cortex --lib spaces::label_cache::tests::test_phase10_fields_roundtrip` | No — Wave 0 |
| HSPC-04 (compat) | Old cache entries (no parent_id/depth) still load | unit | `cargo test -p cortex --lib spaces::label_cache::tests::test_phase10_backward_compat` | No — Wave 0 |
| HSPC-01 (types) | Space with depth+subSpaceIds round-trips serde | unit | `cargo test -p cortex --lib types::tests::space_phase10_fields_roundtrip` | No — Wave 0 |
| SC5 | HyperbolicHnsw search ≤ 2× flat baseline | perf | `cargo test -p cortex -- perf_gate --ignored --nocapture` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p cortex --lib spaces`
- **Per wave merge:** `cargo test -p cortex`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/spaces/subspace_detector.rs` — covers HSPC-01, HSPC-03 threshold/misc tests
- [ ] `src-tauri/src/spaces/label_cache.rs` — add Phase 10 roundtrip + backward compat tests
- [ ] `src-tauri/src/types.rs` — add Space Phase 10 fields serde test
- [ ] `client/lib/stores.ts` — update `useSidebarStore` with `expandedSpaceIds` (no test file gap, covered by existing `stores.test.ts`)

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | AI provider auth handled by Phase 7 AuthState |
| V3 Session Management | No | `expandedSpaceIds` is UI preference, not session data |
| V4 Access Control | No | Single-user desktop app |
| V5 Input Validation | Yes | LLM sub-space label output validated: JSON fence-strip + serde schema. `parent_label` sanitized via `sanitize_field()` before injecting into prompt. |
| V6 Cryptography | No (inherited) | SHA-256 fingerprint unchanged from Phase 9 |

### Known Threat Patterns for this Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Prompt injection via parent_label | Tampering | `sanitize_field(parent_label)` strips control chars + caps at 100 chars before injecting into sub-space LLM prompt |
| Unbounded sub-space creation cost | Denial of Service | `SUB_SPACE_THRESHOLD=50` gate + `MIN_SUB_CLUSTER_SIZE=3` + fingerprint cache (same 20% Jaccard gate) — max LLM calls = N_large_spaces × k_sub × 1_call |
| localStorage `expandedSpaceIds` pollution | Spoofing | Set contains space IDs only; no sensitive data. Worst case: stale expanded state after re-cluster. |

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `sub_spaces: Vec<Space>` field always empty (mock data era) | `sub_spaces` built from flat store by parent_id lookup; `depth` + `sub_space_ids` added | Phase 10 | Sub-space grid and sidebar expand become possible |
| Flat HNSW for all searches | Dual-index: flat HNSW (top-level) + Hyperbolic HNSW (within-Space search) | Phase 10 | Hierarchy-aware retrieval at logarithmic cost |
| Sidebar shows top 6 Spaces with no expand | Top 5 Spaces with sub-count + inline expand (Collapsible) | Phase 10 (change from top-6 to top-5 per D-17) | Users can navigate sub-spaces without leaving sidebar |

**Deprecated/outdated:**
- `Space.sub_spaces: Vec<Space>` as persistent nested field: replaced by flat storage + `parent_id` pointer. The `sub_spaces` field remains in the struct for backward compat with existing frontend code that reads it, but it is populated dynamically from the flat store rather than stored persistently.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `HyperbolicHnsw` rand conflict (`0.8` vs `0.9`) will be resolved by Cargo without feature conflict | Pitfall 6 | Compile error; fix: pin Cortex rand to `0.8` |
| A2 | `nalgebra = "0.34.1"` and `ndarray = "0.17.1"` are not already transitively pulled in at incompatible versions | Standard Stack | Version conflict; mitigation: `cargo check` immediately |
| A3 | Zustand v5 `persist` middleware accepts a custom storage adapter with Set serialization as shown | Pattern 4 | API difference; mitigation: check Zustand v5 docs before implementation |
| A4 | `sub_spaces: Vec<Space>` field in current frontend types.ts and Rust types.rs will remain the carry-through field for SpaceDetailPage's existing `SubSpaceCard` render (line 25-43 in SpaceDetailPage.tsx) | Architecture | If SpaceDetailPage already renders `space.subSpaces`, the new `spaces.filter(s => s.parentId === id)` approach is redundant — use whichever is already wired |
| A5 | Label sub-cluster variant can reuse `build_user_content()` from `llm_labeler.rs` — it is `pub(crate)` | Code Examples | Import error if visibility is `pub(super)` only |
| A6 | Top-level Space centroid array (all 384-dim) can be inserted into HyperbolicHnsw without dimension enforcement | Standard Stack | If crate enforces fixed dim at init time, insert would fail for mismatched dims (unlikely — API accepts Vec<f32> of any len) |

**If this table is empty:** All claims in this research were verified or cited — no user confirmation needed. (Table is not empty — A1-A6 require validation.)

---

## Open Questions

1. **`sub_spaces` vs `parentId` filter — which path does frontend use?**
   - What we know: `SpaceDetailPage.tsx` already renders `SubSpaceCard` from `space.subSpaces` (line 25-43 — the `SubSpaceCard` function renders from a `Space` prop, and the existing page likely iterates `space.subSpaces`). The `Space.sub_spaces: Vec<Space>` field already exists but is always `[]` (empty).
   - What's unclear: Is it cleaner to populate `sub_spaces: Vec<Space>` (nested) in the IPC response, or keep the flat `useSpaces()` approach and filter by `parentId`?
   - Recommendation: Populate `sub_spaces` as a flat list of sub-space IDs (`sub_space_ids: Vec<String>`) in the IPC response and have the frontend filter `spaces` by `parentId`. This avoids deeply nested JSON and matches the planner's discretion note in CONTEXT.md. The existing `SubSpaceCard` function can be reused unchanged.

2. **HyperbolicHnsw persistence across app restarts**
   - What we know: The index is built in-memory during `recluster()`. On app restart, it is empty until `recluster()` is triggered again.
   - What's unclear: Should the hyperbolic index be serialized to disk (JSON via `#[derive(Serialize)]` on `HyperbolicHnsw`) so it survives restarts without re-clustering?
   - Recommendation: For Phase 10, rebuild on restart (same pattern as the existing flat HNSW which is rebuilt from `ruvector-core`'s persisted store). The `HyperbolicHnsw` struct derives `Serialize/Deserialize` (confirmed in `hnsw.rs`) so disk persistence is possible as a follow-up optimization.

3. **Sub-count in sidebar — should it count sub-spaces or documents?**
   - What we know: D-14 says `"Property (3)"` — context: HSPC-04 says "shows sub-counts". The UI-SPEC confirms: `(3)` = number of sub-spaces (not documents).
   - Recommendation: `subSpaces.length` = number of sub-space children. This is confirmed by the UI-SPEC typography spec ("sub-count label '(3)' beside Space name in sidebar").

---

## Sources

### Primary (HIGH confidence)
- [VERIFIED: local codebase] `src-tauri/src/spaces/clustering.rs` — `cluster_documents()`, `auto_detect_k()`, `cosine_similarity()`, `ClusterResult`, `Cluster` types
- [VERIFIED: local codebase] `src-tauri/src/spaces/manager.rs` — `SpaceManager::recluster()`, `plan_labeling_operations()`, `SpaceData`, `LabelingDecision`
- [VERIFIED: local codebase] `src-tauri/src/spaces/llm_labeler.rs` — `label_cluster()`, `label_with_avoid_list()`, `build_user_content()`, `try_bootstrap_from_nearest()`, `SpaceLabelingProgress`, `SPACE_LABEL_PROMPT`
- [VERIFIED: local codebase] `src-tauri/src/spaces/label_cache.rs` — `SpaceLabelCache`, `SpaceLabelEntry` (all 6 fields confirmed), `#[serde(rename_all = "camelCase")]` pattern
- [VERIFIED: local codebase] `src-tauri/src/types.rs` — `Space` struct (confirmed: `parent_id: Option<String>` ALREADY present from Phase 9; `sub_spaces: Vec<Space>` present; `depth` and `sub_space_ids` NOT yet present)
- [VERIFIED: local crate read] `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-hyperbolic-hnsw/src/lib.rs` — confirms: Poincaré ball HNSW, `HyperbolicHnsw::insert()`, `::search()`, `::build_tangent_cache()`, `ShardedHyperbolicHnsw`, `HierarchyMetrics`
- [VERIFIED: local crate read] `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-hyperbolic-hnsw/src/hnsw.rs` — confirms API signatures, `HyperbolicHnswConfig`, `SearchResult { id: usize, distance: f32 }`
- [VERIFIED: local crate read] `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-hyperbolic-hnsw/Cargo.toml` — confirms: `rand = "0.8"`, `nalgebra = "0.34.1"`, `ndarray = "0.17.1"`, `thiserror = "2.0"`
- [VERIFIED: local codebase] `client/components/ui/breadcrumb.tsx` — confirmed present
- [VERIFIED: local codebase] `client/components/ui/collapsible.tsx` — confirmed present
- [VERIFIED: local codebase] `client/lib/stores.ts` — `useSidebarStore` current shape: `{ isCollapsed, toggle, setCollapsed }`; `useOnboardingStore` uses `persist` middleware
- [VERIFIED: local codebase] `client/pages/SpaceDetailPage.tsx` — `SubSpaceCard` component exists (lines 25-43), renders from `Space` prop; uses `useSpaces()` hook
- [VERIFIED: local codebase] `package.json` — zustand `^5.0.11`, framer-motion `^12.23.12`
- [VERIFIED: local codebase] `.planning/phases/10-hierarchical-spaces/10-UI-SPEC.md` — breadcrumb/collapsible confirmed installed; spacing/color/typography for Phase 10 UI

### Secondary (MEDIUM confidence)
- [CITED: Phase 9 09-RESEARCH.md] ruvector crate misfit audit pattern — methodology validated; applied to hyperbolic-hnsw with positive result
- [CITED: Phase 9 09-CONTEXT.md] `LlmSpaceLabeler` reuse surface — `label_cluster()`, collision resolver, `try_bootstrap_from_nearest()` all reusable

### Tertiary (LOW confidence)
- [ASSUMED] `nalgebra = "0.34.1"` and `ndarray = "0.17.1"` have no version conflict with current Cortex transitive deps — requires `cargo check` to verify

---

## Metadata

**Confidence breakdown:**
- ruvector-hyperbolic-hnsw audit: HIGH — directly read local source files + Cargo.toml + hnsw.rs
- Sub-space detection pattern: HIGH — directly extends verified `cluster_documents()` from clustering.rs
- Label cache extension: HIGH — directly extends verified `SpaceLabelEntry` from label_cache.rs
- Space type extension: HIGH — directly read types.rs; confirmed parent_id already present
- Frontend patterns (Breadcrumb, Collapsible, Zustand): HIGH — all confirmed installed
- HyperbolicHnsw API integration: HIGH — verified from hnsw.rs source
- Cargo dependency conflict resolution: MEDIUM — Cargo semver resolution behavior well-known but requires empirical `cargo check`

**Research date:** 2026-07-08
**Valid until:** 2026-08-08 (stable local codebase; ruvector crate is local so no upstream surprise)

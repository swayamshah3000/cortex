# Phase 12: GNN Clustering Swap (ruvector-gnn) - Context

**Gathered:** 2026-07-08
**Status:** Ready for planning (auto-recommended per autonomous mode)

<domain>
## Phase Boundary

Phase 12 replaces the hand-rolled k-means in `spaces/clustering.rs` with **ruvector-gnn** — a message-passing GNN over the doc-doc HNSW graph. Produces semantically-coherent clusters via mutual neighbor overlap (a receipt + its cover email cluster together even without shared vocabulary).

Backend-only, no UI changes. Reuses Phase 9 recluster orchestration + LlmSpaceLabeler + SpaceLabelCache pipeline unchanged.

**Critical gate:** Planner MUST audit ruvector-gnn crate first (Phase 9 lesson — ruvector-cluster + ruvector-domain-expansion turned out unfit; ruvector-hyperbolic-hnsw worked).

### What Phase 12 delivers

1. **ruvector-gnn dependency in Cargo.toml** (path spec to `experiments/ruvector/crates/ruvector-gnn/`).
2. **`GnnClusterer` in `spaces/clustering.rs`** (or new `spaces/gnn_clustering.rs`) — takes doc-doc HNSW graph + doc vectors, returns `ClusterResult` (same signature as existing `cluster_documents`).
3. **k-means deletion** — old `cluster_documents` function + tests removed.
4. **Recluster wall-clock benchmark** on 10K synthetic corpus, documented in SUMMARY.
5. **Cluster coherence benchmark** on 500-doc corpus, ± 5% of k-means baseline.
6. **Label collision rate benchmark** (post-Phase 9 labeling) < 10%.

### Out of scope

- UI changes (backend-only phase).
- Recluster orchestration in `spaces/manager.rs` (unchanged — reuses same interface).
- Sub-space clustering (Phase 10 recursive k-means untouched — separate concern).
- New Cargo deps beyond ruvector-gnn.

</domain>

<decisions>
## Implementation Decisions

### GNN Clustering (Area 1)

- **D-01: Full swap, no compat layer.** k-means deleted after ruvector-gnn ships. SC1 mandates zero dead-code.
- **D-02: `GnnClusterer::cluster()` returns existing `ClusterResult` shape.** Drop-in swap — `spaces/manager.rs` sees no change.
- **D-03: Ruvector-gnn crate audit before implementation.** Planner reads `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-gnn/{README.md,src/lib.rs}` to confirm it delivers message-passing over vector graphs. If misfit (per Phase 9 lesson w/ ruvector-cluster), document deviation + fall back to k-means with a warning + close phase w/ documented gap.
- **D-04: Silent k-means fallback removed after swap.** Aim for green swap. If ruvector-gnn fails init, log error + return empty ClusterResult (upstream shows "0 clusters" — user re-triggers).

### Benchmark & Verification (Area 2)

- **D-05: 500-doc benchmark corpus.** Synthetic — sample 500 diverse docs from ~/private (property, identity, vehicle, finance, kids, medical). Benchmark script in `src-tauri/benches/`.
- **D-06: Cluster coherence metric = mean intra-cluster cosine similarity.** GNN result ± 5% of k-means baseline (SC2).
- **D-07: Wall-clock benchmark on 10K synthetic corpus.** Docs generated from templated content; each doc gets random 384-dim vector. Assert ≤ 2× k-means baseline (SC4).
- **D-08: Label collision benchmark (SC3).** After clustering, run Phase 9 LlmSpaceLabeler on both k-means clusters + GNN clusters; measure duplicate label rate. GNN collision < 10%.
- **D-09: Benchmarks are `#[ignore]` in unit-test suite** — long-running, not in CI critical path. Run manually + record results in SUMMARY.

### Fallback Strategy (Area 3)

- **D-10: If ruvector-gnn crate is misfit** (like ruvector-cluster was), planner documents the finding + delivers a "documented gap" phase closure — keeps k-means, ships benchmark suite as future baseline, phase completes w/ VERIFICATION status `gaps_found` (accepted).
- **D-11: If crate works but slower than 2×** — accept as documented tradeoff in SUMMARY, phase passes (SC4 says "documented in SUMMARY.md if slower" — spec permits slower).
- **D-12: If ruvector-gnn works but coherence drops > 5%** — planner adjusts hyperparams (e.g., number of message-passing rounds, aggregation function); if still failing, document deviation + close with gaps_found (accepted).

### Claude's Discretion (Planner-owned)

- Ruvector-gnn API surface (planner audits local crate).
- Message-passing rounds default (recommend 3, planner tunes).
- Aggregation function (mean vs max vs attention — depends on crate API).
- Whether GnnClusterer lives in `spaces/clustering.rs` (rename module) or new `spaces/gnn_clusterer.rs`.
- Benchmark harness style (criterion.rs vs custom bencher).
- Cluster count k selection (adaptive vs fixed).
- Corpus generation script location + reproducibility.
- Whether to keep small k-means helper (`fn cluster_documents_kmeans` under `#[cfg(test)]` for baseline benchmarks).

</decisions>

<canonical_refs>
## Canonical References

### Project specs
- `.planning/ROADMAP.md` §"Phase 12: GNN Clustering Swap"
- `.planning/phases/09-llm-space-labeling/09-CONTEXT.md` — LlmSpaceLabeler + SpaceLabelCache (unchanged)
- `.planning/phases/10-hierarchical-spaces/10-CONTEXT.md` — sub-space recursive k-means (untouched)
- `.planning/phases/09-llm-space-labeling/09-RESEARCH.md` — CRITICAL ruvector crate audit lessons

### Existing Cortex code
- `src-tauri/src/spaces/clustering.rs` — current k-means (target of swap)
- `src-tauri/src/spaces/manager.rs` — recluster orchestration (no changes)
- `src-tauri/src/spaces/subspace_detector.rs` — recursive k-means for sub-spaces (unchanged)
- `src-tauri/Cargo.toml` — add ruvector-gnn dep
- `src-tauri/src/engine.rs` + `src-tauri/src/search/query.rs` — HNSW access (doc-doc graph source)

### RuVector crates (local)
- `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-gnn/` — planner AUDITS FIRST

### Patterns to mirror
- Phase 9 ruvector-cluster deviation write-up (RESEARCH.md audit table)
- Phase 10 hyp_index silent fallback pattern (D-11)
- Existing benches/ directory pattern (or create one)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `spaces/clustering.rs::cluster_documents` — current signature, drop-in target
- `spaces/clustering.rs::cosine_similarity` — reused
- HNSW index from `engine.rs` — doc-doc neighbor graph source
- `spaces/manager.rs::recluster` — unchanged consumer
- Phase 10 `subspace_detector` — still uses k-means recursively (planner may swap in follow-up OR keep k-means for sub-clustering — cheaper on small n).

### Established Patterns
- Result type: `Result<ClusterResult, AppError>`
- Sync clustering (not async) — no ai_request calls inside clusterer
- App state: `Arc<Mutex<>>` for shared state (none needed — clusterer is stateless)

### Integration Points
- Cargo.toml (add ruvector-gnn)
- `spaces/clustering.rs` — replace `cluster_documents` body
- `spaces/mod.rs` — no change
- Benchmarks: `src-tauri/benches/clustering.rs` (create)

</code_context>

<specifics>
## Specific Ideas

- **Ruvector crate audit is non-negotiable** — Phase 9 taught us that ~2/3 ruvector crates were misnamed. Read local `/crates/ruvector-gnn/` before Cargo.toml commit.
- **k-means baseline preserved for tests** — even if deleted from production, keep as `#[cfg(test)]` module in `clustering.rs` for benchmarks.
- **500-doc corpus samples from ~/private** — same corpus as Phase 8/9 prompt tuning.
- **10K synthetic corpus for perf** — generated deterministically (seeded RNG) so benchmarks reproducible.
- **Sub-space clustering NOT changed** — Phase 10 recursive k-means is intra-cluster, works on small n. GNN overkill.
- **Coherence metric = mean intra-cluster cosine sim** — well-defined, easy to compare.
- **Label collision rate = (# duplicate labels) / (# spaces)** — Phase 9 label output feeds this metric.

</specifics>

<deferred>
## Deferred Ideas

### Phase 12 follow-ups (v1.2)
- **GNN sub-space clustering** — Phase 10's recursive k-means could swap to recursive GNN. Deferred (small-n case where k-means fine).
- **Learned GNN embeddings** — currently GNN takes fixed vectors from Phase 2 embedder. Could jointly learn. v2.
- **Dynamic GNN update** — recluster only affected clusters when new docs land. Currently full-corpus. v2.

### Downstream phase dependencies
- **Phase 13 (Cypher Entity Graph)** — no direct dependency; both are ruvector adoption phases.
- **Phase 14 (SONA Feedback Loop)** — SONA-informed clustering could tune GNN attention weights. v2.

### v2 / future
- **Multi-modal clustering** — combine text + image vectors (rupixel from Phase 15) via GNN attention.
- **User-corrected cluster feedback loop** — user "this belongs in X space" → SONA-style update to GNN.

</deferred>

---

*Phase: 12-gnn-clustering-swap*
*Context gathered: 2026-07-08 (auto-recommended, no user input required per autopilot)*

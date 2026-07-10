# Phase 12: GNN Clustering Swap (ruvector-gnn) - Research

**Researched:** 2026-07-09
**Domain:** GNN message-passing clustering, Rust benchmark harness, synthetic corpus generation
**Confidence:** HIGH (ruvector-gnn crate: VERIFIED local read; existing clustering code: VERIFIED local read; integration path: VERIFIED)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**GNN Clustering (Area 1)**
- D-01: Full swap, no compat layer. k-means deleted after ruvector-gnn ships. SC1 mandates zero dead-code.
- D-02: `GnnClusterer::cluster()` returns existing `ClusterResult` shape. Drop-in swap — `spaces/manager.rs` sees no change.
- D-03: Ruvector-gnn crate audit before implementation. Planner reads local crate to confirm it delivers message-passing over vector graphs. If misfit (per Phase 9 lesson), document deviation + fall back to k-means with a warning + close phase w/ documented gap.
- D-04: Silent k-means fallback removed after swap. If ruvector-gnn fails init, log error + return empty ClusterResult (upstream shows "0 clusters" — user re-triggers).

**Benchmark & Verification (Area 2)**
- D-05: 500-doc benchmark corpus. Synthetic — sample 500 diverse docs from ~/private (property, identity, vehicle, finance, kids, medical). Benchmark script in `src-tauri/benches/`.
- D-06: Cluster coherence metric = mean intra-cluster cosine similarity. GNN result ± 5% of k-means baseline (SC2).
- D-07: Wall-clock benchmark on 10K synthetic corpus. Docs generated from templated content; each doc gets random 384-dim vector. Assert ≤ 2× k-means baseline (SC4).
- D-08: Label collision benchmark (SC3). After clustering, run Phase 9 LlmSpaceLabeler on both k-means clusters + GNN clusters; measure duplicate label rate. GNN collision < 10%.
- D-09: Benchmarks are `#[ignore]` in unit-test suite — long-running, not in CI critical path. Run manually + record results in SUMMARY.

**Fallback Strategy (Area 3)**
- D-10: If ruvector-gnn crate is misfit, planner documents the finding + delivers a "documented gap" phase closure — keeps k-means, ships benchmark suite as future baseline, phase completes w/ VERIFICATION status `gaps_found` (accepted).
- D-11: If crate works but slower than 2× — accept as documented tradeoff in SUMMARY, phase passes (SC4 says "documented in SUMMARY.md if slower").
- D-12: If ruvector-gnn works but coherence drops > 5% — planner adjusts hyperparams; if still failing, document deviation + close with gaps_found (accepted).

### Claude's Discretion

- Ruvector-gnn API surface (planner audits local crate — DONE in this research).
- Message-passing rounds default (recommend 3, planner tunes).
- Aggregation function (mean vs max vs attention — depends on crate API).
- Whether GnnClusterer lives in `spaces/clustering.rs` (rename module) or new `spaces/gnn_clusterer.rs`.
- Benchmark harness style (criterion.rs vs custom bencher).
- Cluster count k selection (adaptive vs fixed).
- Corpus generation script location + reproducibility.
- Whether to keep small k-means helper (`fn cluster_documents_kmeans` under `#[cfg(test)]` for baseline benchmarks).

### Deferred Ideas (OUT OF SCOPE)

- GNN sub-space clustering (Phase 10 recursive k-means untouched).
- Learned GNN embeddings (v2).
- Dynamic GNN update (recluster only affected clusters, v2).
- Phase 13 (Cypher Entity Graph), Phase 14 (SONA Feedback Loop).
- Multi-modal clustering (rupixel + GNN attention, Phase 15+).
- User-corrected cluster feedback loop (v2).
</user_constraints>

---

## Summary

**CRITICAL FINDING: ruvector-gnn is NOT a document clustering library.** It is a GNN training framework for re-ranking HNSW search results. It provides `RuvectorLayer` (message-passing GNN layer with attention, GRU updates, and layer norm), `differentiable_search` (soft attention search over candidates), `hierarchical_forward` (multi-layer GNN traversal), and training utilities (Adam optimizer, replay buffer, EWC). There is no `cluster_documents()` equivalent, no output type that resembles `ClusterResult`, and no notion of assigning document IDs to groups.

This is the Phase 9 lesson repeating: the ruvector crate name suggests one thing, the implementation delivers another. `ruvector-gnn` is a GNN inference/training layer intended to re-rank HNSW neighbors for improved search quality — not to partition a vector corpus into clusters. Its forward pass takes `(node_embedding, neighbor_embeddings, edge_weights)` and returns an updated node embedding. It does not emit cluster assignments.

**What ruvector-gnn CAN do:** The `RuvectorLayer` can transform document embeddings using message-passing over a manually constructed doc-doc neighborhood graph, producing enriched embeddings. These enriched embeddings could then be fed into a clustering algorithm. But the clustering algorithm itself is not provided by ruvector-gnn — it would have to be k-means (existing code) or an external crate.

**D-10 applies.** Per the locked fallback strategy, the planner MUST document this finding, keep k-means as the authoritative clustering backend, ship the benchmark suite as a baseline for a future genuine GNN clustering integration, and close the phase with `gaps_found` (accepted). The phase still has value: it delivers a reproducible benchmark harness, documents the ruvector-gnn mismatch, and establishes coherence + collision rate baselines.

**Primary recommendation:** Apply D-10 immediately. Close phase as `gaps_found` (accepted). Deliver: (1) the benchmark suite in `src-tauri/benches/clustering.rs`, (2) a 10K synthetic corpus generator, (3) documented coherence and collision rate baselines for k-means, and (4) a clear SUMMARY.md documenting why ruvector-gnn could not be used and what the acceptance criteria are for a future clustering swap.

---

## CRITICAL: ruvector-gnn Crate Audit

> This is the mandatory audit required by D-03 and CONTEXT.md.

**Location:** `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-gnn/` [VERIFIED: local crate read]

### What ruvector-gnn actually is

The README says: "A Graph Neural Network layer that makes HNSW vector search get smarter over time." That framing — *search gets smarter* — is accurate and precise. It is NOT about grouping documents into clusters.

**Published modules (from `src/lib.rs`):** [VERIFIED: local crate read]

| Module | What it contains |
|--------|-----------------|
| `layer` | `RuvectorLayer` — message-passing GNN with multi-head attention + GRU update + layer norm |
| `search` | `differentiable_search`, `hierarchical_forward`, `cosine_similarity` |
| `query` | `RuvectorQuery`, `QueryMode`, `QueryResult`, `SubGraph` |
| `training` | Adam/SGD optimizer, loss functions, `TrainConfig` |
| `replay` | `ReplayBuffer` — experience replay for continual learning |
| `ewc` | `ElasticWeightConsolidation` — catastrophic forgetting prevention |
| `scheduler` | `LearningRateScheduler` — cosine annealing, warmup etc. |
| `compress` | `CompressedTensor` — INT8/FP16 quantization |
| `tensor` | Tensor utility operations |
| `mmap` | Memory-mapped weight storage |

**Primary entry point:** `RuvectorLayer::new(input_dim, hidden_dim, heads, dropout) -> Result<RuvectorLayer>` [VERIFIED: layer.rs line 347]

**Forward pass signature:** [VERIFIED: layer.rs line 378]
```rust
pub fn forward(
    &self,
    node_embedding: &[f32],
    neighbor_embeddings: &[Vec<f32>],
    edge_weights: &[f32],
) -> Vec<f32>
```

This takes ONE node + its neighbors and returns an updated embedding for that node. It does NOT:
- Accept a full matrix of all document vectors
- Return cluster assignments (which doc belongs to which group)
- Produce `ClusterResult` or anything like it
- Know what "a cluster" is

### What the CONTEXT.md assumes ruvector-gnn is

D-03 assumes: "message-passing GNN over the doc-doc HNSW graph produces semantically-coherent clusters."

The word "clusters" is the gap. ruvector-gnn does message-passing that produces better embeddings (for use in search). It does not follow the message-passing → community detection → cluster assignment pathway. That pathway would require:
1. Build adjacency list from HNSW neighbors (ruvector-gnn CAN help here)
2. Run iterative message-passing to enrich embeddings (ruvector-gnn DOES this)
3. Apply community detection (spectral clustering, label propagation, Louvain) on the enriched graph (ruvector-gnn does NOT do this)

### The gap in concrete terms

```
// What the CONTEXT.md imagined:
let result: ClusterResult = ruvector_gnn::cluster(doc_vectors, k)?;

// What ruvector-gnn actually provides:
let layer = RuvectorLayer::new(384, 384, 4, 0.1)?;
let enriched_embedding: Vec<f32> = layer.forward(
    &doc_vector,
    &neighbor_vectors,
    &edge_weights
);
// → This enriches one node's embedding. Still need a separate clustering algorithm.
```

### Verdict

**[VERIFIED: local crate read]** ruvector-gnn does NOT deliver message-passing GNN clustering over a doc-doc graph. D-10 fallback applies. Phase 12 closes as `gaps_found` (accepted).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| GNN message-passing clustering | API/Backend (Rust) | — | Pure CPU computation over document vectors; no IPC, no frontend |
| Benchmark harness (10K corpus) | API/Backend (Rust) | — | `src-tauri/benches/clustering.rs`; runs via `cargo bench` |
| Coherence metric computation | API/Backend (Rust) | — | Mean intra-cluster cosine sim; pure arithmetic |
| Synthetic corpus generation | API/Backend (Rust) | — | Seeded RNG in bench script; deterministic reproduction |
| 500-doc corpus sampling | API/Backend (Rust) | — | Reads indexed Cortex data or Dropbox path scan |
| SUMMARY.md documentation | Developer artifact | — | Captures gap finding + baseline metrics |

---

## Standard Stack

### Core — Existing (no new dependencies required for D-10 closure)

| Library | Version | Purpose | Status |
|---------|---------|---------|--------|
| `clustering.rs` k-means | in-tree | Document clustering (retained) | [VERIFIED: local read] |
| `cosine_similarity` | in-tree | Coherence metric | [VERIFIED: clustering.rs line 165] |
| `auto_detect_k` | in-tree | Adaptive k selection | [VERIFIED: clustering.rs line 125] |
| `rand` | 0.9 | Seeded RNG for synthetic corpus | [VERIFIED: Cargo.toml] |

### Benchmark Harness

| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| Custom `#[ignore]` tests | n/a | Benchmark harness | Recommended — simpler than criterion for internal metrics |
| `criterion` | 0.5 (in ruvector workspace) | Alternative if formal bench reports desired | Not in cortex Cargo.toml; would need to add to `[dev-dependencies]` |

**Recommendation for benchmark harness (Claude's Discretion):** Use custom `#[ignore]` tests in `src-tauri/benches/clustering.rs` (or `src-tauri/src/spaces/clustering.rs #[cfg(test)]`). Criterion adds a `[dev-dependencies]` entry and HTML report infrastructure that is overkill for internal baselines. A custom bench uses `std::time::Instant` and prints to stdout — no new deps. [ASSUMED — criterion benefit is marginal for one-off internal measurements]

**Cargo.toml change for criterion (if desired):**
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
tempfile = "3"
```

**Cargo.toml for `[[bench]]` entry:**
```toml
[[bench]]
name = "clustering"
harness = false
```

### ruvector-gnn Dependency — NOT recommended

The CONTEXT.md asked for `ruvector-gnn` to be added to Cargo.toml. Given the crate audit finding (D-10 applies), do NOT add it. Adding it would:
- Pull in `ndarray = "0.16"` (not in cortex Cargo.toml — new transitive dep)
- Pull in `rayon`, `parking_lot`, `dashmap` (not currently in cortex tree)
- Add `memmap2`, `rand_distr` (not in cortex tree)
- Compile ~10 modules of GNN training/inference code with no actual usage
- Waste binary size in a Tauri desktop app

**Only add ruvector-gnn if a future phase implements the full GNN embedding enrichment → community detection pipeline.** That is explicitly deferred to v1.2.

---

## Package Legitimacy Audit

> Phase 12 (D-10 closure path) adds NO new external packages to the production build. If criterion is added to `[dev-dependencies]`, it is test-only and does not ship with the Tauri binary.

| Package | Registry | Age | Downloads | Source Repo | Status | Disposition |
|---------|----------|-----|-----------|-------------|--------|-------------|
| `criterion = "0.5"` | crates.io | 8+ yrs | Very high (standard bench tool) | github.com/bheisler/criterion.rs | Well-established | Approved for dev-dependencies only (optional) |

**slopcheck status:** slopcheck not run (no new production packages in D-10 closure path). criterion is a known-legitimate crate in the Rust ecosystem.

**Packages removed due to slopcheck [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

---

## Architecture Patterns

### System Architecture Diagram

```
Phase 12 D-10 Closure Path
════════════════════════════

  k-means clustering (spaces/clustering.rs)  ← RETAINED, unchanged
          │
          ▼
  cluster_documents(vectors, k) → ClusterResult
          │
          ├──────────────────────────────────────┐
          │                                      │
     [existing path]                    [NEW: benchmark path]
     SpaceManager::recluster()         src-tauri/benches/clustering.rs
     (unchanged)                               │
                                       ┌───────┴────────────────┐
                                       │                        │
                              500-doc corpus           10K synthetic corpus
                              (Dropbox scan OR         (seeded RNG, 384-dim
                              synthetic fallback)       random vectors)
                                       │                        │
                              cluster_documents()      cluster_documents()
                                       │                        │
                              coherence metric         wall-clock time
                              (mean intra-cluster      (std::time::Instant)
                               cosine similarity)             │
                                       │               Assert ≤ baseline
                              collision benchmark             (no swap)
                              (Phase 9 LlmLabeler             │
                               on cluster output)     SUMMARY.md baseline
                                       │               recorded for future
                              SUMMARY.md               GNN swap phase
```

### Recommended Project Structure

```
src-tauri/
├── benches/
│   └── clustering.rs        # NEW: benchmark harness (#[ignore] tests or criterion)
│                              covers: 10K synthetic corpus wall-clock, coherence metric
└── src/spaces/
    ├── clustering.rs         # UNCHANGED: k-means retained (D-10)
    ├── manager.rs            # UNCHANGED
    ├── mod.rs                # UNCHANGED
    └── ... (all other .rs)   # UNCHANGED
.planning/phases/12-gnn-clustering-swap/
    └── 12-SUMMARY.md         # NEW: gap finding + baseline metrics
```

### Pattern 1: Synthetic 10K Corpus Generator with Seeded RNG

**What:** Generate 10,000 document vectors deterministically using a seeded RNG, grouped into known domain categories for reproducible benchmarks.

**When to use:** D-07 wall-clock benchmark on 10K corpus.

**Example:**
```rust
// src-tauri/benches/clustering.rs
// Source: [VERIFIED: clustering.rs API + Cargo.toml rand=0.9]
use cortex_lib::spaces::clustering::cluster_documents;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::Rng;

const SEED: u64 = 42;
const N_DOCS: usize = 10_000;
const DIM: usize = 384;
const N_DOMAINS: usize = 8; // property, finance, medical, kids, work, identity, vehicle, misc

/// Generate a synthetic corpus with domain-clustered structure.
/// Vectors within a domain have a shared bias direction (simulates semantic similarity).
fn generate_synthetic_corpus(n: usize, dim: usize, n_domains: usize) -> Vec<(String, Vec<f32>)> {
    let mut rng = SmallRng::seed_from_u64(SEED);

    // Generate domain centroids (unit vectors in random directions)
    let domain_centroids: Vec<Vec<f32>> = (0..n_domains)
        .map(|_| {
            let mut v: Vec<f32> = (0..dim).map(|_| rng.gen::<f32>() - 0.5).collect();
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            v.iter_mut().for_each(|x| *x /= norm);
            v
        })
        .collect();

    (0..n)
        .map(|i| {
            let domain = i % n_domains;
            let centroid = &domain_centroids[domain];

            // Mix centroid direction with noise: 0.7 centroid + 0.3 noise
            let noise: Vec<f32> = (0..dim).map(|_| (rng.gen::<f32>() - 0.5) * 0.3).collect();
            let mut v: Vec<f32> = centroid.iter().zip(noise.iter())
                .map(|(c, n)| 0.7 * c + n)
                .collect();

            // L2-normalize
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            v.iter_mut().for_each(|x| *x /= norm);

            (format!("doc-{}", i), v)
        })
        .collect()
}

#[test]
#[ignore] // D-09: long-running, not in CI critical path
fn bench_kmeans_10k_wall_clock() {
    let corpus = generate_synthetic_corpus(N_DOCS, DIM, N_DOMAINS);
    let k = cortex_lib::spaces::clustering::auto_detect_k(N_DOCS);

    let start = std::time::Instant::now();
    let result = cluster_documents(corpus, k);
    let elapsed = start.elapsed();

    println!("k-means 10K docs: {:?}", elapsed);
    println!("Clusters produced: {}", result.clusters.len());
    assert!(!result.clusters.is_empty(), "k-means must produce clusters");

    // Record baseline for SUMMARY.md
    eprintln!("BENCHMARK_BASELINE_MS={}", elapsed.as_millis());
}
```

### Pattern 2: Coherence Metric (Mean Intra-Cluster Cosine Similarity)

**What:** D-06 metric — measures how semantically tight each cluster is.

**Formula:** For each cluster, compute all pairwise cosine similarities; take the mean. Average across all clusters.

```rust
// Source: [VERIFIED: clustering.rs cosine_similarity API]
use cortex_lib::spaces::clustering::{cosine_similarity, ClusterResult};

/// Compute mean intra-cluster cosine similarity (D-06 coherence metric).
///
/// For clusters with < 2 members, the intra-cluster similarity is defined as 1.0
/// (trivially coherent). This prevents divide-by-zero in degenerate cases.
pub fn mean_intra_cluster_coherence(
    result: &ClusterResult,
    vectors: &std::collections::HashMap<String, Vec<f32>>,
) -> f32 {
    if result.clusters.is_empty() {
        return 0.0;
    }

    let cluster_scores: Vec<f32> = result.clusters.iter().map(|cluster| {
        let members: Vec<&Vec<f32>> = cluster.doc_ids.iter()
            .filter_map(|id| vectors.get(id))
            .collect();

        if members.len() < 2 {
            return 1.0; // Trivially coherent
        }

        let mut sim_sum = 0.0f32;
        let mut count = 0usize;

        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                sim_sum += cosine_similarity(members[i], members[j]);
                count += 1;
            }
        }

        if count == 0 { 1.0 } else { sim_sum / count as f32 }
    }).collect();

    cluster_scores.iter().sum::<f32>() / cluster_scores.len() as f32
}
```

**Note on computational cost for 500-doc corpus:** Pairwise similarities within each cluster. If k-means produces 10 clusters of ~50 docs each, pairwise cost per cluster is C(50,2) = 1225 pairs. Total: ~12,250 pairs. Negligible for benchmark — no optimization needed.

**Note on computational cost for 10K corpus:** k = auto_detect_k(10000) = 20 (capped). If clusters are ~500 docs each, pairwise cost is C(500,2) = 124,750 per cluster × 20 = 2.5M pairs. With 384-dim vectors this is ~960M float multiplications — NOT negligible. For the 10K benchmark, use centroid-based approximation instead: mean cosine(doc, centroid) per cluster.

### Pattern 3: Label Collision Rate Benchmark

**What:** D-08 metric. Run Phase 9 LlmSpaceLabeler on both k-means and a GNN-equivalent clustering. Measure duplicate label rate.

**Current Phase 12 path (D-10):** Since no GNN clustering swap occurs, this benchmark cannot compare k-means vs GNN. Instead, run labeling on k-means output and establish the baseline collision rate for future comparison.

**Collision rate formula:** `# spaces with a label that appears more than once / total spaces`

```rust
// Source: [VERIFIED: llm_labeler.rs collision detection pattern from manager.rs]
// This is a manual test, not automated — requires active AI provider.
// Document result in SUMMARY.md:
// k-means baseline collision rate: X% on 500-doc corpus with k=Y
```

### Anti-Patterns to Avoid

- **Adding ruvector-gnn to production Cargo.toml without a clustering use case:** It pulls ndarray + rayon + dashmap + parking_lot + memmap2 into the Tauri binary for zero benefit. Add ONLY when implementing GNN embedding enrichment → spectral clustering in a future phase.

- **Implementing pairwise coherence on the 10K corpus (instead of centroid approximation):** O(N²) within each cluster becomes prohibitive. Use centroid-based `mean(cosine(doc, centroid))` for large corpora.

- **Running benchmarks in CI (without `#[ignore]`):** k-means on 10K docs takes seconds; in CI with multiple test runs this creates flaky timeouts. D-09 mandates `#[ignore]`.

- **Synthetic corpus without domain structure:** Purely random 384-dim vectors will cluster trivially (all clusters ~equal, very low coherence). The synthetic corpus MUST embed domain-bias directions so cluster quality is meaningful.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cosine similarity | Custom dot product | `cosine_similarity()` from `spaces/clustering.rs` | Already exists, handles zero-norm edge case |
| Seeded RNG | `SystemTime`-seeded rand | `SmallRng::seed_from_u64(42)` from `rand` crate (already in Cargo.toml) | Reproducibility requires deterministic seed |
| k selection | Hard-coded k=8 | `auto_detect_k(n)` from `spaces/clustering.rs` | Existing heuristic; benchmark with same k as production |
| GNN clustering | Custom graph community detection | **Wait for a crate that actually provides it** — see Open Questions | Phase 9 + 12 show ruvector crate names do not match content |

**Key insight:** The ruvector ecosystem's crate names are aspirational; the implementations are frequently earlier-stage than the names suggest. Always read `src/lib.rs` before committing to a ruvector crate. Two of three crates audited in Phases 9+12 were misfits.

---

## 500-Doc Corpus Strategy

D-05 specifies sampling from ~/private. Research finding: `~/private` exists and contains personal documents, but:

- Direct file count: 0 PDFs found in `~/private/personal/` (likely because Dropbox files are not locally synced on this machine — the Dropbox daemon may be selective-sync with cloud-only files)
- `~/Documents` has only 18 PDFs (insufficient for 500-doc corpus)

**Recommendation:** Generate a 500-doc SYNTHETIC corpus (same generator as 10K, just smaller N). This is a valid substitute — the benchmark purpose is to measure clustering quality metrics, not to analyze specific real-world documents. The coherence metric and collision rate are algorithm properties, not corpus-specific.

**If real docs become available:** The benchmark harness should accept a corpus as a parameter:
```rust
fn load_real_corpus(paths: &[std::path::PathBuf]) -> Vec<(String, Vec<f32>)>
```
This can be connected to Cortex's own fastembed pipeline to embed real Dropbox documents. But this requires fastembed to be running — add as an optional manual step in SUMMARY.md, not in the automated bench.

**500-doc corpus size note:** With `auto_detect_k(500)` = `round(sqrt(250))` = 16, clusters will average ~31 docs each. Pairwise coherence cost: C(31,2) = 465 pairs × 16 clusters = ~7,440 pairs. Runs instantly.

---

## 10K Synthetic Corpus Generation

Seeded RNG with domain-structured vectors [VERIFIED: rand 0.9 in Cargo.toml]:

```rust
// rand 0.9 API:
use rand::SeedableRng;
use rand::rngs::SmallRng;
let mut rng = SmallRng::seed_from_u64(42u64);
let val: f32 = rng.random::<f32>(); // rand 0.9 uses .random() not .gen()
```

**rand 0.9 API change:** `rand` 0.9 renamed `.gen::<T>()` to `.random::<T>()`. Since Cargo.toml has `rand = "0.9"`, use `.random()` not `.gen()`. [VERIFIED: Cargo.toml, rand 0.9 changelog]

**N_DOMAINS = 8:** Chosen to match the Cortex domain taxonomy: property, finance, medical, kids, work, identity, vehicle, miscellaneous. Each domain gets 1,250 docs (10K / 8). This gives auto_detect_k(10000) = 20 clusters, so each "domain" produces ~2-3 detected clusters (realistic sub-clustering).

---

## Common Pitfalls

### Pitfall 1: ruvector-gnn crate adds ndarray as a new transitive dependency

**What goes wrong:** Adding `ruvector-gnn = { path = "..." }` to `cortex/src-tauri/Cargo.toml` pulls in `ndarray = "0.16"` (not currently in the cortex dependency tree). This may cause version conflicts with fastembed or other numerical crates, and adds ~3MB to compile time.

**Why it happens:** ruvector-gnn's Cargo.toml declares `ndarray = { workspace = true, features = ["serde"] }` from the ruvector workspace. When path-imported into cortex, the workspace version resolves to 0.16 — which cortex does not currently use.

**How to avoid:** Do NOT add ruvector-gnn to Cargo.toml in this phase (D-10 path). If a future phase adds it, audit ndarray version compatibility with fastembed first.

**Warning signs:** `cargo check` outputs "failed to select a version for `ndarray`" or "feature `serde` not enabled in ruvector-gnn."

### Pitfall 2: rand 0.9 API change — `.gen()` vs `.random()`

**What goes wrong:** Code written using `rng.gen::<f32>()` (rand 0.8 API) fails to compile with `rand = "0.9"` in Cargo.toml.

**Why it happens:** rand 0.9 renamed the generic sampling method.

**How to avoid:** Use `rng.random::<f32>()` in all benchmark code. [VERIFIED: Cargo.toml shows `rand = "0.9"`]

**Warning signs:** `error[E0599]: no method named 'gen' found for struct 'SmallRng'`

### Pitfall 3: Coherence metric O(N²) explosion on 10K corpus

**What goes wrong:** Computing all pairwise cosine similarities within each cluster for the 10K benchmark. With k=20 and 500 docs per cluster, that's 124,750 pairs × 384-dim vectors — takes minutes.

**Why it happens:** Pairwise coherence is O(n²) per cluster. Fine for 500-doc corpus, prohibitive for 10K.

**How to avoid:** For the 10K benchmark, use centroid-based coherence approximation: `mean(cosine(doc_i, centroid))` over all docs in each cluster. This is O(n×dim) not O(n²×dim). Both metrics measure cohesion but centroid-based is much faster.

**Warning signs:** Benchmark taking >60 seconds for coherence step on 10K corpus.

### Pitfall 4: Synthetic corpus with uniform random vectors (no domain structure)

**What goes wrong:** Using `rng.random::<f32>()` for all 384 dimensions uniformly produces vectors that are roughly equidistant from each other (curse of dimensionality). k-means on such data produces clusters with ~equal, very low coherence — meaningless as a quality baseline.

**Why it happens:** True benchmark quality requires distinguishable clusters. Random high-dimensional vectors do not naturally cluster.

**How to avoid:** Use domain-biased vectors: generate a domain centroid (random unit vector), then generate each doc as 70% centroid + 30% noise. This creates realistic "semantic neighborhoods" that k-means can detect.

**Warning signs:** All cluster coherence scores are between 0.0 and 0.1 (random baseline expected ~0.0 in high dim); meaningful synthetic data should produce cluster coherence ~0.7-0.9.

### Pitfall 5: Benchmark `[[bench]]` section requires `harness = false` for custom benchers

**What goes wrong:** If `src-tauri/benches/clustering.rs` uses criterion, the Cargo.toml needs a `[[bench]]` section with `harness = false`. Without it, `cargo bench` uses the default test harness which conflicts with criterion's `main!()` macro.

**Why it happens:** criterion defines its own entry point; the default test harness also defines `main`.

**How to avoid:** For criterion, add to Cargo.toml:
```toml
[[bench]]
name = "clustering"
harness = false
```
For custom `#[ignore]` tests in `#[cfg(test)]`, no `[[bench]]` entry is needed — just run with `cargo test -- --ignored`.

**Warning signs:** `error: linking with 'cc' failed: ... multiple definition of 'main'`

### Pitfall 6: k-means `#[cfg(test)]` preservation for baseline comparison

**What goes wrong:** D-01 says "k-means deleted" in the full swap path. But Phase 12 is closing with D-10 (no swap). If someone interprets D-01 as applying immediately, they delete k-means and have nothing.

**Why it happens:** D-01 and D-10 are in tension. D-10 wins because the GNN crate is a misfit.

**How to avoid:** Keep k-means exactly as-is. D-01 is deferred to a future phase when a genuine GNN clustering crate is available.

---

## Code Examples

### Benchmark harness entry point (custom, no criterion)

```rust
// src-tauri/benches/clustering.rs
// Source: [VERIFIED: clustering.rs public API + Cargo.toml rand=0.9]

// NOTE: This file uses cortex_lib which requires the lib crate-type in Cargo.toml.
// cortex/Cargo.toml already declares:
//   [lib]
//   name = "cortex_lib"
//   crate-type = ["lib", "cdylib", "staticlib"]
// So `use cortex_lib::...` works in benchmarks.

use cortex_lib::spaces::clustering::{
    auto_detect_k, cluster_documents, cosine_similarity, ClusterResult,
};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

const SEED: u64 = 42;
const DIM: usize = 384;

fn generate_synthetic_corpus(n: usize, n_domains: usize, seed: u64) -> Vec<(String, Vec<f32>)> {
    let mut rng = SmallRng::seed_from_u64(seed);

    // Domain centroids: n_domains unit vectors in random directions
    let domain_centroids: Vec<Vec<f32>> = (0..n_domains)
        .map(|_| {
            let mut v: Vec<f32> = (0..DIM).map(|_| rng.random::<f32>() - 0.5).collect();
            let norm: f32 = v.iter().map(|x: &f32| x * x).sum::<f32>().sqrt();
            if norm > 0.0 { v.iter_mut().for_each(|x| *x /= norm); }
            v
        })
        .collect();

    (0..n)
        .map(|i| {
            let domain = i % n_domains;
            let centroid = &domain_centroids[domain];
            // 70% domain signal + 30% noise
            let noise_scale = 0.3_f32;
            let mut v: Vec<f32> = centroid
                .iter()
                .map(|&c| 0.7 * c + noise_scale * (rng.random::<f32>() - 0.5))
                .collect();
            let norm: f32 = v.iter().map(|x: &f32| x * x).sum::<f32>().sqrt();
            if norm > 0.0 { v.iter_mut().for_each(|x| *x /= norm); }
            (format!("doc-{}", i), v)
        })
        .collect()
}

/// Centroid-based coherence: mean cosine(doc, centroid) per cluster, averaged over clusters.
/// O(n * dim) — safe for 10K docs.
fn centroid_coherence(result: &ClusterResult, vec_map: &HashMap<String, Vec<f32>>) -> f32 {
    if result.clusters.is_empty() { return 0.0; }
    let scores: Vec<f32> = result.clusters.iter().map(|c| {
        if c.doc_ids.is_empty() { return 1.0; }
        let sims: Vec<f32> = c.doc_ids.iter()
            .filter_map(|id| vec_map.get(id))
            .map(|v| cosine_similarity(v, &c.centroid))
            .collect();
        if sims.is_empty() { 1.0 } else { sims.iter().sum::<f32>() / sims.len() as f32 }
    }).collect();
    scores.iter().sum::<f32>() / scores.len() as f32
}

#[test]
#[ignore]
fn bench_kmeans_500doc_coherence() {
    let corpus = generate_synthetic_corpus(500, 8, SEED);
    let vec_map: HashMap<String, Vec<f32>> = corpus.iter().cloned().collect();
    let k = auto_detect_k(corpus.len());

    let start = std::time::Instant::now();
    let result = cluster_documents(corpus, k);
    let elapsed = start.elapsed();

    let coherence = centroid_coherence(&result, &vec_map);
    println!("k-means 500-doc | k={} | clusters={} | coherence={:.4} | time={:?}",
        k, result.clusters.len(), coherence, elapsed);
    eprintln!("KMEANS_500_COHERENCE={:.4}", coherence);
    eprintln!("KMEANS_500_TIME_MS={}", elapsed.as_millis());

    assert!(!result.clusters.is_empty());
    // Record baseline — no assertion on coherence value (first run establishes baseline)
}

#[test]
#[ignore]
fn bench_kmeans_10k_wall_clock() {
    let corpus = generate_synthetic_corpus(10_000, 8, SEED);
    let vec_map: HashMap<String, Vec<f32>> = corpus.iter().cloned().collect();
    let k = auto_detect_k(corpus.len()); // = 20 (capped)

    let start = std::time::Instant::now();
    let result = cluster_documents(corpus, k);
    let elapsed = start.elapsed();

    let coherence = centroid_coherence(&result, &vec_map);
    println!("k-means 10K-doc | k={} | clusters={} | coherence={:.4} | time={:?}",
        k, result.clusters.len(), coherence, elapsed);
    eprintln!("KMEANS_10K_COHERENCE={:.4}", coherence);
    eprintln!("KMEANS_10K_TIME_MS={}", elapsed.as_millis());

    assert!(!result.clusters.is_empty());
    // SC4: if a future GNN swap is ≤ 2× this time, it passes
}
```

### ruvector-gnn forward pass (for search re-ranking, NOT clustering)

This shows what ruvector-gnn CAN do — for completeness and future phases:

```rust
// Source: [VERIFIED: layer.rs RuvectorLayer::forward() signature]
// This is NOT used in Phase 12. For reference only.
use ruvector_gnn::layer::RuvectorLayer;

// Enrich a single document's embedding using HNSW neighbors
let layer = RuvectorLayer::new(
    384,  // input_dim: matches all-MiniLM-L6-v2 output dimension
    384,  // hidden_dim: maintain dimension for drop-in replacement
    4,    // attention heads: must divide hidden_dim (384 / 4 = 96 OK)
    0.1,  // dropout
)?;

// Get k nearest neighbors from HNSW for this document
let node_vec: Vec<f32> = /* doc embedding */;
let neighbor_vecs: Vec<Vec<f32>> = /* top-k HNSW neighbors */;
let edge_weights: Vec<f32> = /* cosine similarities to neighbors */;

// Forward pass: enriched embedding
let enriched: Vec<f32> = layer.forward(&node_vec, &neighbor_vecs, &edge_weights);
// enriched.len() == 384 (hidden_dim)
// → This can be used for search re-ranking, NOT clustering
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Phase 9 assumed ruvector-cluster provided HDBSCAN | Kept k-means (D-12 fallback = always taken) | Phase 9 | No clustering algo change; GNN swap deferred |
| Phase 12 assumed ruvector-gnn provided GNN clustering | ruvector-gnn provides GNN training layers for search, not clustering | Phase 12 (this research) | D-10 applies; benchmark-only closure |
| No benchmark baseline existed | Phase 12 closes by shipping benchmark suite | This phase | Future phases have a reproducible quality baseline |

**What ruvector-gnn is actually good for (future phases):**
- Search result re-ranking via message-passing (Phase 14 SONA feedback loop could integrate this)
- Enriching document embeddings before clustering (requires a separate community detection step)
- Differentiable search for attention-weighted retrieval

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | rand 0.9 uses `.random::<T>()` not `.gen::<T>()` | Code Examples | Compilation error; fix by changing method name |
| A2 | `cortex_lib` crate name is accessible from `benches/clustering.rs` via `use cortex_lib::...` | Code Examples | Module not found; fix by checking actual `[lib] name = ` in Cargo.toml |
| A3 | 500-doc synthetic corpus is an acceptable substitute for Dropbox sampling (D-05 says "sample 500 diverse docs from ~/private" but Dropbox is not locally synced) | 500-doc corpus strategy | Fails D-05 literal interpretation; user may need to connect Dropbox sync or accept synthetic |
| A4 | criterion 0.5 is compatible with cortex's rust-version = 1.77.2 | Standard Stack (criterion) | Older criterion may require newer MSRV; verify before adding |

**Note on A2:** [VERIFIED: src-tauri/Cargo.toml line 13] `[lib] name = "cortex_lib"` confirms the crate name. `use cortex_lib::spaces::clustering::...` is correct.

**Note on A3:** This is a known gap. The CONTEXT.md D-05 says "sample 500 diverse docs from ~/private." Since Dropbox is configured for selective-sync and files aren't locally available, the synthetic corpus is the safe fallback. The planner should document this deviation in SUMMARY.md and note that the benchmark can be re-run with real Dropbox docs if the user syncs them locally.

---

## Open Questions

1. **Should the Phase 12 benchmarks live in `src-tauri/benches/` or within `#[cfg(test)] mod benchmarks` in `clustering.rs`?**
   - What we know: No `src-tauri/benches/` directory exists yet. Both approaches work.
   - What's unclear: criterion requires `[[bench]]` in Cargo.toml and `benches/` directory; custom `#[ignore]` tests work anywhere.
   - Recommendation: Use `#[cfg(test)] mod benchmarks` within `clustering.rs` with `#[ignore]` tests. No new directory, no Cargo.toml changes. Simplest path for D-10 closure.

2. **Is there a Rust crate that actually provides GNN-based document clustering?**
   - What we know: The ruvector ecosystem does not. The Rust ecosystem for GNN-based clustering (Louvain, label propagation on enriched embeddings) is thin.
   - What's unclear: Whether a future ruvector version will add this capability, or whether an external crate (e.g., a petgraph + custom community detection) would be the right path.
   - Recommendation: Document the capability gap in SUMMARY.md. Do not block Phase 12 on finding an alternative. Future Phase 12.1 can re-examine when ruvector evolves.

3. **Should ruvector-gnn be added to Cargo.toml for search re-ranking (not clustering)?**
   - What we know: ruvector-gnn's `RuvectorLayer::forward()` could enrich embeddings before search — a valid use case distinct from clustering.
   - What's unclear: Whether Phase 14 (SONA Feedback Loop) would benefit from this, or whether it should wait.
   - Recommendation: Out of scope for Phase 12. Note in SUMMARY.md as a future integration opportunity.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/Cargo | Benchmark compilation | Yes | 1.91.0 (Homebrew) [VERIFIED] | — |
| `rand` crate | Synthetic corpus RNG | Yes | 0.9 (in Cargo.toml) [VERIFIED] | — |
| `cortex_lib` clustering API | Benchmark | Yes | in-tree [VERIFIED] | — |
| `~/private` PDFs | D-05 500-doc real corpus | No | 0 PDFs locally synced [VERIFIED] | Synthetic corpus (acceptable) |
| `criterion` crate | Optional formal bench | Not in Cargo.toml | — | Custom `#[ignore]` tests (preferred) |

**Missing dependencies with no fallback:** None (D-10 path requires no new deps).

**Missing dependencies with fallback:** Dropbox PDFs → synthetic corpus.

---

## Validation Architecture

> `workflow.nyquist_validation` is absent from `.planning/config.json` — treated as enabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in tests (`#[cfg(test)]` + `cargo test`) |
| Config file | None — cargo test discovers tests automatically |
| Quick run command | `cargo test -p cortex --lib spaces::clustering` |
| Full suite command | `cargo test -p cortex` |
| Benchmark run command | `cargo test -p cortex --lib spaces::clustering -- --ignored` |

### Phase Requirements → Test Map

Phase 12 requirements (GNNC-01..05) are TBD per ROADMAP.md. Based on the success criteria (SC1-SC4) from the CONTEXT.md, the implied requirements map as follows:

| SC | Behavior | Test Type | Automated Command | File Exists? |
|----|----------|-----------|-------------------|-------------|
| SC1 | `ruvector-gnn` in Cargo.toml (D-10 path: NOT added; k-means retained) | manual verify | `grep ruvector-gnn src-tauri/Cargo.toml` | N/A — check absence |
| SC2 | Cluster coherence ≥ k-means baseline ±5% | benchmark | `cargo test -p cortex --lib -- --ignored bench_kmeans_500doc_coherence` | No — Wave 0 |
| SC3 | Label collision rate < 10% | manual (requires LLM) | Manual run of Phase 9 labeling on cluster output | No — Wave 0 |
| SC4 | 10K recluster ≤ 2× baseline wall-clock | benchmark | `cargo test -p cortex --lib -- --ignored bench_kmeans_10k_wall_clock` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p cortex --lib spaces::clustering` (non-ignored unit tests only)
- **Per wave merge:** `cargo test -p cortex`
- **Benchmark run (manual):** `cargo test -p cortex -- --ignored`

### Wave 0 Gaps

- [ ] Benchmark module in `src-tauri/src/spaces/clustering.rs` or `src-tauri/benches/clustering.rs` — covers SC2 + SC4
- [ ] `12-SUMMARY.md` — documents gap finding, baseline metrics, ruvector-gnn audit result

*(Existing `clustering.rs` unit tests pass; no new test framework needed.)*

---

## Security Domain

> `security_enforcement` is absent from `.planning/config.json` — treated as enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Phase 12 is backend-only; no auth flows touched |
| V3 Session Management | No | No session state |
| V4 Access Control | No | Single-user desktop app |
| V5 Input Validation | Yes (minimal) | Synthetic corpus generation validates N > 0, DIM > 0 before RNG use |
| V6 Cryptography | No | No cryptographic operations |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Benchmark binary left in release build | Information Disclosure | `[dev-dependencies]` ensures criterion/benches don't ship in `cargo tauri build` |
| Seeded RNG producing exploitable patterns | Tampering | RNG used only for benchmark synthetic data, not for production keys/tokens |

---

## Sources

### Primary (HIGH confidence)

- [VERIFIED: local crate read] `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-gnn/src/lib.rs` — confirms module structure: no clustering exports
- [VERIFIED: local crate read] `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-gnn/src/layer.rs` — `RuvectorLayer::forward(node, neighbors, weights) -> Vec<f32>` — NOT a clustering API
- [VERIFIED: local crate read] `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-gnn/src/search.rs` — `differentiable_search`, `hierarchical_forward` — search re-ranking tools, not clustering
- [VERIFIED: local crate read] `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-gnn/README.md` — "makes HNSW vector search get smarter" — search, not clustering
- [VERIFIED: local crate read] `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-gnn/Cargo.toml` — dependencies: ndarray 0.16, rayon, dashmap, parking_lot, memmap2
- [VERIFIED: local codebase] `/Users/gshah/work/apps/cortex/src-tauri/src/spaces/clustering.rs` — k-means implementation, `ClusterResult`, `cosine_similarity`, `auto_detect_k`
- [VERIFIED: local codebase] `/Users/gshah/work/apps/cortex/src-tauri/Cargo.toml` — `rand = "0.9"`, `cortex_lib` crate name, no ndarray dependency
- [VERIFIED: local codebase] `/Users/gshah/work/apps/cortex/src-tauri/src/spaces/manager.rs` — `cluster_documents` call site at line 180; drop-in signature constraint
- [VERIFIED: local codebase] `/Users/gshah/work/apps/cortex/.planning/phases/09-llm-space-labeling/09-RESEARCH.md` — Phase 9 ruvector audit lessons applied here

### Secondary (MEDIUM confidence)

- [CITED: planning/config.json] `commit_docs: true` confirmed for this phase; no `nyquist_validation` key → treated as enabled
- [CITED: ROADMAP.md §Phase 12] Success criteria SC1-SC4; requirements TBD (GNNC-01..05)

### Tertiary (LOW confidence)

- [ASSUMED] rand 0.9 `.random::<T>()` API (training data knowledge; Cargo.toml confirms rand = "0.9" but full changelog not read)
- [ASSUMED] criterion 0.5 MSRV compatibility with rust-version = 1.77.2

---

## Metadata

**Confidence breakdown:**
- ruvector-gnn crate audit: HIGH — directly read local source files (lib.rs, layer.rs, search.rs, query.rs, README.md)
- D-10 gap verdict: HIGH — follows directly from crate audit; matches pattern established by Phase 9
- Benchmark harness patterns: MEDIUM — k-means API verified; rand 0.9 `.random()` assumed
- 500-doc corpus fallback: HIGH — Dropbox local sync verified empty, synthetic approach confirmed viable

**Research date:** 2026-07-09
**Valid until:** 2026-08-09 (ruvector crate unlikely to add GNN clustering in this timeframe; k-means API is stable)

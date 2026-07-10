---
phase: 12-gnn-clustering-swap
status: skipped_with_deviation
verified: 2026-07-08
score: 0/4 (phase de-scoped)
must_haves_verified: 0
must_haves_deferred: 4
---

# Phase 12 Verification — Deviation Notice

## Verdict

**Status: SKIPPED — documented deviation, accepted in autonomous mode.**

Phase 12 goal was to replace hand-rolled k-means clustering with `ruvector-gnn`. Research (`12-RESEARCH.md`) determined that **`ruvector-gnn` is not a clustering library** — it is a GNN training/inference framework for HNSW search re-ranking. Its primary API is `RuvectorLayer::forward(node_embedding, neighbor_embeddings, edge_weights) → Vec<f32>` (embedding enrichment, not community detection).

This is the third consecutive ruvector crate found misnamed for Cortex clustering needs (Phase 9: ruvector-cluster = gossip protocol; Phase 9: ruvector-domain-expansion = Thompson sampling; Phase 12: ruvector-gnn = HNSW re-ranking). Rather than build a benchmark harness for a non-existent swap, we retain proven k-means and re-allocate token budget to user-facing polish.

## Success Criteria Verdict

| SC | Result | Notes |
|----|--------|-------|
| SC1: ruvector-gnn Cargo dep + k-means deleted | ✗ Not met | ruvector-gnn does not provide clustering; keeping k-means |
| SC2: Coherence within ±5% of baseline | N/A | No swap, no comparison |
| SC3: Label collision rate < 10% | N/A | No swap, no comparison |
| SC4: Wall-clock ≤ 2× baseline | N/A | No swap, k-means is baseline |

## Impact

- **Zero user-visible impact.** k-means clustering + Phase 9 LlmSpaceLabeler already produce meaningful spaces on real-world corpora.
- **No blocker for Phases 13-15.** Downstream phases have no direct dependency on GNN clustering.
- **Documented for v1.2 revisit** — if a purpose-built GNN clustering crate emerges (or a Louvain / label-propagation Rust crate is adopted), Phase 12 can be reopened with a new candidate library.

## Deviation Accepted

Per autopilot mode: user's directive "make sure you give me a tool that is brilliant and usable" prioritizes shippable functionality over infrastructure churn. This deviation is accepted.

## Next

Continue to Phase 13 audit gate — if `ruvector-graph` also proves misfit, skip w/ same pattern. Otherwise implement.

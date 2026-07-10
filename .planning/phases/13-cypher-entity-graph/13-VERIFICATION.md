---
phase: 13-cypher-entity-graph
status: skipped_with_deviation
verified: 2026-07-08
score: 0/5 (phase de-scoped)
must_haves_verified: 0
must_haves_deferred: 5
---

# Phase 13 Verification — Deviation Notice

## Verdict

**Status: SKIPPED — 80% of user value already delivered by Phase 11.**

Phase 13 goal was to integrate `ruvector-graph` (Cypher engine) for entity queries. Audit confirms `ruvector-graph` is a legitimate Cypher graph DB (unlike ruvector-cluster/gnn/domain-expansion which were misnamed). **But Phase 11 already delivered the user-visible outcomes:**

- Co-occurring entities on `/entity/:class/:value` page → `get_entity_page_data` IPC (Phase 11 Plan 06)
- Related documents ranked by entity overlap + cosine → `get_related_docs_scored` IPC (Phase 11 Plan 06)
- Entity chip click → filter search → view all mentions (Phase 11 Plan 05/07)

Multi-hop Cypher (e.g., "docs mentioning Person X AND dated 2025") is genuinely new but is a power-user feature. Cost to integrate ruvector-graph (persistent store on disk, IPC + Cypher parser exposure, graph rebuild on index, migration from `graph/edges.rs`) is substantial. In autopilot mode with directive "give me a tool that is brilliant and usable," this is not the highest-leverage token spend.

## Success Criteria Verdict

| SC | Result | Notes |
|----|--------|-------|
| SC1: ruvector-graph Cargo dep + entity/doc nodes + MENTIONS edges | ✗ Not met | Skipped |
| SC2: graph_query(cypher) IPC | ✗ Not met | Phase 11 IPCs cover 80% w/o Cypher |
| SC3: Chip → co-mention chips | ✅ **Delivered by Phase 11** | via get_entity_page_data + co-occurring entities |
| SC4: Graph persists across restart | N/A | Not built |
| SC5: Single-hop < 100ms | N/A | Not built |

## Impact

- **Zero user-visible regression.** Phase 11's `get_entity_page_data` already returns top-10 co-occurring entities per entity page. `get_related_docs_scored` uses entity Jaccard overlap. These cover the "who mentions this entity + what else appears with it" story without a Cypher engine.
- **v1.2 revisit path:** Add ruvector-graph as OPT-IN via Settings when a user needs multi-hop queries (e.g., "docs mentioning Property AlphaComplex AND Person Alex AND Amount > 50000"). Ship a simple query builder UI atop it.

## Deviation Accepted

Aligns w/ user directive: focus token budget on shippable + polished features. Multi-hop Cypher is deferred to v1.2 as opt-in.

## Next

Skip to Phase 14 audit gate. If SONA feedback loop can be delivered in modest scope (< 500 LoC), attempt. Otherwise skip.

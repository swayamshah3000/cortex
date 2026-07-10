---
phase: 10-hierarchical-spaces
verified: 2026-07-08T23:00:00Z
status: human_needed
score: 4/5
overrides_applied: 0
gaps:
  - truth: "Sub-space search uses ruvector-hyperbolic-hnsw for hierarchy-aware retrieval; navigating parent → child → grand-child returns results in ≤ 2× the time of a flat top-level search"
    status: partial
    reason: "ruvector-hyperbolic-hnsw is a Cargo.toml dependency and the crate is integrated. The SC5 perf gate passes in an isolated #[ignore] test (0ms vs 5462ms flat). However rebuild_hyp_index() is never called from recluster_spaces or any search path. Plan 10-06 SUMMARY explicitly states: 'The search path consumption (when parent_space_id filter is present) is Phase 11+ work.' The production search path does not use hyperbolic HNSW for any query. SC5 requires the index to be used for sub-space search; it is currently inert AppState state."
    artifacts:
      - path: "src-tauri/src/spaces/hyp_index.rs"
        issue: "rebuild_hyp_index() defined but never called from commands/spaces.rs or manager.rs"
      - path: "src-tauri/src/commands/spaces.rs"
        issue: "recluster_spaces does not invoke rebuild_hyp_index after manager.recluster() returns"
    missing:
      - "Call rebuild_hyp_index(&top_level_spaces, &state.hyp_index, &state.hyp_id_to_space).await after recluster in commands/spaces.rs::recluster_spaces"
      - "Wire hyp_index lookup in the search path when SearchFilters.parent_space_id is present (or accept as Phase 11 work via override)"
human_verification:
  - test: "Open /spaces/:id for a Space with > 50 documents; verify sub-spaces section appears with named cards"
    expected: "Sub-Spaces heading visible; 2+ SubSpaceCard entries with LLM-generated labels; Misc card (if any) has dashed left border"
    why_human: "Requires a real indexed corpus with > 50 documents in a single Space; cannot be verified with grep"
  - test: "Click the chevron next to a top-level Space in the Sidebar; verify sub-space list expands inline without page navigation"
    expected: "Chevron rotates 90°; indented list of sub-space names appears beneath the Space entry; clicking a sub-space navigates to /spaces/:id"
    why_human: "Interactive UI behavior; shadcn Collapsible animation requires browser runtime"
  - test: "Navigate to a sub-space /spaces/:id; verify breadcrumb and ParentContextBanner"
    expected: "Breadcrumb shows 'Spaces / {ParentName} / {SubName}'; ParentContextBanner banner shows 'Sub-space of {ParentName}' with clickable link back to parent"
    why_human: "Requires live data with actual parent-child Space relationships; cannot mock hierarchy in grep"
  - test: "Verify LLM sub-space labeling: after recluster on a corpus with a Space > 50 docs, sub-spaces receive LLM-generated labels"
    expected: "Sub-space names are 2-4 word labels derived from content (e.g., 'Property Tax' not 'Sub-space 1'); labels visible in SpaceDetailPage sub-space grid"
    why_human: "Requires active AI provider and real corpus; LLM output cannot be verified statically"
---

# Phase 10: Hierarchical Spaces Verification Report

**Phase Goal:** Large Smart Spaces automatically split into navigable sub-spaces, matching the mockSpaces hierarchy shape (e.g., Property → Tax → Insurance) — users can drill into sub-categories from both the sidebar and /spaces/:id. Underlying index becomes hierarchy-aware via `ruvector-hyperbolic-hnsw` so sub-space search stays logarithmic at depth.
**Verified:** 2026-07-08T23:00:00Z
**Status:** human_needed (SC5 is partial — hyperbolic index inert; 4 human UAT items needed)
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | HSPC-01: A Space > 50 docs automatically shows sub-spaces on /spaces/:id; Space < 50 shows none | VERIFIED | `subspace_detector::detect()` gates on `SUB_SPACE_THRESHOLD = 50`; called in `manager.rs::recluster()` step 10; `SpaceDetailPage.tsx` filters `s.parentId === space.id` to render sub-space grid |
| 2 | HSPC-02: Sub-spaces are LLM-labeled and clickable with parent breadcrumb | ? UNCERTAIN (human_needed) | Backend: `label_sub_cluster()` in `llm_labeler.rs` wired in manager sub-space loop; shadcn `Breadcrumb` + `ParentContextBanner` in `SpaceDetailPage.tsx`. Actual LLM labeling on real corpus requires human UAT |
| 3 | HSPC-03: Unclustered docs appear in "Misc" sub-space; no docs silently dropped | VERIFIED | `build_misc_space()` in `subspace_detector.rs`; manager wires it at step 10 (`if let Some(misc_cluster) = build_misc_space(...)`); hardcoded label "Misc"; `SubSpaceCard` with `isMisc` prop adds `border-dashed` |
| 4 | HSPC-04: Sidebar shows top 5 Spaces with sub-counts "(N)"; chevron expands sub-spaces inline | VERIFIED | `Sidebar.tsx` filters `!s.parentId`, sorts by `documentCount` desc, slices to 5; sub-count `(space.subSpaceIds.length)` rendered as `text-xs text-text-tertiary`; shadcn `Collapsible` with `toggleSpaceExpanded`; `useSidebarStore.expandedSpaceIds` in `stores.ts` |
| 5 | SC5: Sub-space search uses ruvector-hyperbolic-hnsw; parent→child→grand-child ≤ 2× flat top-level time | FAILED | `rebuild_hyp_index()` defined in `hyp_index.rs` but never called from `commands/spaces.rs` or any search path. SC5 perf gate passes as isolated `#[ignore]` test only. Plan 10-06 SUMMARY explicitly defers search path wiring to Phase 11+. No user query ever routes through the hyperbolic index. |

**Score:** 4/5 truths verified (SC5 partial/failed; HSPC-02 needs human UAT)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/spaces/subspace_detector.rs` | detect() + build_misc_space() + SUB_SPACE_THRESHOLD=50 | VERIFIED | Substantive; 5 unit tests; wired in manager.rs step 10 |
| `src-tauri/src/spaces/hyp_index.rs` | rebuild_hyp_index() + HypIndexState type aliases | VERIFIED (ORPHANED) | Exists, substantive (D-11 fallback, 3 unit tests, SC5 perf test). NOT wired into production call path. |
| `src-tauri/src/spaces/llm_labeler.rs::label_sub_cluster` | Parent-context sub-space LLM labeling | VERIFIED | `label_sub_cluster()` at line 340; `SUB_SPACE_LABEL_PREFIX` constant; called in manager sub-space loop |
| `src-tauri/src/spaces/label_cache.rs::SpaceLabelEntry.parent_id` | `parent_id: Option<String>` + `depth: u8` with serde defaults | VERIFIED | Both fields present at lines 70-75 with `#[serde(default)]` |
| `src-tauri/src/types.rs::Space.depth` | `depth: u8` + `sub_space_ids: Vec<String>` + `parent_id: Option<String>` | VERIFIED | All three fields present at lines 178, 189, 196; `#[serde(default)]` on new fields |
| `client/lib/types.ts::Space.depth` | `depth?: number` + `subSpaceIds?: string[]` | VERIFIED | Both fields at lines 153-155 with comment referencing D-03/D-07 |
| `client/lib/stores.ts::useSidebarStore.expandedSpaceIds` | `expandedSpaceIds: Set<string>` + `toggleSpaceExpanded` | VERIFIED | Present at lines 26-48; session-only (no persist middleware per D-13) |
| `client/components/spaces/SubSpaceCard.tsx` | Compact card with isMisc dashed variant | VERIFIED | Substantive; `isMisc` prop adds `border-dashed`; `Link to /spaces/:id`; icon 18px vs 24px |
| `client/components/spaces/ParentContextBanner.tsx` | "Sub-space of {parent}" banner with parent link | VERIFIED | Substantive; ArrowLeft + "Sub-space of" text + accent Link to parent |
| `client/pages/SpaceDetailPage.tsx` | shadcn Breadcrumb + ParentContextBanner + sub-space grid | VERIFIED | Breadcrumb shows 2-level hierarchy; ParentContextBanner conditional on `space.parentId && parentSpace`; sub-space grid via flat filter |
| `client/components/layout/Sidebar.tsx` | Top-5 filter + sub-count + Collapsible chevron + sub-list | VERIFIED | `!s.parentId` filter, `.slice(0, 5)`, `subSpaceIds.length` count, shadcn Collapsible, CSS rotate-90 chevron |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `subspace_detector::detect()` | `manager.rs::recluster()` step 10 | `subspace_detector::detect(parent_doc_ids, parent_vectors)` at line ~579 | WIRED | Called for each qualifying parent (> threshold) |
| `llm_labeler::label_sub_cluster()` | `manager.rs` sub-space loop | Direct call at line ~664 | WIRED | Called for `LlmLabel` decisions in sub-space pass |
| `SpaceDetailPage` | `useSpaces()` hook | `spaces.filter(s => s.parentId === space.id)` | WIRED | Flat filter correctly derives sub-spaces from flat list |
| `Sidebar.tsx` | `useSidebarStore.expandedSpaceIds` | `toggleSpaceExpanded(space.id)` in `onOpenChange` | WIRED | Collapsible open/close bound to store |
| `hyp_index::rebuild_hyp_index()` | `commands/spaces.rs::recluster_spaces` | MISSING — never called | NOT_WIRED | AppState fields initialized but function never invoked from production code |
| `Space.subSpaceIds` | `Sidebar.tsx` sub-count display | `space.subSpaceIds.length` | WIRED | Sub-count rendered when `subSpaceIds.length > 0` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `SpaceDetailPage.tsx` | `sortedSubSpaces` | `spaces.filter(s => s.parentId === space.id)` from `useSpaces()` | Yes — flat list from `get_spaces` IPC, which includes sub-spaces with `parentId` set after recluster | FLOWING |
| `Sidebar.tsx` | `sidebarSpaces` | `useSpaces()` filtered to `!s.parentId`, sorted, sliced | Yes — same IPC source; `subSpaceIds` populated by manager step 10 | FLOWING |
| `hyp_index.rs` | index search results | Never queried — `rebuild_hyp_index` not called | No — index stays `None` permanently in production | DISCONNECTED |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| TypeScript type check | `bunx tsc --noEmit` | exit 0 | PASS |
| Frontend test suite | `bunx vitest run` | 315/315 passed, 34 test files | PASS |
| Rust cargo check | `cargo check` | 0 errors, warnings only | PASS |
| SC5 perf gate | `cargo test -- --ignored --nocapture perf_gate` | Not run (explicit #[ignore] required, would need release build) | SKIP — claimed PASS in Plan 10-06 SUMMARY (0ms vs 5462ms) but test is isolated/ignored |

### Probe Execution

No probe scripts declared or found at `scripts/*/tests/probe-*.sh` for Phase 10.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| HSPC-01 | Plans 10-01, 10-03, 10-05 | Top-level Spaces auto-split when cluster exceeds 50 docs | SATISFIED | `detect()` gates on `SUB_SPACE_THRESHOLD=50`; wired in `recluster()` |
| HSPC-02 | Plans 10-04, 10-08 | Sub-spaces LLM-labeled and clickable with parent breadcrumb | PARTIALLY SATISFIED | Backend wired; UI wired; actual LLM output on real corpus → human_needed |
| HSPC-03 | Plans 10-03, 10-05, 10-08 | Unclustered docs surface in "Misc" sub-space | SATISFIED | `build_misc_space()` + manager wiring + `isMisc` dashed UI |
| HSPC-04 | Plans 10-02, 10-07 | Sidebar top 5 with sub-counts + inline expand | SATISFIED | `useSidebarStore.expandedSpaceIds` + Collapsible + filter/sort/slice |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src-tauri/src/spaces/manager.rs` | ~590 | Comment "Re-read vectors for this parent's docs" admits second collection read per parent for sub-clustering | INFO | Performance: each qualifying parent (> 50 docs) re-reads its docs from the collection. Acceptable for Phase 10; worth noting for Phase 12 GNN swap. |
| `src-tauri/src/spaces/hyp_index.rs` | 169 | `#[ignore = "run explicitly: ..."]` on SC5 perf gate | WARNING | SC5 cannot auto-run in CI; gate only passes when developer manually triggers with `--release`. No unreferenced TBD/FIXME/XXX markers found. |

No TBD, FIXME, or XXX markers found in Phase 10 modified files.

### Human Verification Required

### 1. Sub-space grid on /spaces/:id with real corpus

**Test:** Index a folder with > 50 documents of mixed sub-topics (e.g., ~/private/docs/ or a large work folder). Run recluster. Open the resulting Space in /spaces/:id.
**Expected:** "Sub-Spaces" section appears above Documents. 2+ SubSpaceCard entries with LLM-generated labels (2-4 words, content-derived, not "Sub-space 1"). If any sub-cluster was too small, a "Misc" card appears last with a dashed left border.
**Why human:** Requires an active AI provider and a real corpus with > 50 documents that cluster into sub-topics. Cannot replicate with grep or unit tests.

### 2. Sidebar chevron expand behavior

**Test:** After recluster produces a Space with sub-spaces, open the Sidebar. Find the top-5 Space entry. Hover to reveal the chevron. Click the chevron.
**Expected:** Chevron animates to rotate-90°. Indented sub-space list appears below the parent entry without any page navigation. Clicking a sub-space link navigates to /spaces/:sub-id. Second chevron click collapses the list.
**Why human:** Interactive Collapsible animation and state are browser-runtime behaviors. Sidebar layout and hover reveal require visual inspection.

### 3. Breadcrumb and ParentContextBanner on sub-space detail page

**Test:** Navigate to /spaces/:sub-id for a sub-space (with parentId set).
**Expected:** Breadcrumb displays "Spaces / {ParentName} / {SubName}" — "Spaces" links to /spaces, parent name links to /spaces/:parent-id, current name is plain text. Below breadcrumb, ParentContextBanner shows "Sub-space of {ParentName}" with an ArrowLeft icon and the parent name as a clickable link.
**Why human:** Requires live hierarchy data; navigation between /spaces/:parent and /spaces/:sub must be manually traced.

### 4. LLM sub-space label quality on real corpus

**Test:** After recluster with an active AI provider (Anthropic, OpenAI, or Ollama), inspect sub-space labels generated for a corpus-derived Space > 50 docs.
**Expected:** Sub-space labels are 2-4 word, content-derived, distinct from the parent label (e.g., "Property Tax Records" → sub: "Municipal Assessments", "Insurance Claims"). Not generic ("Sub-space 1", "Cluster A"). The parent-context prompt variant (SUB_SPACE_LABEL_PREFIX) should produce labels visibly nested in concept.
**Why human:** LLM output quality cannot be verified statically. Requires active provider and real document content.

---

## Gaps Summary

**1 BLOCKER-level gap found (SC5 wiring):**

SC5 requires sub-space search to use `ruvector-hyperbolic-hnsw`. The crate is a Cargo dependency, `hyp_index.rs` implements the integration, and AppState fields are initialized — but `rebuild_hyp_index()` is never called from `commands/spaces.rs::recluster_spaces` or any search path. The index stays at `None` for every user query. This is explicitly documented in Plan 10-06 SUMMARY as "Future Wiring (Phase 11+)."

The SC5 perf gate (0ms vs 5462ms flat baseline) only runs as an ignored unit test in isolation — it does not prove production wiring.

**If this deviation is intentional** (hyperbolic wiring deferred to Phase 11), add an override:

```yaml
overrides:
  - must_have: "Sub-space search uses ruvector-hyperbolic-hnsw for hierarchy-aware retrieval; navigating parent child grand-child returns results in ≤ 2× the time of a flat top-level search"
    reason: "ruvector-hyperbolic-hnsw integrated and SC5 perf gate passes in isolation (0ms vs 5462ms). Production search-path wiring deferred to Phase 11 per Plan 10-06 SUMMARY — Phase 11 owns parent_space_id filter consumption."
    accepted_by: "{your name}"
    accepted_at: "{ISO timestamp}"
```

Then re-run verification to apply.

**4 human UAT items** are required for full sign-off (HSPC-02 LLM labeling quality, Sidebar chevron, breadcrumb navigation, and sub-space grid on real corpus).

---

*Verified: 2026-07-08T23:00:00Z*
*Verifier: Claude (gsd-verifier)*

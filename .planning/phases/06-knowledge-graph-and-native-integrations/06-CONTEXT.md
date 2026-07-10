# Phase 6: Knowledge Graph and Native Integrations - Context

**Gathered:** 2026-06-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Cortex moves from "doc auto-organizer" to "knowledge-graph-backed personal brain."

Three workstreams in this phase:

1. **Knowledge graph** — promote extracted entities (Date, Amount, Person, Organization, Location, Email) to first-class graph nodes that the user can click to surface every document mentioning them. Includes alias normalization, a dedicated `/entities` index, an `/entities/:id` detail page, and IPC queries to power them.
2. **Native folder picker** — replace the manual path text-input in WatchedPage with a real OS folder picker via `tauri-plugin-dialog`.
3. **In-app preview + Open in OS** — render PDF / image / text / markdown previews on DocumentPage (replacing the 200-char excerpt), and add "Open file" / "Reveal in Finder" actions on DocumentPage, search results, and recent documents via `tauri-plugin-opener`.

**Out of scope:** new entity types beyond Person/Organization/Location/Date/Amount/Email; DOCX/XLSX in-app preview (Open in OS is sufficient); cloud-based NER; entity merging across types; geocoding.

</domain>

<decisions>
## Implementation Decisions

### Entity Extraction
- **D-01:** Use a local ONNX NER model (e.g., `dslim/bert-base-NER`) for Person / Organization / Location. Same pattern as the existing fastembed pipeline — load once, call inside `spawn_blocking`. No Ollama dependency.
- **D-02:** Keep the existing regex extractors for Date / Amount / Email (`pipeline/entities.rs`) — they are deterministic and fast for structured forms where bert-base-NER is mediocre. Merge regex + NER results, dedup by `(value, entity_type)`.
- **D-03:** Backfill existing indexed documents on app startup as a Tokio task. Emit progress as a Tauri event (mirrors the existing indexing event flow from Phase 5). UI stays responsive; user sees backfill in the TopBar indicator.
- **D-04:** Per-doc entity cap of 20 from current `entities.rs` stays; bert-base-NER outputs are merged into the same cap.

### Entity Normalization
- **D-05:** Alias merging uses **embedding similarity** — embed each entity surface form using the same fastembed model used for documents, cluster aliases by cosine ≥ 0.85. Reuses existing embedding infra; catches semantic variants like "John Smith" / "J. Smith" / "Smith, John".
- **D-06:** Merge pass runs (a) once after the NER backfill completes, and (b) incrementally on every new document — embed new entities and check against the existing canonical set. No full O(n²) re-cluster on every doc.
- **D-07:** Canonical surface form is **most frequent** — the variant that appears in the most documents wins. Deterministic, no UI needed.
- **D-08:** Wrong merges are recoverable via a **Split alias** action on `/entities/:id`. Shows the alias list with per-alias split buttons. Defensive against false-positive merges (e.g., two distinct "John Smith" people).

### Entity Click-Through UX
- **D-09:** Click an entity chip on DocumentPage → navigate to a dedicated **`/entities/:id`** route. Page shows: canonical name + type badge, alias list (with Split action), "Documents mentioning this" list, "Related entities" panel. Mirrors the SpaceDetailPage pattern. Bookmarkable.
- **D-10:** Add an **`/entities` index** to the sidebar (placed under "Tags"). Shows all entities grouped by type, sorted by document count. Click any → `/entities/:id`. Makes the knowledge graph discoverable.
- **D-11:** "Related entities" computed via **co-occurrence in same document** — two entities are related if they appear together in ≥ 2 documents, ranked by co-occurrence count. Cheap, deterministic, no extra embedding work.
- **D-12:** User actions on `/entities/:id` are scoped to **Rename canonical** + **Split alias**. No manual "merge two entities" action — auto-merge handles that. No hide action.

### File Preview
- **D-13:** **PDF**: render via Tauri asset protocol (`convertFileSrc(path)`) inside an `<iframe>` or `<embed>`. Uses WebView's built-in PDF viewer — zero new dependencies, full zoom/scroll/find. Streams large PDFs natively.
- **D-14:** **Image**: `<img src={convertFileSrc(path)} />`. **Text / code**: monospace `<pre>` block with syntax highlighting if a highlighter (Prism/Shiki) is already in deps, else plain. **Markdown**: render via `react-markdown` (add as new dep). All three covered to honor PAGE-13.
- **D-15:** **Size guard — soft limit with "Load anyway"**: PDFs > 50 MB, text > 5 MB, images > 20 MB show a placeholder card: "Large file (X MB) — [Load preview] [Open in default app]". User chooses. Prevents WebView freeze on huge files.
- **D-16:** Both backend (read content + emit metadata) and frontend (per-type renderer components) live behind a `usePreview(documentId)` hook so adding DOCX/XLSX previews later is incremental.

### Open in OS
- **D-17:** Use **`tauri-plugin-opener`** for both `openPath` (open with default app) and `revealItemInDir` (Reveal in Finder / Show in Explorer).
- **D-18:** Action surfaces: **DocumentPage header** (two visible buttons), and **right-click context menu** on document rows in `/search`, `/recent`, `/favorites`, `/spaces/:id`. No always-visible inline icons (visual noise).

### Native Folder Picker
- **D-19:** Replace the `@tauri-apps/plugin-dialog` dynamic import + ts-ignore hack in `client/pages/WatchedPage.tsx` with a proper dependency. Single-folder select only (no multi-select). On cancel, do nothing silently. Validate the returned path exists and is a directory before submitting to `addWatchedFolder`.

### Claude's Discretion
- Exact ONNX NER model file path / quantization choice (e.g., int8 vs fp16) — pick based on size + accuracy benchmarks during research.
- Storage schema for entities: separate RuVector collection vs. SQLite vs. in-memory HashMap rebuilt on startup. Lean toward persistent storage since NER backfill is expensive — but planner decides exact mechanism.
- Co-occurrence threshold for "Related entities" (chose ≥ 2 as a starting heuristic — planner may tune).
- IPC command shape: `get_entities_by_type`, `get_entity(id)`, `get_documents_for_entity(id)`, `get_related_entities(id)`, `rename_entity_canonical`, `split_entity_alias`. Exact signatures finalized in plan.
- Sidebar icon for Entities (suggest Lucide `Network` or `GitBranch`).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project specs
- `.planning/ROADMAP.md` §"Phase 6: Knowledge Graph and Native Integrations" — phase goal, requirements list (KG-01..KG-05, UX-05, PAGE-13, UX-06), success criteria
- `.planning/REQUIREMENTS.md` — note that KG-* / UX-05 / PAGE-13 / UX-06 are **not yet listed** in REQUIREMENTS.md; roadmap goal text is authoritative. Plan must add them to REQUIREMENTS.md.
- `.planning/PROJECT.md` §Constraints — privacy-first (no cloud calls by default), local-only NER required
- `CLAUDE.md` §"How RuVector Powers Cortex" — entity extraction listed under Doc Parser layer; RuVector graph crate available if needed

### Existing code (must read before modifying)
- `src-tauri/src/pipeline/entities.rs` — current regex extractor (date / amount / person / email). Stays. NER runs alongside.
- `src-tauri/src/pipeline/embedder.rs` — fastembed initialization pattern. Mirror for NER model load.
- `src-tauri/src/pipeline/indexer.rs` — current ingest flow; NER hooks in here.
- `src-tauri/src/graph/edges.rs` — DocumentGraph in-memory adjacency, edges built from shared entities. Extend for entity nodes or build sibling EntityGraph.
- `src-tauri/src/graph/related.rs` — `get_related_impl` pattern for `get_related_documents`. Mirror for `get_related_entities`.
- `src-tauri/src/types.rs` — `ExtractedEntity` struct already has `label / value / entity_type`. Extend with canonical ID once entities become first-class.
- `src-tauri/src/commands/documents.rs` — IPC command pattern (serde camelCase, async, spawn_blocking).
- `client/pages/DocumentPage.tsx` — entity chips with `entityTypeIcon()` already render; click is a no-op currently. "Open in Finder" placeholder comment.
- `client/pages/WatchedPage.tsx` — dynamic `@tauri-apps/plugin-dialog` import with ts-ignore + text-input fallback. Replace with proper dep.
- `client/hooks/useTauri.ts` — React Query hook factory pattern; add entity hooks here.
- `client/components/layout/Sidebar.tsx` — sidebar nav structure; add "Entities" link under "Tags".

### Tauri plugins (new deps to add)
- `tauri-plugin-dialog` — folder picker (replace ts-ignore hack)
- `tauri-plugin-opener` — `openPath` and `revealItemInDir`

### Patterns to mirror
- Phase 3 entity-as-side-channel pattern (`graph/edges.rs` shared-entity edge weighting) — extend for entity-as-node
- Phase 4 hook factory pattern (`useTauri.ts`) — adopt for entity queries
- Phase 5 background indexing + Tauri event pattern — adopt for NER backfill progress

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `fastembed` integration in `pipeline/embedder.rs` — same load-once-via-spawn_blocking pattern transfers directly to the NER model. Reuse the embed model for entity-value embeddings too (no second model needed).
- `DocumentGraph` in `graph/edges.rs` already weights doc-doc edges by shared entities — that weighting logic can be re-projected to produce co-occurrence counts between entities.
- `entityTypeIcon()` in `DocumentPage.tsx` — color-coded icon mapping already implemented for date / amount / person / organization / location; just needs Organization to actually be emitted by the extractor and a click handler wired.
- React Query `useDocument`, `useRelatedDocuments`, `useSpaces` hooks in `client/hooks/useTauri.ts` — mirror the same factory for `useEntity`, `useEntitiesByType`, `useRelatedEntities`.
- `ResizablePanelGroup` in DocumentPage is already split-pane ready — slot the in-app preview into the existing 65% panel.

### Established Patterns
- All IPC commands use `#[tauri::command] async + spawn_blocking` for CPU-bound work and `#[serde(rename_all = "camelCase")]` (Phase 1 / Phase 4 decision). New entity commands must match.
- Settings persist via JSON file under app data dir (Phase 5). Entity canonical-name overrides should persist the same way OR as a separate `entities.json` sidecar — planner decides.
- Sidebar collapsed state in Zustand store with persist middleware (Phase 4). No new store needed for entities — React Query is enough.
- Tauri events for long-running tasks (`indexing-progress` from Phase 5). Add `entity-backfill-progress` event.

### Integration Points
- `pipeline/indexer.rs` — hook NER into the per-document ingest after parser, before embedding finalize.
- `engine.rs` / `state.rs` — `AppState` gains an `EntityGraph` field (or extends `DocumentGraph` if planner prefers).
- `commands/mod.rs` — register new entity commands and the rename/split commands.
- `client/App.tsx` — register `/entities` and `/entities/:id` routes inside the AppShell route group.
- `client/components/layout/Sidebar.tsx` — add "Entities" nav item.
- WatchedPage `addWatchedFolder` flow — swap the ts-ignore dynamic import for the proper plugin-dialog dep + remove the text-input fallback.

</code_context>

<specifics>
## Specific Ideas

- User asked about LLM-based extraction directly — surfaced ONNX NER as the local-first answer; LLM (Ollama) explicitly rejected for the hot path due to seconds-per-doc latency at the 10K-document scale.
- Roadmap KG-01 lists 5 entity types as first-class. Current `entities.rs` only emits 3 distinct types (Person, Date, Amount; Email is mapped to "person"; no Organization or Location emitted). NER closes the gap — Organization + Location actually start being emitted in Phase 6.
- "123 Main St" merging with "Main Street property" called out in roadmap success criterion #2 — embedding-based normalization (D-05) handles this kind of semantic alias.
- Roadmap success #5 says "Open in Finder / Open with default app works from Document detail **and search results**" — both surfaces required; recent / favorites / space-detail context menus added as bonus consistency.

</specifics>

<deferred>
## Deferred Ideas

- DOCX / XLSX in-app preview — not in PAGE-13's literal list (PDF / image / text / markdown). Open in OS will cover them. Defer to a future phase.
- Manual cross-entity merge action ("merge these two entities I know are the same") — auto-merge plus split-alias covers v1. Add later if users hit cases where embedding similarity misses.
- "Hide entity" action (entities user explicitly doesn't care about — e.g., boilerplate signatures). Defer until users complain.
- Geocoding for Location entities — would let "123 Main St" map to coordinates and a real map. Out of scope for v1.
- Entity-driven smart-space generation — entities could seed new Smart Spaces (e.g., "everything about John Smith" as a virtual space). Future phase.
- DOCX/XLSX-aware preview thumbnails — render to image server-side. Future phase.
- Pinned/favorite entities in the sidebar — possible UX once `/entities` index is in use.

</deferred>

---

*Phase: 06-knowledge-graph-and-native-integrations*
*Context gathered: 2026-06-29*

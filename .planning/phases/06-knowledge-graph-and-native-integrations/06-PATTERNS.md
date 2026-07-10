# Phase 6: Knowledge Graph and Native Integrations - Pattern Map

**Mapped:** 2026-06-29
**Files analyzed:** 35 (new + modified)
**Analogs found:** 33 / 35

This document maps each new/modified file in Phase 6 to its closest existing analog in the Cortex codebase. Planner consumes this to write per-plan action sections that say "copy lines X-Y from file Z" instead of "follow the pattern."

---

## File Classification

### Backend (Rust) — `src-tauri/src/`

| New / Modified File | Role | Data Flow | Closest Analog | Match |
|---------------------|------|-----------|----------------|-------|
| `src-tauri/src/pipeline/ner.rs` (NEW) | inference-service / module | CPU-bound transform (text → entities) | `src-tauri/src/pipeline/embedder.rs` | exact-role |
| `src-tauri/src/pipeline/entities.rs` (MODIFY) | extractor / module | text → entity list | self (extend in-place) | self-extend |
| `src-tauri/src/pipeline/backfill.rs` (NEW) | background-task / module | event-driven (Tokio task + Tauri emit) | `src-tauri/src/commands/folders.rs` `trigger_scan` lines 81-167 | role-match |
| `src-tauri/src/graph/entity_store.rs` (NEW) | in-memory graph store / module | reverse-index lookups | `src-tauri/src/graph/edges.rs` (`DocumentGraph`) | exact-role |
| `src-tauri/src/graph/related.rs` (MODIFY OR sibling `entity_related.rs`) | graph traversal helper | id → ranked list | self (existing `get_related_impl`) | self-extend |
| `src-tauri/src/commands/entities.rs` (NEW) | Tauri command module | request-response IPC | `src-tauri/src/commands/documents.rs` | exact-role |
| `src-tauri/src/commands/documents.rs` (MODIFY) | Tauri commands | add `read_document_text`, `open_path`, `reveal_item_in_dir` wrappers (or use plugin directly from FE) | self | self-extend |
| `src-tauri/src/commands/folders.rs` (MODIFY) | Tauri commands | optional `validate_folder_path` helper | self | self-extend |
| `src-tauri/src/commands/mod.rs` (MODIFY) | module registry | one-line addition | self | self-extend |
| `src-tauri/src/state.rs` (MODIFY) | AppState struct | add `Arc<NerService>` + `Arc<Mutex<EntityStore>>` + `Arc<Mutex<BackfillStatus>>` | self | self-extend |
| `src-tauri/src/lib.rs` (MODIFY) | app bootstrap | wire new services, register new commands, init plugins, spawn backfill | self (existing setup block lines 28-112) | self-extend |
| `src-tauri/src/pipeline/indexer.rs` (MODIFY) | document indexer | hook NerService into `index_file` after embedding; add `backfill_entities(doc_id, ...)` helper | self | self-extend |
| `src-tauri/src/error.rs` (MODIFY) | error enum | optional new variant `Ner(String)` (or reuse `Embedding`) | self | self-extend |
| `src-tauri/src/types.rs` (MODIFY) | shared serde types | add `CanonicalEntity`, `EntitySummary`, `RelatedEntity`, `EntityBackfillProgress`, `DocumentTextPreview` | self | self-extend |
| `src-tauri/Cargo.toml` (MODIFY) | manifest | add `ort`, `tokenizers`, `ndarray`, `tauri-plugin-dialog`, `tauri-plugin-opener` | self | self-extend |
| `src-tauri/capabilities/default.json` (MODIFY) | capability config | add dialog + opener permissions | self | self-extend |
| `src-tauri/tauri.conf.json` (MODIFY) | tauri config | add asset protocol scope, tighten CSP, add `bundle.resources` for `models/*` | self | self-extend |
| `src-tauri/models/bert-base-NER.onnx` (NEW asset) | model file | n/a — binary asset | none | n/a |
| `src-tauri/models/tokenizer.json` (NEW asset) | tokenizer file | n/a | none | n/a |
| `src-tauri/models/config.json` (NEW asset) | id2label map | n/a | none | n/a |

### Frontend (React/TS) — `client/`

| New / Modified File | Role | Data Flow | Closest Analog | Match |
|---------------------|------|-----------|----------------|-------|
| `client/pages/EntitiesPage.tsx` (NEW) | route page | React Query → grouped grid | `client/pages/SpacesPage.tsx` (lines 99-213) + `client/pages/TagsPage.tsx` (filter pattern) | exact-role |
| `client/pages/EntityDetailPage.tsx` (NEW) | route page | React Query → sectioned detail with mutation actions | `client/pages/SpaceDetailPage.tsx` (lines 101-233) | exact-role |
| `client/pages/DocumentPage.tsx` (MODIFY) | route page | adds preview + open/reveal + clickable entities | self | self-extend |
| `client/pages/WatchedPage.tsx` (MODIFY) | route page | swap dynamic-import for plugin-dialog | self | self-extend |
| `client/components/entities/EntityChip.tsx` (NEW) | leaf component | render + Link | inline entity rendering in `DocumentPage.tsx` lines 259-274; tag pill `DocumentPage.tsx` lines 246-253 | role-match |
| `client/components/entities/EntityTypeBadge.tsx` (NEW) | leaf component | pure render | doc-type badge `DocumentPage.tsx` lines 125-127 | role-match |
| `client/components/entities/EntityCard.tsx` (NEW) | leaf component | Link-wrapped card | `SubSpaceCard` in `SpaceDetailPage.tsx` lines 61-80; `SpaceCard` in `SpacesPage.tsx` lines 40-76 | exact-role |
| `client/components/entities/EntityTypeFilterBar.tsx` (NEW) | leaf component | pill toggle group | `FilterChip` in `SearchPage.tsx` lines 36-58 | role-match |
| `client/components/entities/EntityDetailHeader.tsx` (NEW) | composite component | inline-edit header | `SpaceDetailPage.tsx` header lines 174-187 | role-match |
| `client/components/entities/AliasChipList.tsx` (NEW) | composite | flex-wrap chip list | tags row `DocumentPage.tsx` lines 245-253 | role-match |
| `client/components/entities/AliasChip.tsx` (NEW) | leaf | chip with hover-revealed action | tag pill + `WatchedPage.tsx` action buttons lines 317-358 | role-match |
| `client/components/entities/RelatedEntityChip.tsx` (NEW) | leaf | EntityChip + count badge | uses EntityChip (above) | role-derived |
| `client/components/entities/SplitAliasDialog.tsx` (NEW) | dialog | shadcn AlertDialog wrapper | `WatchedPage.tsx` `renderConfirmDialog` lines 230-258 (custom dialog), shadcn alert-dialog is preferred | role-match |
| `client/components/documents/DocumentContextMenu.tsx` (NEW) | wrapper component | radix context menu | `client/components/ui/context-menu.tsx` (primitive) | uses-primitive |
| `client/components/documents/DocumentRow.tsx` (NEW — extract) | leaf component | row link | `SpaceDetailPage.tsx` `DocumentRow` lines 46-59 | self-extract |
| `client/components/preview/FilePreview.tsx` (NEW) | dispatcher | type-switch render | mock-data type maps in `RecentPage.tsx` lines 15-28 (icons-by-type pattern) | role-match |
| `client/components/preview/PdfPreview.tsx` (NEW) | leaf | iframe | none in codebase — see RESEARCH.md Example 4 | no-analog |
| `client/components/preview/ImagePreview.tsx` (NEW) | leaf | img tag | none — see RESEARCH.md | no-analog |
| `client/components/preview/TextPreview.tsx` (NEW) | leaf | pre block + fetch | none — see RESEARCH.md | no-analog |
| `client/components/preview/MarkdownPreview.tsx` (NEW) | leaf | react-markdown | none — see RESEARCH.md Example 5 | no-analog |
| `client/components/preview/SizeGuardCard.tsx` (NEW) | leaf | card with 2 buttons | empty-state cards across pages (e.g., `WatchedPage.tsx` lines 155-173) | role-match |
| `client/components/preview/UnsupportedPreview.tsx` (NEW) | leaf | card with 2 buttons | same as SizeGuardCard | role-match |
| `client/components/layout/BackfillIndicator.tsx` (NEW) | leaf | TopBar chip | indexing chip in `TopBar.tsx` lines 37-56 | exact-role |
| `client/components/layout/Sidebar.tsx` (MODIFY) | layout | add link to `bottomLinks` | self | self-extend |
| `client/components/layout/TopBar.tsx` (MODIFY) | layout | add BackfillIndicator slot | self | self-extend |
| `client/hooks/useTauri.ts` (MODIFY) | hook factory | add ~10 entity/preview hooks | self (existing factories) | self-extend |
| `client/hooks/usePreview.ts` (NEW) | hook | fetch + size-guard | `useDocument` factory in `useTauri.ts` lines 135-146 | role-match |
| `client/hooks/useBackfillProgress.ts` (NEW) | hook | listen to Tauri event → Zustand | event listener in `WatchedPage.tsx` lines 45-68 + `useIndexingStore` in `stores.ts` lines 60-79 | role-match |
| `client/lib/stores.ts` (MODIFY) | Zustand stores | add `useBackfillStore` | `useIndexingStore` in self lines 44-79 | self-extend |
| `client/lib/types.ts` (MODIFY) | shared types | add CanonicalEntity, EntitySummary, RelatedEntity, EntityBackfillProgress, DocumentTextPreview | self | self-extend |
| `client/App.tsx` (MODIFY) | router | add 2 routes inside AppShell group | self lines 33-53 | self-extend |
| `client/global.css` (POSSIBLY MODIFY) | tokens | optionally add `@plugin "@tailwindcss/typography"` if not already enabled | self | self-extend |
| `package.json` (MODIFY) | manifest | add 4 npm deps | self | self-extend |

### Files With No Direct Analog (Use RESEARCH.md Instead)

| File | Role | Why no analog | Source to copy from |
|------|------|---------------|---------------------|
| `client/components/preview/PdfPreview.tsx` | iframe + convertFileSrc | First use of asset protocol in codebase | RESEARCH.md "Pattern 2" + "Code Examples > Example 4" |
| `client/components/preview/MarkdownPreview.tsx` | react-markdown | First markdown render in codebase | RESEARCH.md "Code Examples > Example 5" + "Markdown Pipeline" |

---

## Pattern Assignments

### `src-tauri/src/pipeline/ner.rs` (NEW — inference service)

**Analog:** `src-tauri/src/pipeline/embedder.rs` (entire file, 113 lines).

**Imports + struct shape pattern** (embedder.rs lines 1-13):
```rust
use crate::error::AppError;

/// Local embedding service wrapping fastembed's TextEmbedding model.
pub struct EmbeddingService {
    model: std::sync::Mutex<fastembed::TextEmbedding>,
    pub dimensions: usize,
}
```
Copy this exact "single Mutex wrapping an inference handle + public scalar metadata" shape for `NerService { session: Mutex<ort::Session>, tokenizer: Tokenizer, id2label: Vec<String> }`.

**Constructor pattern with AppError mapping** (embedder.rs lines 18-28):
```rust
pub fn new_local() -> Result<Self, AppError> {
    let model = fastembed::TextEmbedding::try_new(
        fastembed::InitOptions::new(fastembed::EmbeddingModel::AllMiniLML6V2),
    )
    .map_err(|e| AppError::Embedding(e.to_string()))?;

    Ok(Self {
        model: std::sync::Mutex::new(model),
        dimensions: 384,
    })
}
```
For `NerService::new(model_path, tokenizer_path)`, mirror exactly — `ort::Session::builder().commit_from_file(model_path).map_err(|e| AppError::Embedding(e.to_string()))`.

**Inference pattern with lock + truncate** (embedder.rs lines 32-45):
```rust
pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, AppError> {
    let chunk = truncate_to_chars(text, 2000);
    let mut model = self
        .model
        .lock()
        .map_err(|e| AppError::Embedding(e.to_string()))?;
    let mut results = model
        .embed(vec![chunk.as_str()], None)
        .map_err(|e| AppError::Embedding(e.to_string()))?;
    if results.is_empty() {
        return Err(AppError::Embedding("Empty embedding result".to_string()));
    }
    Ok(results.remove(0))
}
```
Mirror for `extract(&self, text: &str) -> Result<Vec<ExtractedEntity>, AppError>` — chunk text (by sentence per RESEARCH Pitfall 1, not by chars), acquire lock, run inference, decode BIO with `encoding.get_offsets()` (Pitfall 2).

**Test pattern (#[ignore] for model-load tests)** (embedder.rs lines 80-112): Use the same `#[test] #[ignore]` annotation for the test that loads the .onnx model so CI doesn't download/load it; keep one fast unit test for the BIO decode helper that doesn't need the model.

---

### `src-tauri/src/pipeline/entities.rs` (MODIFY — extend extractor)

**Analog:** self.

**Existing pattern to extend** (entities.rs lines 33-78): The current `EntityExtractor::extract(text)` returns `Vec<ExtractedEntity>` and applies sort+dedup+truncate-at-20 at the end. Add an optional `ner: Option<&NerService>` parameter (or a new `extract_with_ner` method) that calls `ner.extract(text)?` and appends those results before the dedup pass. Keep the existing regex passes unchanged. Per CONTEXT D-04, the 20-entity cap (line 75) stays.

Dedup must change: current dedup is by `value` only (line 72). Per CONTEXT D-02, change to dedup by `(value, entity_type)` — modify line 72:
```rust
// BEFORE:
entities.dedup_by(|a, b| a.value == b.value);
// AFTER:
entities.dedup_by(|a, b| a.value == b.value && a.entity_type == b.entity_type);
```

Email entity_type fix per CONTEXT (KG-01 requires Email as first-class): change line 58 from `entity_type: "person"` to `entity_type: "email"`. This is a behavior change — add a test asserting `entity_type == "email"`.

---

### `src-tauri/src/pipeline/backfill.rs` (NEW — Tokio task + Tauri event emit)

**Analog:** `src-tauri/src/commands/folders.rs` `trigger_scan` (lines 81-167) — the existing pattern for "spawn background work + emit progress events".

**Spawn + emit pattern** (folders.rs lines 81-107):
```rust
tauri::async_runtime::spawn(async move {
    // ... walk files ...
    for file_path in entries {
        // ... per-item work ...
        let _ = app_handle.emit("index-progress", IndexProgress {
            file_path: path_str.clone(),
            status: "indexing".to_string(),
            doc_id: None,
            error: None,
            folder_id: Some(fid.clone()),
        });

        let eng = engine.clone();
        let emb = embedding_service.clone();
        let idx = indexer.clone();
        let fp = file_path.clone();

        let result = tokio::task::spawn_blocking(move || {
            let engine_guard = eng.blocking_lock();
            idx.index_file(&fp, &engine_guard, &emb)
        }).await;
        // ... match result, emit per-status events ...
    }

    // Final "complete" emit
    let _ = app_handle.emit("index-progress", IndexProgress {
        file_path: folder_config.path,
        status: "complete".to_string(),
        // ...
    });
});
```
Mirror exactly for `spawn_entity_backfill`:
1. Outer `tauri::async_runtime::spawn`.
2. Inner `tokio::task::spawn_blocking` for each NER-per-doc call (CPU-bound).
3. Per-event `app_handle.emit("entity-backfill-progress", EntityBackfillProgress { processed, total, status, error })`.
4. Final "complete" emit after loop.

**Throttling** (new — not in analog): Per RESEARCH Anti-Pattern, emit only every 25 docs OR every 500ms (whichever is sooner). See RESEARCH Example 1 code block lines 633-661 for exact throttle structure.

**Event payload struct pattern** (worker.rs lines 18-26 — `IndexProgress`):
```rust
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub file_path: String,
    pub status: String,
    pub doc_id: Option<String>,
    pub error: Option<String>,
    pub folder_id: Option<String>,
}
```
Mirror for `EntityBackfillProgress { processed: u32, total: u32, status: String, error: Option<String> }` — same `#[derive(Clone, Serialize)] #[serde(rename_all = "camelCase")]`.

---

### `src-tauri/src/graph/entity_store.rs` (NEW — in-memory graph store)

**Analog:** `src-tauri/src/graph/edges.rs` (`DocumentGraph` — 367 lines).

**Struct shape + new() pattern** (edges.rs lines 27-37):
```rust
pub struct DocumentGraph {
    /// doc_id -> list of edges from that doc
    edges: HashMap<String, Vec<DocumentEdge>>,
}

impl DocumentGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
        }
    }
    // ...
}

impl Default for DocumentGraph {
    fn default() -> Self {
        Self::new()
    }
}
```
Mirror for:
```rust
pub struct EntityStore {
    canonicals: HashMap<String, CanonicalEntity>,   // canonical_id -> entity
    alias_map: HashMap<String, String>,             // surface_form_lowercase -> canonical_id
    reverse_index: HashMap<String, HashSet<String>>,// canonical_id -> doc_ids
}
```

**Rebuild-from-collection pattern** (edges.rs `build_edges` lines 47-87, especially the metadata extraction lines 54-87):
```rust
let collection_arc = engine
    .collections
    .get_collection("documents_384")
    .ok_or_else(|| {
        AppError::VectorStorage("documents_384 collection not found".to_string())
    })?;

let collection = collection_arc.read();
let all_ids = collection
    .db
    .keys()
    .map_err(|e| AppError::VectorStorage(e.to_string()))?;

for id in &all_ids {
    let entry = collection.db.get(id).map_err(|e| AppError::VectorStorage(e.to_string()))?;
    if let Some(entry) = entry {
        if let Some(metadata) = entry.metadata {
            // ... extract relevant field ...
        }
    }
}
```
Mirror for `EntityStore::rebuild_from_collection(&CortexEngine)` that scans every doc, extracts `extracted_entities` metadata, and populates the three indexes. This is the exact pattern used in `DocumentIndexer::rebuild_path_index` (`src-tauri/src/pipeline/indexer.rs` lines 33-66) — see that function for an alternative cleaner shape using collection.db.get inside a separate scope to release the read lock between fetches.

**Helper extractor pattern** (edges.rs lines 254-280):
```rust
/// Extract entity values (person names, organizations) from metadata.
fn extract_entity_values(meta: Option<&HashMap<String, serde_json::Value>>) -> Vec<String> {
    meta.and_then(|m| m.get("extracted_entities"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|e| {
                    let et = e.get("entity_type").and_then(|v| v.as_str());
                    et == Some("person") || et == Some("organization")
                })
                .filter_map(|e| e.get("value").and_then(|v| v.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
```
Mirror for a new helper `extract_entities_full(meta)` returning `Vec<ExtractedEntity>` (all 6 types, with canonical_id once schema is extended).

**Sorting + truncating output** (edges.rs `get_neighbors` lines 180-194):
```rust
pub fn get_neighbors(&self, doc_id: &str, limit: usize) -> Vec<&DocumentEdge> {
    match self.edges.get(doc_id) {
        Some(edges) => {
            let mut sorted: Vec<&DocumentEdge> = edges.iter().collect();
            sorted.sort_by(|a, b| {
                b.weight
                    .partial_cmp(&a.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            sorted.truncate(limit);
            sorted
        }
        None => vec![],
    }
}
```
Mirror for `get_related_entities(&self, canonical_id: &str, limit: usize) -> Vec<RelatedEntity>` and `get_entities_by_type(&self, entity_type: &str) -> Vec<EntitySummary>`.

**Tests pattern** (edges.rs lines 282-366): Match the unit-test style — `test_*_new`, `test_*_empty`, `test_extract_*_missing`. All tests fully in-memory, no engine init.

---

### `src-tauri/src/commands/entities.rs` (NEW — Tauri commands)

**Analog:** `src-tauri/src/commands/documents.rs` (entire file, 367 lines).

**Imports** (documents.rs lines 1-4):
```rust
use tauri::State;
use crate::error::AppError;
use crate::state::AppState;
use crate::types::*;
```
Copy verbatim.

**Per-command pattern — read-only query** (documents.rs `get_document` lines 127-163):
```rust
#[tauri::command]
pub async fn get_document(
    id: String,
    state: State<'_, AppState>,
) -> Result<Document, AppError> {
    let engine = state.engine.clone();

    let result = tokio::task::spawn_blocking(move || {
        let engine_guard = engine.blocking_lock();
        let collection_arc = engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                AppError::VectorStorage("documents_384 collection not found".to_string())
            })?;

        let collection = collection_arc.read();
        let entry = collection
            .db
            .get(&id)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;

        match entry {
            Some(entry) => {
                let metadata = entry.metadata.as_ref().ok_or_else(|| {
                    AppError::Internal(format!("Document {} has no metadata", id))
                })?;
                Ok::<Document, AppError>(
                    crate::search::query::build_document_from_metadata(&id, metadata),
                )
            }
            None => Err(AppError::NotFound(format!("Document {} not found", id))),
        }
    })
    .await??;
    Ok(result)
}
```
Mirror this exact shape (`#[tauri::command] pub async fn` → `state.X.clone()` → `tokio::task::spawn_blocking` → `engine.blocking_lock()` → match return) for:
- `get_entity(id) -> CanonicalEntity` — read from `entity_store.lock().get_canonical(&id)`.
- `get_entities_by_type(entity_type) -> Vec<EntitySummary>` — call `entity_store.get_by_type`.
- `get_documents_for_entity(id) -> Vec<Document>` — entity_store reverse index → fetch each doc via engine (mirror `get_related_documents` lines 165-184 below).
- `get_related_entities(id, limit) -> Vec<RelatedEntity>` — call `entity_store.get_related`.

**Per-command pattern — graph traversal** (documents.rs `get_related_documents` lines 165-184):
```rust
#[tauri::command]
pub async fn get_related_documents(
    id: String,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<Document>, AppError> {
    let graph = state.doc_graph.clone();
    let engine = state.engine.clone();
    let id_owned = id;

    let results = tokio::task::spawn_blocking(move || {
        let graph_guard = graph
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let engine_guard = engine.blocking_lock();
        crate::graph::related::get_related_impl(&id_owned, limit, &graph_guard, &engine_guard)
    })
    .await??;
    Ok(results)
}
```
Mirror exactly for `get_documents_for_entity` (uses entity_store + engine).

**Per-command pattern — mutation with metadata upsert** (documents.rs `toggle_favorite` lines 186-234) — this is the closest analog for `rename_entity_canonical` and `split_entity_alias` because both rewrite document metadata via the read-modify-write upsert pattern:
```rust
let collection = collection_arc.read();
let entry = collection
    .db
    .get(&doc_id)
    .map_err(|e| AppError::VectorStorage(e.to_string()))?;

match entry {
    Some(mut entry) => {
        let metadata = entry.metadata.get_or_insert_with(std::collections::HashMap::new);
        // ... mutate metadata ...
        collection
            .db
            .insert(entry)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;
        Ok::<bool, AppError>(new_value)
    }
    None => Err(AppError::NotFound(format!("Document {} not found", doc_id))),
}
```
For `rename_entity_canonical(id, new_name)`: update `entity_store` canonical map directly (no doc metadata change needed unless we choose to denormalize).
For `split_entity_alias(canonical_id, alias)`: this DOES need doc metadata updates (each affected doc's `extracted_entities[]` gets the canonical_id of one entry switched to a new canonical_id). Iterate `entity_store.reverse_index[canonical_id]`, for each doc do read-modify-write per the toggle_favorite pattern above.

---

### `src-tauri/src/commands/documents.rs` (MODIFY — add preview text command)

**Analog:** self.

Add `read_document_text(id) -> DocumentTextPreview` following the `get_document` pattern (lines 127-163 above). Differences:
- Read `path` and `size` from doc metadata.
- If `size > 5_242_880` (5 MB per CONTEXT D-15) return `DocumentTextPreview { text: None, truncated: true, size }`.
- Else read the file via `std::fs::read_to_string(path)`, wrap in `tokio::task::spawn_blocking` (file I/O could be slow on cold cache).
- Return `DocumentTextPreview { text: Some(content), truncated: false, size }`.

Open / reveal are handled client-side via tauri-plugin-opener — NO Rust command needed for those.

---

### `src-tauri/src/state.rs` (MODIFY — extend AppState)

**Analog:** self (lines 35-59).

Existing pattern for adding a service:
```rust
pub struct AppState {
    pub engine: Arc<Mutex<CortexEngine>>,
    // ... existing 11 fields ...
    pub activity_log: Arc<std::sync::Mutex<ActivityLog>>,
}
```
Mirror the existing `Arc<...>` wrapping conventions:
- Tokio-mutex-wrapped (async-held): only `engine`. Use `tokio::sync::Mutex`.
- std-mutex-wrapped (sync, short-lived): `registry`, `space_manager`, `doc_graph`, `search_learner`, `search_tracker`, `activity_log`. Use `std::sync::Mutex`.
- Bare Arc (read-only after init): `embedding_service`, `indexer`.

Add:
```rust
pub ner_service: Arc<crate::pipeline::ner::NerService>,
pub entity_store: Arc<std::sync::Mutex<crate::graph::entity_store::EntityStore>>,
```
(`ner_service` is bare-Arc following `embedding_service` convention since NerService uses internal Mutex; `entity_store` is std-mutex following `doc_graph` convention.)

---

### `src-tauri/src/lib.rs` (MODIFY — bootstrap NER, EntityStore, plugins, backfill)

**Analog:** self (lines 27-145).

**Plugin registration pattern** — current code doesn't use plugins. Add per Tauri 2 docs (RESEARCH §Tauri Plugin APIs) at the start of the builder chain BEFORE `.setup`:
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .setup(|app| {
        // existing init...
```

**Service-init pattern** (lib.rs lines 30-95) — mirror the existing pattern of:
1. Construct the service: `let svc = Service::new_local()?` (lines 41-44 for `EmbeddingService`).
2. Wrap in Arc: `let svc = Arc::new(svc);` (line 42).
3. Pass into manage call (line 96-109).

Mirror for `NerService`:
```rust
let model_dir = app.path().resource_dir()
    .expect("could not resolve resource dir")
    .join("models");
let ner_service = Arc::new(
    pipeline::ner::NerService::new(
        &model_dir.join("bert-base-NER.onnx"),
        &model_dir.join("tokenizer.json"),
    ).expect("NER model init failed"),
);
```

And for `EntityStore`:
```rust
let entity_store = Arc::new(std::sync::Mutex::new(graph::entity_store::EntityStore::new()));
// Optional: rebuild from existing collection metadata at startup
{
    let engine_guard = engine_arc.blocking_lock();
    let mut store = entity_store.lock().unwrap();
    if let Err(e) = store.rebuild_from_collection(&engine_guard) {
        eprintln!("Warning: failed to rebuild entity store: {}", e);
    }
}
```
This mirrors the existing `indexer.rebuild_path_index` call (lib.rs lines 60-66).

**Background-task spawn pattern** (lib.rs lines 83-94 — `spawn_watcher_task`):
```rust
let app_handle = app.handle().clone();
watcher::worker::spawn_watcher_task(
    app_handle,
    engine_arc.clone(),
    embedding_service.clone(),
    indexer.clone(),
    registry.clone(),
    registry_path.clone(),
    watcher_rx,
    activity_log.clone(),
);
```
Mirror for `spawn_entity_backfill`:
```rust
let app_handle_bf = app.handle().clone();
pipeline::backfill::spawn_entity_backfill(
    app_handle_bf,
    engine_arc.clone(),
    ner_service.clone(),
    entity_store.clone(),
    embedding_service.clone(),
);
```

**Command registration pattern** (lib.rs lines 113-142 — `invoke_handler` with `tauri::generate_handler!`):
```rust
.invoke_handler(tauri::generate_handler![
    // documents (5)
    commands::documents::index_document,
    // ...
])
```
Add new entity commands group after the existing groups:
```rust
// entities (6)
commands::entities::get_entities_by_type,
commands::entities::get_entity,
commands::entities::get_documents_for_entity,
commands::entities::get_related_entities,
commands::entities::rename_entity_canonical,
commands::entities::split_entity_alias,
// preview (1)
commands::documents::read_document_text,
```

---

### `src-tauri/src/pipeline/indexer.rs` (MODIFY — hook NER into ingest)

**Analog:** self.

**Existing hook point** (indexer.rs lines 152-156):
```rust
// Step 7: Generate embedding (CPU-intensive — done outside collection lock scope)
let embedding = embedding_service.embed_text(&parsed.text)?;

// Step 8: Extract entities
let entities = self.extractor.extract(&parsed.text);
```
Replace line 156 with a NER-augmented call:
```rust
let entities = self.extractor.extract_with_ner(&parsed.text, ner_service)?;
```
This requires changing the `index_file` signature to accept `&NerService` (propagate through to all callers in `commands/documents.rs::index_document`, `commands/folders.rs::trigger_scan`, `watcher/worker.rs`). Mirror the existing pattern for threading services through — see `embedding_service` being passed from state through `index_file` (lines 11-21 in `commands/documents.rs::index_document`).

**New helper `backfill_entities(doc_id, ...)`**: mirrors the existing `index_file` (lines 74-223) but skips parse + embed + hash steps. Only updates the `extracted_entities` and `entities_version` metadata fields on the existing entry. Use the read-modify-write upsert pattern from `commands/documents.rs::toggle_favorite` lines 209-225.

**Metadata schema change** — add a new field `entities_version: u32` next to `content_hash` (indexer.rs line 188):
```rust
metadata.insert("entities_version".to_string(), serde_json::Value::Number(serde_json::Number::from(2)));
```
Backfill skips docs whose `entities_version >= 2`.

---

### `src-tauri/src/types.rs` (MODIFY — add types)

**Analog:** self (lines 23-29 for ExtractedEntity).

**Pattern** — every type uses `#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]`. Mirror for:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalEntity {
    pub id: String,
    pub canonical_name: String,
    pub entity_type: String,
    pub aliases: Vec<String>,
    pub document_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitySummary {
    pub id: String,
    pub canonical_name: String,
    pub entity_type: String,
    pub document_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedEntity {
    pub entity: EntitySummary,
    pub co_occurrence_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTextPreview {
    pub text: Option<String>,
    pub truncated: bool,
    pub size: u64,
}
```
Also extend `ExtractedEntity` (types.rs lines 23-29) with `pub canonical_id: Option<String>` to link each occurrence to its canonical (per CONTEXT "Existing Code Insights").

---

### `client/pages/EntitiesPage.tsx` (NEW)

**Analog:** `client/pages/SpacesPage.tsx` (lines 99-213) + `client/pages/TagsPage.tsx` (filter pattern, lines 16-94).

**Top-level page skeleton** (SpacesPage.tsx lines 99-131):
```tsx
export default function SpacesPage() {
  const { data: spaces, isLoading, isError } = useSpaces();
  const [viewMode, setViewMode] = useState<ViewMode>("grid");
  const [sortKey, setSortKey] = useState<SortKey>("documentCount");

  const sortedSpaces = useMemo(() => {
    // ...
  }, [spaces, sortKey]);

  if (isError) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center space-y-2">
          <p className="text-text-primary font-medium">Failed to load spaces</p>
          <p className="text-text-secondary text-sm">Please try again later.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="page-title text-text-primary">Smart Spaces</h1>
          <p className="text-text-secondary text-sm mt-1">
            Auto-organized virtual folders based on document content.
          </p>
        </div>
        // ... view-mode buttons ...
      </div>
      // ... content ...
    </div>
  );
}
```
Mirror exactly: replace `useSpaces` → `useEntities`, the page-title copy comes from UI-SPEC ("Entities" / "Click any entity to see every document mentioning it.").

**Skeleton state** (SpacesPage.tsx `SkeletonGrid` lines 22-38):
```tsx
function SkeletonGrid() {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {Array.from({ length: 6 }).map((_, i) => (
        <div key={i} className="card p-6 animate-pulse">
          <div className="flex items-start gap-3">
            <div className="h-10 w-10 rounded-lg bg-bg-tertiary" />
            <div className="flex-1 space-y-2">
              <div className="h-4 w-24 rounded bg-bg-tertiary" />
              <div className="h-3 w-16 rounded bg-bg-tertiary" />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
```
Copy structure.

**Grouped-by-type pattern** — no exact analog in the codebase, but use the same grid-+-section-header pattern from `SpaceDetailPage.tsx` lines 189-218 (sub-spaces section).

**Filter bar pattern** (TagsPage.tsx lines 22-33 + 81-94):
```tsx
const filtered = useMemo(() => {
  if (!tags) return [];
  if (filter === "all") return tags;
  return tags.filter((t) => t.tagType === filter);
}, [tags, filter]);

// ... in render ...
<select
  value={filter}
  onChange={(e) => setFilter(e.target.value as TagFilter)}
  className="text-sm bg-bg-secondary border border-border-primary rounded-lg px-3 py-1.5 text-text-primary focus:outline-none focus:ring-1 focus:ring-accent-primary"
>
  <option value="all">All Tags</option>
  <option value="auto">Auto-generated</option>
  <option value="user">User-created</option>
</select>
```
Mirror for entity-type filter — but UI-SPEC says pill toggle group, not select. Use the FilterChip pattern from SearchPage.tsx (see EntityTypeFilterBar below).

**Empty state** (SpacesPage.tsx lines 186-197):
```tsx
<div className="flex flex-col items-center justify-center min-h-[40vh] text-center space-y-4">
  <div className="p-4 rounded-full bg-bg-secondary">
    <FolderOpen size={40} className="text-text-tertiary" />
  </div>
  <div className="space-y-2">
    <p className="text-text-primary font-medium">No Smart Spaces discovered yet</p>
    <p className="text-text-secondary text-sm max-w-sm">
      Add watched folders and index documents to auto-generate spaces.
    </p>
  </div>
</div>
```
Mirror with `Network` icon and the UI-SPEC empty state copy ("No entities yet" / "Once Cortex finishes scanning your documents…").

---

### `client/pages/EntityDetailPage.tsx` (NEW)

**Analog:** `client/pages/SpaceDetailPage.tsx` (lines 101-233).

**Page structure** (SpaceDetailPage.tsx lines 101-171):
```tsx
export default function SpaceDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { data: spaces, isLoading: spacesLoading } = useSpaces();
  const { data: documents, isLoading: docsLoading } = useSpaceDocuments(id ?? "");

  const space = useMemo(() => {
    if (!spaces || !id) return undefined;
    // ...
  }, [spaces, id]);

  const isLoading = spacesLoading || docsLoading;
  if (isLoading) return <SkeletonDetail />;
  if (!space) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <div className="text-center space-y-2">
          <p className="text-text-primary font-medium">Space not found</p>
          <Link to="/spaces" className="text-sm text-accent-primary hover:text-accent-hover">
            Back to Spaces
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Breadcrumb */}
      <nav className="flex items-center gap-1 text-sm text-text-tertiary">
        <Link to="/" className="hover:text-text-secondary transition-colors">Home</Link>
        <ChevronRight size={14} />
        <Link to="/spaces" className="hover:text-text-secondary transition-colors">Spaces</Link>
        <ChevronRight size={14} />
        <span className="text-text-primary font-medium">{space.name}</span>
      </nav>
      // ... header, sections ...
    </div>
  );
}
```
Mirror exactly — swap `useSpaces`/`useSpaceDocuments` for `useEntity(id)` + `useEntityDocuments(id)` + `useRelatedEntities(id)`. Use `space-y-8` per UI-SPEC instead of `space-y-6` for entity detail.

**Header pattern** (SpaceDetailPage.tsx lines 174-187):
```tsx
<div className="flex items-center gap-4">
  <div
    className="p-3 rounded-lg"
    style={{ backgroundColor: `${space.color}15`, color: space.color }}
  >
    <Icon size={28} />
  </div>
  <div>
    <h1 className="page-title text-text-primary">{space.name}</h1>
    <p className="text-text-secondary text-sm">
      {space.documentCount} documents -- Updated {formatRelativeTime(space.lastUpdated)}
    </p>
  </div>
</div>
```
Mirror with type-color background (UI-SPEC entity color palette) and inline-edit pencil (new — see EntityDetailHeader spec).

**Documents section pattern** (SpaceDetailPage.tsx lines 201-218):
```tsx
<div className="space-y-3">
  <h2 className="section-header text-text-primary">
    Documents {documents && `(${documents.length})`}
  </h2>
  {!documents || documents.length === 0 ? (
    <div className="flex flex-col items-center justify-center py-12 text-center space-y-3">
      <FolderOpen size={32} className="text-text-tertiary" />
      <p className="text-text-secondary text-sm">No documents in this space yet.</p>
    </div>
  ) : (
    <div className="space-y-2">
      {documents.map((doc) => (
        <DocumentRow key={doc.id} doc={doc} />
      ))}
    </div>
  )}
</div>
```
Copy verbatim, change copy to "Documents mentioning this".

---

### `client/pages/DocumentPage.tsx` (MODIFY)

**Analog:** self.

**Existing entity render** (DocumentPage.tsx lines 259-274) — replace this block with `<EntityChip>` mapping:
```tsx
// BEFORE:
{doc.extractedEntities.map((e, i) => (
  <div key={i} className="flex items-center gap-2 text-sm">
    {entityTypeIcon(e.entityType)}
    <span className="text-text-tertiary">{e.label}</span>
    <span className="text-text-primary font-medium ml-auto">{e.value}</span>
  </div>
))}

// AFTER:
<div className="flex flex-wrap gap-2">
  {doc.extractedEntities.map((e) => (
    <EntityChip key={`${e.entityType}-${e.value}`} entity={e} />
  ))}
</div>
```

**Existing "Open in Finder placeholder"** (DocumentPage.tsx lines 146-155) — DELETE entirely. Replace with two action buttons in the header (next to title block lines 121-130):
```tsx
<div className="flex items-center gap-2">
  <button
    onClick={() => openPath(doc.path)}
    className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-accent-primary text-white rounded-lg hover:bg-accent-hover transition-colors text-sm font-medium"
  >
    <ExternalLink size={14} />
    Open in default app
  </button>
  <button
    onClick={() => revealItemInDir(doc.path)}
    className="inline-flex items-center gap-1.5 px-3 py-1.5 border border-border-primary text-text-secondary rounded-lg hover:bg-bg-tertiary transition-colors text-sm font-medium"
  >
    <FolderOpen size={14} />
    Reveal in Finder
  </button>
</div>
```
The primary-button style mirrors `WatchedPage.tsx` lines 164-171 (Add Folder CTA). The secondary-button style mirrors the absent ghost variant — derive from `cancel` button in `WatchedPage.tsx` line 209-214.

**Existing excerpt block** (DocumentPage.tsx lines 133-144) — REPLACE with `<FilePreview doc={doc} />`. The wrapper `<div className="rounded-lg bg-bg-secondary border border-border-primary p-4">` is removed since FilePreview owns its frame.

**Existing `entityTypeIcon()`** (DocumentPage.tsx lines 43-58) — EXTRACT into `client/components/entities/EntityChip.tsx` per UI-SPEC. Update Organization icon from `Users` → `Building2`, add Email case with `Mail` icon + cyan-400.

---

### `client/pages/WatchedPage.tsx` (MODIFY)

**Analog:** self.

**Existing dynamic-import hack** (WatchedPage.tsx lines 70-86):
```tsx
const handleAddFolder = useCallback(async () => {
  if (isTauri()) {
    try {
      const mod = "@tauri-apps/" + "plugin-dialog";
      const { open } = await import(mod);
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === "string") {
        addFolder(selected, { onSuccess: () => setShowAddDialog(false) });
      }
    } catch {
      // Fallback: show text input
      setShowAddDialog(true);
    }
  } else {
    setShowAddDialog(true);
  }
}, [addFolder]);
```
REPLACE with proper import per CONTEXT D-19:
```tsx
import { open } from "@tauri-apps/plugin-dialog";
// ...
const handleAddFolder = useCallback(async () => {
  if (!isTauri()) return;  // Browser dev — button is disabled per UI-SPEC
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Add Watched Folder",
    });
    if (selected && typeof selected === "string") {
      addFolder(selected);
    }
    // null = cancel — do nothing silently per D-19
  } catch (e) {
    toast.error("That folder could not be added. It may not exist or be inaccessible.");
  }
}, [addFolder]);
```

**Delete dead code**: `renderAddDialog()` (lines 179-228), `showAddDialog` + `newFolderPath` state (lines 39-40), `handleAddFolderSubmit` (lines 88-96), both `renderAddDialog()` invocations (lines 174, 367). The text-input dialog is hard-removed per UI-SPEC.

---

### `client/components/entities/EntityChip.tsx` (NEW)

**Analog:** inline entity rendering in `DocumentPage.tsx` lines 259-274 + tag pill in DocumentPage.tsx lines 246-253:
```tsx
{doc.tags.map((tag) => (
  <span
    key={tag}
    className="px-2 py-0.5 text-xs rounded-full bg-accent-subtle text-accent-primary"
  >
    {tag}
  </span>
))}
```

**Shape** per UI-SPEC §Component Spec EntityChip:
```tsx
import { Link } from "react-router-dom";
import { entityTypeIcon } from "./EntityTypeBadge"; // moved here

export function EntityChip({ entity }: { entity: ExtractedEntity }) {
  return (
    <Link
      to={`/entities/${entity.canonicalId ?? entity.value}`}
      aria-label={`Entity: ${entity.value}, ${entity.entityType}`}
      className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full border border-border-secondary bg-bg-tertiary hover:bg-accent-subtle focus-visible:ring-2 focus-visible:ring-accent-primary focus-visible:ring-offset-2 focus-visible:outline-none transition-colors"
    >
      {entityTypeIcon(entity.entityType)}
      <span className="text-sm text-text-primary truncate max-w-[160px]">{entity.value}</span>
    </Link>
  );
}
```

---

### `client/components/entities/EntityCard.tsx` (NEW)

**Analog:** `SubSpaceCard` in `SpaceDetailPage.tsx` lines 61-80:
```tsx
function SubSpaceCard({ space }: { space: Space }) {
  const Icon = resolveIcon(space.icon);
  return (
    <Link
      to={`/spaces/${space.id}`}
      className="card p-4 hover:shadow-md hover:border-accent-primary/50 transition-all border-l-4"
      style={{ borderLeftColor: space.color }}
    >
      <div className="flex items-center gap-3">
        <div className="p-2 rounded-lg bg-accent-subtle text-accent-primary">
          <Icon size={18} />
        </div>
        <div>
          <p className="font-medium text-text-primary">{space.name}</p>
          <p className="text-xs text-text-tertiary">{space.documentCount} docs</p>
        </div>
      </div>
    </Link>
  );
}
```
Mirror exactly: replace `Icon` with entity-type icon, color tokens from UI-SPEC entity palette (e.g., `bg-purple-400/10 text-purple-400`), drop `border-l-4` (UI-SPEC doesn't ask for it), use canonical_name + documentCount.

---

### `client/components/entities/EntityTypeFilterBar.tsx` (NEW)

**Analog:** `FilterChip` in `SearchPage.tsx` lines 36-58:
```tsx
function FilterChip({ label, active, onClick }: {...}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "px-3 py-1 rounded-full text-xs font-medium transition-colors border",
        active
          ? "bg-accent-primary text-white border-accent-primary"
          : "bg-bg-secondary text-text-secondary border-border-primary hover:bg-bg-tertiary",
      )}
    >
      {label}
    </button>
  );
}
```
Mirror for entity-type pills (All + 6 types).

---

### `client/components/entities/SplitAliasDialog.tsx` (NEW)

**Analog:** `client/components/ui/alert-dialog.tsx` (shadcn primitive, already in deps).

**Confirmation-dialog usage pattern** (not directly in code but standard shadcn): the closest in-code analog is `WatchedPage.tsx` `renderConfirmDialog` lines 230-258 (a CUSTOM dialog rather than shadcn) — but per UI-SPEC we MUST use shadcn AlertDialog. Reference shadcn alert-dialog primitive directly. Confirm button uses `bg-accent-primary` (NOT destructive red) per UI-SPEC ("Split alias is recoverable").

---

### `client/components/documents/DocumentContextMenu.tsx` (NEW)

**Analog:** `client/components/ui/context-menu.tsx` (radix wrapper, already installed).

**Pattern** — wrap child trigger in ContextMenu primitives:
```tsx
import {
  ContextMenu,
  ContextMenuTrigger,
  ContextMenuContent,
  ContextMenuItem,
} from "@/components/ui/context-menu";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";

export function DocumentContextMenu({
  doc,
  children,
}: { doc: Document; children: React.ReactNode }) {
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={() => navigate(`/document/${doc.id}`)}>
          <FileText className="mr-2 h-4 w-4" /> Open
        </ContextMenuItem>
        <ContextMenuItem onClick={() => openPath(doc.path)}>
          <ExternalLink className="mr-2 h-4 w-4" /> Open in default app
        </ContextMenuItem>
        <ContextMenuItem onClick={() => revealItemInDir(doc.path)}>
          <FolderOpen className="mr-2 h-4 w-4" /> Reveal in Finder
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
```
The icon-+-label-in-item shape mirrors radix examples; no in-codebase analog of context-menu usage exists yet (this is the first).

---

### `client/components/preview/FilePreview.tsx` (NEW dispatcher)

**Analog:** `RecentPage.tsx` `getFileIcon(docType)` dispatch lines 15-28:
```tsx
const fileTypeIcons: Record<string, typeof FileText> = {
  pdf: FileText,
  docx: FileText,
  // ...
};

function getFileIcon(docType: string) {
  return fileTypeIcons[docType] ?? File;
}
```
Mirror as switch (UI-SPEC code block already provided):
```tsx
export function FilePreview({ doc }: { doc: Document }) {
  switch (doc.docType) {
    case "pdf":  return <PdfPreview doc={doc} />;
    case "png":
    case "jpg":  return <ImagePreview doc={doc} />;
    case "md":   return <MarkdownPreview doc={doc} />;
    case "txt":
    case "csv":  return <TextPreview doc={doc} />;
    default:     return <UnsupportedPreview doc={doc} />;
  }
}
```

---

### `client/components/preview/PdfPreview.tsx`, `ImagePreview.tsx`, `TextPreview.tsx`, `MarkdownPreview.tsx` (NEW — no in-codebase analogs)

**Source:** RESEARCH.md §Code Examples (Examples 4-5) — full code blocks. Planner copies those verbatim. Size-guard logic per UI-SPEC §Per-component visual spec for each renderer. Use `convertFileSrc` import from `@tauri-apps/api/core`.

---

### `client/components/preview/SizeGuardCard.tsx` + `UnsupportedPreview.tsx` (NEW)

**Analog:** empty-state cards across pages, closest is `WatchedPage.tsx` lines 155-173:
```tsx
<div className="flex items-center justify-center min-h-[50vh]">
  <div className="text-center space-y-4">
    <div className="mx-auto w-16 h-16 rounded-full bg-bg-tertiary flex items-center justify-center">
      <FolderOpen size={32} className="text-text-tertiary" />
    </div>
    <h2 className="text-xl font-semibold text-text-primary">No folders being watched</h2>
    <p className="text-text-secondary max-w-sm">
      Add a folder to start discovering and organizing your documents.
    </p>
    <button
      type="button"
      onClick={handleAddFolder}
      className="inline-flex items-center gap-2 mt-2 px-4 py-2 bg-accent-primary text-white rounded-lg hover:bg-accent-hover transition-colors text-sm font-medium"
    >
      <Plus size={16} />
      Add Folder
    </button>
  </div>
</div>
```
Mirror exactly — change icon, copy from UI-SPEC §Per-component visual spec.

---

### `client/components/layout/BackfillIndicator.tsx` (NEW)

**Analog:** indexing chip in `TopBar.tsx` lines 37-56:
```tsx
{isIndexing && (
  <Tooltip>
    <TooltipTrigger asChild>
      <div className="flex items-center gap-2 rounded-md bg-accent-primary/10 px-2.5 py-1.5 text-xs text-accent-primary">
        <Loader2 size={14} className="animate-spin" />
        <span className="hidden sm:inline">
          Indexing {filesProcessed}/{totalFiles}
        </span>
      </div>
    </TooltipTrigger>
    <TooltipContent>
      <p className="text-xs">
        Indexing: {currentFile || "Processing..."}
      </p>
      <p className="text-xs text-muted-foreground">
        {filesProcessed} of {totalFiles} files
      </p>
    </TooltipContent>
  </Tooltip>
)}
```
Mirror exactly — replace `useIndexingStore` with `useBackfillStore`, icon `Loader2` → `Brain` (animate-pulse) per UI-SPEC, copy from UI-SPEC §Cross-cutting Surface 1.

---

### `client/components/layout/Sidebar.tsx` (MODIFY)

**Analog:** self lines 54-59.

**Existing**:
```tsx
const bottomLinks = [
  { path: "/tags", label: "Tags", icon: Tag },
  { path: "/watched", label: "Watched Folders", icon: Folder },
  { path: "/insights", label: "Insights", icon: BarChart3 },
  { path: "/settings", label: "Settings", icon: Settings },
];
```
**Add** (per UI-SPEC §Sidebar Modification):
```tsx
import { Network } from "lucide-react";
// ...
const bottomLinks = [
  { path: "/tags", label: "Tags", icon: Tag },
  { path: "/entities", label: "Entities", icon: Network },  // NEW (between Tags and Watched)
  { path: "/watched", label: "Watched Folders", icon: Folder },
  { path: "/insights", label: "Insights", icon: BarChart3 },
  { path: "/settings", label: "Settings", icon: Settings },
];
```

---

### `client/components/layout/TopBar.tsx` (MODIFY)

**Analog:** self lines 35-56.

**Add slot** for BackfillIndicator next to existing indexing chip (between line 56 and 58):
```tsx
{/* Indexing indicator (UX-04) */}
{isIndexing && (
  /* ... existing ... */
)}

{/* Entity backfill indicator (NEW Phase 6) */}
<BackfillIndicator />

{/* Theme toggle */}
{/* ... existing ... */}
```
The conditional rendering (chip is hidden when status is idle) lives inside BackfillIndicator itself, mirroring how `{isIndexing && ...}` works.

---

### `client/hooks/useTauri.ts` (MODIFY — add entity + preview hooks)

**Analog:** self.

**Query-key pattern** (useTauri.ts lines 45-60):
```tsx
export const queryKeys = {
  spaces: ["spaces"] as const,
  document: (id: string) => ["documents", id] as const,
  // ...
};
```
Add:
```tsx
entities: ["entities"] as const,
entitiesByType: (type: string) => ["entities", "byType", type] as const,
entity: (id: string) => ["entities", id] as const,
entityDocuments: (id: string) => ["entities", id, "documents"] as const,
relatedEntities: (id: string) => ["entities", id, "related"] as const,
documentText: (id: string) => ["documents", id, "text"] as const,
```

**Query hook pattern** (useTauri.ts `useDocument` lines 135-146):
```tsx
export function useDocument(id: string) {
  return useQuery({
    queryKey: queryKeys.document(id),
    queryFn: () =>
      tauriInvoke<Document>(
        "get_document",
        { id },
        () => mockDocuments.find((d) => d.id === id) ?? mockDocuments[0],
      ),
    enabled: Boolean(id),
  });
}
```
Mirror exactly for `useEntity`, `useEntityDocuments`, `useRelatedEntities`, `useEntitiesByType`, `useDocumentText`. Browser-mode fallbacks point to mock data (planner adds mock entities to `mock-data.ts`).

**Mutation hook pattern** (useTauri.ts `useToggleFavorite` lines 224-234):
```tsx
export function useToggleFavorite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (docId: string) =>
      tauriInvoke<void>("toggle_favorite", { docId }, () => undefined),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.favoriteDocuments });
      queryClient.invalidateQueries({ queryKey: queryKeys.recentDocuments });
    },
  });
}
```
Mirror for:
- `useRenameEntityCanonical` — invalidates `queryKeys.entity(id)` + `queryKeys.entities`.
- `useSplitEntityAlias` — invalidates `queryKeys.entity(id)` + `queryKeys.entities` + `queryKeys.entityDocuments(id)`.

---

### `client/hooks/usePreview.ts` (NEW)

**Analog:** `useDocument` factory in `useTauri.ts` lines 135-146 (shape) + `useRecordSearchClick` mutation lines 261-270 (no-op fallback).

Simple wrapper:
```tsx
export function usePreview(documentId: string) {
  return useQuery({
    queryKey: queryKeys.documentText(documentId),
    queryFn: () =>
      tauriInvoke<DocumentTextPreview>(
        "read_document_text",
        { id: documentId },
        () => ({ text: "(mock preview text)", truncated: false, size: 100 }),
      ),
    enabled: Boolean(documentId),
  });
}
```

---

### `client/hooks/useBackfillProgress.ts` (NEW)

**Analog:** event listener in `WatchedPage.tsx` lines 45-68 + `useIndexingStore` consumer pattern in `TopBar.tsx` lines 14-16.

**Existing event listener pattern** (WatchedPage.tsx lines 45-68):
```tsx
useEffect(() => {
  if (!isTauri()) return;
  let unlisten: (() => void) | undefined;

  (async () => {
    try {
      const { listen } = await import("@tauri-apps/api/event");
      unlisten = await listen<{ folderId: string; status: string }>(
        "index-progress",
        (event) => {
          if (event.payload.status === "complete" || event.payload.status === "error") {
            setScanning((prev) => ({ ...prev, [event.payload.folderId]: false }));
          }
        },
      );
    } catch {
      // Not in Tauri environment
    }
  })();

  return () => {
    unlisten?.();
  };
}, []);
```
Mirror inside `useBackfillProgress()`:
- Listen on `"entity-backfill-progress"` event.
- Push payload into `useBackfillStore` (new Zustand store).
- Cleanup on unmount.
- Mounted once at app level (in `AppShell` or `App.tsx`).

---

### `client/lib/stores.ts` (MODIFY — add useBackfillStore)

**Analog:** self `useIndexingStore` lines 44-79.

**Existing**:
```tsx
interface IndexingState {
  isIndexing: boolean;
  currentFile: string;
  filesProcessed: number;
  totalFiles: number;
  setProgress: (progress: {
    currentFile?: string;
    filesProcessed?: number;
    totalFiles?: number;
    isIndexing?: boolean;
  }) => void;
  reset: () => void;
}

export const useIndexingStore = create<IndexingState>((set) => ({
  isIndexing: false,
  currentFile: "",
  filesProcessed: 0,
  totalFiles: 0,
  setProgress: (progress) => set((s) => ({...})),
  reset: () => set({...}),
}));
```
Mirror exactly:
```tsx
interface BackfillState {
  status: "idle" | "running" | "complete" | "error";
  processed: number;
  total: number;
  error: string | null;
  setProgress: (p: { status?: ...; processed?: number; total?: number; error?: string | null }) => void;
  reset: () => void;
}

export const useBackfillStore = create<BackfillState>((set) => ({
  status: "idle",
  processed: 0,
  total: 0,
  error: null,
  setProgress: (p) => set((s) => ({ ...s, ...p })),
  reset: () => set({ status: "idle", processed: 0, total: 0, error: null }),
}));
```

---

### `client/lib/types.ts` (MODIFY — add shared types)

**Analog:** self.

All types use plain interfaces. Add (mirror existing `Document` and `Tag` styles):
```tsx
export interface CanonicalEntity {
  id: string;
  canonicalName: string;
  entityType: string;
  aliases: string[];
  documentCount: number;
}

export interface EntitySummary {
  id: string;
  canonicalName: string;
  entityType: string;
  documentCount: number;
}

export interface RelatedEntity {
  entity: EntitySummary;
  coOccurrenceCount: number;
}

export interface DocumentTextPreview {
  text: string | null;
  truncated: boolean;
  size: number;
}

export interface EntityBackfillProgress {
  processed: number;
  total: number;
  status: "running" | "complete" | "error";
  error?: string;
}
```
Also extend `Document.extractedEntities[]` (line 21-25) to optionally include `canonicalId?: string`.

---

### `client/App.tsx` (MODIFY — register routes)

**Analog:** self lines 33-53.

**Existing**:
```tsx
<Route element={<AppShell />}>
  <Route path="/" element={<Index />} />
  <Route path="/spaces" element={<SpacesPage />} />
  <Route path="/spaces/:id" element={<SpaceDetailPage />} />
  {/* ... */}
  <Route path="/document/:id" element={<DocumentPage />} />
</Route>
```
**Add** (before catch-all "*"):
```tsx
<Route path="/entities" element={<EntitiesPage />} />
<Route path="/entities/:id" element={<EntityDetailPage />} />
```
Plus imports at top mirroring lines 9-22.

---

### `src-tauri/capabilities/default.json` (MODIFY)

**Analog:** self.

**Existing**:
```json
{
  "$schema": "...",
  "identifier": "default",
  "description": "Default capability for Cortex",
  "windows": ["main"],
  "permissions": [
    "core:default"
  ]
}
```
**Add** (per RESEARCH §Pattern 3):
```json
"permissions": [
  "core:default",
  "dialog:allow-open",
  "opener:allow-open-path",
  "opener:allow-reveal-item-in-dir"
]
```

---

### `src-tauri/tauri.conf.json` (MODIFY)

**Analog:** self.

**Existing security block**:
```json
"security": {
  "csp": null
}
```
**Replace with** (per RESEARCH §Pattern 2 — sets CSP, enables asset protocol):
```json
"security": {
  "csp": "default-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data:; frame-src 'self' asset: http://asset.localhost; object-src 'self' asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'",
  "assetProtocol": {
    "enable": true,
    "scope": ["**"]
  }
}
```
**Also add** to `bundle` block (per RESEARCH §Runtime State Inventory):
```json
"bundle": {
  ...,
  "resources": ["models/*"]
}
```

---

## Shared Patterns

These patterns apply to multiple Phase 6 files. Listed once here; per-file plan sections reference back to this section.

### Shared Pattern A: All Tauri commands

**Source:** `src-tauri/src/commands/documents.rs` lines 6-21 + 127-163.
**Apply to:** All 6 new entity commands in `commands/entities.rs` + new `read_document_text` in `commands/documents.rs`.
**Excerpt:**
```rust
#[tauri::command]
pub async fn COMMAND_NAME(
    ARG: TYPE,
    state: State<'_, AppState>,
) -> Result<RETURN_TYPE, AppError> {
    let X = state.X.clone();
    let result = tokio::task::spawn_blocking(move || {
        let X_guard = X.blocking_lock();   // tokio mutex
        // OR
        let X_guard = X.lock().map_err(|e| AppError::Internal(e.to_string()))?;  // std mutex
        // ... work ...
        Ok::<RETURN_TYPE, AppError>(value)
    })
    .await??;
    Ok(result)
}
```

### Shared Pattern B: All shared serde types

**Source:** `src-tauri/src/types.rs` lines 5-29.
**Apply to:** All new structs in `types.rs`.
**Excerpt:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Foo { pub bar_baz: String }
```

### Shared Pattern C: Tauri-event payloads

**Source:** `src-tauri/src/watcher/worker.rs` lines 18-26.
**Apply to:** `EntityBackfillProgress` struct in `pipeline/backfill.rs`.
**Excerpt:**
```rust
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub file_path: String,
    pub status: String,
    pub doc_id: Option<String>,
    pub error: Option<String>,
    pub folder_id: Option<String>,
}
```

### Shared Pattern D: All React Query hooks

**Source:** `client/hooks/useTauri.ts` lines 135-146 (query) + 224-234 (mutation).
**Apply to:** All new entity / preview hooks.
**Excerpt — Query**:
```tsx
export function useFoo(id: string) {
  return useQuery({
    queryKey: queryKeys.foo(id),
    queryFn: () =>
      tauriInvoke<Foo>("get_foo", { id }, () => mockFoo),
    enabled: Boolean(id),
  });
}
```
**Excerpt — Mutation**:
```tsx
export function useUpdateFoo() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: {...}) =>
      tauriInvoke<void>("update_foo", args, () => undefined),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.foo(args.id) });
    },
  });
}
```

### Shared Pattern E: Browser-dev fallback

**Source:** `client/lib/tauri.ts` lines 15-38 + `WatchedPage.tsx` lines 70-86 (isTauri guard).
**Apply to:** All new preview / open / reveal interactions.
**Pattern:** `if (!isTauri()) return; /* or fall back to mock */`. Every Tauri-plugin call (`open()`, `openPath()`, `revealItemInDir()`, `convertFileSrc()`) must be guarded for browser dev mode where `__TAURI__` is undefined.

### Shared Pattern F: Lucide icon + per-type color palette

**Source:** `DocumentPage.tsx` `entityTypeIcon()` lines 43-58.
**Apply to:** `EntityChip`, `EntityTypeBadge`, `EntityCard`, `RelatedEntityChip` — all need the per-type icon + color mapping.
**Excerpt**:
```tsx
function entityTypeIcon(entityType: string) {
  switch (entityType) {
    case "date":         return <Calendar size={14} className="text-blue-400" />;
    case "amount":       return <DollarSign size={14} className="text-green-400" />;
    case "person":       return <Users size={14} className="text-purple-400" />;
    case "organization": return <Users size={14} className="text-amber-400" />;
    case "location":     return <MapPin size={14} className="text-red-400" />;
    default:             return <Tag size={14} className="text-text-tertiary" />;
  }
}
```
**Phase-6 modifications** (per UI-SPEC):
- `organization` → use `Building2` not `Users` (disambiguates from person).
- Add `email` → `<Mail size={14} className="text-cyan-400" />`.

### Shared Pattern G: Page-level error + empty + loading triplet

**Source:** `SpaceDetailPage.tsx` lines 137-150 (error) + `SpacesPage.tsx` lines 186-197 (empty) + skeleton patterns across pages.
**Apply to:** EntitiesPage, EntityDetailPage, every preview component.
**Pattern**:
```tsx
if (isLoading) return <Skeleton />;
if (isError || !data) return <ErrorPane />;
if (data.length === 0) return <EmptyState />;
return <ActualContent />;
```

### Shared Pattern H: Toast notifications

**Source:** `client/components/ui/sonner.tsx` (already mounted in App.tsx).
**Apply to:** every async action that can fail in Phase 6 (openPath, revealItemInDir, rename, split, addFolder).
**Pattern**: `import { toast } from "sonner"; toast.error("...")`. Already-mounted Toaster handles display.

---

## No Analog Found

| File | Role | Reason | Use instead |
|------|------|--------|-------------|
| `src-tauri/src/pipeline/ner.rs` (BIO decode helper) | NER subword decoding | First ONNX inference in codebase | RESEARCH.md §Pattern 1 + §Pitfall 2 (use `encoding.get_offsets()`) |
| `client/components/preview/PdfPreview.tsx` | iframe via asset URL | First asset-protocol use | RESEARCH.md §Code Examples Example 4 (verbatim) |
| `client/components/preview/MarkdownPreview.tsx` | react-markdown render | First markdown render in codebase | RESEARCH.md §Code Examples Example 5 (verbatim) |
| `src-tauri/models/*.onnx` etc. | binary assets | Not code — binary files | RESEARCH.md §Pitfall 8 (4 files to bundle) |

---

## Metadata

**Analog search scope:**
- `src-tauri/src/commands/` (5 files)
- `src-tauri/src/pipeline/` (5 files)
- `src-tauri/src/graph/` (3 files)
- `src-tauri/src/{lib,state,types,error,engine}.rs`
- `src-tauri/src/watcher/worker.rs` (background-task pattern)
- `client/pages/` (12 files surveyed; closest: SpacesPage, SpaceDetailPage, TagsPage, SearchPage, RecentPage, DocumentPage, WatchedPage)
- `client/components/layout/` (TopBar, Sidebar, AppShell)
- `client/components/ui/` (context-menu, sonner — shadcn primitives)
- `client/hooks/useTauri.ts`
- `client/lib/{stores,tauri,types,utils,icons}.ts`
- `client/App.tsx`
- `src-tauri/{Cargo.toml,tauri.conf.json}` + `src-tauri/capabilities/default.json`
- `package.json`

**Files scanned:** ~35 source files read end-to-end; ~10 more grep-sampled.

**Pattern extraction date:** 2026-06-29.

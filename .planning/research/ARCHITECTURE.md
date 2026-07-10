# Architecture Research

**Domain:** Tauri 2 Desktop App with Embedded Rust Vector Database (RuVector)
**Researched:** 2026-02-27
**Confidence:** HIGH (Tauri 2 IPC and state management patterns verified via official docs + community sources; RuVector structure verified from source)

---

## Standard Architecture

### System Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                     WEBVIEW LAYER (Tauri)                          │
│                                                                    │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐               │
│  │  React Pages │ │  Components  │ │  React Query │               │
│  │  (12 routes) │ │  (40+ comps) │ │  (data cache)│               │
│  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘               │
│         │                │                │                        │
│  ┌──────▼────────────────▼────────────────▼───────┐               │
│  │         Tauri IPC Layer (@tauri-apps/api)       │               │
│  │   invoke() commands + listen() events           │               │
│  └──────────────────────┬─────────────────────────┘               │
└─────────────────────────│──────────────────────────────────────────┘
                          │ JSON-RPC over WebView bridge
┌─────────────────────────▼──────────────────────────────────────────┐
│                     TAURI CORE (Rust)                              │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    AppState (Arc<Mutex<T>>)                  │  │
│  │  WatcherHandle │ IndexerState │ EngineHandle │ Settings      │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐ │
│  │  commands/   │  │  events/     │  │  background_tasks/       │ │
│  │  documents   │  │  file_index  │  │  watcher (notify-rs)     │ │
│  │  spaces      │  │  space_upd   │  │  indexer (pipeline)      │ │
│  │  search      │  │  progress    │  │  reclustering (periodic) │ │
│  │  folders     │  │              │  │                          │ │
│  │  analytics   │  │              │  │                          │ │
│  │  settings    │  │              │  │                          │ │
│  └──────┬───────┘  └──────────────┘  └──────────────────────────┘ │
└─────────│──────────────────────────────────────────────────────────┘
          │ Rust function calls (no IPC overhead)
┌─────────▼──────────────────────────────────────────────────────────┐
│                   RUVECTOR ENGINE LAYER                            │
│                                                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐ │
│  │ ruvector-    │  │ ruvector-    │  │ ruvector-graph           │ │
│  │ core         │  │ gnn          │  │ (Cypher queries,         │ │
│  │ (VectorDB,   │  │ (GNN layers, │  │  hyperedges,             │ │
│  │  HNSW,       │  │  clustering, │  │  related docs,           │ │
│  │  SIMD,       │  │  training,   │  │  space network)          │ │
│  │  storage)    │  │  EWC)        │  │                          │ │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘ │
│                                                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐ │
│  │ ruvector-    │  │ ruvector-    │  │ sona                     │ │
│  │ filter       │  │ attention    │  │ (SONA self-learning,     │ │
│  │ (metadata    │  │ (re-ranking  │  │  LoRA, EWC++,            │ │
│  │  pre-filter) │  │  search)     │  │  trajectory learning)    │ │
│  └──────────────┘  └──────────────┘  └──────────────────────────┘ │
│                                                                    │
│  ┌──────────────┐  ┌──────────────┐                               │
│  │ ruvector-    │  │ ruvector-    │                               │
│  │ collections  │  │ domain-      │                               │
│  │ (multi-index │  │ expansion    │                               │
│  │  per space)  │  │ (transfer    │                               │
│  │              │  │  learning)   │                               │
│  └──────────────┘  └──────────────┘                               │
└────────────────────────────────────────────────────────────────────┘
          │ Rust function calls (in-process)
┌─────────▼──────────────────────────────────────────────────────────┐
│                 DOCUMENT PIPELINE (Rust, in-process)               │
│                                                                    │
│  notify-rs (file watcher) → Parser → ONNX Runtime (ort crate)     │
│  (pdf-extract / docx-rs / calamine / tesseract-rs / direct read)  │
│  → 384-dim or 1536-dim embeddings → RuVector store                │
└────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Communicates With |
|-----------|----------------|-------------------|
| React Frontend | Renders UI, manages local UI state via Zustand | Tauri IPC (commands + events) |
| React Query | Caches server state, handles background refetch | invoke() calls |
| Tauri IPC Bridge | Serializes JS→Rust calls, delivers Rust→JS events | Frontend + Rust backend |
| AppState | Shared state across all Tauri commands; wraps subsystem handles | All Tauri commands |
| commands/documents | CRUD for document metadata, favorites, entity queries | RuVector engine |
| commands/spaces | Get/update/recluster smart spaces | RuVector GNN + graph |
| commands/search | Semantic search with hybrid filter | RuVector filter + core + attention |
| commands/folders | Add/remove/pause watched folders, trigger scans | File watcher subsystem |
| commands/analytics | Stats, space graph data, search analytics | RuVector query layer |
| commands/settings | Read/write settings to app config store | Settings state |
| File Watcher (notify-rs) | Monitors watched directories, debounces events | Indexing pipeline (mpsc channel) |
| Document Parser | Extracts text from PDF/DOCX/images/spreadsheets | ONNX Runtime + RuVector |
| ONNX Runtime (ort) | Generates 384-dim embeddings from text chunks | RuVector core (store) |
| ruvector-core | HNSW vector storage, SIMD search, REDB persistence | ruvector-filter, ruvector-gnn |
| ruvector-gnn | GNN layers, continual clustering, EWC forgetting prevention | ruvector-core (reads HNSW graph) |
| ruvector-graph | Cypher queries, hyperedges, document relationship graph | ruvector-core |
| ruvector-filter | Pre-filter by type/date/space/tag before vector search | ruvector-core |
| ruvector-attention | 46 attention mechanisms for search result re-ranking | ruvector-core |
| ruvector-collections | Separate vector index per Smart Space | ruvector-core |
| ruvector-domain-expansion | Transfer learning: new spaces inherit from known domains | ruvector-gnn |
| sona | SONA self-learning engine: LoRA adaptation from search signals | ruvector-core |

---

## Recommended Project Structure

```
cortex/
├── src/                         # React frontend (existing)
│   ├── components/
│   ├── pages/
│   ├── hooks/
│   │   ├── useTauri.ts          # Generic Tauri invoke wrapper
│   │   ├── useDocuments.ts      # Document data hooks (React Query)
│   │   ├── useSpaces.ts         # Space data hooks
│   │   └── useSearch.ts         # Search hooks
│   ├── lib/
│   │   └── mock-data.ts         # Mock data for dev-without-Tauri
│   └── App.tsx
│
└── src-tauri/                   # Rust backend (Tauri 2 convention)
    ├── Cargo.toml               # Declares ruvector crates as path deps
    ├── tauri.conf.json
    ├── build.rs
    ├── capabilities/            # Tauri permission capabilities
    ├── resources/               # Bundled app resources
    │   └── models/
    │       └── all-MiniLM-L6-v2.onnx    # ~23MB, bundled at build
    └── src/
        ├── main.rs              # Desktop entry: calls lib::run()
        ├── lib.rs               # Mobile entry + Builder setup
        │
        ├── state.rs             # AppState struct and initialization
        │   # pub struct AppState { watcher, indexer, engine, settings }
        │
        ├── commands/            # Tauri IPC command module
        │   ├── mod.rs           # pub mod declarations
        │   ├── documents.rs     # index_document, get_document, get_related
        │   ├── spaces.rs        # get_spaces, move_document, recluster_spaces
        │   ├── search.rs        # search_documents (hybrid filter + vector)
        │   ├── folders.rs       # add/remove/pause watched folders, trigger_scan
        │   ├── analytics.rs     # get_stats, get_space_graph, search_analytics
        │   └── settings.rs      # get_settings, update_settings
        │
        ├── background/          # Long-running background subsystems
        │   ├── mod.rs
        │   ├── watcher.rs       # notify-rs file watcher → mpsc channel
        │   └── indexer.rs       # Indexing pipeline: parse→embed→store→cluster
        │
        ├── engine/              # RuVector integration layer
        │   ├── mod.rs
        │   ├── vector_store.rs  # ruvector-core wrapper (VectorDB, HNSW)
        │   ├── gnn_cluster.rs   # ruvector-gnn clustering scheduler
        │   ├── graph_engine.rs  # ruvector-graph queries + hyperedge mgmt
        │   ├── filter.rs        # ruvector-filter metadata filter builder
        │   ├── attention.rs     # ruvector-attention re-ranking wrapper
        │   ├── learning.rs      # sona SONA engine: search signal feedback
        │   └── collections.rs   # ruvector-collections space index manager
        │
        ├── pipeline/            # Document processing pipeline
        │   ├── mod.rs
        │   ├── parser.rs        # Multi-format parser (PDF/DOCX/CSV/image)
        │   ├── embedder.rs      # ONNX Runtime via ort crate
        │   └── extractor.rs     # Entity extraction (dates, amounts, names)
        │
        ├── storage/             # Non-vector persistence
        │   └── settings.rs      # App settings via Tauri store plugin
        │
        └── error.rs             # Unified error type with serde::Serialize
```

### Structure Rationale

- **commands/**: Groups IPC handlers by domain (documents, spaces, search, etc). Each file corresponds to a domain in the frontend hook layer. Prevents the common anti-pattern of a 2000-line lib.rs.
- **background/**: Isolates long-running Tokio tasks (file watcher, indexer pipeline). These own mpsc channels and never block the IPC thread.
- **engine/**: Thin wrappers around RuVector crates. Wrapping prevents RuVector API changes from propagating into command handlers.
- **pipeline/**: Document processing steps decoupled from the storage layer. Parser outputs raw text; embedder outputs vectors; both are pure functions.
- **state.rs**: Single `AppState` struct managed by Tauri's `.manage()`. Commands receive it via `tauri::State<AppState>`.

---

## Architectural Patterns

### Pattern 1: Managed State with Subsystem Handles

**What:** A single `AppState` struct holding `Arc<Mutex<T>>` or `Arc<T>` handles to subsystems. Tauri wraps this in its own `Arc`, so subsystems avoid double-wrapping.

**When to use:** Any data or resource that must outlive a single command invocation — file watcher handles, vector DB handles, settings.

**Trade-offs:** Single state struct keeps initialization simple; downside is all subsystems must be initialized at startup even if not immediately used.

**Example:**
```rust
// src-tauri/src/state.rs
use parking_lot::Mutex;
use std::sync::Arc;

pub struct AppState {
    pub engine: Arc<CortexEngine>,    // RuVector wrapper (thread-safe internally)
    pub watcher_tx: Arc<Mutex<tokio::sync::mpsc::Sender<WatcherCmd>>>,
    pub indexer_tx: Arc<Mutex<tokio::sync::mpsc::Sender<IndexCmd>>>,
    pub settings: Arc<Mutex<Settings>>,
}

// In lib.rs setup:
tauri::Builder::default()
    .manage(AppState::new().await)
    .invoke_handler(tauri::generate_handler![
        commands::documents::index_document,
        commands::search::search_documents,
        // ...all commands
    ])
```

### Pattern 2: Modular Command Registration

**What:** Declare commands in domain-specific modules, collect them all in `generate_handler![]` in `lib.rs`. Each command receives state via `tauri::State<AppState>`.

**When to use:** Any Tauri 2 backend with more than ~5 commands. Standard practice for teams.

**Trade-offs:** The `generate_handler![]` macro requires compile-time knowledge of all commands. Cannot dynamically register; must list every command explicitly.

**Example:**
```rust
// src-tauri/src/commands/search.rs
#[tauri::command]
pub async fn search_documents(
    query: String,
    filters: SearchFilters,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, AppError> {
    let results = state.engine
        .search(&query, &filters)
        .await?;
    // Feed signal to SONA for self-learning
    state.engine.record_search_signal(&query, results.len()).await;
    Ok(results)
}
```

### Pattern 3: Background Tasks with mpsc Channels + Tauri Events

**What:** Long-running background tasks (file watcher, periodic re-clustering) run as `tauri::async_runtime::spawn()` tasks. They receive commands via `tokio::mpsc::Receiver` and notify the frontend via `app_handle.emit()`.

**When to use:** File system watching, periodic background jobs (GNN re-clustering), or any operation where the backend pushes updates to the frontend without a command trigger.

**Trade-offs:** Events are not type-safe (JSON only) and cannot return values. Suitable for fire-and-forget notifications; use commands for request-response patterns.

**Example:**
```rust
// src-tauri/src/background/watcher.rs
pub async fn run_watcher(
    app_handle: tauri::AppHandle,
    mut cmd_rx: tokio::sync::mpsc::Receiver<WatcherCmd>,
    index_tx: tokio::sync::mpsc::Sender<IndexCmd>,
) {
    let mut watcher = notify::recommended_watcher(move |event| {
        // debounce + filter, then send to indexer
        let _ = index_tx.blocking_send(IndexCmd::IndexFile(path));
    }).unwrap();

    loop {
        match cmd_rx.recv().await {
            Some(WatcherCmd::Watch(path)) => { watcher.watch(&path, ...); }
            Some(WatcherCmd::Stop) => break,
            None => break,
        }
    }
}

// Indexer emits event after each document indexed:
app_handle.emit("document-indexed", IndexedEvent { doc_id, space_ids })?;
app_handle.emit("indexing-progress", ProgressEvent { done, total })?;
```

### Pattern 4: Path Dependency for RuVector Crates

**What:** Reference RuVector crates as local path dependencies in `src-tauri/Cargo.toml`. This keeps RuVector embedded in the binary with zero IPC overhead and full Rust type safety across the boundary.

**When to use:** Always, for this project. RuVector is not published to crates.io, and even if it were, local path deps give faster iteration.

**Trade-offs:** Changes to RuVector source require a Cargo rebuild of `src-tauri`. No runtime version negotiation — must keep both in sync. Workspace exclusions in RuVector's `Cargo.toml` (like `ruvector-hyperbolic-hnsw`) must be handled.

**Example:**
```toml
# src-tauri/Cargo.toml
[dependencies]
ruvector-core = { path = "../../experiments/ruvector/crates/ruvector-core", default-features = false, features = ["simd", "storage", "hnsw"] }
ruvector-gnn  = { path = "../../experiments/ruvector/crates/ruvector-gnn" }
ruvector-graph = { path = "../../experiments/ruvector/crates/ruvector-graph" }
ruvector-filter = { path = "../../experiments/ruvector/crates/ruvector-filter" }
ruvector-attention = { path = "../../experiments/ruvector/crates/ruvector-attention" }
ruvector-collections = { path = "../../experiments/ruvector/crates/ruvector-collections" }
ruvector-domain-expansion = { path = "../../experiments/ruvector/crates/ruvector-domain-expansion" }
sona = { path = "../../experiments/ruvector/crates/sona" }
ort = "2.0.0-rc.11"  # ONNX Runtime bindings
```

### Pattern 5: Dual-Mode Frontend Hooks (Mock → Real)

**What:** React Query hooks in the frontend check a build flag or runtime environment to return either mock data or live Tauri `invoke()` calls. During frontend-only development (`bun dev`), no Tauri is needed.

**When to use:** Always — this allows UI development to proceed independently of backend completion.

**Trade-offs:** Requires maintaining mock data that matches production API shapes. Type discipline is critical: TypeScript types must match Rust `serde` output exactly.

**Example:**
```typescript
// src/hooks/useDocuments.ts
import { invoke } from '@tauri-apps/api/core';
import { mockDocuments } from '@/lib/mock-data';

export function useDocuments(spaceId?: string) {
  return useQuery({
    queryKey: ['documents', spaceId],
    queryFn: async () => {
      if (window.__TAURI_INTERNALS__) {
        return invoke<Document[]>('get_space_documents', { spaceId });
      }
      return mockDocuments.filter(d => d.spaceIds.includes(spaceId ?? ''));
    },
  });
}
```

---

## Data Flow

### Indexing Flow (File → Smart Space)

```
File system change detected (notify-rs, background thread)
    │
    ▼ (debounce 300ms, filter by extension)
IndexCmd sent via tokio::mpsc channel
    │
    ▼
Document Parser (pipeline/parser.rs)
├── PDF → pdf-extract text
├── DOCX → docx-rs paragraphs
├── XLSX/CSV → calamine cell values
├── Images → tesseract-rs OCR text
└── TXT/MD → direct read
    │
    ▼
Entity Extraction (pipeline/extractor.rs)
- dates, amounts, people, organizations, locations
    │
    ▼
ONNX Runtime (ort, pipeline/embedder.rs)
- all-MiniLM-L6-v2: text chunks → 384-dim f32 vectors
- optional: OpenAI API → 1536-dim (if user opted in)
    │
    ▼
RuVector Core (engine/vector_store.rs)
- HNSW insert (O(log n))
- REDB persistence to app data dir
    │
    ▼
GNN Update check (engine/gnn_cluster.rs)
- Incremental: assign to nearest existing cluster if similarity > threshold
- Periodic (every N docs or time interval): full re-cluster
    ├── Existing cluster → Assign to Space
    └── New cluster detected → Create Space → Name via LLM (Ollama/API)
    │
    ▼
Graph Edge Update (engine/graph_engine.rs)
- ContentSimilar edges to k nearest neighbors
- SharedTag / SharedEntity edges computed
    │
    ▼
Tauri Event emitted to frontend (app_handle.emit)
- "document-indexed" → React Query invalidates relevant queries
- "space-updated" → Sidebar space list refreshes
```

### Search Flow (Query → Results → Learning)

```
User types query (React SearchBar component)
    │
    ▼ (debounced 150ms)
invoke('search_documents', { query, filters })
    │
    ▼ (Rust command handler)
Generate query embedding (ONNX Runtime, same model as indexing)
    │
    ▼
ruvector-filter: apply metadata pre-filters
- type IN ['pdf', 'docx'], date BETWEEN ..., space_id = ...
    │
    ▼
ruvector-core: HNSW approximate nearest neighbor search
- Returns top-K candidates with similarity scores
    │
    ▼
ruvector-attention: re-rank top-K results
- Attention mechanisms applied to reorder by relevance
    │
    ▼
Return Vec<SearchResult> to frontend (JSON via IPC)
    │ (async, parallel)
    ▼
sona SONA engine: record LearningSignal
- trajectory: query → result positions viewed → clicks
- MicroLoRA adapts in <1ms (instant loop)
- BackgroundLoop trains BaseLoRA over time

Frontend displays results → user clicks → React Query caches
```

### State Synchronization (Frontend ↔ Backend)

```
Frontend State (Zustand)          Backend Events (Tauri)
─────────────────────           ─────────────────────
sidebarCollapsed                "document-indexed" → invalidate document queries
theme (dark/light)              "space-updated" → invalidate space list
commandPaletteOpen              "indexing-progress" → update progress bar state
                                "scan-complete" → trigger full refresh

React Query Cache               Tauri Commands (request/response)
─────────────────────           ──────────────────────────────────
documents (by space/tag)   ←→  get_space_documents, search_documents
spaces (list + detail)     ←→  get_spaces, get_space_documents
stats (dashboard)          ←→  get_stats
search results             ←→  search_documents
settings                   ←→  get_settings, update_settings
```

---

## Build Order Implications

The architecture has clear dependency layers. Build in this order to avoid blocked work:

### Layer 1: Foundation (no dependencies on other layers)
1. **Tauri 2 project scaffold** — `src-tauri/` directory, `Cargo.toml`, `tauri.conf.json`, `build.rs`
2. **Error types** (`src-tauri/src/error.rs`) — used by all commands
3. **AppState struct** (`src-tauri/src/state.rs`) — skeleton, with stub subsystem handles
4. **Frontend Tauri hooks** (`src/hooks/useTauri.ts`, `useDocuments.ts`, etc.) — dual-mode with mock fallback

### Layer 2: Storage Engine (depends on Layer 1)
5. **RuVector path dependency wiring** — Cargo.toml path deps, feature flags, compile check
6. **Vector store wrapper** (`engine/vector_store.rs`) — wraps ruvector-core VectorDB
7. **Settings storage** (`storage/settings.rs`) — Tauri store plugin for app config

### Layer 3: Document Pipeline (depends on Layer 2)
8. **Document parser** (`pipeline/parser.rs`) — multi-format text extraction, no RuVector dependency
9. **ONNX embedder** (`pipeline/embedder.rs`) — ort integration, model loading, tokenizer
10. **Entity extractor** (`pipeline/extractor.rs`) — regex/NLP entity extraction from text

### Layer 4: Background Subsystems (depends on Layer 3)
11. **Indexing pipeline** (`background/indexer.rs`) — orchestrates parser → embedder → vector_store
12. **File watcher** (`background/watcher.rs`) — notify-rs, debouncing, sends to indexer

### Layer 5: Intelligence Layer (depends on Layer 2 + 4)
13. **GNN clustering** (`engine/gnn_cluster.rs`) — ruvector-gnn, periodic re-cluster scheduler
14. **Graph engine** (`engine/graph_engine.rs`) — ruvector-graph, edge management
15. **Attention re-ranker** (`engine/attention.rs`) — ruvector-attention search re-ranking
16. **Filter builder** (`engine/filter.rs`) — ruvector-filter metadata filters
17. **SONA learning** (`engine/learning.rs`) — sona crate, search signal recording

### Layer 6: IPC Commands (depends on all layers)
18. **commands/folders** — add/remove watched folders (Layer 4 dependency)
19. **commands/search** — semantic search (Layer 5 dependency)
20. **commands/documents** — document metadata queries (Layer 2 dependency)
21. **commands/spaces** — smart spaces CRUD + recluster trigger (Layer 5)
22. **commands/analytics** — stats + graph visualization data (Layer 5)
23. **commands/settings** — settings read/write (Layer 2)

### Layer 7: Frontend Integration (depends on Layer 6)
24. **Replace mock data with Tauri invoke calls** — flip dual-mode hooks to live backend
25. **Event listeners** — listen for `document-indexed`, `space-updated`, `indexing-progress`
26. **End-to-end testing** — full indexing pipeline integration tests

---

## Anti-Patterns

### Anti-Pattern 1: Direct RuVector Calls from Command Handlers

**What people do:** Import `ruvector-core` directly in `commands/search.rs` and call `VectorDB::search()` inline.

**Why it's wrong:** Commands become coupled to RuVector's API surface. When RuVector's API changes (it's version 2.0.5, still evolving), every command must be updated. Harder to test command logic in isolation.

**Do this instead:** Create `engine/` wrapper modules. Commands call `state.engine.search(...)`. The engine module absorbs RuVector API changes internally.

### Anti-Pattern 2: Blocking Calls in Async Command Handlers

**What people do:** Call synchronous file I/O or CPU-intensive operations directly inside `async` Tauri commands without `spawn_blocking`.

**Why it's wrong:** Tauri's async commands share the Tokio runtime. A blocking call stalls all async tasks, freezing the UI for the duration of the operation. PDF parsing of a large document (100+ pages) can take seconds.

**Do this instead:** Use `tokio::task::spawn_blocking` for CPU-bound or blocking I/O. The indexer pipeline already runs as a background task via `tauri::async_runtime::spawn` — parsing happens there, not in the command handler.

### Anti-Pattern 3: Using React Portals for Persistent Layout in Tauri

**What people do:** Use `ReactDOM.createPortal()` to render sidebars or floating panels into `document.body`.

**Why it's wrong:** Per project constraints (CLAUDE.md and Tauri desktop app best practices), React portals break layout persistence in Tauri's WebView. Document state and event handling become decoupled from the component tree.

**Do this instead:** Use DOM reparenting (direct `appendChild`/`removeChild` manipulation) for any layout-persistent elements. The AppShell component handles this correctly already.

### Anti-Pattern 4: Storing Large Payloads in Tauri IPC Response

**What people do:** Return full document content (file bytes, base64-encoded images) via Tauri `invoke()` response.

**Why it's wrong:** IPC responses are JSON-serialized. Large payloads (>1MB) cause perceptible latency spikes in the UI. Tauri's documentation explicitly warns: "This can slow down your application if you try to return a large data such as a file."

**Do this instead:** For document preview, use Tauri's asset protocol to serve files from disk. For streaming large results, use `tauri::ipc::Channel` for chunked delivery. Return only metadata and excerpts via `invoke()`.

### Anti-Pattern 5: Running notify-rs Watcher Inside a Tauri Command

**What people do:** Start the file watcher as a side effect of an `add_watched_folder` command.

**Why it's wrong:** The watcher needs to outlive the command invocation. Commands run and return; state managed inside the command handler is dropped. The watcher must live in `AppState` or as a `tauri::async_runtime::spawn`ed task.

**Do this instead:** The `add_watched_folder` command sends a `WatcherCmd::Watch(path)` message through the mpsc channel to the already-running background watcher task. The watcher is started in `tauri::Builder::setup()` before any commands can be invoked.

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| ONNX Runtime (local) | `ort` crate in `pipeline/embedder.rs`, model bundled in `resources/models/` | ~23MB for all-MiniLM-L6-v2; loaded once at startup, kept in AppState |
| OpenAI API (optional) | `reqwest` HTTP calls from `pipeline/embedder.rs` when user enables cloud embeddings | Gated by Settings flag; API key stored in Tauri secure store |
| Ollama (local LLM) | HTTP to `localhost:11434` from `engine/gnn_cluster.rs` for space naming | Falls back to generic name if Ollama unavailable |
| notify-rs | Direct Rust dep in `background/watcher.rs` | Use `recommended_watcher()` for cross-platform support; debounce 300ms |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| Frontend ↔ Tauri commands | `invoke()` JSON-RPC | Synchronous request-response; all errors as `Result<T, E>` |
| Tauri backend ↔ Frontend (push) | `app_handle.emit()` events | Async, fire-and-forget; frontend uses `listen()` |
| Commands ↔ Background tasks | `tokio::mpsc` channels via AppState | Decouples IPC thread from long-running work |
| Engine layer ↔ RuVector crates | Direct Rust function calls | No serialization overhead; in-process |
| Background indexer ↔ Engine layer | Direct function calls (same process) | Indexer holds `Arc<CortexEngine>` handle |
| Pipeline stages | Function call sequence (parse → embed → store) | Each stage pure function, composable for testing |

---

## Scalability Considerations

This is a local-first desktop app. "Scalability" here means: how does it stay performant as the document collection grows?

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 0-5K docs | Default: single HNSW index, full re-cluster OK, all in memory |
| 5K-50K docs | Enable ruvector-collections: per-space indices for faster scoped search; GNN cold-tier (mmap) for graph training data |
| 50K-500K docs | Enable quantization in ruvector-core (Scalar 4x or Int4 8x); incremental re-cluster instead of full; background indexer priority queue |
| 500K+ docs | Enable ruvector-cluster for sharded indices per watched folder; memory-mapped storage throughout; HNSW parameter tuning (ef_construction) |

### Scaling Priorities

1. **First bottleneck:** GNN re-clustering time as corpus grows. Mitigation: move from full re-cluster to incremental cluster assignment for new documents, with full re-cluster only on user request or nightly schedule.
2. **Second bottleneck:** Memory footprint of HNSW index. Mitigation: Int4 quantization (8x compression) reduces 50K × 384-dim from ~75MB to ~9MB at minimal accuracy cost.

---

## Sources

- Tauri 2 Architecture: [https://v2.tauri.app/concept/architecture/](https://v2.tauri.app/concept/architecture/) — HIGH confidence (official docs)
- Tauri 2 IPC Commands: [https://v2.tauri.app/develop/calling-rust/](https://v2.tauri.app/develop/calling-rust/) — HIGH confidence (official docs)
- Tauri 2 State Management: [https://v2.tauri.app/develop/state-management/](https://v2.tauri.app/develop/state-management/) — HIGH confidence (official docs)
- Tauri 2 Project Structure: [https://v2.tauri.app/start/project-structure/](https://v2.tauri.app/start/project-structure/) — HIGH confidence (official docs)
- Async Background Tasks in Tauri: [https://rfdonnelly.github.io/posts/tauri-async-rust-process/](https://rfdonnelly.github.io/posts/tauri-async-rust-process/) — MEDIUM confidence (community, verified against official patterns)
- Long-running tasks Tauri v2: [https://sneakycrow.dev/blog/2024-05-12-running-async-tasks-in-tauri-v2](https://sneakycrow.dev/blog/2024-05-12-running-async-tasks-in-tauri-v2) — MEDIUM confidence (community)
- Tauri command module organization: [https://dev.to/n3rd/how-to-reasonably-keep-your-tauri-commands-organized-in-rust-2gmo](https://dev.to/n3rd/how-to-reasonably-keep-your-tauri-commands-organized-in-rust-2gmo) — MEDIUM confidence (community, patterns are standard Rust module practice)
- ort (ONNX Runtime Rust): [https://ort.pyke.io](https://ort.pyke.io) — HIGH confidence (official ort docs)
- RuVector source: `/Users/gshah/work/apps/experiments/ruvector/` — HIGH confidence (direct source inspection)
  - Crates verified: ruvector-core, ruvector-gnn, ruvector-graph, ruvector-filter, ruvector-attention, ruvector-collections, ruvector-domain-expansion, sona
  - Key finding: ruvector-core's AgenticDB uses placeholder hash embeddings — real ONNX/API embeddings MUST be integrated externally (Cortex's embedder does this)

---

*Architecture research for: Tauri 2 + RuVector desktop document intelligence app (Cortex)*
*Researched: 2026-02-27*

# Phase 1: Tauri Foundation - Research

**Researched:** 2026-02-27
**Domain:** Tauri 2 desktop shell, Rust IPC contracts, RuVector core initialization
**Confidence:** HIGH (codebase verified directly, RuVector APIs read from source)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Migration approach:** Wrap existing React frontend in Tauri 2 WebView — client/ code stays intact
- **Express removal:** Remove Express server entirely (clean break, not dual-mode)
- **React/Tailwind upgrade:** React 18 → 19 and TailwindCSS 3 → 4 as part of this phase
- **Package manager:** Keep pnpm (already configured, lockfile exists)
- **IPC command names:** Domain-prefixed matching CLAUDE.md spec (`search_documents`, `get_spaces`, `add_watched_folder`, `get_stats`, etc.)
- **Data visibility:** Index size visible in Settings > Storage with option to clear/rebuild
- **Dev workflow:** Frontend runs standalone without Tauri (pnpm dev) — hooks fall back to mock data
- **Tests:** Unit tests for AppError serialization, IPC command stubs, RuVector initialization
- **Dual-mode hooks:** Detect Tauri runtime → use invoke(), otherwise → use mock data

### Claude's Discretion

- Server/ directory disposition (delete or archive — evaluate if any code is reusable)
- RuVector storage location (standard app data dir recommended by Tauri conventions)
- First-launch behavior before onboarding exists (Phase 4 builds real onboarding)
- IPC stub behavior (return mock data vs "not implemented" errors)
- IPC error granularity (typed enum vs simple messages — evolve as needed)
- Tauri event system setup timing (now vs Phase 2)
- Embedding model switch strategy (separate collections vs re-index)
- Developer onboarding documentation approach
- CI/CD setup timing

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TAURI-01 | Tauri 2 shell wraps existing React frontend with WebView | Tauri 2 `create-tauri-app` + `beforeDevUrl`/`devUrl` config + existing client/ preserved as-is |
| TAURI-02 | Express server removed, replaced by Tauri IPC command stubs | server/ dir has index.ts + routes/ — no code is reusable; clean delete; all 20 IPC commands become stub `#[tauri::command]` fns |
| TAURI-03 | AppError enum with serde::Serialize for all IPC error handling | Pattern: `thiserror` + `#[derive(Debug, Serialize)]` on enum; Tauri requires `Result<T, String>` or custom serializable error |
| TAURI-04 | spawn_blocking pattern established for all CPU-bound operations | `tokio::task::spawn_blocking` wraps sync-heavy work; used in every IPC command stub as prep for Phase 2 |
| TAURI-05 | Dual-mode frontend hooks (mock data in dev, Tauri invoke in production) | `window.__TAURI__` global detection; React hooks return mock data when absent, call `invoke()` when present |
| TAURI-06 | AppState struct with Arc<CortexEngine> and channel senders | Tauri `manage()` API; `AppState` holds `Arc<Mutex<CortexEngine>>` + `tokio::sync::mpsc::Sender` for background tasks |
| VSTOR-01 | RuVector core integration with HNSW indexing | `ruvector-core` v2.0.2 at local path; `CollectionManager::new(path)` + `create_collection("documents", CollectionConfig { dimensions: 384, distance_metric: Cosine, .. })` |
| VSTOR-02 | Multi-collection support (separate indices per embedding dimension) | `ruvector-collections` crate: `CollectionManager` with `create_collection()` per dimension (384 for local ONNX, 1536 for OpenAI API) |
| VSTOR-03 | Metadata filtering (type, date range, space, tags) before vector search | `ruvector-filter` crate: `PayloadIndexManager` + `FilterExpression` with `eq`, `gte`, `lte`, `and`, `or` operators; index fields: `doc_type`, `created_at`, `space_ids`, `tags` |
| VSTOR-04 | Hybrid queries: structured filters + semantic similarity | Filter pre-pass with `FilterEvaluator` narrows candidate set; HNSW search runs on filtered IDs — pattern from ruvector-filter examples |
</phase_requirements>

---

## Summary

Phase 1 converts an existing React+Express web prototype into a Tauri 2 desktop app. The existing `client/` directory is preserved intact and served as the Tauri WebView. The `server/` Express layer is deleted entirely — it contains only `index.ts` and a `routes/` directory with no logic worth porting. All backend functionality is replaced by stubbed `#[tauri::command]` functions that return typed mock data until Phase 2 builds real implementations.

The RuVector crates are available as local path dependencies at `/Users/gshah/work/apps/experiments/ruvector/`. Key crates for Phase 1 are `ruvector-core` (v2.0.2), `ruvector-collections`, and `ruvector-filter`. Their public APIs have been verified directly from source: `CollectionManager::new(path)` initializes multi-collection storage, `FilterExpression` provides the metadata filtering DSL, and `PayloadIndexManager` indexes document metadata for pre-search filtering.

The frontend currently runs React 18 + TailwindCSS 3 and needs upgrading to React 19 + TailwindCSS 4 as part of this phase. The dual-mode hook pattern is straightforward: detect `window.__TAURI__` at runtime and branch between `invoke()` and mock data. No `useTauri.ts` exists yet — it must be created.

**Primary recommendation:** Scaffold `src-tauri/` with `cargo tauri init`, configure `beforeDevUrl` to point at `pnpm dev` (port 5173), wire all 20 CLAUDE.md IPC commands as typed stubs, initialize `CollectionManager` in `AppState`, then upgrade React/Tailwind. Do these in order — Tauri scaffold first ensures the build pipeline is validated before frontend changes compound complexity.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri | 2.x | Desktop shell + IPC bridge | The project's chosen framework; Tauri 2 is stable release |
| tauri-build | 2.x | Build script integration | Required companion for tauri 2 |
| serde | 1.x | Serialize/Deserialize for IPC types | Required by Tauri for command args/return types |
| serde_json | 1.x | JSON for IPC payloads | Tauri serializes via JSON over IPC bridge |
| thiserror | 1.x | Derive `Error` on AppError enum | Idiomatic Rust error derivation; less boilerplate than manual impl |
| tokio | 1.x (full) | Async runtime + spawn_blocking | Tauri 2 uses Tokio; `spawn_blocking` is in `tokio::task` |
| ruvector-core | 2.0.2 (local path) | Vector storage + HNSW indexing | Project's chosen vector engine |
| ruvector-collections | 2.0.2 (local path) | Multi-collection management | Needed for VSTOR-02 |
| ruvector-filter | 2.0.2 (local path) | Metadata filtering DSL | Needed for VSTOR-03 |

### Supporting (Frontend)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @tauri-apps/api | 2.x | `invoke()`, `event`, `window` JS APIs | Required for frontend IPC calls |
| @tauri-apps/plugin-shell | 2.x | If shell access needed | Only if needed in later phases |
| react | 19.x | UI framework | Upgrading from 18 per locked decision |
| tailwindcss | 4.x | CSS framework | Upgrading from 3 per locked decision |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| thiserror | anyhow | anyhow is for apps, thiserror for library-style typed errors — AppError needs typed variants for frontend dispatch |
| tokio::sync::Mutex | std::sync::Mutex | std Mutex in async context causes deadlocks; tokio Mutex is async-aware |
| ruvector-collections | raw ruvector-core | Collections adds alias support and per-collection config — cleaner multi-dimension management |

**Installation (Rust side):**
```bash
cargo tauri init
# Then add to src-tauri/Cargo.toml
```

**Installation (JS side):**
```bash
pnpm add @tauri-apps/api
```

---

## Architecture Patterns

### Recommended Project Structure

```
cortex/
├── client/                  # React frontend — UNCHANGED from current
│   ├── App.tsx
│   ├── components/
│   ├── hooks/
│   │   └── useTauri.ts      # NEW: dual-mode Tauri hooks (create this)
│   ├── lib/
│   │   └── mock-data.ts     # Existing mock data (keep)
│   └── pages/
├── src-tauri/               # NEW: Tauri backend
│   ├── Cargo.toml           # tauri 2, ruvector-* path deps
│   ├── build.rs             # tauri-build::build()
│   ├── tauri.conf.json      # beforeDevUrl, app metadata
│   ├── icons/               # App icons
│   └── src/
│       ├── main.rs          # Tauri builder setup, manage(AppState)
│       ├── lib.rs           # run() function
│       ├── error.rs         # AppError enum + Serialize impl
│       ├── state.rs         # AppState struct
│       ├── engine.rs        # CortexEngine with RuVector init
│       └── commands/
│           ├── mod.rs       # re-exports all command modules
│           ├── documents.rs # index_document, search_documents, get_document, get_related
│           ├── spaces.rs    # get_spaces, get_space_documents, move_document_to_space, recluster_spaces
│           ├── folders.rs   # add_watched_folder, remove_watched_folder, trigger_scan
│           ├── analytics.rs # get_stats, get_space_graph, get_search_analytics
│           └── settings.rs  # get_settings, update_settings
├── server/                  # DELETE entirely (Express — no reusable code)
├── package.json             # Add tauri script, remove server scripts
└── pnpm-lock.yaml
```

### Pattern 1: AppError Enum — Serializable IPC Errors

**What:** A typed error enum that implements `serde::Serialize` so Tauri can send structured errors to the frontend.
**When to use:** Every `#[tauri::command]` returns `Result<T, AppError>`.

```rust
// src-tauri/src/error.rs
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("Vector storage error: {0}")]
    VectorStorage(String),
    #[error("Document not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Not implemented")]
    NotImplemented,
    #[error("Internal error: {0}")]
    Internal(String),
}

// CRITICAL: Tauri requires the error type to implement Into<String> OR be Serialize.
// Using serde tagging lets the frontend pattern-match on error.kind.
```

### Pattern 2: AppState — Managed State via Tauri

**What:** A struct holding the shared engine, managed by Tauri's `manage()` API.

```rust
// src-tauri/src/state.rs
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::engine::CortexEngine;

pub struct AppState {
    pub engine: Arc<Mutex<CortexEngine>>,
}
```

```rust
// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .manage(AppState {
            engine: Arc::new(Mutex::new(CortexEngine::new().expect("engine init failed"))),
        })
        .invoke_handler(tauri::generate_handler![
            commands::documents::index_document,
            commands::documents::search_documents,
            // ... all 20 commands
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Pattern 3: IPC Command Stub with spawn_blocking

**What:** Every command acquires state, does its (stub) work inside `spawn_blocking`, returns typed result.
**When to use:** All CPU-bound operations — establishes the pattern for Phase 2 real implementations.

```rust
// src-tauri/src/commands/documents.rs
use tauri::State;
use crate::{state::AppState, error::AppError};

#[tauri::command]
pub async fn search_documents(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, AppError> {
    // spawn_blocking for anything CPU-bound (Phase 2 will put real work here)
    let results = tokio::task::spawn_blocking(move || {
        // stub: return mock data
        Ok::<Vec<SearchResult>, AppError>(vec![])
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;

    Ok(results)
}
```

### Pattern 4: Dual-Mode Frontend Hook

**What:** Hooks detect `window.__TAURI__` — use `invoke()` in Tauri context, mock data in plain browser.

```typescript
// client/hooks/useTauri.ts
import { invoke } from '@tauri-apps/api/core';
import { mockSpaces } from '../lib/mock-data';

const isTauri = () => typeof window !== 'undefined' && '__TAURI__' in window;

export function useSpaces() {
  return useQuery({
    queryKey: ['spaces'],
    queryFn: async () => {
      if (isTauri()) {
        return invoke<Space[]>('get_spaces');
      }
      // fallback: mock data for pnpm dev without Tauri
      return mockSpaces;
    },
  });
}
```

### Pattern 5: RuVector Initialization in CortexEngine

**What:** `CortexEngine` wraps `CollectionManager` and `PayloadIndexManager` — initialized once at startup.

```rust
// src-tauri/src/engine.rs
use ruvector_collections::{CollectionManager, CollectionConfig};
use ruvector_core::types::{DistanceMetric, HnswConfig};
use ruvector_filter::{PayloadIndexManager, IndexType};
use std::path::PathBuf;

pub struct CortexEngine {
    pub collections: CollectionManager,
    pub filter_index: PayloadIndexManager,
}

impl CortexEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Use Tauri's app data dir convention (set via tauri::AppHandle in real init)
        let data_dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".local/share/cortex/vectors");

        let collections = CollectionManager::new(data_dir.clone())?;

        // Create collections for each embedding dimension
        // 384-dim: local ONNX (all-MiniLM-L6-v2)
        collections.create_collection("documents_384", CollectionConfig {
            dimensions: 384,
            distance_metric: DistanceMetric::Cosine,
            hnsw_config: Some(HnswConfig::default()),
            quantization: None,
            on_disk_payload: true,
        }).ok(); // ok() = ignore AlreadyExists error on restart

        // 1536-dim: OpenAI API (opt-in, Phase 2)
        collections.create_collection("documents_1536", CollectionConfig {
            dimensions: 1536,
            distance_metric: DistanceMetric::Cosine,
            hnsw_config: Some(HnswConfig::default()),
            quantization: None,
            on_disk_payload: true,
        }).ok();

        // Set up metadata filter indices
        let mut filter_index = PayloadIndexManager::new();
        filter_index.create_index("doc_type", IndexType::Keyword)?;
        filter_index.create_index("created_at", IndexType::Integer)?;
        filter_index.create_index("space_ids", IndexType::Keyword)?;
        filter_index.create_index("tags", IndexType::Keyword)?;

        Ok(Self { collections, filter_index })
    }
}
```

### Anti-Patterns to Avoid

- **Blocking in async context:** Never call synchronous RuVector operations directly in `async fn`. Always use `spawn_blocking`. Tokio will deadlock or stall other tasks.
- **Panicking in IPC commands:** Never `unwrap()` or `expect()` in command handlers. Every error path must return `Err(AppError::...)`.
- **Raw `String` errors in IPC:** `Result<T, String>` loses type information. Frontend can't pattern-match. Use `AppError` with `#[serde(tag = "kind")]`.
- **Skipping `manage()` registration:** State accessed via `State<AppState>` only works if `manage(AppState {...})` is called in `Builder`. Missing it causes a runtime panic.
- **std::sync::Mutex in async:** Use `tokio::sync::Mutex` for state shared across `.await` points. std Mutex held across await = instant deadlock.
- **React portals for layout:** Per project CLAUDE.md — use DOM reparenting for layout persistence, not React portals.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HNSW indexing | Custom approximate nearest neighbor | ruvector-core (already in workspace) | ANN is non-trivial; HNSW has tuning parameters (M, ef_construction) that need benchmark validation |
| Multi-collection management | HashMap<String, VectorIndex> | ruvector-collections CollectionManager | Handles aliases, per-collection config, persistence, thread-safety |
| Metadata filter DSL | Custom filter parsing | ruvector-filter FilterExpression | Supports AND/OR/NOT, geo, range, keyword — months of work to replicate |
| Error type display | Manual `Display` impl on AppError | thiserror `#[error("...")]` | Eliminates boilerplate; correctly implements Error trait chain |
| Runtime detection (frontend) | Custom Tauri detection logic | `'__TAURI__' in window` check | Official pattern from Tauri docs |

**Key insight:** RuVector crates exist as local path deps — use them. Do not build vector storage, collection management, or metadata filtering from scratch. Phase 1 only initializes them; real usage comes in Phase 2.

---

## Common Pitfalls

### Pitfall 1: Cargo Workspace Conflict

**What goes wrong:** Adding `src-tauri/Cargo.toml` alongside `ruvector` path deps — Cargo may try to resolve the ruvector workspace, causing "duplicate package" or "not a member" errors.
**Why it happens:** RuVector is its own workspace. Cortex's `src-tauri/` is a separate workspace. They must not be nested.
**How to avoid:** Cortex `src-tauri/Cargo.toml` uses `[workspace]` at the top (Tauri init does this by default). RuVector crates are referenced as path deps: `ruvector-core = { path = "../../experiments/ruvector/crates/ruvector-core" }`. Verify the paths are correct relative to `src-tauri/`.
**Warning signs:** `cargo build` errors mentioning "is not a member of workspace" or "found duplicate packages."

**Note from STATE.md:** "RuVector workspace exclusions need verification during Phase 1 Cargo.toml setup" — this is a flagged concern.

### Pitfall 2: RuVector Excluded Crates

**What goes wrong:** Some RuVector crates are excluded from the workspace (`ruvector-hyperbolic-hnsw`, WASM crates). Their path deps may exist but not compile as part of the workspace.
**Why it happens:** The RuVector `Cargo.toml` workspace `exclude` list prevents certain crates from being built.
**How to avoid:** Only depend on crates that ARE in the RuVector workspace members list. For Phase 1: `ruvector-core`, `ruvector-collections`, `ruvector-filter` — all confirmed as workspace members.

### Pitfall 3: Tauri IPC Type Serialization

**What goes wrong:** Returning a Rust type from a command that doesn't implement `Serialize` — compile error, or returns garbled JSON.
**Why it happens:** Tauri serializes command return values via `serde_json`. All types in `Result<T, E>` must be `Serialize`.
**How to avoid:** Every struct/enum returned from commands (including `AppError`) must `#[derive(Serialize, Deserialize)]`. Add `serde` as a dep immediately.

### Pitfall 4: React 18 → 19 + TailwindCSS 3 → 4 Upgrade Ordering

**What goes wrong:** Upgrading both simultaneously obscures which change broke what.
**Why it happens:** Both upgrades can touch the same render paths and styling.
**How to avoid:** Upgrade TailwindCSS 4 first (CSS-only, isolated risk), verify app renders, then upgrade React 19. TailwindCSS 4 drops the PostCSS plugin approach — configuration format changes significantly (CSS-first config in `globals.css` replaces `tailwind.config.ts`).
**Warning signs:** TailwindCSS 4 uses `@import "tailwindcss"` in CSS, not `@tailwind base/components/utilities` directives.

### Pitfall 5: pnpm Dev Port Conflict with Tauri

**What goes wrong:** Tauri's `beforeDevUrl` points to `http://localhost:5173` but Vite is configured for a different port.
**Why it happens:** `tauri.conf.json` `app.windows.url` or `build.devUrl` must match Vite's dev server port exactly.
**How to avoid:** Confirm Vite port in `vite.config.ts`, set `tauri.conf.json` `build.devUrl = "http://localhost:5173"`.

### Pitfall 6: AppState Not Initialized Before Commands Are Called

**What goes wrong:** RuVector `CollectionManager::new()` fails (bad path, permissions) and the engine panics at startup.
**Why it happens:** Engine initialization runs in `main()` before any window opens.
**How to avoid:** Use `Result` in `main()`, log errors clearly. For Phase 1, storage path can be a relative dev path (`./cortex-data/`). Tauri's `app.path().app_data_dir()` requires `AppHandle` — wire proper path resolution using `setup` hook.

---

## Code Examples

### tauri.conf.json skeleton

```json
{
  "productName": "Cortex",
  "version": "0.1.0",
  "identifier": "com.cortex.app",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Cortex",
        "width": 1400,
        "height": 900,
        "minWidth": 900,
        "minHeight": 600
      }
    ]
  }
}
```

### src-tauri/Cargo.toml skeleton

```toml
[package]
name = "cortex"
version = "0.1.0"
edition = "2021"

[lib]
name = "cortex_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[dependencies]
tauri = { version = "2", features = [] }
tauri-build = { version = "2", build-dependencies = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["full"] }
ruvector-core = { path = "../../experiments/ruvector/crates/ruvector-core" }
ruvector-collections = { path = "../../experiments/ruvector/crates/ruvector-collections" }
ruvector-filter = { path = "../../experiments/ruvector/crates/ruvector-filter" }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

### All 20 IPC Commands (stub signatures from CLAUDE.md)

```rust
// Documents
#[tauri::command] pub async fn index_document(path: String, state: State<'_, AppState>) -> Result<DocumentMeta, AppError>
#[tauri::command] pub async fn search_documents(query: String, filters: SearchFilters, state: State<'_, AppState>) -> Result<Vec<SearchResult>, AppError>
#[tauri::command] pub async fn get_document(id: String, state: State<'_, AppState>) -> Result<Document, AppError>
#[tauri::command] pub async fn get_related_documents(id: String, limit: usize, state: State<'_, AppState>) -> Result<Vec<Document>, AppError>

// Spaces
#[tauri::command] pub async fn get_spaces(state: State<'_, AppState>) -> Result<Vec<Space>, AppError>
#[tauri::command] pub async fn get_space_documents(space_id: String, state: State<'_, AppState>) -> Result<Vec<Document>, AppError>
#[tauri::command] pub async fn move_document_to_space(doc_id: String, space_id: String, state: State<'_, AppState>) -> Result<(), AppError>
#[tauri::command] pub async fn recluster_spaces(state: State<'_, AppState>) -> Result<Vec<Space>, AppError>

// Folders
#[tauri::command] pub async fn add_watched_folder(path: String, state: State<'_, AppState>) -> Result<WatchedFolder, AppError>
#[tauri::command] pub async fn remove_watched_folder(id: String, state: State<'_, AppState>) -> Result<(), AppError>
#[tauri::command] pub async fn trigger_scan(folder_id: String, state: State<'_, AppState>) -> Result<ScanProgress, AppError>

// Analytics
#[tauri::command] pub async fn get_stats(state: State<'_, AppState>) -> Result<Stats, AppError>
#[tauri::command] pub async fn get_space_graph(state: State<'_, AppState>) -> Result<SpaceGraph, AppError>
#[tauri::command] pub async fn get_search_analytics(state: State<'_, AppState>) -> Result<SearchAnalytics, AppError>

// Settings
#[tauri::command] pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, AppError>
#[tauri::command] pub async fn update_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), AppError>
```

### Tauri proper storage path (using setup hook)

```rust
tauri::Builder::default()
    .setup(|app| {
        let data_dir = app.path().app_data_dir()
            .expect("could not resolve app data dir")
            .join("vectors");
        std::fs::create_dir_all(&data_dir)?;

        let engine = CortexEngine::new_with_path(data_dir)
            .expect("RuVector initialization failed");

        app.manage(AppState {
            engine: Arc::new(tokio::sync::Mutex::new(engine)),
        });
        Ok(())
    })
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `tailwind.config.js` + PostCSS directives | CSS-first config: `@import "tailwindcss"` in globals.css | TailwindCSS v4 (2025) | `tailwind.config.ts` is no longer the primary config location; existing config must be migrated |
| `@tauri-apps/api/tauri` (invoke) | `@tauri-apps/api/core` (invoke) | Tauri 2.0 | Import path changed; old path causes "module not found" |
| `#[tauri::command]` returning `String` errors | `#[tauri::command]` returning `Result<T, impl Serialize>` | Tauri 2.0 best practice | Typed errors enable frontend pattern-matching |
| React 18 concurrent mode | React 19 (stable, server components optional) | React 19 release | `use()`, improved transitions; no breaking changes for client-only apps like this |

**Deprecated/outdated:**
- `tailwindcss@3` config: `tailwind.config.ts` with `content`, `theme`, `plugins` — replace with `@import "tailwindcss"` + `@theme` blocks in CSS
- Express server in `server/` — entirely removed, no migration path
- Vite config with `server.proxy` (for Express API) — remove once Express is gone

---

## Open Questions

1. **RuVector path dependency resolution across workspaces**
   - What we know: RuVector is at `/Users/gshah/work/apps/experiments/ruvector/` with its own `[workspace]`. Cortex `src-tauri/` will be a separate workspace.
   - What's unclear: Whether Cargo resolves the path deps correctly when the paths are absolute vs relative. Absolute paths work in Cargo.toml but are not portable for other contributors.
   - Recommendation: Use relative paths from `src-tauri/` (`../../experiments/ruvector/crates/...`). Document the sibling-directory assumption in CLAUDE.md.

2. **Correct Tauri 2 API version for @tauri-apps/api**
   - What we know: Tauri 2 is the target; `@tauri-apps/api` v2 must match.
   - What's unclear: Exact latest stable version of `@tauri-apps/api` v2.x at time of implementation.
   - Recommendation: `pnpm add @tauri-apps/api@^2` at implementation time; verify against tauri.app/v2 docs.

3. **TailwindCSS 4 migration scope**
   - What we know: TailwindCSS 4 uses CSS-first config. The existing `tailwind.config.ts` uses v3 format.
   - What's unclear: How much of the existing config (content paths, theme extensions, plugins) auto-migrates vs requires manual rewrite.
   - Recommendation: Run the official `@tailwindcss/upgrade` codemod first; review diff; fix manually if needed.

4. **React 19 compatibility with existing dependencies**
   - What we know: Package.json has `react@^18.3.1`. Many `@radix-ui/*` and `framer-motion` packages are present.
   - What's unclear: Whether all @radix-ui versions in the lockfile peer-dep on React 18 strictly.
   - Recommendation: Upgrade React 19 with `--legacy-peer-deps` if needed; run `pnpm dev` and verify no runtime errors before proceeding.

---

## Sources

### Primary (HIGH confidence)

- RuVector source at `/Users/gshah/work/apps/experiments/ruvector/` — directly read `crates/ruvector-collections/src/lib.rs`, `crates/ruvector-filter/src/lib.rs`, `crates/ruvector-core/Cargo.toml`, workspace `Cargo.toml`
- CLAUDE.md (project) — IPC command signatures, data types, tech stack, conventions
- CONTEXT.md — locked decisions, discretion areas

### Secondary (MEDIUM confidence)

- Tauri 2 configuration patterns — from training knowledge (Tauri 2 released late 2024); specific API paths may need verification against tauri.app/v2 docs at implementation time
- TailwindCSS 4 migration — CSS-first config approach verified as current direction; `@tailwindcss/upgrade` codemod existence confirmed from training

### Tertiary (LOW confidence)

- React 19 peer-dep compatibility with specific `@radix-ui` versions in lockfile — unverified, flagged as open question

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — RuVector APIs read from source; Tauri 2 + serde + thiserror are well-established
- Architecture: HIGH — patterns derived from verified API shapes (CollectionManager, FilterExpression, tauri::command signatures)
- Pitfalls: MEDIUM — workspace conflict and Tailwind migration are experience-based; exact behavior depends on Cargo version and codemod output
- React/Tailwind upgrades: MEDIUM — direction verified, exact peer-dep compatibility LOW

**Research date:** 2026-02-27
**Valid until:** 2026-03-27 (30 days — Tauri 2 and RuVector stable; Tailwind 4 fast-moving but migration path documented)

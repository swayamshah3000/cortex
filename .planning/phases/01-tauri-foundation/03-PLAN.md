---
wave: 3
depends_on: [PLAN-01, PLAN-02]
requirements: [TAURI-04]
files_modified:
  - src-tauri/src/commands/mod.rs
  - src-tauri/src/commands/documents.rs
  - src-tauri/src/commands/spaces.rs
  - src-tauri/src/commands/folders.rs
  - src-tauri/src/commands/analytics.rs
  - src-tauri/src/commands/settings.rs
  - src-tauri/src/types.rs
  - src-tauri/src/lib.rs
autonomous: true
---

# Plan 03: IPC Command Stubs with spawn_blocking Pattern

## Goal

All 20 IPC commands are stubbed as `#[tauri::command]` async functions (16 from CLAUDE.md + 4 frontend-implied: `get_watched_folders`, `get_tags`, `toggle_favorite`, `get_activity_feed`). Every command uses `spawn_blocking` for CPU-bound work (establishing the pattern Phase 2 fills in). All IPC types (request/response structs) are defined with Serialize/Deserialize. Commands return mock data or `AppError::NotImplemented` as appropriate. The Tauri builder registers all 20 command handlers.

## Context

- TAURI-04: spawn_blocking pattern established for all CPU-bound operations.
- The 20 commands are defined in CLAUDE.md under "RuVector Integration Points."
- Commands are grouped into 5 modules: documents (5), spaces (4), folders (4), analytics (5), settings (2) — total 20 commands. 16 from CLAUDE.md + 4 frontend-implied: `get_watched_folders`, `get_tags`, `toggle_favorite`, `get_activity_feed`.
- Each stub acquires `State<AppState>`, runs logic inside `spawn_blocking`, returns `Result<T, AppError>`.
- IPC response types must match the TypeScript interfaces in CLAUDE.md's Data Types section.
- Stubs should return reasonable mock data (not empty) so Tauri invoke from frontend shows something.

## Tasks

<task id="03.1" effort="M">
<title>Define IPC types for all command arguments and return values</title>
<detail>
Create `src-tauri/src/types.rs` with all structs needed for IPC:

```rust
use serde::{Deserialize, Serialize};

// === Document types ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub name: String,
    pub path: String,
    pub doc_type: String,  // "pdf", "docx", "txt", etc.
    pub size: u64,
    pub created_at: String,  // ISO 8601
    pub modified_at: String,
    pub excerpt: Option<String>,
    pub space_ids: Vec<String>,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub extracted_entities: Vec<ExtractedEntity>,
    pub thumbnail_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub label: String,
    pub value: String,
    pub entity_type: String,  // "date", "amount", "person", "organization", "location"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMeta {
    pub id: String,
    pub name: String,
    pub path: String,
    pub doc_type: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilters {
    pub doc_type: Option<String>,
    pub space_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub score: f64,
    pub matched_excerpt: Option<String>,
}

// === Space types ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub document_count: u32,
    pub last_updated: String,
    pub sub_spaces: Vec<Space>,
    pub parent_id: Option<String>,
    pub sample_files: Vec<String>,
}

// === Folder types ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedFolder {
    pub id: String,
    pub path: String,
    pub document_count: u32,
    pub last_scan: String,
    pub status: String,  // "watching", "paused", "error"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub folder_id: String,
    pub total_files: u32,
    pub processed_files: u32,
    pub status: String,  // "scanning", "complete", "error"
}

// === Analytics types ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total_documents: u32,
    pub smart_spaces: u32,
    pub last_scan: String,
    pub index_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceGraphNode {
    pub id: String,
    pub name: String,
    pub color: String,
    pub document_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceGraphEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceGraph {
    pub nodes: Vec<SpaceGraphNode>,
    pub edges: Vec<SpaceGraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchAnalytics {
    pub total_searches: u32,
    pub top_queries: Vec<String>,
    pub avg_results_per_query: f64,
}

// === Settings types ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,          // "dark", "light", "system"
    pub sidebar_collapsed: bool,
    pub embedding_model: String,  // "local", "openai"
    pub watched_folders: Vec<String>,
    pub excluded_patterns: Vec<String>,
    pub index_on_startup: bool,
    pub index_size: u64,        // Bytes — visible in Settings > Storage (locked decision)
    pub storage_path: String,   // Path to RuVector data dir — visible in Settings > Storage
}

// === Tag types ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub document_count: u32,
    pub tag_type: String,  // "auto", "user"
}

// === Activity types ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: String,
    pub action: String,    // "indexed", "moved", "tagged", "searched"
    pub subject: String,
    pub timestamp: String,
}
```

Add `mod types;` to `lib.rs`.

Run `cargo check` — all types must compile.
</detail>
</task>

<task id="03.2" effort="L">
<title>Implement all 20 IPC command stubs with spawn_blocking</title>
<detail>
Create `src-tauri/src/commands/mod.rs`:
```rust
pub mod documents;
pub mod spaces;
pub mod folders;
pub mod analytics;
pub mod settings;
```

Each command module follows this pattern — every function:
1. Is `pub async fn` with `#[tauri::command]`
2. Takes typed args + `state: State<'_, AppState>`
3. Wraps work in `tokio::task::spawn_blocking`
4. Returns `Result<T, AppError>`
5. Returns stub mock data for now

**src-tauri/src/commands/documents.rs** (4 commands):
- `index_document(path: String, state)` -> `Result<DocumentMeta, AppError>` — stub returns mock meta
- `search_documents(query: String, filters: SearchFilters, state)` -> `Result<Vec<SearchResult>, AppError>` — stub returns empty vec
- `get_document(id: String, state)` -> `Result<Document, AppError>` — stub returns mock document
- `get_related_documents(id: String, limit: usize, state)` -> `Result<Vec<Document>, AppError>` — stub returns empty vec

**src-tauri/src/commands/spaces.rs** (4 commands):
- `get_spaces(state)` -> `Result<Vec<Space>, AppError>` — stub returns empty vec
- `get_space_documents(space_id: String, state)` -> `Result<Vec<Document>, AppError>` — stub returns empty vec
- `move_document_to_space(doc_id: String, space_id: String, state)` -> `Result<(), AppError>` — stub returns Ok(())
- `recluster_spaces(state)` -> `Result<Vec<Space>, AppError>` — stub returns empty vec

**src-tauri/src/commands/folders.rs** (4 commands):
- `add_watched_folder(path: String, state)` -> `Result<WatchedFolder, AppError>` — stub returns mock folder
- `remove_watched_folder(id: String, state)` -> `Result<(), AppError>` — stub returns Ok(())
- `trigger_scan(folder_id: String, state)` -> `Result<ScanProgress, AppError>` — stub returns mock progress
- `get_watched_folders(state)` -> `Result<Vec<WatchedFolder>, AppError>` — stub returns empty vec

**src-tauri/src/commands/analytics.rs** (3 commands):
- `get_stats(state)` -> `Result<Stats, AppError>` — stub returns zeroed stats
- `get_space_graph(state)` -> `Result<SpaceGraph, AppError>` — stub returns empty graph
- `get_search_analytics(state)` -> `Result<SearchAnalytics, AppError>` — stub returns empty analytics

**src-tauri/src/commands/documents.rs** — add 1 more command (total 5):
- `toggle_favorite(id: String, state)` -> `Result<bool, AppError>` — stub returns `true`

**src-tauri/src/commands/analytics.rs** — add 2 more commands (total 5):
- `get_tags(state)` -> `Result<Vec<Tag>, AppError>` — stub returns empty vec
- `get_activity_feed(state)` -> `Result<Vec<ActivityItem>, AppError>` — stub returns empty vec

**src-tauri/src/commands/settings.rs** (2 commands):
- `get_settings(state)` -> `Result<Settings, AppError>` — stub returns default settings with `index_size: 0` and `storage_path` set to a placeholder (e.g. `"~/Library/Application Support/com.cortex.app/vectors"`)
- `update_settings(settings: Settings, state)` -> `Result<(), AppError>` — stub returns Ok(())

Example command pattern (use this exactly for ALL commands):
```rust
use tauri::State;
use crate::error::AppError;
use crate::state::AppState;
use crate::types::*;

#[tauri::command]
pub async fn search_documents(
    query: String,
    filters: SearchFilters,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, AppError> {
    let _engine = state.engine.clone();
    let results = tokio::task::spawn_blocking(move || {
        // Phase 2 will implement real search via RuVector
        Ok::<Vec<SearchResult>, AppError>(vec![])
    })
    .await??;
    Ok(results)
}
```

Update `src-tauri/src/lib.rs` to register all commands:
```rust
mod commands;
// ... existing mods ...

pub fn run() {
    // ... existing setup ...
    tauri::Builder::default()
        .manage(/* ... */)
        .invoke_handler(tauri::generate_handler![
            // documents (5)
            commands::documents::index_document,
            commands::documents::search_documents,
            commands::documents::get_document,
            commands::documents::get_related_documents,
            commands::documents::toggle_favorite,
            // spaces (4)
            commands::spaces::get_spaces,
            commands::spaces::get_space_documents,
            commands::spaces::move_document_to_space,
            commands::spaces::recluster_spaces,
            // folders (4)
            commands::folders::add_watched_folder,
            commands::folders::remove_watched_folder,
            commands::folders::trigger_scan,
            commands::folders::get_watched_folders,
            // analytics (5)
            commands::analytics::get_stats,
            commands::analytics::get_space_graph,
            commands::analytics::get_search_analytics,
            commands::analytics::get_tags,
            commands::analytics::get_activity_feed,
            // settings (2)
            commands::settings::get_settings,
            commands::settings::update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Run `cargo check` — all commands must compile.
Run `cargo test` — existing tests still pass.
</detail>
</task>

## Verification

```bash
# 1. Compiles with all commands registered
cd src-tauri && cargo check

# 2. All tests pass
cd src-tauri && cargo test

# 3. Command modules exist
test -f src-tauri/src/commands/mod.rs && \
test -f src-tauri/src/commands/documents.rs && \
test -f src-tauri/src/commands/spaces.rs && \
test -f src-tauri/src/commands/folders.rs && \
test -f src-tauri/src/commands/analytics.rs && \
test -f src-tauri/src/commands/settings.rs && \
echo "PASS" || echo "FAIL"

# 4. spawn_blocking is used in every command file
grep -l "spawn_blocking" src-tauri/src/commands/*.rs | wc -l
# Expected: 5 (all 5 command files)
```

## must_haves

- [ ] `src-tauri/src/types.rs` defines all IPC types (Document, Space, WatchedFolder, Stats, Settings, etc.) with Serialize + Deserialize
- [ ] All 20 IPC commands are stubbed as `#[tauri::command]` async functions (16 from CLAUDE.md + `get_watched_folders`, `get_tags`, `toggle_favorite`, `get_activity_feed`)
- [ ] Every command uses `tokio::task::spawn_blocking` for its body
- [ ] Every command returns `Result<T, AppError>` (not `Result<T, String>`)
- [ ] Every command takes `State<'_, AppState>` parameter
- [ ] All 20 commands are registered in `invoke_handler(tauri::generate_handler![...])`
- [ ] `cargo check` succeeds with all commands wired

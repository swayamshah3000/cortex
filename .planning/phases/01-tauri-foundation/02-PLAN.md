---
wave: 2
depends_on: [PLAN-01]
requirements: [TAURI-03, TAURI-06]
files_modified:
  - src-tauri/src/error.rs
  - src-tauri/src/state.rs
  - src-tauri/src/engine.rs
  - src-tauri/src/lib.rs
  - src-tauri/Cargo.toml
autonomous: true
---

# Plan 02: AppError Enum and AppState Struct

## Goal

The `AppError` enum and `AppState` struct are defined, compile, and are wired into the Tauri builder. `AppError` serializes to tagged JSON for frontend consumption. `CortexEngine` is a placeholder struct that will hold RuVector references in Plan 04.

## Context

- TAURI-03: AppError enum with `serde::Serialize` for all IPC error handling.
- TAURI-06: AppState struct with `Arc<CortexEngine>` and channel senders.
- Research specifies: `thiserror` for Error derive, `serde(tag = "kind", content = "message")` for tagged JSON serialization.
- `CortexEngine` is initialized as an empty struct now — RuVector fields added in Plan 04 (Wave 2).
- Use `tokio::sync::Mutex` (NOT `std::sync::Mutex`) because state is shared across async `.await` points.
- Plan 01 creates `src-tauri/` — this plan adds files inside it. This plan depends on Plan 01 (Wave 2 after Plan 01's Wave 1) because `src-tauri/` must exist before we can add `error.rs`, `state.rs`, `engine.rs`.

## Tasks

<task id="02.1" effort="M">
<title>Create AppError enum with thiserror and serde serialization</title>
<detail>
Add `thiserror = "1"` to `src-tauri/Cargo.toml` dependencies (if not already present from Plan 01).

Create `src-tauri/src/error.rs`:

```rust
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

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Not implemented")]
    NotImplemented,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        AppError::Internal(e.to_string())
    }
}
```

Write a unit test in the same file (or `src-tauri/src/error.rs` with `#[cfg(test)]` module):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_serializes_to_tagged_json() {
        let err = AppError::NotFound("doc-123".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""kind":"NotFound""#));
        assert!(json.contains(r#""message":"doc-123""#));
    }

    #[test]
    fn test_not_implemented_serializes() {
        let err = AppError::NotImplemented;
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains(r#""kind":"NotImplemented""#));
    }
}
```

Run `cargo test` in `src-tauri/` — both tests must pass.
</detail>
</task>

<task id="02.2" effort="M">
<title>Create AppState struct and placeholder CortexEngine</title>
<detail>
Create `src-tauri/src/engine.rs`:

```rust
/// CortexEngine holds all backend state (RuVector collections, filter indices).
/// Phase 1 Plan 04 will add real RuVector fields.
/// Phase 2+ will add document pipeline, file watcher channels.
pub struct CortexEngine {
    // Placeholder — RuVector fields added in Plan 04
}

impl CortexEngine {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {})
    }
}
```

Create `src-tauri/src/state.rs`:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use crate::engine::CortexEngine;

/// Commands sent to the file watcher background task.
pub enum WatcherCommand {
    Pause,
    Resume,
    Shutdown,
}

/// Events emitted by the indexing pipeline.
pub enum IndexEvent {
    DocumentIndexed { path: String },
    ScanComplete { folder_id: String },
    Error(String),
}

pub struct AppState {
    pub engine: Arc<Mutex<CortexEngine>>,
    /// Send commands to the file watcher (Phase 2+).
    pub watcher_tx: mpsc::Sender<WatcherCommand>,
    /// Receive indexing events from background pipeline (Phase 2+).
    pub index_rx: Arc<Mutex<mpsc::Receiver<IndexEvent>>>,
}
```

Update `src-tauri/src/lib.rs` to declare modules and wire AppState into Tauri builder:

```rust
mod error;
mod state;
mod engine;

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use state::AppState;
use engine::CortexEngine;

pub fn run() {
    let engine = CortexEngine::new().expect("CortexEngine initialization failed");

    // Placeholder channels — Phase 2+ will connect these to real background tasks.
    let (watcher_tx, _watcher_rx) = mpsc::channel(32);
    let (_index_tx, index_rx) = mpsc::channel(32);

    tauri::Builder::default()
        .manage(AppState {
            engine: Arc::new(Mutex::new(engine)),
            watcher_tx,
            index_rx: Arc::new(Mutex::new(index_rx)),
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Run `cargo check` in `src-tauri/` — must compile with no errors.
Run `cargo test` in `src-tauri/` — AppError tests still pass.
</detail>
</task>

## Verification

```bash
# 1. Compiles
cd src-tauri && cargo check

# 2. Tests pass
cd src-tauri && cargo test

# 3. Files exist
test -f src-tauri/src/error.rs && test -f src-tauri/src/state.rs && test -f src-tauri/src/engine.rs && echo "PASS" || echo "FAIL"

# 4. AppError serialization test specifically
cd src-tauri && cargo test test_app_error_serializes_to_tagged_json -- --exact
```

## must_haves

- [ ] `src-tauri/src/error.rs` exists with `AppError` enum deriving `Debug, Error, Serialize`
- [ ] `AppError` uses `#[serde(tag = "kind", content = "message")]` for tagged JSON
- [ ] `AppError` has variants: VectorStorage, NotFound, Io, Parse, NotImplemented, Internal
- [ ] `From<std::io::Error>` and `From<tokio::task::JoinError>` impls exist on AppError
- [ ] Unit test proves AppError serializes to `{"kind":"NotFound","message":"doc-123"}` format
- [ ] `src-tauri/src/state.rs` has `AppState` with `Arc<Mutex<CortexEngine>>`, `mpsc::Sender<WatcherCommand>`, and `Arc<Mutex<mpsc::Receiver<IndexEvent>>>`
- [ ] `src-tauri/src/engine.rs` has `CortexEngine` with `new()` returning `Result`
- [ ] `lib.rs` wires `AppState` into `tauri::Builder::default().manage()`
- [ ] `cargo check` and `cargo test` pass in `src-tauri/`

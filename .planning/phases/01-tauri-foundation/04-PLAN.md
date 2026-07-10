---
wave: 3
depends_on: [PLAN-02]
requirements: [VSTOR-01, VSTOR-02, VSTOR-03, VSTOR-04]
files_modified:
  - src-tauri/Cargo.toml
  - src-tauri/src/engine.rs
  - src-tauri/src/lib.rs
autonomous: true
---

# Plan 04: RuVector Core Integration and Multi-Collection Storage

## Goal

CortexEngine initializes RuVector with multi-collection support (384-dim for local ONNX, 1536-dim for OpenAI API), metadata filter indices are created for `doc_type`, `created_at`, `space_ids`, and `tags`, and hybrid query plumbing (filter + vector) is structurally ready. The engine uses Tauri's `app_data_dir()` for storage via the `setup` hook.

## Context

- VSTOR-01: RuVector core integration with HNSW indexing.
- VSTOR-02: Multi-collection support (separate indices per embedding dimension).
- VSTOR-03: Metadata filtering (type, date range, space, tags) before vector search.
- VSTOR-04: Hybrid queries: structured filters + semantic similarity.
- RuVector source is at `/Users/gshah/work/apps/experiments/ruvector/`.
- Research confirms: `ruvector-core`, `ruvector-collections`, `ruvector-filter` are all workspace members.
- Path deps from `src-tauri/`: `../../experiments/ruvector/crates/ruvector-core` (etc.).
- **Pitfall**: RuVector workspace exclusions — only depend on confirmed workspace members.
- **Pitfall**: Cargo workspace conflict — Cortex's `src-tauri/` is its own workspace, RuVector is separate. Path deps must resolve correctly.
- Use Tauri `setup` hook to get `app.path().app_data_dir()` for proper storage location.
- Use `tokio::sync::Mutex` for engine (shared across async commands).

## Tasks

<task id="04.1" effort="S">
<title>Add and verify RuVector path dependencies</title>
<detail>
Add these to `src-tauri/Cargo.toml` `[dependencies]`:
```toml
ruvector-core = { path = "../../../experiments/ruvector/crates/ruvector-core" }
ruvector-collections = { path = "../../../experiments/ruvector/crates/ruvector-collections" }
ruvector-filter = { path = "../../../experiments/ruvector/crates/ruvector-filter" }
```

Note the path: from `src-tauri/` up to `cortex/` (../) up to `apps/` (../../) then into `experiments/ruvector/crates/...` (../../../experiments/ruvector/crates/...). Verify by running `ls` from `src-tauri/` with the relative path.

Run `cargo check` — if path deps fail, adjust paths. This is the flagged pitfall from STATE.md.

Done when: `cargo check` succeeds with all three ruvector path deps resolving.
</detail>
</task>

<task id="04.2" effort="M">
<title>Implement CortexEngine with RuVector fields and write test</title>
<detail>
Update `src-tauri/src/engine.rs`:

```rust
use ruvector_collections::{CollectionManager, CollectionConfig};
use ruvector_core::types::{DistanceMetric, HnswConfig};
use ruvector_filter::{PayloadIndexManager, IndexType};
use std::path::PathBuf;

pub struct CortexEngine {
    pub collections: CollectionManager,
    pub filter_index: PayloadIndexManager,
}

impl CortexEngine {
    pub fn new_with_path(data_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&data_dir)?;

        let collections = CollectionManager::new(data_dir)?;

        // 384-dim: local ONNX embeddings (all-MiniLM-L6-v2)
        collections.create_collection("documents_384", CollectionConfig {
            dimensions: 384,
            distance_metric: DistanceMetric::Cosine,
            hnsw_config: Some(HnswConfig::default()),
            quantization: None,
            on_disk_payload: true,
        }).ok(); // Ignore AlreadyExists on restart

        // 1536-dim: OpenAI API embeddings (opt-in, Phase 2)
        collections.create_collection("documents_1536", CollectionConfig {
            dimensions: 1536,
            distance_metric: DistanceMetric::Cosine,
            hnsw_config: Some(HnswConfig::default()),
            quantization: None,
            on_disk_payload: true,
        }).ok();

        // Metadata filter indices for pre-search filtering
        let mut filter_index = PayloadIndexManager::new();
        filter_index.create_index("doc_type", IndexType::Keyword)?;
        filter_index.create_index("created_at", IndexType::Integer)?;
        filter_index.create_index("space_ids", IndexType::Keyword)?;
        filter_index.create_index("tags", IndexType::Keyword)?;

        Ok(Self { collections, filter_index })
    }
}
```

IMPORTANT: The exact RuVector API calls (constructor args, method names, config structs) MUST be verified against the actual source at `/Users/gshah/work/apps/experiments/ruvector/crates/`. The research document provides the expected API, but the executor MUST read the actual source files if compilation fails. Key files to check:
- `ruvector/crates/ruvector-collections/src/lib.rs` — `CollectionManager` constructor and `create_collection` signature
- `ruvector/crates/ruvector-core/src/types.rs` or `src/lib.rs` — `DistanceMetric`, `HnswConfig` location
- `ruvector/crates/ruvector-filter/src/lib.rs` — `PayloadIndexManager` and `IndexType` location

Adapt the code to match the actual API if it differs from research.

**Wire engine initialization via Tauri setup hook**

Update `src-tauri/src/lib.rs` to use the `setup` hook:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use state::AppState;
use engine::CortexEngine;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()
                .expect("could not resolve app data dir")
                .join("vectors");

            let engine = CortexEngine::new_with_path(data_dir)
                .expect("RuVector initialization failed");

            app.manage(AppState {
                engine: Arc::new(Mutex::new(engine)),
            });
            Ok(())
        })
        // .invoke_handler(...) from Plan 03
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Note: If Plan 03's invoke_handler is already in place, keep it. Just move the `.manage()` call into `.setup()`.

Write a test for CortexEngine initialization using a temp directory:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_engine_initializes_with_temp_dir() {
        let tmp = std::env::temp_dir().join("cortex-test-engine");
        let engine = CortexEngine::new_with_path(tmp.clone());
        assert!(engine.is_ok(), "Engine failed to initialize: {:?}", engine.err());
        // Cleanup
        let _ = std::fs::remove_dir_all(tmp);
    }
}
```

Run `cargo check` — must compile.
Run `cargo test` — all tests pass including engine init test.
</detail>
</task>

## Verification

```bash
# 1. RuVector path deps resolve
cd src-tauri && cargo check

# 2. All tests pass including engine init
cd src-tauri && cargo test

# 3. Engine test specifically
cd src-tauri && cargo test test_engine_initializes_with_temp_dir -- --exact

# 4. RuVector deps in Cargo.toml
grep "ruvector-core" src-tauri/Cargo.toml && echo "PASS" || echo "FAIL"
grep "ruvector-collections" src-tauri/Cargo.toml && echo "PASS" || echo "FAIL"
grep "ruvector-filter" src-tauri/Cargo.toml && echo "PASS" || echo "FAIL"
```

## must_haves

- [ ] `src-tauri/Cargo.toml` has path dependencies for `ruvector-core`, `ruvector-collections`, `ruvector-filter`
- [ ] `cargo check` succeeds with RuVector path deps resolving correctly
- [ ] `CortexEngine` has `collections: CollectionManager` and `filter_index: PayloadIndexManager` fields
- [ ] `CortexEngine::new_with_path()` creates two collections: `documents_384` (384-dim, Cosine) and `documents_1536` (1536-dim, Cosine)
- [ ] Four metadata filter indices created: `doc_type`, `created_at`, `space_ids`, `tags`
- [ ] Engine initialization uses Tauri `setup` hook with `app.path().app_data_dir()`
- [ ] Unit test proves `CortexEngine::new_with_path(temp_dir)` succeeds
- [ ] `cargo test` passes all tests

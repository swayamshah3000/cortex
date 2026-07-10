# Stack Research

**Domain:** Tauri 2 desktop app — Rust backend with document processing, ONNX embeddings, and RuVector vector search
**Researched:** 2026-02-27
**Confidence:** HIGH (core Tauri/Rust) | MEDIUM (document parsing) | HIGH (embeddings via fastembed-rs)

---

## Scope

This document covers only the **Rust backend layer** being added to the existing React frontend. The frontend stack (React 19, TypeScript, Vite, TailwindCSS 4, shadcn/ui, Zustand, React Query, React Router v7, Lucide React) is already decided and in place. Do not re-evaluate or change those choices.

---

## Recommended Stack

### Core Framework

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| tauri | 2.10.2 | Desktop shell + IPC bridge | Latest stable as of Feb 2026. Replaces Express server. Native file system access, system tray, auto-updater — all built-in. Smallest binary footprint of any Rust desktop framework. |
| tauri-build | 2.x (match tauri) | Build-time Tauri codegen | Required as `build-dependency`. Must match tauri minor version exactly or builds fail. |
| tokio | 1.41 | Async runtime | Already pinned in RuVector workspace. All Tauri async commands run on Tokio's multi-thread runtime. Use `features = ["rt-multi-thread", "sync", "macros", "fs"]`. |
| serde / serde_json | 1.0 | IPC serialization | All Tauri command parameters and return values cross the IPC boundary as JSON. Mandatory. `derive` feature required for struct annotations. |

### Tauri Official Plugins

| Plugin | Version | Purpose | Why |
|--------|---------|---------|-----|
| tauri-plugin-fs | 2.x | File system read access | Scoped access to user directories (~/Documents, ~/Downloads). Replaces raw `std::fs` for frontend-triggered reads. |
| tauri-plugin-dialog | 2.4.2 | Native folder picker | Required for onboarding "Select Folders" step. Uses OS-native dialog — no web emulation. |
| tauri-plugin-notification | 2.3.3 | System notifications | Background indexing complete alerts. Push to system notification center. |
| tauri-plugin-updater | 2.9.0 | Auto-update mechanism | In-app update check and install. Required for desktop distribution. |
| tauri-plugin-shell | 2.x | Shell command access | Optional — only needed if spawning external OCR processes (tesseract binary). Skip if using leptess bindings directly. |

### File Watching

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| notify | 8.2.0 | Cross-platform file watching | Official Rust file watcher. Uses FSEvents (macOS), inotify (Linux), ReadDirectoryChangesW (Windows). v8.x stable, v9.x RC only. Bridge to Tokio: use `notify`'s channel feature with `tokio::sync::mpsc`. Do NOT use `async-watcher` third-party — notify 8.x handles this via `crossbeam-channel` + Tokio bridge pattern. |

**Tokio bridge pattern (required):**
```rust
let (tx, mut rx) = tokio::sync::mpsc::channel(100);
let mut watcher = notify::recommended_watcher(move |res| {
    tx.blocking_send(res).ok();
})?;
tokio::spawn(async move {
    while let Some(event) = rx.recv().await { /* handle */ }
});
```

### Document Parsing

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| pdf-extract | 0.10.0 | PDF text extraction | Highest-level API for text-only extraction. Builds on lopdf internally. Use this — not lopdf directly — for the document indexing pipeline. lopdf is for low-level manipulation (editing, form filling) which we never need. |
| docx-rust | latest | DOCX text extraction | Use `docx-rust` (not `docx-rs`). `docx-rs` is a writer-first library; `docx-rust` supports both reading and writing. For read-only text extraction from user .docx files, `docx-rust` is the correct choice. |
| calamine | 0.33.0 | XLSX/XLS/ODS reading | Pure Rust, zero native deps. Reads all Excel formats. Read-only, which is exactly what Cortex needs (we never write spreadsheets). |
| leptess | 0.14.0 | OCR for image files | Rust bindings for Leptonica + Tesseract 4/5. Requires tesseract installed on host system. Make this optional — gate behind a `ocr` feature flag and skip gracefully if tesseract not present. Alternative: skip OCR at v1, add later. |

**What NOT to use for parsing:**
- `lopdf` directly — too low-level, pdf-extract wraps it correctly
- `docx-rs` — writer library, not optimized for reading/extraction
- Any PDF-to-image conversion approach — unnecessary complexity

### Embedding Engine

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| fastembed | 5 | Local ONNX text embeddings | **Primary recommendation.** Wraps ort (ONNX Runtime via pykeio/ort) with a high-level API. Ships with bundled ONNX models, no separate model download step at runtime. Directly supports `EmbeddingModel::AllMiniLML6V2` (all-MiniLM-L6-v2, 384-dim). Downloads model once, caches locally. Eliminates the need to manually manage ort session lifecycle. |
| ort | 2.0.0-rc.11 | ONNX Runtime Rust bindings | **Use indirectly via fastembed.** Only add ort as a direct dependency if you need custom model sessions beyond what fastembed provides. ort 2.0.0-rc.11 is the final RC before 2.0 stable. It powers fastembed under the hood. |
| reqwest | 0.12 | Optional API embeddings | HTTP client for OpenAI `text-embedding-3-small` API calls when user opts into cloud embeddings. Use `features = ["json", "rustls-tls"]`. Avoid openssl feature — rustls ships without system dep. |

**Why fastembed over raw ort:** fastembed bundles model management, tokenization, and batching. Using raw ort requires manual ONNX session setup, tokenizer integration, and model file distribution — 3-4x more code with the same result.

**Why NOT rust-bert:** rust-bert depends on tch-rs (PyTorch bindings) which requires a ~500MB libtorch download and system PyTorch installation. Completely incompatible with the <50MB app size target.

### RuVector Integration (Local Path Dependency)

| Crate | Source | Purpose |
|-------|--------|---------|
| ruvector-core | path = `../../experiments/ruvector/crates/ruvector-core` | Vector storage + HNSW indexing |
| ruvector-gnn | path = `../../experiments/ruvector/crates/ruvector-gnn` | GNN clustering → Smart Spaces |
| ruvector-graph | path = `../../experiments/ruvector/crates/ruvector-graph` | Cypher graph queries, document relationships |
| ruvector-cluster | path = `../../experiments/ruvector/crates/ruvector-cluster` | Clustering algorithms |
| ruvector-filter | path = `../../experiments/ruvector/crates/ruvector-filter` | Metadata pre-filtering |
| ruvector-collections | path = `../../experiments/ruvector/crates/ruvector-collections` | Per-space separate indices |
| ruvector-domain-expansion | path = `../../experiments/ruvector/crates/ruvector-domain-expansion` | Transfer learning between spaces |
| ruvector-attention | path = `../../experiments/ruvector/crates/ruvector-attention` | Search result re-ranking (46 mechanisms) |
| sona | path = `../../experiments/ruvector/crates/sona` | SONA self-learning engine |

**Feature flags for ruvector-core:** Use `default-features = false, features = ["simd", "storage", "hnsw", "parallel"]`. Omit `api-embeddings` — Cortex manages its own embedding pipeline via fastembed.

**Cargo.toml path reference format:**
```toml
[dependencies]
ruvector-core = { path = "../../experiments/ruvector/crates/ruvector-core", default-features = false, features = ["simd", "storage", "hnsw", "parallel"] }
ruvector-gnn = { path = "../../experiments/ruvector/crates/ruvector-gnn" }
```

### Entity Extraction

| Approach | Library | Why |
|----------|---------|-----|
| Pattern-based (Phase 1) | `regex` 1.x | Dates, amounts, email addresses, phone numbers via regex patterns. Zero native deps, <1ms per document. Sufficient for MVP. |
| ML-based (Phase 2, optional) | rust-bert NER pipeline | Full named entity recognition (Person, Organization, Location). Heavy: requires ~400MB model. Gate behind user setting. Use `rust_bert::pipelines::ner::NERModel`. |

**Start with regex-based extraction.** It covers 80% of the value (dates for "property tax documents from last year", dollar amounts, emails) with 0% of the complexity. Add ML NER in a later phase if needed.

### Space Naming (LLM)

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| ollama-rs | latest | Local LLM via Ollama | Type-safe Rust client for Ollama API. Connects to localhost:11434. Use for naming auto-discovered GNN clusters. Model: llama3.2 or phi3-mini (fast, small). |
| reqwest | 0.12 | OpenAI/Claude API fallback | Direct HTTP client for cloud LLM APIs when Ollama not installed. Use same reqwest instance as embedding API client. |

**Both are optional:** Space naming is a best-effort feature. If Ollama is not installed and no API key is configured, name clusters with generic labels ("Space 1", "Space 2") until user configures an LLM.

### Async & Concurrency

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| tokio | 1.41 | Async runtime | See above. All Tauri commands are `async fn`. Use `tokio::sync::Mutex` only when guard must be held across `.await` points — otherwise use `parking_lot::Mutex` (already in RuVector, faster). |
| crossbeam | 0.8 | Channels + work-stealing | Already in RuVector workspace. Use for notify → indexing pipeline handoff. |
| rayon | 1.10 | CPU parallelism | Already in RuVector workspace. Use for batch document parsing. |

### Error Handling

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| thiserror | 2.0 | Error type definitions | Already in RuVector. Define `CortexError` with variants for each subsystem (ParseError, EmbeddingError, WatcherError, StorageError). |
| anyhow | 1.0 | Tauri command errors | Tauri commands return `Result<T, String>` to JS. Use `anyhow::Error` internally, convert to String at the command boundary. |

**Tauri command error pattern:**
```rust
#[tauri::command]
async fn search_documents(query: String, state: State<'_, AppState>) -> Result<Vec<SearchResult>, String> {
    state.search(&query).await.map_err(|e| e.to_string())
}
```

### Supporting Utilities

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| chrono | 0.4 | DateTime handling | Already in RuVector. For created_at, modified_at, last_scan timestamps. |
| uuid | 1.11 | Document/space IDs | Already in RuVector (v4 feature). Use same UUID generation. |
| tracing | 0.1 | Structured logging | Already in RuVector. Wire to Tauri's log plugin or tracing-subscriber for file output. |
| sha2 | latest | Content hash for change detection | SHA-256 hash of file contents for `content_hash` field. Detects file changes without stat comparison. |

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Embeddings | fastembed 5 | raw ort 2.x | ort requires manual ONNX session, tokenizer, model management — 3-4x more code for same result |
| Embeddings | fastembed 5 | rust-bert | Requires libtorch (~500MB), system PyTorch — incompatible with <50MB app target |
| PDF parsing | pdf-extract 0.10 | lopdf directly | lopdf is low-level manipulation; pdf-extract wraps it correctly for text extraction |
| DOCX parsing | docx-rust | docx-rs | docx-rs is writer-first; docx-rust is designed for both read and write |
| File watching | notify 8.2 | async-watcher | async-watcher is third-party thin wrapper; use notify 8.x directly with Tokio channel bridge |
| Desktop shell | Tauri 2 | Electron | Tauri: <50MB binary, Rust backend, native WebView. Electron: 100-200MB, V8 runtime, JS backend |
| LLM client | ollama-rs | direct reqwest | ollama-rs provides type-safe models, streaming support, error handling — saves 200+ lines of boilerplate |
| Entity extraction | regex (Phase 1) | rust-bert NER (Phase 2) | rust-bert NER requires large model download; regex covers MVP needs at zero cost |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `docx-rs` | Writer-first library, not optimized for text extraction reads | `docx-rust` |
| `rust-bert` for embeddings | Requires libtorch 500MB, incompatible with app size target | `fastembed` |
| `onnxruntime` (old crate) | Unmaintained since 2021, wraps ONNX Runtime 1.8 | `ort` (pykeio/ort) via `fastembed` |
| `wry` directly | Low-level WebView — Tauri 2 wraps it correctly | `tauri` |
| `async-watcher` | Thin wrapper with less active maintenance | `notify` 8.x + Tokio bridge |
| `openssl` feature in reqwest | Requires system OpenSSL, causes packaging nightmares on macOS | `rustls-tls` feature |
| `std::sync::Mutex` in Tauri commands | Cannot hold guard across `.await` points | `tokio::sync::Mutex` when holding across await; `parking_lot::Mutex` otherwise |
| `tauri::State` with borrowed `&str` params | Async commands cannot borrow across await | Convert to `String` params in all async commands |

---

## src-tauri Cargo.toml Structure

```toml
[package]
name = "cortex"
version = "0.1.0"
edition = "2021"
rust-version = "1.77"

[lib]
name = "cortex_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
# Tauri core
tauri = { version = "2.10", features = ["tray-icon", "image-png"] }
tauri-plugin-fs = "2"
tauri-plugin-dialog = "2"
tauri-plugin-notification = "2"
tauri-plugin-updater = "2"

# RuVector (local path)
ruvector-core = { path = "../../experiments/ruvector/crates/ruvector-core", default-features = false, features = ["simd", "storage", "hnsw", "parallel"] }
ruvector-gnn = { path = "../../experiments/ruvector/crates/ruvector-gnn" }
ruvector-graph = { path = "../../experiments/ruvector/crates/ruvector-graph" }
ruvector-filter = { path = "../../experiments/ruvector/crates/ruvector-filter" }
ruvector-cluster = { path = "../../experiments/ruvector/crates/ruvector-cluster" }
ruvector-collections = { path = "../../experiments/ruvector/crates/ruvector-collections" }
ruvector-attention = { path = "../../experiments/ruvector/crates/ruvector-attention" }
sona = { path = "../../experiments/ruvector/crates/sona" }

# Document parsing
pdf-extract = "0.10"
docx-rust = "0.2"          # verify latest on crates.io
calamine = "0.33"
leptess = { version = "0.14", optional = true }

# Embeddings
fastembed = "5"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }

# LLM (space naming)
ollama-rs = { version = "0.2", optional = true }  # verify latest on crates.io

# File watching
notify = "8.2"

# Async
tokio = { version = "1.41", features = ["rt-multi-thread", "sync", "macros", "fs"] }

# Entity extraction
regex = "1"
sha2 = "0.10"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "2.0"
anyhow = "1.0"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.11", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[features]
default = []
ocr = ["leptess"]
ollama = ["ollama-rs"]
```

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| tauri 2.10.2 | tauri-build 2.x | Must match major+minor. tauri-build 2.x pinned to same minor as tauri. |
| tauri 2.10.2 | tauri-plugin-* 2.x | All official plugins follow tauri major version. Plugin minor can lag. |
| fastembed 5 | ort 2.0.0-rc.11 | fastembed 5 uses ort internally. Do NOT add separate ort dependency unless you know the exact version fastembed uses — version conflicts will break compilation. |
| ruvector-core 2.0.5 | tokio 1.41 | RuVector workspace pins tokio 1.41. Cortex must use same major.minor. |
| notify 8.2.0 | tokio 1.41 | notify 8.x uses crossbeam-channel which is Tokio-compatible via channel bridge pattern. |
| pdf-extract 0.10 | lopdf 0.38+ | pdf-extract depends on lopdf internally. Do NOT add lopdf as direct dependency. |
| reqwest 0.12 | tokio 1.x | reqwest 0.12 is the tokio 1.x compatible version. reqwest 0.11 uses tokio 0.x. |

---

## Tauri Command Patterns

### Standard async command
```rust
#[tauri::command]
async fn search_documents(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    state.vector_engine
        .search(&query)
        .await
        .map_err(|e| e.to_string())
}
```

### Emitting progress events to frontend (for indexing)
```rust
app_handle.emit("indexing-progress", IndexingProgress { processed: n, total: m }).unwrap();
```

### AppState structure
```rust
pub struct AppState {
    pub vector_engine: Arc<tokio::sync::Mutex<VectorEngine>>,
    pub settings: Arc<parking_lot::Mutex<Settings>>,
    pub watcher: Arc<tokio::sync::Mutex<Option<RecommendedWatcher>>>,
}
```

**Rule:** Use `tokio::sync::Mutex` for any state accessed across `.await` points. Use `parking_lot::Mutex` for synchronous settings access (faster, no async overhead).

---

## Installation

```bash
# Add Tauri CLI (if not installed)
cargo install tauri-cli --version "^2"

# Initialize Tauri in existing React project
# Run from /Users/gshah/work/apps/cortex
cargo tauri init

# This creates src-tauri/ with:
#   Cargo.toml, build.rs, src/main.rs, src/lib.rs, tauri.conf.json, capabilities/

# Dev (frontend + Tauri backend together)
cargo tauri dev

# Production build
cargo tauri build
```

**tauri.conf.json key settings:**
```json
{
  "build": {
    "beforeDevCommand": "bun dev",
    "beforeBuildCommand": "bun run build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../dist"
  },
  "app": {
    "identifier": "com.cortex.app"
  }
}
```

---

## Sources

- [Tauri 2 Releases](https://github.com/tauri-apps/tauri/releases) — verified v2.10.2 latest stable (Feb 2026) | HIGH confidence
- [Tauri 2 State Management](https://v2.tauri.app/develop/state-management/) — AppState patterns, Mutex guidance | HIGH confidence
- [Tauri 2 Calling Rust](https://v2.tauri.app/develop/calling-rust/) — IPC command signatures | HIGH confidence
- [Tauri 2 Project Structure](https://v2.tauri.app/start/project-structure/) — src-tauri layout | HIGH confidence
- [tauri-plugin-dialog 2.4.2](https://docs.rs/crate/tauri-plugin-dialog/latest) — confirmed version | HIGH confidence
- [tauri-plugin-updater 2.9.0](https://docs.rs/crate/tauri-plugin-updater/latest) — confirmed version | HIGH confidence
- [fastembed-rs GitHub](https://github.com/Anush008/fastembed-rs) — version 5, all-MiniLM-L6-v2 support confirmed | HIGH confidence
- [ort GitHub releases](https://github.com/pykeio/ort/releases) — v2.0.0-rc.11 (Jan 2025), final RC | MEDIUM confidence
- [notify GitHub releases](https://github.com/notify-rs/notify/releases) — v8.2.0 stable, v9.0.0-rc.2 pre-release | HIGH confidence
- [pdf-extract crates.io](https://crates.io/crates/pdf-extract) — v0.10.0 (Oct 2025) | HIGH confidence
- [calamine crates.io](https://crates.io/crates/calamine) — v0.33.0 latest | HIGH confidence
- [RuVector Cargo.toml](file:///Users/gshah/work/apps/experiments/ruvector/Cargo.toml) — workspace deps, versions 2.0.5 | HIGH confidence (local source)
- [ollama-rs GitHub](https://github.com/pepperoni21/ollama-rs) — Rust Ollama client | MEDIUM confidence (version not pinned)
- [Tauri 2 Calling Frontend](https://v2.tauri.app/develop/calling-frontend/) — event emission for progress | HIGH confidence

---

*Stack research for: Cortex — Rust backend + RuVector integration (subsequent milestone)*
*Researched: 2026-02-27*

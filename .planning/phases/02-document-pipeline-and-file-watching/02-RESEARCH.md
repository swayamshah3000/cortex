# Phase 2: Document Pipeline and File Watching - Research

**Researched:** 2026-02-27
**Domain:** Document parsing (PDF/DOCX/XLSX), local embeddings (fastembed), file watching (notify-rs), content hashing, entity extraction, Tauri background events
**Confidence:** HIGH — RuVector APIs verified from source; library APIs verified from official docs; Tauri event API from official v2 docs

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DPIP-01 | PDF text extraction via pdf-extract/lopdf | `pdf-extract` crate v0.10.0 — `extract_text(&bytes)` or `extract_text_from_mem(&bytes)`. Verified from crates.io. |
| DPIP-02 | DOCX parsing via docx-rust | `docx-rust` crate — `DocxFile::from_file(path)?.parse()?` then iterate paragraphs. Verified from docs. |
| DPIP-03 | Plain text and Markdown direct read | `std::fs::read_to_string(path)` — no external crate needed. |
| DPIP-04 | Spreadsheet indexing (XLSX, CSV) via calamine | `calamine` crate — `open_workbook_auto(path)`, `worksheet_range("Sheet1")`, iterate DataType::String cells. Verified from docs. |
| DPIP-05 | OCR for images via tesseract bindings (opt-in per folder) | `tesseract` crate with system `libtesseract` — `tesseract::ocr(path, "eng")`. Deferred; needs system dep research. LOW confidence until verified. |
| DPIP-06 | Local ONNX embedding generation (all-MiniLM-L6-v2, 384-dim) via fastembed | `fastembed` v5.11.0 — `TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))` + `model.embed(texts, None)`. Verified from docs.rs. |
| DPIP-07 | Optional API embedding (OpenAI text-embedding-3-small, 1536-dim) | ruvector-core `ApiEmbedding::openai(key, "text-embedding-3-small")` already wired; 1536-dim collection `documents_1536` already exists. Read from source. |
| DPIP-08 | Content hash computation for change detection | `sha2` crate + `digest` — read file in 4KB chunks, SHA-256 hash → hex string. Store hash in VectorEntry metadata. HIGH confidence. |
| DPIP-09 | Entity extraction: dates, amounts, people, organizations, locations | Regex-based approach using `regex` crate — patterns for ISO dates, monetary amounts, capitalized sequences. Rule-based for structured types; heuristic for persons/orgs. No ML model needed for v1. HIGH confidence for structured entities, MEDIUM for named entities. |
| FWAT-01 | Watched folder monitoring via notify-rs with debounce (300ms) | `notify-debouncer-mini` crate — `new_debouncer(Duration::from_millis(300), handler)` + `watcher.watch(path, RecursiveMode::Recursive)`. Verified from docs.rs. |
| FWAT-02 | Polling fallback for event-dropped scenarios | notify-rs `PollWatcher` as fallback when `RecommendedWatcher` fails — or polling timer as secondary check. MEDIUM confidence on exact API. |
| FWAT-03 | File type toggles per watched folder | Stored in `WatcherRegistry` as per-folder config; checked in the file event handler before dispatching to indexing pipeline. Pure application logic — no library needed. |
| FWAT-04 | Exclusion patterns (node_modules, .git, hidden files) | Path prefix/suffix matching in the event handler — check against `excluded_patterns` list stored with the watched folder. Pure application logic. |
| FWAT-05 | Background indexing as Tokio task with progress events emitted to frontend | `tauri::AppHandle` cloned into background task, `app_handle.emit("index-progress", payload)` from `tauri::Emitter` trait. Verified from Tauri v2 official docs. |
| FWAT-06 | Re-index on document modification (content hash comparison) | Compute hash of modified file, compare with stored hash in vector metadata, skip if equal, delete old vector + insert new if changed. Uses `VectorDB::delete(id)` + `VectorDB::insert(entry)`. |
</phase_requirements>

---

## Summary

Phase 2 builds the complete document ingestion pipeline: watched folders are monitored by `notify-debouncer-mini`, file change events flow into a background Tokio task, each file is parsed by type-specific extractors (pdf-extract, docx-rust, calamine, or plain `read_to_string`), text is embedded by `fastembed` running all-MiniLM-L6-v2 locally (or optionally OpenAI API), the SHA-256 content hash is checked for changes, entities are extracted via regex patterns, and the full document vector + metadata is upserted into RuVector via `CollectionManager::get_collection("documents_384").db.insert()`.

The Tauri event system (`app_handle.emit("index-progress", payload)`) connects the background pipeline to the frontend without blocking the UI. The `AppState` struct in Phase 1 already includes `watcher_tx: mpsc::Sender<WatcherCommand>` and `index_rx: mpsc::Receiver<IndexEvent>` channels as stubs waiting for Phase 2 implementation.

**Critical discovery:** The ruvector-core `embeddings.rs` module documents `CandleEmbedding` as a stub that always errors. The library explicitly warns: "For production use, we recommend: 1. Using the API-based providers (simpler, always up-to-date) 2. Using ONNX Runtime with pre-exported models." This means `fastembed` (which wraps ONNX Runtime) is the correct path for local embeddings — NOT ruvector's built-in embedding system. fastembed handles model download, tokenization, and ONNX inference in one package.

**Primary recommendation:** Use `fastembed` for all local embedding generation. Wire notify-debouncer-mini in a persistent background task spawned at app startup. Store WatchedFolder state in a JSON file (sled or simple JSON persistence) since RuVector stores vectors, not configuration. Implement a `WatcherRegistry` struct that persists folder configs across restarts.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| fastembed | 5.11.0 | Local ONNX embedding generation (all-MiniLM-L6-v2, 384-dim) | Self-contained ONNX runtime, downloads model on first run, fastest local embedding path for Rust |
| notify-debouncer-mini | latest | File system event watching with debounce | Official companion to `notify` crate; handles event collapsing automatically at configurable interval |
| pdf-extract | 0.10.0 | PDF text extraction | Simple `extract_text(&bytes)` API, pure Rust, no system deps |
| docx-rust | latest | DOCX parsing | `DocxFile::from_file(path).parse()` returns structured Docx |
| calamine | latest | XLSX/XLS/ODS reading | Pure Rust Excel reader; `open_workbook_auto` detects format automatically |
| sha2 | 0.10+ | SHA-256 content hashing for change detection | RustCrypto standard; chunked file read avoids memory issues on large docs |
| regex | 1.x | Entity extraction pattern matching | Zero-dependency regex, linear time matching |
| uuid | 1.x (features=["v4"]) | Stable document IDs | Already in ruvector-core workspace; v4 UUIDs as document primary keys |
| serde_json | 1.x | WatcherRegistry persistence | Already in Cargo.toml; use for folder config file |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio::fs | (tokio "full") | Async file reading for large docs | Use for reading text/markdown; pdf/docx parsers use sync APIs internally |
| tauri::Emitter | (tauri 2.x) | Emit progress events to frontend | Required for FWAT-05; already a dep |
| ruvector-core ApiEmbedding | (path dep, feature = "api-embeddings") | OpenAI API embedding (DPIP-07) | Only activated when user selects "openai" in settings; 1536-dim collection |
| tesseract | 0.x | OCR for images | Only if DPIP-05 is in scope — has system library dependency (`libtesseract`) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| fastembed | ort (ONNX Runtime directly) | fastembed bundles model downloads and tokenization; ort requires manual model management |
| fastembed | candle-transformers | candle is a stub in ruvector-core; requires significant custom implementation |
| notify-debouncer-mini | notify + manual debounce | notify-debouncer-mini is the official companion package — less error-prone |
| pdf-extract | pdfium-render | pdfium requires distributing native binaries; pdf-extract is pure Rust |
| regex entity extraction | rust-bert NLP pipeline | rust-bert adds large model deps; overkill for structured entity types in v1 |
| JSON file for WatcherRegistry | sled embedded DB | sled adds complexity; simple JSON serialization is sufficient for watched folder config |

**Installation additions to src-tauri/Cargo.toml:**
```toml
fastembed = "5"
notify-debouncer-mini = "*"
pdf-extract = "0.10"
docx-rust = "*"
calamine = { version = "*", features = ["dates"] }
sha2 = "0.10"
regex = "1"
uuid = { version = "1", features = ["v4"] }
```

---

## Architecture Patterns

### Recommended Module Structure

```
src-tauri/src/
├── commands/
│   ├── documents.rs     # index_document (REAL impl), search_documents stub
│   ├── folders.rs       # add_watched_folder, remove_watched_folder, trigger_scan, get_watched_folders (REAL impl)
│   └── ...              # other commands unchanged
├── pipeline/            # NEW: Phase 2 core modules
│   ├── mod.rs           # re-exports
│   ├── parser.rs        # DocumentParser: per-type text extraction
│   ├── embedder.rs      # EmbeddingService: fastembed wrapper, lazy-init
│   ├── hasher.rs        # content_hash(path) -> String
│   ├── entities.rs      # EntityExtractor: regex patterns
│   └── indexer.rs       # DocumentIndexer: orchestrates parse→embed→store
├── watcher/             # NEW: file watching
│   ├── mod.rs           # re-exports
│   ├── registry.rs      # WatcherRegistry: persists folder configs
│   └── worker.rs        # background task: notify-debouncer-mini loop
├── engine.rs            # CortexEngine (add embedding_service field)
└── state.rs             # AppState (add AppHandle for event emission)
```

### Pattern 1: DocumentParser — Type-Dispatched Text Extraction

**What:** Single entry point that dispatches to the right parser based on file extension.
**When to use:** Called by `DocumentIndexer` for every file event.

```rust
// src-tauri/src/pipeline/parser.rs
use std::path::Path;

pub struct ParsedDocument {
    pub text: String,
    pub title: String,
    pub doc_type: String,
}

pub fn parse_document(path: &Path) -> Result<ParsedDocument, AppError> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let text = match ext.as_str() {
        "pdf" => {
            let bytes = std::fs::read(path)?;
            pdf_extract::extract_text_from_mem(&bytes)
                .map_err(|e| AppError::Parse(e.to_string()))?
        }
        "docx" | "doc" => {
            use docx_rust::DocxFile;
            let docx = DocxFile::from_file(path)
                .map_err(|e| AppError::Parse(e.to_string()))?;
            let doc = docx.parse()
                .map_err(|e| AppError::Parse(e.to_string()))?;
            extract_docx_text(&doc)
        }
        "txt" | "md" | "csv" => {
            std::fs::read_to_string(path)?
        }
        "xlsx" | "xls" | "ods" => {
            parse_spreadsheet(path)?
        }
        _ => return Err(AppError::Parse(format!("Unsupported file type: {}", ext))),
    };

    Ok(ParsedDocument {
        text,
        title: path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string(),
        doc_type: ext,
    })
}

fn parse_spreadsheet(path: &Path) -> Result<String, AppError> {
    use calamine::{open_workbook_auto, Reader, DataType};
    let mut workbook = open_workbook_auto(path)
        .map_err(|e| AppError::Parse(e.to_string()))?;

    let mut text_parts = Vec::new();
    for sheet_name in workbook.sheet_names().to_owned() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            for row in range.rows() {
                let row_text: Vec<String> = row.iter()
                    .filter_map(|cell| match cell {
                        DataType::String(s) => Some(s.clone()),
                        DataType::Float(f) => Some(f.to_string()),
                        DataType::Int(i) => Some(i.to_string()),
                        _ => None,
                    })
                    .collect();
                if !row_text.is_empty() {
                    text_parts.push(row_text.join(" "));
                }
            }
        }
    }
    Ok(text_parts.join("\n"))
}
```

### Pattern 2: EmbeddingService — Lazy-Init fastembed Wrapper

**What:** Wraps `TextEmbedding` from fastembed, initialized once and shared across threads.
**Critical:** `TextEmbedding` is NOT `Send` by default in older fastembed versions. In v5 it implements `Send + Sync`. Wrap in `Arc<Mutex<...>>` for shared state.
**Text chunking:** For long documents, split into 512-token chunks (fastembed max). Embed each chunk, use first chunk's embedding as document embedding (or mean pool if desired).

```rust
// src-tauri/src/pipeline/embedder.rs
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::sync::{Arc, Mutex};

pub struct EmbeddingService {
    model: Arc<Mutex<TextEmbedding>>,
    pub dimensions: usize,
}

impl EmbeddingService {
    pub fn new_local() -> Result<Self, AppError> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
        ).map_err(|e| AppError::Internal(format!("Embedding model init failed: {}", e)))?;

        Ok(Self {
            model: Arc::new(Mutex::new(model)),
            dimensions: 384,
        })
    }

    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, AppError> {
        // Chunk text to 512 tokens (approx 2000 chars) for MiniLM
        let chunk = truncate_to_chars(text, 2000);
        let model = self.model.lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let embeddings = model.embed(vec![chunk.as_str()], None)
            .map_err(|e| AppError::Internal(format!("Embedding failed: {}", e)))?;
        embeddings.into_iter().next()
            .ok_or_else(|| AppError::Internal("Empty embedding result".to_string()))
    }
}

fn truncate_to_chars(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        text.chars().take(max_chars).collect()
    }
}
```

### Pattern 3: Content Hasher — SHA-256 of File Bytes

**What:** Reads file in 4KB chunks, produces hex SHA-256 digest for change detection.
**When to use:** Before embedding. If hash matches stored hash → skip. If different → delete old vector, insert new.

```rust
// src-tauri/src/pipeline/hasher.rs
use sha2::{Sha256, Digest};
use std::io::Read;
use std::path::Path;

pub fn content_hash(path: &Path) -> Result<String, AppError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 { break; }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
```

### Pattern 4: Entity Extractor — Regex-Based

**What:** Scans document text for structured entity patterns.
**When to use:** After parsing, before storing. Results go into `extracted_entities` in document metadata.

```rust
// src-tauri/src/pipeline/entities.rs
use regex::Regex;
use crate::types::ExtractedEntity;

pub struct EntityExtractor {
    date_re: Regex,
    amount_re: Regex,
    email_re: Regex,
}

impl EntityExtractor {
    pub fn new() -> Self {
        Self {
            // ISO dates, US dates, written dates
            date_re: Regex::new(
                r"\b(\d{4}-\d{2}-\d{2}|\d{1,2}/\d{1,2}/\d{2,4}|(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[a-z]*\.?\s+\d{1,2},?\s+\d{4})\b"
            ).unwrap(),
            // Dollar amounts: $1,234.56 or $1234 or USD 1,234
            amount_re: Regex::new(
                r"\b(?:USD\s*)?[$£€]\s*[\d,]+(?:\.\d{2})?\b|\b[\d,]+(?:\.\d{2})?\s*(?:USD|EUR|GBP)\b"
            ).unwrap(),
            email_re: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(),
        }
    }

    pub fn extract(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();

        for m in self.date_re.find_iter(text) {
            entities.push(ExtractedEntity {
                label: "Date".to_string(),
                value: m.as_str().to_string(),
                entity_type: "date".to_string(),
            });
        }

        for m in self.amount_re.find_iter(text) {
            entities.push(ExtractedEntity {
                label: "Amount".to_string(),
                value: m.as_str().to_string(),
                entity_type: "amount".to_string(),
            });
        }

        // People and organizations: heuristic — capitalized 2-word sequences
        // (Named entity recognition without ML model)
        let person_re = Regex::new(r"\b([A-Z][a-z]+\s+[A-Z][a-z]+)\b").unwrap();
        for m in person_re.find_iter(text) {
            entities.push(ExtractedEntity {
                label: "Person/Org".to_string(),
                value: m.as_str().to_string(),
                entity_type: "person".to_string(),
            });
        }

        // Deduplicate by value
        entities.sort_by(|a, b| a.value.cmp(&b.value));
        entities.dedup_by(|a, b| a.value == b.value);
        entities.truncate(20); // Cap at 20 entities per document

        entities
    }
}
```

### Pattern 5: DocumentIndexer — Full Pipeline Orchestration

**What:** Orchestrates parse → hash-check → embed → extract-entities → upsert into RuVector.
**When to use:** Called by the file watcher worker for each new/modified file.

```rust
// src-tauri/src/pipeline/indexer.rs
use crate::engine::CortexEngine;
use crate::pipeline::{parser, hasher, entities::EntityExtractor};
use ruvector_core::types::{VectorEntry, SearchQuery};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

pub struct DocumentIndexer {
    extractor: EntityExtractor,
}

impl DocumentIndexer {
    pub fn new() -> Self {
        Self { extractor: EntityExtractor::new() }
    }

    pub fn index_file(
        &self,
        path: &Path,
        engine: &CortexEngine,
        embedding_service: &EmbeddingService,
    ) -> Result<String, AppError> {
        // 1. Compute content hash
        let hash = hasher::content_hash(path)?;

        // 2. Check if this path was already indexed with the same hash
        let collection = engine.collections
            .get_collection("documents_384")
            .ok_or_else(|| AppError::VectorStorage("Collection missing".to_string()))?;
        let coll = collection.read();

        // Try to find existing document by path (stored in metadata)
        // If hash matches → skip; if different → delete old and re-index
        let path_str = path.to_string_lossy().to_string();
        if let Some(existing_id) = self.find_by_path(&coll.db, &path_str)? {
            if let Ok(Some(entry)) = coll.db.get(&existing_id) {
                let stored_hash = entry.metadata
                    .as_ref()
                    .and_then(|m| m.get("content_hash"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if stored_hash == hash {
                    return Ok(existing_id); // Unchanged — skip
                }
            }
            // Changed — delete old vector
            coll.db.delete(&existing_id)?;
        }

        // 3. Parse document text
        let parsed = parser::parse_document(path)?;
        if parsed.text.trim().is_empty() {
            return Err(AppError::Parse("Document produced no text".to_string()));
        }

        // 4. Generate embedding
        let vector = embedding_service.embed_text(&parsed.text)?;

        // 5. Extract entities
        let entities = self.extractor.extract(&parsed.text);
        let entities_json = serde_json::to_value(&entities)
            .unwrap_or(serde_json::Value::Null);

        // 6. Build metadata payload
        let file_meta = std::fs::metadata(path)?;
        let doc_id = Uuid::new_v4().to_string();

        let mut metadata = HashMap::new();
        metadata.insert("path".to_string(), serde_json::Value::String(path_str));
        metadata.insert("doc_type".to_string(), serde_json::Value::String(parsed.doc_type));
        metadata.insert("content_hash".to_string(), serde_json::Value::String(hash));
        metadata.insert("extracted_entities".to_string(), entities_json);
        metadata.insert(
            "size".to_string(),
            serde_json::Value::Number(file_meta.len().into()),
        );
        metadata.insert(
            "title".to_string(),
            serde_json::Value::String(parsed.title),
        );

        // 7. Insert into RuVector
        let entry = VectorEntry {
            id: Some(doc_id.clone()),
            vector,
            metadata: Some(metadata),
        };
        drop(coll); // Release read lock before write
        let coll = collection.write();
        coll.db.insert(entry)?;

        Ok(doc_id)
    }

    fn find_by_path(&self, db: &VectorDB, path: &str) -> Result<Option<String>, AppError> {
        // Search all vectors for matching path in metadata
        // NOTE: This is a linear scan — acceptable for Phase 2, optimize with
        // a path→id index (HashMap in memory) in Phase 3+
        for id in db.keys()? {
            if let Ok(Some(entry)) = db.get(&id) {
                if let Some(metadata) = &entry.metadata {
                    if metadata.get("path").and_then(|v| v.as_str()) == Some(path) {
                        return Ok(Some(id));
                    }
                }
            }
        }
        Ok(None)
    }
}
```

### Pattern 6: WatcherRegistry — Persistent Folder Config

**What:** Stores watched folder configurations (path, enabled file types, exclusion patterns) as JSON. Survives app restarts.
**Storage:** `{app_data_dir}/watcher-registry.json` — simple JSON, loaded at startup.

```rust
// src-tauri/src/watcher/registry.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedFolderConfig {
    pub id: String,
    pub path: String,
    pub enabled_types: Vec<String>, // ["pdf", "docx", "txt", ...]
    pub excluded_patterns: Vec<String>, // ["node_modules", ".git", ".*"]
    pub is_paused: bool,
    pub document_count: u32,
    pub last_scan: Option<String>, // ISO 8601
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WatcherRegistry {
    pub folders: HashMap<String, WatchedFolderConfig>,
}

impl WatcherRegistry {
    pub fn load(registry_path: &PathBuf) -> Self {
        std::fs::read_to_string(registry_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, registry_path: &PathBuf) -> Result<(), AppError> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(registry_path, json)?;
        Ok(())
    }

    pub fn is_excluded(&self, folder_id: &str, path: &std::path::Path) -> bool {
        let config = match self.folders.get(folder_id) {
            Some(c) => c,
            None => return false,
        };

        let path_str = path.to_string_lossy();

        // Hidden files (starts with .)
        if path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
        {
            return true;
        }

        // Exclusion patterns (substring match in path)
        config.excluded_patterns.iter().any(|pat| path_str.contains(pat))
    }

    pub fn is_type_enabled(&self, folder_id: &str, ext: &str) -> bool {
        self.folders.get(folder_id)
            .map(|c| c.is_paused || c.enabled_types.is_empty() || c.enabled_types.contains(&ext.to_lowercase()))
            .unwrap_or(false)
    }
}
```

### Pattern 7: Watcher Worker — Background Tauri Task

**What:** Spawned once at app startup via `tauri::async_runtime::spawn`. Owns the debouncer, routes file events to the indexer, emits progress to frontend.
**AppHandle cloning:** The `AppHandle` must be cloned before moving into the background task. It implements `Clone` and `Emitter`.

```rust
// src-tauri/src/watcher/worker.rs
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

#[derive(Clone, serde::Serialize)]
pub struct IndexProgress {
    pub file_path: String,
    pub status: String, // "indexing", "indexed", "skipped", "error"
    pub doc_id: Option<String>,
    pub error: Option<String>,
}

pub fn spawn_watcher_task(
    app_handle: AppHandle,
    engine: Arc<Mutex<CortexEngine>>,
    registry_path: PathBuf,
    mut cmd_rx: mpsc::Receiver<WatcherCommand>,
) {
    tauri::async_runtime::spawn(async move {
        let (tx, mut rx) = mpsc::channel::<DebounceEventResult>(256);

        let debouncer_tx = tx.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(300),
            move |res: DebounceEventResult| {
                let _ = debouncer_tx.blocking_send(res);
            },
        ).expect("Failed to create file watcher");

        // Watch all registered folders
        let registry = WatcherRegistry::load(&registry_path);
        for config in registry.folders.values() {
            if !config.is_paused {
                let _ = debouncer.watcher()
                    .watch(std::path::Path::new(&config.path), RecursiveMode::Recursive);
            }
        }

        let indexer = DocumentIndexer::new();
        let embedding_service = EmbeddingService::new_local()
            .expect("Embedding service init failed");

        loop {
            tokio::select! {
                // Handle file system events
                Some(result) = rx.recv() => {
                    match result {
                        Ok(events) => {
                            for event in events {
                                let path = &event.path;
                                // Filter by exclusions and enabled types
                                let ext = path.extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("");

                                // Emit indexing start
                                let _ = app_handle.emit("index-progress", IndexProgress {
                                    file_path: path.to_string_lossy().to_string(),
                                    status: "indexing".to_string(),
                                    doc_id: None,
                                    error: None,
                                });

                                // Run indexing (blocking — use spawn_blocking)
                                let engine_clone = engine.clone();
                                let path_clone = path.clone();
                                let handle_clone = app_handle.clone();
                                let indexer_ref = &indexer;
                                let embedding_ref = &embedding_service;

                                // In practice: acquire engine lock, call indexer
                                let result = tokio::task::spawn_blocking({
                                    let engine = engine_clone;
                                    let path = path_clone;
                                    move || -> Result<String, AppError> {
                                        let engine = engine.blocking_lock();
                                        // indexer.index_file(&path, &engine, &embedding_service)
                                        Ok("doc-id".to_string()) // stub
                                    }
                                }).await;

                                let progress = match result {
                                    Ok(Ok(doc_id)) => IndexProgress {
                                        file_path: path.to_string_lossy().to_string(),
                                        status: "indexed".to_string(),
                                        doc_id: Some(doc_id),
                                        error: None,
                                    },
                                    Ok(Err(e)) | Err(e) => IndexProgress {
                                        file_path: path.to_string_lossy().to_string(),
                                        status: "error".to_string(),
                                        doc_id: None,
                                        error: Some(e.to_string()),
                                    },
                                };
                                let _ = app_handle.emit("index-progress", progress);
                            }
                        }
                        Err(errors) => {
                            for e in errors {
                                tracing::warn!("Watch error: {:?}", e);
                            }
                        }
                    }
                }

                // Handle control commands
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        WatcherCommand::Shutdown => break,
                        WatcherCommand::Pause => { /* stop watching */ }
                        WatcherCommand::Resume => { /* restart watching */ }
                    }
                }
            }
        }
    });
}
```

### Pattern 8: AppState Extended with AppHandle

**What:** `AppState` needs `AppHandle` for background tasks to emit events, and the `EmbeddingService` should live in `AppState` (not recreated per-request).

```rust
// UPDATED src-tauri/src/state.rs
pub struct AppState {
    pub engine: Arc<Mutex<CortexEngine>>,
    pub watcher_tx: mpsc::Sender<WatcherCommand>,
    pub index_rx: Arc<Mutex<mpsc::Receiver<IndexEvent>>>,
    // NEW in Phase 2:
    pub embedding_service: Arc<EmbeddingService>,  // shared, thread-safe
    pub registry_path: PathBuf,                     // path to watcher-registry.json
}
```

**In lib.rs setup hook:**
```rust
.setup(|app| {
    let data_dir = app.path().app_data_dir()?.join("vectors");
    let registry_path = app.path().app_data_dir()?.join("watcher-registry.json");
    let engine = CortexEngine::new_with_path(data_dir)?;
    let embedding_service = EmbeddingService::new_local()?;
    let (watcher_tx, watcher_rx) = mpsc::channel(32);
    let (_index_tx, index_rx) = mpsc::channel(32);

    let app_handle = app.handle().clone();
    let engine_arc = Arc::new(Mutex::new(engine));

    // Spawn the persistent watcher task
    spawn_watcher_task(app_handle, engine_arc.clone(), registry_path.clone(), watcher_rx);

    app.manage(AppState {
        engine: engine_arc,
        watcher_tx,
        index_rx: Arc::new(Mutex::new(index_rx)),
        embedding_service: Arc::new(embedding_service),
        registry_path,
    });
    Ok(())
})
```

### Anti-Patterns to Avoid

- **Initializing TextEmbedding per-request:** fastembed downloads the model (~90MB) on first init. Initialize ONCE at startup, share via `Arc<Mutex<TextEmbedding>>` or `Arc<EmbeddingService>`. If `TextEmbedding` is `Send + Sync` (v5), prefer `Arc` without Mutex.
- **Linear path-to-ID lookup at scale:** `find_by_path` iterates all vectors. This is acceptable for Phase 2 (< 10K docs) but must be replaced with an in-memory `HashMap<path, doc_id>` in Phase 3 for performance.
- **Holding engine Arc lock during embedding:** Embedding is CPU-intensive. Parse and embed text BEFORE acquiring the engine lock. Only hold the lock during the actual `db.insert()` call.
- **Blocking the Tokio runtime:** All file parsing (pdf-extract, docx-rust, calamine) uses sync APIs. MUST run inside `tokio::task::spawn_blocking`. Fastembed's `embed()` is also sync — same rule.
- **Embedding entire documents without chunking:** all-MiniLM-L6-v2 has a 256-token limit (fastembed's default max_length is typically 512 subword tokens ≈ 350-400 words). Truncate to ~2000 chars for the first-chunk embedding. Long documents lose tail content but the excerpt matches the embedded region.
- **Not releasing collection RwLock before re-acquiring as write:** `collection.read()` then `collection.write()` within the same scope causes deadlock. Drop the read guard before acquiring write.
- **Using `emit_all` (Tauri v1):** In Tauri 2, the method is `app_handle.emit("event-name", payload)` via the `tauri::Emitter` trait. The old `emit_all` from v1 does not exist in v2.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ONNX inference + model download | Custom tokenizer + inference loop | fastembed (wraps ort + tokenizers) | fastembed handles HuggingFace model download, tokenization, ONNX session, batching, normalization |
| PDF text extraction | PDF format parser | pdf-extract `extract_text_from_mem()` | PDF format is complex (streams, fonts, encoding tables) — months of work |
| DOCX parsing | ZIP + XML parser | docx-rust `DocxFile::from_file()` | OOXML format has 6000-page spec; parsing edge cases alone is weeks of work |
| File event debouncing | Manual timer-based event collapse | notify-debouncer-mini | Debouncing has subtle edge cases (moved files = delete+create, rapid edits) — the mini debouncer handles this correctly |
| SHA-256 hashing | Custom content fingerprint | sha2 `Sha256::new()` + `update()` | sha2 is the RustCrypto standard; battle-tested, zero dependencies |
| Content-addressed path lookup | SQL database | In-memory HashMap<path, doc_id> | A full SQLite would add complexity; HashMap reloaded from RuVector metadata keys at startup is sufficient for Phase 2 |

**Key insight:** fastembed is the right abstraction level. Using `ort` directly requires downloading and shipping the model separately, writing tokenization code, handling padding/truncation, and normalizing outputs. fastembed does all of this with a 2-line initialization.

---

## Common Pitfalls

### Pitfall 1: fastembed Model Download Blocks Startup

**What goes wrong:** `TextEmbedding::try_new()` downloads ~90MB model on first run, blocking startup for several seconds.
**Why it happens:** Model is cached in `~/.cache/fastembed/` after first download. Subsequent startups are fast. But first launch looks frozen.
**How to avoid:** Initialize `EmbeddingService` in a `spawn_blocking` call during setup, or spawn it in a background task and make the app functional with stubs until the model is ready. Show a "Model loading..." indicator to the user.
**Warning signs:** App hangs at startup for 30+ seconds on first launch.

### Pitfall 2: Collection RwLock Deadlock

**What goes wrong:** Code holds a read lock on a collection, then tries to acquire a write lock in the same scope — deadlock.
**Why it happens:** `parking_lot::RwLock` used by CollectionManager is not reentrant. `collection.read()` then `collection.write()` in the same thread blocks forever.
**How to avoid:** Explicitly drop the read guard (`drop(coll_read)`) before acquiring `collection.write()`. Or restructure code to do all reads first, then all writes.
**Warning signs:** App freezes permanently during indexing. `cargo test` hangs.

### Pitfall 3: Watcher Task Holds Debouncer Alive

**What goes wrong:** The `debouncer` variable is dropped when the closure returns, stopping file watching.
**Why it happens:** `notify-debouncer-mini` stops watching when the `Debouncer` is dropped. If it's not stored in the loop scope (or a struct), it dies immediately.
**How to avoid:** Keep `debouncer` alive for the lifetime of the watcher task. Store it as a local variable in the loop task — do NOT move it out of the task.

### Pitfall 4: spawn_blocking Cannot Return Non-Send Types

**What goes wrong:** `TextEmbedding` or parser types that are not `Send` cannot be moved into `spawn_blocking` closures.
**Why it happens:** `spawn_blocking` requires the closure to be `Send`. fastembed v5's `TextEmbedding` IS `Send + Sync`, so wrapping in `Arc` (without Mutex) is sufficient. Older versions may not be — use `Arc<Mutex<>>` defensively.
**How to avoid:** Wrap `TextEmbedding` in `Arc<EmbeddingService>` in `AppState`. Clone the `Arc` before `spawn_blocking`, not the `TextEmbedding` itself.

### Pitfall 5: File Events on Directory Creation Trigger Before Files Are Ready

**What goes wrong:** notify fires `Create` events for files while they are still being written (partial file). pdf-extract then fails parsing a truncated PDF.
**Why it happens:** Large file copies write data over multiple OS write calls. The first `Create` event arrives before the file is complete.
**How to avoid:** The 300ms debounce window helps but doesn't fully solve this. After receiving a `Create` event, add a secondary check: wait for file size to stabilize (compare two reads 100ms apart). For rename events, the file is always complete.
**Warning signs:** PDF parsing errors that disappear when manually re-triggering scan.

### Pitfall 6: Tauri emit() Requires Emitter Trait in Scope

**What goes wrong:** `app_handle.emit(...)` compilation error: "method not found in `AppHandle`".
**Why it happens:** Tauri 2 puts `emit()` behind the `tauri::Emitter` trait. The trait must be in scope with `use tauri::Emitter;`.
**How to avoid:** Add `use tauri::Emitter;` to any file calling `app_handle.emit()`.

### Pitfall 7: Path-Based Document Lookup Is O(n)

**What goes wrong:** Checking if a file was already indexed requires iterating all vectors via `db.keys()`. At 10K documents, each watcher event triggers a scan of 10K entries.
**Why it happens:** RuVector's `VectorDB` has no secondary index on metadata fields like `path`.
**How to avoid:** Maintain an in-memory `HashMap<String, String>` mapping `path → doc_id` in `AppState`. Populate it at startup from `db.keys()` → `db.get(id)` → extract path from metadata. Phase 2 builds this; Phase 3 may optimize it further.

### Pitfall 8: AppError Missing Variants for Parse Errors

**What goes wrong:** `pdf-extract` and `docx-rust` return their own error types. `From` conversions don't exist.
**Why it happens:** Phase 1 added `From<std::io::Error>` and `From<tokio::task::JoinError>`. Parser library errors need explicit mapping.
**How to avoid:** In each parser call, use `.map_err(|e| AppError::Parse(e.to_string()))`. Add `AppError::Embedding(String)` for fastembed errors. Phase 2 extends the `AppError` enum with new variants.

---

## Code Examples

### Adding fastembed to Cargo.toml

```toml
# src-tauri/Cargo.toml additions
fastembed = "5"
notify-debouncer-mini = "0.4"
pdf-extract = "0.10"
docx-rust = "0.2"
calamine = { version = "0.24", features = ["dates"] }
sha2 = "0.10"
regex = "1"
uuid = { version = "1", features = ["v4"] }
# For Tauri v2 event emission — already present:
# tauri = { version = "2", features = [] }
```

**Note:** `notify-debouncer-mini` is a companion crate to `notify`. It re-exports `notify` types so you don't need `notify` as a separate dependency.

### Tauri v2 Event Emission from Background Task

```rust
// CRITICAL: import Emitter trait — emit() is not on AppHandle by default
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
struct IndexProgress {
    file_path: String,
    status: String,
}

// In background task:
let app_handle: AppHandle = app.handle().clone(); // clone in setup
tauri::async_runtime::spawn(async move {
    app_handle.emit("index-progress", IndexProgress {
        file_path: "/path/to/file.pdf".to_string(),
        status: "indexed".to_string(),
    }).expect("emit failed");
});
```

### RuVector VectorDB Insert Pattern (Verified from Source)

```rust
// Pattern verified from ruvector-collections/src/collection.rs and ruvector-core/src/vector_db.rs
use ruvector_core::types::VectorEntry;
use std::collections::HashMap;

// Get collection (read or write lock)
let collection = engine.collections
    .get_collection("documents_384")
    .expect("Collection not found");

// For INSERT: need write access via collection.db directly
// Collection.db is pub in ruvector-collections/src/collection.rs
let coll = collection.write();
let entry = VectorEntry {
    id: Some("doc-uuid-here".to_string()),  // None = auto-generate
    vector: vec![0.1f32; 384],              // 384-dim from fastembed
    metadata: Some({
        let mut m = HashMap::new();
        m.insert("path".to_string(), serde_json::json!("/path/to/doc.pdf"));
        m.insert("doc_type".to_string(), serde_json::json!("pdf"));
        m.insert("content_hash".to_string(), serde_json::json!("abc123..."));
        m
    }),
};
let doc_id = coll.db.insert(entry)?;

// For DELETE (re-index case):
coll.db.delete("existing-doc-id")?;
```

### Frontend Event Listening (TypeScript)

```typescript
// src/hooks/useIndexingProgress.ts
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

interface IndexProgress {
  file_path: string;
  status: 'indexing' | 'indexed' | 'skipped' | 'error';
  doc_id?: string;
  error?: string;
}

export function useIndexingProgress() {
  const [progress, setProgress] = useState<IndexProgress | null>(null);
  const [isIndexing, setIsIndexing] = useState(false);

  useEffect(() => {
    const unlisten = listen<IndexProgress>('index-progress', (event) => {
      setProgress(event.payload);
      setIsIndexing(event.payload.status === 'indexing');
    });

    return () => { unlisten.then(fn => fn()); };
  }, []);

  return { progress, isIndexing };
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `candle-transformers` for local ONNX | `fastembed` wrapping `ort` ONNX Runtime | 2024 | fastembed handles model download, tokenization, batching — 10x faster to integrate |
| `notify` raw events | `notify-debouncer-mini` | notify v6 era | Debouncing separated into companion crate; raw notify still exists for performance-critical uses |
| `emit_all()` | `emit()` via `tauri::Emitter` trait | Tauri 2.0 | API renamed; trait must be in scope or compiler gives cryptic "method not found" error |
| `pdf-extract` 0.6.x | `pdf-extract` 0.10.0 | Late 2024 | `extract_text_from_mem(&bytes)` API — load bytes first, avoids reopening file |
| Manual UUID generation | `uuid` crate v4 feature | Already in ruvector-core workspace | uuid is already a workspace dep; use `Uuid::new_v4().to_string()` for doc IDs |

**Deprecated/outdated:**
- ruvector-core `CandleEmbedding`: **Always fails with unimplemented error**. Do not use. Use fastembed instead.
- ruvector-core `HashEmbedding`: Produces non-semantic embeddings (same chars = similar, not same meaning). ONLY for tests. Never use in production indexing.
- `emit_all()` from Tauri v1: Does not exist in Tauri v2. The replacement is `app_handle.emit()` with `use tauri::Emitter;`.

---

## Open Questions

1. **fastembed model caching location on macOS**
   - What we know: fastembed caches models in `~/.cache/fastembed/` by default.
   - What's unclear: Whether InitOptions allows specifying a custom cache directory (e.g., inside `app_data_dir()`). For a desktop app, `~/.cache` may conflict with macOS app sandboxing.
   - Recommendation: Check `InitOptions::with_cache_dir()` or similar at implementation time. Fall back to `app_data_dir()/models/` if configurable.

2. **Collection write access pattern — `collection.read()` vs `collection.write()`**
   - What we know: `CollectionManager::get_collection()` returns `Arc<RwLock<Collection>>`. `Collection.db` is `pub VectorDB`. `VectorDB::insert()` takes `&self` (not `&mut self`), which means insert works through a shared reference via internal locking.
   - What's unclear: Whether `collection.read()` is sufficient for insert (since `VectorDB.index` uses `Arc<RwLock<...>>` internally) or if we need `collection.write()`.
   - Recommendation: Try `collection.read()` first since VectorDB uses internal RwLock for its index. If tests show data races, switch to `collection.write()`.

3. **DPIP-05: OCR via tesseract — system library requirement**
   - What we know: The `tesseract` Rust crate wraps `libtesseract` via FFI. macOS requires `brew install tesseract`.
   - What's unclear: Whether this works in Tauri production builds without requiring users to install Tesseract separately. Bundling the shared library requires additional Tauri bundle config.
   - Recommendation: Mark DPIP-05 as a "Phase 2 stretch goal." Implement the text extraction dispatcher to handle images gracefully (return empty string or skip) until OCR is wired. This avoids blocking the rest of Phase 2 on a system dep.

4. **EmbeddingService thread safety in spawn_blocking**
   - What we know: fastembed v5 `TextEmbedding` implements `Send + Sync` per the docs page.
   - What's unclear: Whether the ONNX Runtime session inside fastembed is truly thread-safe for concurrent inference calls.
   - Recommendation: Wrap `EmbeddingService` in `Arc<Mutex<EmbeddingService>>` defensively. If performance testing shows the mutex is a bottleneck (embed is called per-file), evaluate whether fastembed supports parallel calls and remove the Mutex.

5. **WatchedFolder persistence — what happens when app_data_dir changes**
   - What we know: `app.path().app_data_dir()` returns the correct OS-specific path (`~/Library/Application Support/com.cortex.app/` on macOS).
   - What's unclear: If the Tauri app identifier changes (e.g., during development), the data dir path changes and existing registry is lost.
   - Recommendation: During development, use a fixed path like `~/.cortex/` for registry to avoid data loss on identifier changes. Switch to proper `app_data_dir()` before shipping.

---

## Sources

### Primary (HIGH confidence)

- `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-core/src/vector_db.rs` — `VectorDB::insert(&self, entry)`, `VectorDB::delete(id)`, `VectorDB::get(id)`, `VectorDB::keys()` verified from source
- `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-core/src/types.rs` — `VectorEntry { id: Option<String>, vector: Vec<f32>, metadata: Option<HashMap<String, serde_json::Value>> }` verified from source
- `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-collections/src/collection.rs` — `Collection.db: VectorDB` is `pub`; `CollectionManager::get_collection()` returns `Arc<RwLock<Collection>>`
- `/Users/gshah/work/apps/experiments/ruvector/crates/ruvector-core/src/embeddings.rs` — `CandleEmbedding` confirmed as stub that always returns error; `ApiEmbedding::openai()` confirmed working
- `/Users/gshah/work/apps/cortex/src-tauri/src/state.rs` — existing `WatcherCommand`, `IndexEvent`, `AppState` channels verified
- `/Users/gshah/work/apps/cortex/src-tauri/src/engine.rs` — `CortexEngine` with `collections: CollectionManager` and `filter_index: PayloadIndexManager` verified
- [Tauri v2 Calling Frontend docs](https://v2.tauri.app/develop/calling-frontend/) — `app_handle.emit()` via `tauri::Emitter` trait confirmed

### Secondary (MEDIUM confidence)

- [fastembed docs.rs](https://docs.rs/fastembed/latest/fastembed/) — `TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))`, `model.embed(texts, None)` — v5.11.0 confirmed
- [notify-debouncer-mini docs.rs](https://docs.rs/notify-debouncer-mini/latest/notify_debouncer_mini/) — `new_debouncer(Duration, handler)` + `watcher.watch(path, RecursiveMode::Recursive)` confirmed
- [pdf-extract docs.rs](https://docs.rs/pdf-extract/latest/pdf_extract/) — `extract_text(&bytes)` / `extract_text_from_mem(&bytes)` confirmed
- [sha2 crate pattern](https://docs.rs/sha2) — `Sha256::new()` + `update()` chunked file reading — standard pattern

### Tertiary (LOW confidence — validate at implementation time)

- DPIP-05 tesseract Rust bindings — system library requirement, bundling behavior in Tauri production builds not verified
- calamine `open_workbook_auto` with XLS (legacy format) — XLSX verified; XLS may have edge cases
- fastembed model cache directory configurability — `InitOptions::with_cache_dir()` existence not verified in docs
- Person/organization entity extraction via capitalized sequence heuristic — will produce false positives; acceptable for v1

---

## Metadata

**Confidence breakdown:**
- Standard stack (fastembed, notify-debouncer-mini, pdf-extract, sha2): HIGH — all APIs verified from official docs
- RuVector insert/delete API: HIGH — read directly from ruvector-core/ruvector-collections source code
- Architecture patterns (WatcherRegistry, DocumentIndexer): HIGH — derived from verified API shapes and existing AppState structure
- Entity extraction (regex patterns): MEDIUM — regex approach is correct; specific patterns will need tuning for real documents
- OCR (DPIP-05): LOW — system library dep not fully investigated; flagged as stretch goal
- fastembed thread safety: MEDIUM — documented as Send+Sync in v5 but ONNX session concurrency not confirmed

**Research date:** 2026-02-27
**Valid until:** 2026-03-27 (30 days — fastembed and notify-debouncer-mini are stable; RuVector source is local)

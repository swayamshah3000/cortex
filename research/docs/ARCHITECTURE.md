# Cortex System Architecture

## System Layers

### Layer 1: Frontend (React 19 + Tauri WebView)

The UI runs in Tauri's WebView. All 12 routes render from mock data during development and switch to Tauri IPC commands for production.

**Data flow**: Components → React Query hooks → Tauri invoke() → Rust backend → RuVector

```
src/hooks/useDocuments.ts    →  invoke('search_documents', { query, filters })
src/hooks/useSpaces.ts       →  invoke('get_spaces')
src/hooks/useSearch.ts       →  invoke('search_documents', { query })
src/hooks/useTauri.ts        →  invoke('get_stats'), invoke('trigger_scan'), etc.
```

During development, these hooks return mock data. In production, they call Tauri commands.

### Layer 2: Tauri 2 Bridge

Tauri provides:
- **IPC commands** — typed Rust functions callable from JS
- **File system access** — read/watch user directories
- **Native dialogs** — folder picker, save dialogs
- **System tray** — background indexing indicator
- **Auto-updater** — desktop app updates

### Layer 3: Rust Backend

Three subsystems that feed into RuVector:

#### 3a. File Watcher (notify-rs)
- Monitors watched folders for new/modified/deleted files
- Debounces rapid changes (300ms)
- Emits events to the indexing pipeline
- Runs as background Tokio task

#### 3b. Document Parser
- PDF: `pdf-extract` or `lopdf` for text extraction
- DOCX: `docx-rs` for Word document parsing
- Text/Markdown: direct read
- Images: optional OCR via `tesseract` bindings
- Spreadsheets: `calamine` for xlsx/csv
- Outputs: raw text content + file metadata

#### 3c. Embedding Engine
- **Local mode**: ONNX Runtime with `all-MiniLM-L6-v2` (384-dim embeddings)
- **API mode**: OpenAI `text-embedding-3-small` (1536-dim)
- Configurable in Settings → AI & Models
- Embeddings are cached in RuVector's vector store

### Layer 4: RuVector Engine

The core intelligence layer. RuVector provides:

#### Vector Storage (ruvector-core)
- HNSW index for fast approximate nearest neighbor search
- SIMD-accelerated distance computation
- Supports 384-dim (local) and 1536-dim (API) embeddings

#### GNN Clustering (ruvector-gnn)
- Graph Neural Network that learns document relationships
- Automatically discovers clusters → these become Smart Spaces
- Re-clusters periodically as new documents arrive
- Each cluster gets a centroid vector for space similarity

#### Graph Engine (ruvector-graph)
- Cypher query support for relationship queries
- Documents connected by: same-space, similar-content, shared-tags, shared-entities
- Powers "Related Documents" and "Space Network" visualizations
- Hyperedges connect 3+ documents sharing a common theme

#### Self-Learning (SONA Engine)
- Every search query generates a learning signal
- Click-through data tunes ranking over time
- Results improve automatically — no manual tuning needed
- Adaptation happens in <1ms per signal

#### Metadata Filtering (ruvector-filter)
- Pre-filter by type, date range, space, tags before vector search
- Hybrid queries: structured filters + semantic similarity
- Fast: filters applied before scanning vectors

## Data Model (Rust Side)

```rust
// Stored in RuVector
struct DocumentVector {
    id: String,
    embedding: Vec<f32>,          // 384 or 1536 dimensions
    metadata: DocumentMetadata,
}

struct DocumentMetadata {
    name: String,
    path: String,
    file_type: FileType,
    size: u64,
    created_at: DateTime<Utc>,
    modified_at: DateTime<Utc>,
    content_hash: String,         // For change detection
    space_ids: Vec<String>,
    tags: Vec<String>,
    extracted_entities: Vec<Entity>,
}

struct Space {
    id: String,
    name: String,
    icon: String,
    color: String,
    centroid: Vec<f32>,           // Cluster centroid embedding
    document_count: usize,
    sub_spaces: Vec<String>,
    auto_generated: bool,
}

// GNN graph connections
struct DocumentEdge {
    source: String,               // document id
    target: String,               // document id
    weight: f32,                  // similarity score
    edge_type: EdgeType,          // ContentSimilar, SameSpace, SharedTag, SharedEntity
}
```

## Indexing Pipeline

```
File Change Detected (notify-rs)
    │
    ▼
Parse Document → Extract Text + Metadata
    │
    ▼
Generate Embedding (ONNX local or API)
    │
    ▼
Store in RuVector Core (HNSW insert)
    │
    ▼
Extract Entities (dates, amounts, names, orgs)
    │
    ▼
Run GNN Clustering (periodic, not per-doc)
    │
    ├── Existing cluster match? → Assign to Space
    │
    └── New cluster? → Create Space → Name via LLM
    │
    ▼
Update Graph Edges (similarity connections)
    │
    ▼
Emit Frontend Event (new doc indexed, space updated)
```

## Search Pipeline

```
User Query ("property tax documents")
    │
    ▼
Generate Query Embedding
    │
    ▼
Apply Metadata Filters (ruvector-filter)
    │
    ▼
HNSW Nearest Neighbor Search (ruvector-core)
    │
    ▼
Re-rank with GNN attention (ruvector-attention)
    │
    ▼
Return Results + Feed Learning Signal (SONA)
    │
    ▼
Frontend Displays Results with Highlighted Excerpts
```

## Deployment

### Development
- Frontend: `bun dev` (Vite dev server with HMR)
- Backend: mock data via `src/lib/mock-data.ts`
- No Tauri needed for frontend development

### Production
- `cargo tauri build` produces platform-specific installers
- macOS: .dmg + .app bundle
- Windows: .msi + .exe
- Linux: .deb + .AppImage
- RuVector embedded as Rust dependency (no separate service)
- ONNX model bundled in app resources (~20MB)
- Total app size target: <50MB

## Performance Targets

| Operation | Target |
|-----------|--------|
| Search (10K docs) | <100ms |
| Index single document | <500ms |
| GNN re-cluster | <2s (background) |
| App cold start | <1s |
| File watcher latency | <300ms |
| Embedding generation (local) | <200ms per doc |

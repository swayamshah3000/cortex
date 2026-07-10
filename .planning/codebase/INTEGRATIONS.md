# External Integrations

**Analysis Date:** 2026-02-27

## APIs & External Services

**Currently Implemented:**
- None active in current codebase

**Planned (from CLAUDE.md & research/):**

**OpenAI API** - Embeddings and Document Naming
- Service: OpenAI text-embedding-3-small, GPT models for space naming
- SDK/Client: Would be `openai` package (not currently installed)
- Auth: Environment variable `OPENAI_API_KEY` (planned)
- Usage: Optional API mode for embeddings (1536-dim), LLM naming of spaces
- Alternative: Local ONNX Runtime for embeddings (no API required)

**Anthropic/Claude API** - Alternative LLM for Space Naming
- Service: Claude models for intelligent space naming
- SDK/Client: Would be `@anthropic-ai/sdk` (not currently installed)
- Auth: Environment variable `ANTHROPIC_API_KEY` (planned)
- Usage: Alternative to OpenAI for space naming via LLM
- Status: Listed in CLAUDE.md as possible integration

**Ollama** - Local LLM (preferred for privacy)
- Service: Local LLM endpoint for space naming
- Connection: HTTP to local Ollama service (default localhost:11434)
- Auth: None (local service)
- Usage: Privacy-first alternative, no cloud API calls
- Status: Referenced in CLAUDE.md as primary naming approach

## Data Storage

**Databases:**
- SQLite or RocksDB (planned, not yet integrated)
  - Purpose: Metadata storage, watched folder configuration
  - Status: To be integrated in backend

**Vector Storage:**
- RuVector (local crate dependency, planned)
  - Purpose: Vector storage, HNSW indexing, GNN clustering
  - Source: `/Users/gshah/work/apps/experiments/ruvector/` (sibling project)
  - Crates: ruvector-core, ruvector-gnn, ruvector-graph, ruvector-filter, ruvector-cluster, ruvector-domain-expansion, ruvector-attention, ruvector-hyperbolic-hnsw
  - Storage: Embedded in Tauri app, runs in-process
  - Status: In development, core architecture defined

**File Storage:**
- Local filesystem only (current)
  - Watched directories: ~/Documents, ~/Desktop, ~/Downloads (user-configurable)
  - File types: PDF, DOCX, TXT, PNG, JPG, XLSX, CSV, MD, other
  - Parser: Planned use of `pdf-extract`, `docx-rs`, `tesseract` (OCR), `calamine` (spreadsheets)

**Caching:**
- None currently configured
- Planned: RuVector's HNSW index provides implicit caching via nearest neighbor search

## Authentication & Identity

**Auth Provider:**
- None currently
- Custom authentication not planned for current phase
- Privacy-first: No user accounts, local data only
- Future: Possible Tauri secure store for API keys

## Monitoring & Observability

**Error Tracking:**
- None configured

**Logging:**
- Planned: Rust backend logging via `tracing` or `log` crate (not in current stack)
- Frontend: Console logging via React development tools
- Status: Not yet implemented

## CI/CD & Deployment

**Hosting:**
- Netlify (current via `netlify.toml`)
  - Build: Frontend SPA only
  - Functions: Express server via serverless
  - Static assets: `dist/spa/`
- Planned Desktop: Tauri desktop app distribution (macOS, Windows, Linux)

**CI Pipeline:**
- None detected in codebase
- GitHub Actions/GitLab CI: Not yet configured

## Environment Configuration

**Required env vars (planned):**
- `OPENAI_API_KEY` - Optional, for cloud embeddings
- `ANTHROPIC_API_KEY` - Optional, for Claude-based naming
- `PORT` - Server port (default: 3000)
- `PING_MESSAGE` - Dev test message (default: "ping")
- `OLLAMA_API_URL` - Local Ollama endpoint (optional, default: localhost:11434)

**Secrets location:**
- `.env` file (312 bytes, not in git)
- Planned: Tauri secure credential store for sensitive keys

**Configuration files:**
- Environment: `.env` (not tracked in git)
- Application settings: Planned storage in `~/.cortex/` or app preferences

## Webhooks & Callbacks

**Incoming:**
- None currently

**Outgoing:**
- None currently

## File Format Support

**Document Parsing (planned Rust):**

- PDF: `pdf-extract` or `lopdf` crates
  - Text extraction via library

- DOCX: `docx-rs` crate
  - Programmatic Word document parsing

- Text/Markdown: Standard file read
  - Direct text extraction

- Images: Optional `tesseract` OCR
  - Optical character recognition for scanned documents

- Spreadsheets: `calamine` crate
  - XLSX and CSV parsing
  - Cell data extraction

**Embedding Dimension Support:**

- Local ONNX: 384-dim embeddings (all-MiniLM-L6-v2 model)
  - Fast, on-device, privacy-preserving
  - Downloaded on first run

- OpenAI API: 1536-dim embeddings (text-embedding-3-small)
  - Higher quality, requires API key
  - ~$0.02 per 1M tokens

## Planned Backend Integrations (Not Yet Implemented)

**File Watching (notify-rs):**
- Monitors watched folders for file changes
- Debounces rapid changes (300ms)
- Status: Ready for implementation

**Document Parsing:**
- Pipeline: File watcher → Parser → Embeddings → RuVector
- Status: Parser crates selected, not yet integrated

**Tauri IPC Bridge:**
- Commands from React → Rust backend
- Planned commands: `index_document`, `search_documents`, `get_spaces`, `move_document_to_space`, `add_watched_folder`, `trigger_scan`, `get_stats`, `get_space_graph`
- Status: Architecture defined, code generation ready

---

*Integration audit: 2026-02-27*

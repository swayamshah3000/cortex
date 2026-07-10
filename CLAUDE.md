# Cortex — Self-Organizing Document Intelligence

## What Is This

Cortex is a **Tauri 2 desktop app** that auto-organizes your personal documents using semantic search, vector embeddings, and GNN-based clustering. Think "Finder/Explorer meets a brain" — you drop folders, and Cortex automatically creates Smart Spaces (virtual categories) by understanding document content, not just filenames.

**Core value prop**: "Find anything. Organize nothing." Documents sort themselves into spaces like Property, Kids, Work, Medical, Invoices — and the system keeps learning from your behavior.

### Key Capabilities

- **Watched Folders** — monitors ~/Documents, ~/Desktop, ~/Downloads for changes
- **Semantic Search** — natural language queries ("all my property tax documents from last year")
- **Smart Spaces** — AI-generated virtual folders based on document content similarity (GNN clustering)
- **Entity Extraction** — pulls dates, amounts, people, organizations from documents
- **Local-first** — all processing runs on-device, no cloud required (optional API embeddings)
- **Privacy-first** — document content never leaves the machine unless user opts into cloud embeddings

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│                 FRONTEND                     │
│  React 19 + TypeScript + TailwindCSS 4      │
│  Zustand (UI state) + React Query (data)    │
│  12 routes, 40+ components                  │
├─────────────────────────────────────────────┤
│               TAURI 2 BRIDGE                │
│  IPC commands for all backend operations    │
│  File system access, native dialogs         │
├─────────────────────────────────────────────┤
│                 BACKEND (Rust)              │
│  ┌─────────────┐  ┌──────────────────────┐ │
│  │ File Watcher │  │   RuVector Engine    │ │
│  │ (notify-rs)  │  │                      │ │
│  ├─────────────┤  │  - Vector storage     │ │
│  │ Doc Parser   │  │  - HNSW indexing     │ │
│  │ (PDF, DOCX,  │  │  - GNN clustering    │ │
│  │  text, OCR)  │  │  - Graph queries     │ │
│  ├─────────────┤  │  - Semantic search    │ │
│  │ Embeddings   │  │  - Self-learning     │ │
│  │ (local ONNX  │  │  - Domain expansion  │ │
│  │  or API)     │  │                      │ │
│  └─────────────┘  └──────────────────────┘ │
└─────────────────────────────────────────────┘
```

## How RuVector Powers Cortex

[RuVector](https://github.com/ruvnet/ruvector) (`/Users/gshah/work/apps/experiments/ruvector/`) is the self-learning vector database that powers ALL intelligence in Cortex. It's not just storage — it learns from every query.

### RuVector Crates Used by Cortex

| Crate | Purpose in Cortex |
|-------|-------------------|
| `ruvector-core` | Vector storage + HNSW indexing for document embeddings |
| `ruvector-gnn` | GNN-based clustering — auto-discovers Smart Spaces from document similarity |
| `ruvector-gnn-wasm` | WASM build for optional in-browser previews |
| `ruvector-graph` | Cypher graph queries — finds related documents, space relationships |
| `ruvector-cluster` | Clustering algorithms for grouping documents into spaces |
| `ruvector-filter` | Metadata filtering — search by type, date, space, tags |
| `ruvector-collections` | Multi-collection support — separate indices per space |
| `ruvector-domain-expansion` | Transfer learning — new spaces bootstrap from existing knowledge |
| `ruvector-attention` | 46 attention mechanisms for ranking search results |
| `ruvector-hyperbolic-hnsw` | Hierarchy-aware search — perfect for nested space/sub-space taxonomy |

### How the Intelligence Works

```
Document Added → Parse Content → Generate Embedding (ONNX/API)
                                          │
                                          ▼
                               RuVector Core (store vector)
                                          │
                                          ▼
                               RuVector GNN (cluster analysis)
                                          │
                              ┌────────────┴────────────┐
                              │                         │
                    Assign to existing          Create new Space
                    Space (high similarity)     (new cluster detected)
                              │                         │
                              ▼                         ▼
                    Update Space graph         Name space via LLM
                    connections                (local Ollama or API)
```

**Self-learning loop**: Every search query feeds back into the GNN, improving future results. RuVector's SONA engine auto-tunes routing and ranking based on user behavior.

### RuVector Integration Points (Tauri Commands)

These Tauri IPC commands will bridge frontend → RuVector:

```rust
// Document operations
#[tauri::command] fn index_document(path: &str) -> Result<DocumentMeta>
#[tauri::command] fn search_documents(query: &str, filters: SearchFilters) -> Result<Vec<SearchResult>>
#[tauri::command] fn get_document(id: &str) -> Result<Document>
#[tauri::command] fn get_related_documents(id: &str, limit: usize) -> Result<Vec<Document>>

// Space operations
#[tauri::command] fn get_spaces() -> Result<Vec<Space>>
#[tauri::command] fn get_space_documents(space_id: &str) -> Result<Vec<Document>>
#[tauri::command] fn move_document_to_space(doc_id: &str, space_id: &str) -> Result<()>
#[tauri::command] fn recluster_spaces() -> Result<Vec<Space>>

// Folder watching
#[tauri::command] fn add_watched_folder(path: &str) -> Result<WatchedFolder>
#[tauri::command] fn remove_watched_folder(id: &str) -> Result<()>
#[tauri::command] fn trigger_scan(folder_id: &str) -> Result<ScanProgress>

// Analytics
#[tauri::command] fn get_stats() -> Result<Stats>
#[tauri::command] fn get_space_graph() -> Result<SpaceGraph>
#[tauri::command] fn get_search_analytics() -> Result<SearchAnalytics>

// Settings
#[tauri::command] fn get_settings() -> Result<Settings>
#[tauri::command] fn update_settings(settings: Settings) -> Result<()>
```

## Tech Stack

### Frontend
| Layer | Technology |
|-------|-----------|
| Framework | React 19 |
| Language | TypeScript 5.x |
| Build | Vite |
| Styling | TailwindCSS 4 |
| Components | shadcn/ui (customized to Cortex theme) |
| State | Zustand (global UI) + React Query (server/data state) |
| Routing | React Router v7 |
| Charts | Recharts |
| Graph Viz | react-force-graph or Sigma.js |
| PDF Preview | react-pdf |
| Icons | Lucide React |
| Animations | Framer Motion |
| Forms | React Hook Form + Zod |

### Backend (Rust / Tauri 2)
| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri 2 |
| Vector DB | RuVector (local crate dependency) |
| GNN clustering | ruvector-gnn |
| Graph queries | ruvector-graph (Cypher engine) |
| Embeddings | ONNX Runtime (local) or OpenAI API |
| File watching | notify-rs |
| Doc parsing | pdf-extract, docx-rs, image + tesseract (OCR) |
| LLM (naming) | Ollama (local) or Claude/OpenAI API |

## Frontend Specification

The full 936-line frontend spec lives at `../FRONTEND_SPEC.md`. It defines:

### 12 Routes
1. `/onboarding` — 4-step first-time wizard (Welcome → Select Folders → Scanning → Spaces Ready)
2. `/` — Dashboard (greeting, stats with sparklines, recent docs, top spaces, activity feed)
3. `/spaces` — Smart Spaces grid (auto-organized virtual folders, grid/list toggle)
4. `/spaces/:id` — Space detail (sub-spaces, document list, related spaces)
5. `/search` — Semantic search (split-pane: results + preview panel, filters)
6. `/recent` — Timeline grouped by day (Today/Yesterday/This Week)
7. `/favorites` — Starred documents grid with sort
8. `/tags` — Tag cloud + list (auto-generated + user-created)
9. `/watched` — Watched folders management (pause/remove, file type toggles, exclusion patterns)
10. `/insights` — Analytics (donut chart, area chart, bar chart, space network graph, top searches)
11. `/settings` — 6 tabs: General, Indexing, AI & Models, Privacy, Storage, About
12. `/document/:id` — Split view: document preview (65%) + metadata sidebar (35%)

### Layout
- **Sidebar**: 240px expanded / 64px collapsed, nav links + spaces list + storage bar
- **TopBar**: 52px sticky, breadcrumb + search + indexing indicator + theme toggle
- **Responsive**: >1200px full, 900-1200px collapsed sidebar, <900px overlay sidebar
- **Command Palette**: Cmd+K overlay for search/navigation from anywhere

### Design System
- **Dark mode default** with light mode toggle
- **Colors**: Indigo/violet accent (#6D28D9 light, #8B5CF6 dark), deep dark backgrounds (#0F0F14, #1A1A24)
- **Typography**: Inter (body) + Plus Jakarta Sans (display) + JetBrains Mono (paths/code)
- **Spacing**: 4px base grid (4, 8, 12, 16, 20, 24, 32, 40, 48, 64, 80, 96)
- **Icons**: Lucide React, 1.5px stroke

### 40+ Components (see FRONTEND_SPEC.md for full inventory)
- Layout: AppShell, Sidebar, TopBar, CommandPalette
- Documents: DocumentCard, DocumentRow, DocumentPreview, DocumentMeta, FileTypeIcon
- Spaces: SpaceCard, SpaceGrid, SpaceBreadcrumb, SubSpaceRow
- Search: SearchBar, FilterChip, FilterBar, SearchResult
- Charts: DonutChart, AreaChart, BarChart, StatCard, NetworkGraph
- Forms: Toggle, Slider, Select, TextInput, FolderPicker, TagInput
- Common: EmptyState, ConfirmDialog, Toast

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+K` | Command palette / search |
| `Cmd+1/2/3` | Dashboard / Spaces / Search |
| `Cmd+,` | Settings |
| `Cmd+D` | Toggle dark mode |
| `Cmd+\` | Toggle sidebar |
| `/` | Focus search (when not in input) |
| `Esc` | Close modal / palette |

## Build & Dev

```bash
# Frontend dev
bun install
bun dev

# Tauri dev (once backend is set up)
cargo tauri dev

# Build for production
cargo tauri build
```

## Conventions

- Use **named exports** for components, **default exports** for route pages
- Use **Zustand** for UI state (sidebar, theme, command palette), **React Query** for server data
- All mock data lives in `src/lib/mock-data.ts` during frontend development
- Tauri IPC hooks go in `src/hooks/useTauri.ts`
- Use `cn()` helper (clsx + tailwind-merge) for conditional classnames
- Follow the design tokens from `src/styles/globals.css` — never hardcode colors
- For Tauri terminal apps: use DOM reparenting not React portals for layout persistence
- File type icons use Lucide icons mapped by extension (see FileTypeIcon component)
- Space icons are dynamically resolved from string names to Lucide components

## Data Types (Core)

```typescript
interface Document {
  id: string;
  name: string;
  path: string;
  type: 'pdf' | 'docx' | 'txt' | 'png' | 'jpg' | 'xlsx' | 'csv' | 'md' | 'other';
  size: number;
  createdAt: Date;
  modifiedAt: Date;
  excerpt?: string;
  spaceIds: string[];
  tags: string[];
  isFavorite: boolean;
  extractedEntities?: { label: string; value: string; type: 'date' | 'amount' | 'person' | 'organization' | 'location' }[];
  thumbnailColor?: string;
}

interface Space {
  id: string;
  name: string;
  icon: string;        // Lucide icon name (e.g., 'Home', 'Briefcase')
  color: string;       // Hex color for accent
  documentCount: number;
  lastUpdated: Date;
  subSpaces?: Space[];
  parentId?: string;
  sampleFiles?: string[];
}

interface Tag {
  id: string;
  name: string;
  color: string;
  documentCount: number;
  type: 'auto' | 'user';  // auto = AI-generated, user = manually created
}

interface WatchedFolder {
  id: string;
  path: string;
  documentCount: number;
  lastScan: Date;
  status: 'watching' | 'paused' | 'error';
}

interface Stats {
  totalDocuments: number;
  smartSpaces: number;
  lastScan: Date;
  indexSize: number;
}
```

## Project References

| Resource | Path |
|----------|------|
| Frontend Spec (full) | `../FRONTEND_SPEC.md` |
| Design System Master | `../design-system/cortex/MASTER.md` |
| RuVector Source | `/Users/gshah/work/apps/experiments/ruvector/` |
| RuVector Docs | `/Users/gshah/work/apps/experiments/ruvector/docs/` |
| Previous Frontend (reference) | `../cortex-app/` (React 19 prototype, mock data only) |

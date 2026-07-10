# Pitfalls Research

**Domain:** Tauri 2 desktop app with Rust backend, document intelligence, vector search (RuVector/HNSW), ONNX embeddings
**Researched:** 2026-02-27
**Confidence:** MEDIUM-HIGH (IPC/async/serialization pitfalls: HIGH; RuVector-specific: MEDIUM — RuVector is custom, community data thin)

---

## Critical Pitfalls

### Pitfall 1: Blocking the Tokio Async Runtime with CPU-Intensive Work

**What goes wrong:**
CPU-bound operations (embedding generation, GNN re-clustering, PDF parsing, OCR) are placed in `async` Tauri commands or `.await` chains. This starves the Tokio executor, freezing the UI and causing all other commands to queue. The window becomes unresponsive. This is distinct from I/O blocking — Rust async is cooperative, not preemptive, so a long-running sync computation inside async context blocks the thread entirely.

**Why it happens:**
Developers from JS backgrounds assume async = non-blocking for all work. In Rust, `async fn` is non-blocking only for I/O (`await` yields at `.await` points). Pure CPU work — matrix multiplications for ONNX inference, graph traversals for GNN — never yields, so it monopolizes the executor thread.

**How to avoid:**
- Wrap all CPU-intensive work in `tokio::task::spawn_blocking()` or use a dedicated thread pool (rayon for parallelism)
- ONNX inference, PDF parsing, GNN clustering must never run directly in `async` commands
- File watcher event processing, embedding generation, and clustering should all go through `spawn_blocking` or dedicated background threads
- Use `tauri::async_runtime::spawn()` only for I/O-bound background tasks (network calls to Ollama API, file reads)

**Warning signs:**
- UI freezes or stops responding for 1–5 seconds during indexing
- `cargo tauri dev` console shows no concurrent command execution
- Adding more documents makes the app progressively more unresponsive
- `tauri::command` handlers that call ONNX session `run()` without `spawn_blocking`

**Phase to address:** Tauri shell integration phase (Phase 1 of backend work) — establish the async/blocking boundary before writing any document processing code.

---

### Pitfall 2: IPC Serialization Failures with Complex Rust Types

**What goes wrong:**
Tauri IPC serializes all command arguments and return values as JSON. Types that don't implement `serde::Serialize` / `serde::Deserialize` cause silent runtime failures or hanging Promises on the frontend. Common culprits: standard library error types (`std::io::Error`, `anyhow::Error`), types from external crates, `HashMap` with non-string keys, custom enums without `#[derive(Serialize, Deserialize)]`. The frontend Promise never resolves — not even to an error state.

**Why it happens:**
The Tauri `#[tauri::command]` macro enforces serialization at compile time only for basic type errors. Complex failures (e.g., a nested type missing `Serialize`) may compile but fail at runtime. The documented issue #10327 confirms that returning complex values can cause Promises to hang indefinitely.

**How to avoid:**
- Define a project-wide `AppError` enum that implements `serde::Serialize` — never return raw `std::io::Error` or `anyhow::Error` from commands
- All IPC return types use `Result<T, AppError>` where both `T` and `AppError` are JSON-serializable
- Derive `#[derive(Debug, Serialize, Deserialize, Clone)]` on every struct crossing the IPC boundary
- Add integration tests for each Tauri command that exercise serialization round-trips
- Consider `taurpc` crate for end-to-end type safety with TypeScript generation

**Warning signs:**
- Frontend `invoke()` calls that never resolve (no `.then()`, no `.catch()`)
- Console shows no Rust error but JS Promise hangs
- Working with `Vec<HashMap<String, SomeType>>` as return types
- Any `map_err(|e| e.to_string())` workarounds appearing in command handlers

**Phase to address:** Tauri IPC setup phase — establish the `AppError` type and serialization patterns before building any commands.

---

### Pitfall 3: Embedding Dimension Mismatch After Model Switching or Index Migration

**What goes wrong:**
The HNSW index is built with 384-dimensional vectors (all-MiniLM-L6-v2). The user later enables OpenAI embeddings (1536-dim) in Settings → AI & Models. New documents get 1536-dim embeddings stored in the same index that expects 384-dim. Search either crashes, returns garbage results, or silently falls back to brute-force scan. Existing 384-dim documents become incompatible with new 1536-dim queries. Migrating requires re-indexing every document — which may take hours for large collections.

**Why it happens:**
HNSW indexes are dimension-fixed at creation time. Developers hardcode the dimension during initial setup, don't account for the settings toggle being a live configuration change, and don't build migration infrastructure.

**How to avoid:**
- Store the embedding model name and dimension as metadata in the vector index and in app settings
- When the embedding model changes, require full re-index — surface this as a modal warning: "Switching models requires re-indexing all N documents. This will take approximately X minutes."
- Maintain separate HNSW collections per embedding dimension, named by model identifier (e.g., `docs_minilm_384`, `docs_openai_1536`)
- Never mix dimensions in a single collection
- Version the index schema: persist `{ model: "all-MiniLM-L6-v2", dim: 384, index_version: 1 }` in a settings file alongside the index

**Warning signs:**
- Settings page has an AI model toggle but no re-index UI flow
- `DocumentVector.embedding: Vec<f32>` is stored without a `model_id` field
- Integration tests only test one embedding provider
- No migration path documented in architecture

**Phase to address:** Embedding engine phase — design the multi-model schema before writing index creation code.

---

### Pitfall 4: notify-rs Missing Events at High File Volume

**What goes wrong:**
When watching directories with hundreds or thousands of files (typical ~/Documents), `notify-rs` drops events at scale. The documented GitHub issue #412 confirms that watching 1,500 files caused ~16% of modification events to be silently missed. The Linux inotify backend has per-process limits on watched file descriptors. On macOS, FSEvents coalesces rapid events. Documents modified while the watcher is overloaded are never indexed.

**Why it happens:**
Developers test with small folders (10–50 files). The production case — watching ~/Documents, ~/Downloads, ~/Desktop simultaneously with thousands of nested files — hits OS-level limits that weren't exercised in development.

**How to avoid:**
- Watch directories recursively, not individual files — reduces the descriptor count dramatically
- Implement a polling fallback: periodic full-directory scan every N minutes as a safety net for missed events
- Debounce events (300ms minimum) to reduce event storm volume during bulk file operations
- Track `content_hash` for each indexed document; periodic scans compare hashes to detect silently-missed changes
- On Linux, increase `inotify.max_user_watches` via sysctl documentation in the onboarding flow
- Use `notify::RecommendedWatcher` (event-driven) combined with scheduled polling scans

**Warning signs:**
- File watcher tests only run against directories with <50 files
- No periodic re-scan background task in the architecture
- No content hash comparison for change detection
- App only tested on dev machine's small `~/Documents`

**Phase to address:** File watcher implementation phase — build polling fallback from day one, not as a retrofit.

---

### Pitfall 5: GNN Re-Clustering Triggered Per Document (Pipeline Design Error)

**What goes wrong:**
The indexing pipeline runs GNN clustering synchronously after every document is indexed. With a 10,000-document corpus, each new file triggers a full graph re-traversal. Re-clustering takes 1–2 seconds (per the architecture's performance targets). With batch imports (50 documents dropped at once), this serializes to 50–100 seconds of blocking GNN work, stalls the indexing pipeline, floods the frontend with "space updated" events, and may cause Smart Spaces to thrash (create and destroy spaces) as interim cluster states propagate to the UI.

**Why it happens:**
The architecture diagram shows GNN clustering as a step in the per-document pipeline. This is correct for conceptual understanding but wrong for implementation — it should be a separate, rate-limited background job.

**How to avoid:**
- GNN clustering must be decoupled from the per-document indexing pipeline — run on a schedule (e.g., every 30 seconds if new documents arrived, or after a quiet period of 10 seconds with no new files)
- The per-document pipeline only: parse → embed → store vector → extract entities → update graph edges
- GNN re-clustering runs as a separate background Tokio task triggered by a debounced timer
- Emit a single "spaces updated" frontend event after a batch clustering run, not one per document
- Implement a `ClusteringState` enum: `Idle`, `Pending(document_count)`, `Running`, `Complete` — expose via a Tauri event

**Warning signs:**
- Architecture pipeline shows GNN clustering inside the per-document loop
- No debounce/cooldown mechanism for clustering triggers
- Frontend receives one space-updated event per indexed document
- Integration tests only index one document at a time

**Phase to address:** Indexing pipeline design phase — establish the per-document vs. background-job separation before writing any pipeline code.

---

### Pitfall 6: Tauri Event Memory Leak from Uncleared Frontend Subscriptions

**What goes wrong:**
The Rust backend emits events to the frontend (`emit("indexing_progress", ...)`) as documents are indexed. React components subscribe to these events via `listen()` from `@tauri-apps/api`. If the `unlisten` function is not called in `useEffect` cleanup, subscriptions accumulate on every component mount/unmount cycle. The documented Tauri issue #12724 shows memory usage reaching 1.1GB after sustained event emission. Hot module reloading in development compounds this — each HMR refresh adds a new listener layer without cleaning up the previous one.

**Why it happens:**
Developers familiar with `addEventListener` / `removeEventListener` patterns don't realize that Tauri's `listen()` is async and returns a cleanup function via a Promise, requiring careful handling in React's `useEffect`.

**How to avoid:**
- Wrap all `listen()` calls in `useEffect` with a cleanup function that calls the returned `unlisten()`
- Pattern: `const unlisten = await listen('event', handler); return () => unlisten();`
- Since `listen()` is async, use a flag or ref to prevent calling `unlisten` on an already-cleaned-up effect
- Test memory usage explicitly during sustained indexing of 1,000+ documents
- Limit the frequency of progress events — debounce backend emissions to max 10 events/second, not one per document

**Warning signs:**
- `listen()` calls without corresponding cleanup in `useEffect`
- Memory usage grows continuously during indexing sessions
- Browser devtools show accumulating event listeners on the window
- Components that subscribe to Tauri events unmount/remount frequently (e.g., inside virtualized lists)

**Phase to address:** Tauri frontend integration phase — establish the `useTauriEvent` hook pattern with built-in cleanup before building any event-driven UI.

---

### Pitfall 7: React Query Cache Not Invalidated After Backend Mutations

**What goes wrong:**
A document is indexed by the Rust backend. The backend emits a Tauri event. The React frontend receives the event but doesn't invalidate the React Query cache for affected queries (`['documents']`, `['spaces']`, `['stats']`). The UI shows stale data — the dashboard still shows the old document count, the space still shows the old file list — until the user navigates away and back, or until `staleTime` expires. The inverse also occurs: React Query refetches on window focus by default, causing unexpected flashes of loading state.

**Why it happens:**
React Query's cache invalidation is explicit-by-design. Developers wire up the Tauri event listener to update local state but don't connect it to `queryClient.invalidateQueries()`. Desktop apps also behave differently than web apps — "window focus" is less meaningful for a Tauri app that's always in the foreground.

**How to avoid:**
- Create a centralized `useTauriEventInvalidation` hook that listens for backend events (`document_indexed`, `spaces_updated`, `scan_complete`) and calls `queryClient.invalidateQueries()` with the correct keys
- Register this hook once in `AppShell` — not per-component
- Disable `refetchOnWindowFocus` for Tauri apps (irrelevant behavior, causes visual noise): `queryClient = new QueryClient({ defaultOptions: { queries: { refetchOnWindowFocus: false } } })`
- Define query key constants (`QUERY_KEYS.documents`, `QUERY_KEYS.spaces`) to prevent key typos causing missed invalidations

**Warning signs:**
- Dashboard stats don't update after indexing without manual refresh
- `useQueryClient()` not imported in any event listener hook
- React Query configured with web-app defaults (refetchOnWindowFocus: true)
- No test for "index document → verify UI updates"

**Phase to address:** Tauri frontend integration phase — define query key constants and the invalidation hook before building any data-displaying page.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Return `String` as error type from all Tauri commands | Fastest path through Rust error handling | Cannot distinguish error types on frontend; no structured error UI | Prototype only — replace with `AppError` enum before any real user testing |
| Run GNN clustering synchronously per document | Simplest pipeline implementation | Causes UI freezes and space thrashing at scale >20 docs | Never — wrong architecture from day one |
| Single HNSW collection for all embedding models | Simpler initial setup | Breaks entirely when user switches embedding model | Never — multi-model support must be designed upfront |
| Use React `useState` for Tauri event data (no React Query) | Faster initial wiring | State lost on component remount; no cache; no deduplication | Never for data that persists — only for ephemeral progress indicators |
| Poll Tauri commands instead of using events for progress | Avoids event cleanup complexity | Hammers the IPC layer; burns CPU; adds latency | Never for anything with >1 update/second |
| Skip `spawn_blocking` for ONNX inference (call directly in async) | Less boilerplate | UI freezes on every embedding request | Never |
| Bundle ONNX model in source repo | Avoids download UX | Binary size blows past 50MB target; repository becomes large | Never — use Tauri resource download or ship via installer |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| ONNX Runtime (`ort` crate) | Loading the session on every inference call — `Session::builder()...build()` is expensive | Load session once at app startup, store in Tauri `State<OnnxSession>`, reuse across commands |
| Ollama (space naming) | Assuming Ollama is always running locally — hard-coded `http://localhost:11434` | Check Ollama availability at startup; degrade gracefully with a generic space name if unavailable; set 30-second timeout for 7B models, 180 seconds for larger ones |
| Ollama on Windows via Tauri | Requests come from `https://tauri.localhost`, blocked by Ollama's CORS policy | Set `OLLAMA_ORIGINS=*` or document this in Windows setup instructions; check `tauri.localhost` vs `tauri://` per platform |
| `notify-rs` on Linux | inotify `max_user_watches` limit (~8192 by default) causes watcher to silently fail on large directories | Document the sysctl fix; surface an OS limit warning in the UI when watch fails to initialize |
| pdf-extract crate | Panics on malformed or encrypted PDFs — `unwrap()` on parse result | Wrap every parse call in `std::panic::catch_unwind` or use oxidize-pdf which has 98.8% success rate on malformed docs; always return `Result<String, ParseError>` |
| HNSW index persistence | `hnsw_rs` serializes graph structure but not vectors separately — must persist both | Persist vectors and index structure atomically; use a WAL-style write to avoid partial state on crash |
| Tauri `fs` plugin | Using `allow-read-*` globally for all paths — overly permissive | Scope fs permissions to the specific watched folder paths the user has explicitly added; never grant filesystem root access |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Serializing full document content in IPC responses | Search results take 2–5 seconds to display; IPC call returns a `Vec<Document>` where each `Document.content` is the full extracted text | Never send document content over IPC — send only metadata + excerpt (first 500 chars); fetch full content lazily when user opens the document | At ~50 documents returned in a single query |
| ONNX inference on main Tokio thread | App freezes for 200ms per document during indexing | `spawn_blocking` for all ONNX `session.run()` calls | Every single document if not addressed |
| Re-clustering GNN on every IPC trigger | `recluster_spaces` command takes 2s and blocks UI | Rate-limit clustering to once per 30-second quiet period; clustering is background-only, never triggered by UI directly | Anytime the user clicks "Refresh" during an indexing session |
| React Query `staleTime: 0` (default) | Every component mount triggers a Tauri IPC call; search result page re-queries on every keystroke | Set `staleTime: 30_000` for spaces/stats queries; use `keepPreviousData: true` for search results while typing | Immediately — causes IPC call storm during initial render |
| Embedding all document content without chunking | Long documents (50-page PDFs) produce a single 384-dim vector that averages the meaning into noise | Chunk documents at paragraph or page boundaries; embed each chunk; aggregate with max-pooling or CLS token approach | Documents >~2,000 words where semantic specificity matters |
| Building HNSW index incrementally during bulk import | CPU spikes to 100%; timeouts during initial scan of 5,000-document corpus | Collect all embeddings first, then build HNSW index in a single pass; expose a "building index" progress state | At ~500 documents being imported simultaneously |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Granting `fs:allow-read-*` without path scoping | Frontend WebView (or any injected script) can read any file on the system | Scope `fs` plugin permissions to only the paths the user has explicitly added as watched folders; use `$HOME` variables, not root paths |
| No Content Security Policy in `tauri.conf.json` | XSS in the WebView (e.g., via malicious document content rendered in preview) could call Tauri commands | Set strict CSP: `default-src 'self'; script-src 'self'`; never render raw document HTML in the WebView without sanitization |
| Sending document content to OpenAI API without user opt-in | Privacy violation — users assume local-first | The API embedding path must be gated behind explicit user consent in Settings → AI & Models; never call external APIs unless the user has toggled them on AND confirmed the privacy implication |
| Logging document paths and content to console in dev mode | Sensitive file paths leak in development logs | Use conditional logging (`if cfg!(debug_assertions)`) for anything containing file paths or content; strip all logging in release builds |
| Using `webview_url: "http://localhost:1420"` in production capability | Exposes Tauri IPC to any localhost page — not just the app | Production capabilities must use `tauri://localhost`, not `http://localhost` |
| Development server on unencrypted HTTP on shared network | MITM can inject malicious frontend code into the development Tauri app that has file system access | Develop only on trusted networks; never use `tauri dev` on public WiFi |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No progress visibility during initial scan | User drops 5,000 documents, app appears frozen for minutes with no feedback | Emit granular `scan_progress` events (every 50 documents); show a live counter "Indexing 342 / 5,000 documents" in the TopBar indicator |
| Smart Spaces thrashing during bulk import | Spaces appear and disappear rapidly as GNN re-clusters each batch | Hold all space updates until indexing is complete or a quiet period passes; show "Spaces will update when indexing finishes" |
| Blocking the UI with the "Recluster Spaces" button | User clicks recluster, app freezes for 2 seconds | Recluster always runs in background; disable the button during clustering with a spinner; show a toast when complete |
| No explanation for Smart Space membership | Users confused about why a document ended up in "Property" space | Show the top 3 semantic similarity scores in the document metadata sidebar: "Added to Property (94% match), Work (67%), Home (61%)" |
| Offering "Switch to OpenAI embeddings" without re-index warning | User switches models; existing documents become unsearchable | Show a modal: "Switching requires re-indexing all N documents (~X minutes). Your documents remain accessible during re-indexing." |
| Search showing stale results during background indexing | User searches immediately after dropping files; new files don't appear | Show "N new documents still being indexed" banner on search results page when `scan_in_progress == true` |

---

## "Looks Done But Isn't" Checklist

- [ ] **Tauri IPC error handling:** Commands return `Result<T, String>` but frontend only handles `Ok` — verify `.catch()` and error display exist for every `invoke()` call
- [ ] **ONNX model bundling:** Settings say "local embedding enabled" but ONNX model file path in production build points to dev-time absolute path — verify model is bundled in `src-tauri/resources/` and accessed via `tauri::api::path::resource_dir()`
- [ ] **File watcher restart:** App crashes or is force-quit; file watcher state is lost; re-launch does not re-watch previously configured folders — verify watched folder list is persisted to disk and watcher is restored on startup
- [ ] **Incremental re-indexing:** Modified documents are re-indexed but the old vector is not removed from HNSW — verify deletion and re-insertion logic exists using `content_hash` change detection
- [ ] **Embedding cleanup on document delete:** User removes a watched folder; documents are removed from UI but old vectors remain in HNSW index inflating search results — verify `remove_vector` is called in the document deletion path
- [ ] **macOS entitlements for WebView JIT:** App works unsigned in dev but crashes on first launch after notarization — verify `com.apple.security.cs.allow-jit` and `com.apple.security.cs.allow-unsigned-executable-memory` entitlements are set in `Entitlements.plist`
- [ ] **Ollama availability check at startup:** App silently fails to name new Smart Spaces when Ollama is not running — verify graceful fallback to generic names like "Space 1", "Space 2" with a settings notification
- [ ] **React Query invalidation on space update:** User triggers a manual recluster; spaces are reclustered in Rust; frontend spaces grid still shows stale data — verify the `recluster_complete` event calls `queryClient.invalidateQueries(['spaces'])`

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Embedding dimension mismatch discovered after shipping | HIGH | Implement force re-index command; communicate migration to users; provide progress UI; ~1 hour for 10K documents |
| GNN blocking per-document (wrong architecture) | HIGH | Refactor pipeline to decouple GNN into background task; requires test suite rewrite; 3–5 days |
| HNSW index corruption on crash | MEDIUM | Implement WAL + checkpoint strategy upfront; recovery = replay WAL from last checkpoint; expose "Rebuild Index" command in Settings → Storage |
| notify-rs missing events discovered in production | MEDIUM | Add polling fallback as background scan task; deploy via app update; no data loss, only missed real-time updates |
| Memory leak from Tauri event subscriptions | LOW | Add `unlisten` cleanup to all `useEffect` hooks; deploy via app update; no data loss |
| React Query stale data after indexing | LOW | Add `queryClient.invalidateQueries` to event handler; deploy via app update |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Blocking Tokio runtime with CPU work | Phase: Tauri shell setup — define `spawn_blocking` pattern before first command | Every Tauri command handler reviewed: no sync CPU work without `spawn_blocking` |
| IPC serialization failures | Phase: Tauri shell setup — define `AppError` and serialization contracts | Round-trip integration tests for every IPC command including error paths |
| Embedding dimension mismatch | Phase: Embedding engine — design multi-model schema before writing index code | Integration test: switch model in settings → verify re-index is required and works |
| notify-rs missed events at scale | Phase: File watcher — build polling fallback alongside event watcher | Load test with 2,000-file directory; verify no missed events via hash comparison |
| GNN clustering per-document | Phase: Indexing pipeline design — separate per-doc pipeline from background jobs | Integration test: index 100 documents; verify only 1 clustering run occurs |
| Tauri event memory leak | Phase: Tauri frontend integration — establish `useTauriEvent` hook pattern | Memory profiling test: subscribe/unsubscribe 100 times; verify no listener accumulation |
| React Query cache invalidation | Phase: Tauri frontend integration — define invalidation hooks before building pages | E2E test: trigger backend change → verify UI reflects update without manual refresh |
| macOS entitlements for JIT | Phase: Distribution/signing — validate Entitlements.plist before first test flight | Notarized build test on a clean macOS machine (no dev tools) |
| ONNX model path in production | Phase: Embedding engine + build pipeline | Production build test on clean machine without dev artifacts |
| Index persistence on crash | Phase: RuVector integration — design write path with crash-safety | Force-kill during indexing; relaunch; verify index consistency |

---

## Sources

- [Tauri + Rust breaks under pressure (IPC/async pitfalls)](https://medium.com/@srish5945/tauri-rust-speed-but-heres-where-it-breaks-under-pressure-fef3e8e2dcb3) — MEDIUM confidence (WebSearch)
- [Tauri GitHub Issue #12724: Memory leak when emitting events](https://github.com/tauri-apps/tauri/issues/12724) — HIGH confidence (official repo)
- [Tauri GitHub Issue #10327: IPC Promise hanging with complex values](https://github.com/tauri-apps/tauri/issues/10327) — HIGH confidence (official repo)
- [Tauri GitHub Issue #12388: Event subscription documentation gaps](https://github.com/tauri-apps/tauri/issues/12388) — HIGH confidence (official repo)
- [Tauri Calling Rust from Frontend (official docs)](https://v2.tauri.app/develop/calling-rust/) — HIGH confidence (official)
- [Tauri App Size Optimization (official docs)](https://v2.tauri.app/concept/size/) — HIGH confidence (official)
- [Tauri Application Lifecycle Threats (official docs)](https://v2.tauri.app/security/lifecycle/) — HIGH confidence (official)
- [Tauri File System Plugin (official docs)](https://v2.tauri.app/plugin/file-system/) — HIGH confidence (official)
- [notify-rs GitHub Issue #412: Large scale watching drops events](https://github.com/notify-rs/notify/issues/412) — HIGH confidence (official repo)
- [Resolving Vector Dimension Mismatches in AI Workflows](https://dev.to/hijazi313/resolving-vector-dimension-mismatches-in-ai-workflows-47m) — MEDIUM confidence (WebSearch, corroborated by multiple sources)
- [HNSW embedding dimension hardcoded issue](https://github.com/ruvnet/claude-flow/issues/1143) — MEDIUM confidence (ruvnet repo issue)
- [Qdrant: Vector Search in Production](https://qdrant.tech/articles/vector-search-production/) — MEDIUM confidence (source, fetch blocked)
- [Tauri macOS code signing and notarization issues](https://dev.to/massi_24/shipping-a-production-macos-app-with-tauri-20-code-signing-notarization-and-homebrewpublished-o10) — MEDIUM confidence (WebSearch)
- [Tauri Ollama ORIGINS Windows issue](https://github.com/ollama/ollama/issues/10507) — HIGH confidence (official repo)
- [TauRPC: Typesafe IPC layer](https://github.com/MatsDK/TauRPC) — MEDIUM confidence (community)
- [Pitfalls of React Query](https://nickb.dev/blog/pitfalls-of-react-query/) — MEDIUM confidence (WebSearch)
- [Identifying and Analyzing Pitfalls in GNN Systems (USENIX ATC '25)](https://www.usenix.org/system/files/atc25-gong.pdf) — HIGH confidence (peer-reviewed)

---
*Pitfalls research for: Tauri 2 + Rust backend + RuVector document intelligence desktop app*
*Researched: 2026-02-27*

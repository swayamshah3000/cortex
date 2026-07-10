# Codebase Concerns

**Analysis Date:** 2026-02-27

## Architecture Misalignment

**Backend Implementation Missing:**
- Issue: CLAUDE.md specifies a Tauri 2 desktop app with Rust backend, RuVector integration, and complex IPC commands, but the actual codebase is a web-only React + Express application with no Tauri build files, no Cargo.toml, and no Rust implementation
- Files: `Entire /Users/gshah/work/apps/cortex/` project root - no `src-tauri/`, no Tauri configuration
- Impact: Fundamental architectural mismatch prevents any functionality described in CLAUDE.md from working. File watching, RuVector embeddings, document parsing (PDF, DOCX, OCR), and semantic search are entirely missing
- Fix approach: Either (1) implement full Tauri 2 backend with Rust, RuVector integration, and document pipeline, or (2) pivot the frontend to target a different architecture (web-based, server-based, or hybrid) and update CLAUDE.md to match

## Tech Stack Gaps

**Missing Core Dependencies:**
- Issue: CLAUDE.md specifies RuVector crates (ruvector-core, ruvector-gnn, ruvector-graph, etc.), ONNX Runtime for embeddings, pdf-extract, docx-rs, image processing libraries, and tesseract OCR - none are in dependencies
- Files: `package.json` - only contains React, Express, Tailwind, shadcn/ui, but missing all backend infrastructure
- Impact: Zero document processing capabilities. No semantic search, no vector embeddings, no clustering, no LLM integration for space naming
- Fix approach: Create `src-tauri/` directory structure with Cargo.toml containing ruvector workspace, embedding providers (ONNX or API), and document parsing libraries

**Missing Optional API Integrations:**
- Issue: CLAUDE.md references "local Ollama or Claude/OpenAI API" for space naming and "ONNX Runtime or OpenAI API" for embeddings, but no API clients, no ollama-rs dependency, no openai SDK
- Files: `package.json` and `netlify/functions/api.ts` are empty of these integrations
- Impact: Can't name spaces automatically or generate embeddings for documents
- Fix approach: Add ollama-rs and openai/anthropic crates to Cargo.toml; add env var handling for API keys

## Incomplete Frontend

**Hardcoded Mock Data:**
- Issue: All route pages except dashboard are stubbed out with `Placeholder` component; 10 of 12 routes not implemented
- Files: `client/pages/Placeholder.tsx`, `client/App.tsx` lines 27-36 (9 placeholder routes)
- Impact: Unfinished product. Users can't access Smart Spaces, Search, Recent, Favorites, Tags, Watched Folders, Insights, Settings, or Onboarding
- Fix approach: Implement each route page component matching `../FRONTEND_SPEC.md` specification

**Hardcoded Dashboard Data:**
- Issue: Dashboard shows mock stats ("3.9K documents", "24 spaces") and hardcoded recent documents (Property Tax 2025, Invoice Feb 2026)
- Files: `client/pages/Index.tsx` lines 15-97 (all mock data arrays), Sidebar lines 129-157 (hardcoded space list)
- Impact: Dashboard doesn't reflect actual user documents or system state
- Fix approach: Wire React Query to fetch real stats from backend; implement Tauri IPC commands for getting documents, spaces, and stats

**No State Management Integration:**
- Issue: CLAUDE.md specifies Zustand for UI state (sidebar, theme, command palette) and React Query for server data, but neither is set up beyond imports
- Files: `client/App.tsx` has QueryClientProvider and ThemeProvider but no Zustand store; `client/components/layout/Sidebar.tsx` uses local useState instead of Zustand
- Impact: Sidebar collapse state isn't persisted, no global command palette, no centralized error handling
- Fix approach: Create `client/lib/store.ts` with Zustand store for UI state; implement CommandPalette component with modal state

## Testing Gaps

**Minimal Test Coverage:**
- Issue: Only 1 test file exists (`client/lib/utils.spec.ts`) with 5 tests for `cn()` utility function; 0 tests for components, pages, or business logic
- Files: `client/lib/utils.spec.ts` - only tests
- Impact: No confidence in component behavior; no regression detection for critical features like document indexing, space clustering, search
- Fix approach: Add unit tests for components (Sidebar, TopBar, dashboard sections); add integration tests for future Tauri IPC commands; add E2E tests via Tauri/Webdriver

**No Backend Testing Setup:**
- Issue: Express server in `server/index.ts` and `server/routes/demo.ts` have zero tests; Tauri backend (when implemented) will need Rust tests
- Files: `server/index.ts`, `server/routes/demo.ts` - no .test files
- Impact: Backend changes (document indexing, vector queries) can silently break without detection
- Fix approach: Create `server/__tests__/` directory; add Vitest tests for Express routes; add cargo test setup for Rust backend

## TypeScript Configuration

**Strict Mode Disabled:**
- Issue: tsconfig.json disables strict type checking, allowing implicitly any types, unused variables/parameters, null checks, and fallthrough cases
- Files: `tsconfig.json` lines 21-26 ("strict": false, "noUnusedLocals": false, "noUnusedParameters": false, "noImplicitAny": false, "strictNullChecks": false)
- Impact: Type safety is compromised; runtime errors possible from null/undefined values; dead code accumulates; refactoring becomes risky
- Fix approach: Enable strict mode incrementally (start with `strict: true`, then relax specific rules by exception with `// @ts-expect-error` comments where necessary)

## Server Configuration Issues

**CORS Enabled Globally:**
- Issue: `server/index.ts` line 10 enables CORS with default settings (all origins, all methods), no origin whitelist
- Files: `server/index.ts` line 10: `app.use(cors())`
- Impact: In production, any domain can make requests to the server, opening CSRF and data exposure vulnerabilities
- Fix approach: Configure CORS with explicit allowlist: `cors({ origin: process.env.ALLOWED_ORIGINS?.split(',') })`

**Environment Configuration Incomplete:**
- Issue: `.env` references `VITE_PUBLIC_BUILDER_KEY` (not used), `PING_MESSAGE` (test only); missing all real config variables for RuVector, embeddings, LLM, API keys
- Files: `.env` lines 5-6 - only test variables
- Impact: When backend is implemented, no way to configure API keys, model selection, database paths, or folder watching settings
- Fix approach: Document required env vars in `.env.example`: EMBEDDING_API_KEY, OLLAMA_URL, WATCHED_FOLDERS, INDEX_PATH, LOG_LEVEL, etc.

## Database/Vector Store Gaps

**No Vector Storage Implementation:**
- Issue: CLAUDE.md specifies RuVector core with HNSW indexing, but there's no database setup, no migrations, no storage abstraction
- Files: No database configuration anywhere; `shared/api.ts` only has types
- Impact: Can't persist document embeddings, search results, or clustering state across sessions
- Fix approach: Implement RuVector storage layer in Rust backend; add data persistence to AppData or Documents folder

**No Local Data Directory:**
- Issue: Index path, vector database location, and cache directory are undefined
- Files: None - configuration is missing
- Impact: No clear where user data lives; potential data loss if app overwrites directories
- Fix approach: Create `~/.cortex/` directory structure: `~/.cortex/data/` for index, `~/.cortex/cache/` for embeddings, `~/.cortex/logs/` for telemetry

## Security Considerations

**No Authentication/Authorization:**
- Issue: CLAUDE.md shows no authentication scheme; local-first architecture implies all documents are accessible to any process that can access the file system
- Files: No auth implementation anywhere; file watching via notify-rs has no access control
- Impact: If user runs Cortex and another untrusted application, the untrusted app could read all indexed documents through the RuVector database
- Current mitigation: File system permissions are the only boundary (no elevation required to read user documents)
- Recommendations: (1) Encrypt vector index with user-provided password or system keyring, (2) implement file ACLs within RuVector, (3) document that Cortex does not provide isolation from other processes on the same machine

**No Input Validation on File Watching:**
- Issue: `add_watched_folder(path: &str)` in CLAUDE.md has no validation; user could specify paths outside their home directory
- Files: Future Tauri command - not implemented yet
- Impact: App could be used to index system files, creating privacy violation or performance issue
- Recommendations: Validate watched folder paths against home directory; reject system paths (/System, /Library, /usr, /etc, Windows: C:\Windows, etc.)

**Vector Index Not Encrypted:**
- Issue: Document embeddings and extracted metadata stored in plain text in RuVector; an attacker with file access reads all document summaries/entities
- Files: Future RuVector database - not yet implemented
- Impact: Privacy risk: entity extraction contains dates, amounts, person names, organizations that reveal sensitive information
- Recommendations: Add AES-256 encryption to RuVector storage layer with key from system keyring (Keychain on macOS, Credential Manager on Windows)

## Performance Concerns

**GNN Clustering Scalability Unknown:**
- Issue: CLAUDE.md references "auto-discovers Smart Spaces from document similarity" via GNN, but complexity of clustering is O(n²) or worse for large document sets
- Files: Future RuVector GNN implementation - unproven at scale
- Impact: With 3.9K documents (dashboard mock data), clustering could take minutes or cause UI freeze
- Fix approach: Benchmark RuVector GNN with increasing document counts (1K, 5K, 10K, 50K); implement async clustering with progress reporting; implement incremental clustering for new documents

**HNSW Index Memory Usage:**
- Issue: HNSW indexing stores vectors in memory for fast search; with 3.9K documents at 384-1536 dimensions, memory could be 10-100MB or more
- Files: Future RuVector core implementation
- Impact: On resource-constrained machines (older laptops, RPi deployment), app could become unresponsive or crash
- Fix approach: Profile memory usage; implement configurable index pruning; add memory usage monitoring to `/insights` dashboard

**No Pagination/Lazy Loading:**
- Issue: Dashboard shows "3.9K total documents" but only displays 4 recent documents; if all documents rendered in /search or /favorites, UI would freeze
- Files: `client/pages/Index.tsx` shows mock data, but true implementation TBD
- Impact: Large document collections will cause poor performance
- Fix approach: Implement infinite scroll or pagination in search results; use React.lazy() for route components; virtualize document lists

## Fragile Areas

**Hardcoded CSS Color Values:**
- Issue: `client/pages/Index.tsx` lines 20-40 and `client/pages/Placeholder.tsx` use hardcoded color classes (bg-blue-500, text-purple-500) instead of design token imports
- Files: `client/pages/Index.tsx` lines 20-40 (stat colors), `client/components/layout/Sidebar.tsx` lines 130-133 (space colors)
- Impact: Design changes require manual updates across multiple files; inconsistent with design system
- Safe modification: Create color token export in `client/lib/colors.ts` or import from design system, use sparingly with `cn()` utility
- Test coverage: Missing unit tests for color application; no design regression tests

**Fixed Layout Margin:**
- Issue: `client/components/layout/AppShell.tsx` line 12 has `ml-60` hardcoded, but Sidebar has responsive behavior (collapsible to 64px on `sm:` breakpoint but margin doesn't adjust)
- Files: `client/components/layout/AppShell.tsx` line 12: `ml-60`
- Impact: On mobile (below 640px), the fixed 240px margin breaks layout; sidebar hides on smaller screens but leaves margin space
- Safe modification: Implement responsive margin that matches sidebar width: `ml-60 sm:ml-20 md:ml-60` or use CSS custom property from sidebar component
- Test coverage: Missing responsive layout tests for mobile breakpoints

**Dynamic Route Parameter Not Validated:**
- Issue: `client/pages/Index.tsx` line 188 constructs space link as `/spaces/${space.name.toLowerCase()}` but routes expect `:id` parameter; no validation that ID exists
- Files: `client/App.tsx` line 28: `<Route path="/spaces/:id" element={<Placeholder />} />`, `client/pages/Index.tsx` line 188
- Impact: User can navigate to invalid space URL (e.g., `/spaces/nonexistent`), showing placeholder instead of error or redirect
- Safe modification: Implement route guard in Placeholder component; fetch space by ID in useEffect; show 404 if not found
- Test coverage: Missing navigation tests for invalid routes

## Dependencies at Risk

**React Query Version Outdated Usage:**
- Issue: package.json uses `@tanstack/react-query@^5.84.2` (v5 stable), but frontend doesn't use it at all - all data is hardcoded mock
- Files: `package.json` line 61, but no useQuery/useMutation in client code
- Impact: When backend is implemented, will require learning new Query hooks; dead dependency until then
- Migration plan: Use React Query for all future data fetching; implement custom hooks in `client/hooks/useTauri.ts` that wrap Tauri commands with React Query

**Vite SWC Plugin Configuration:**
- Issue: `package.json` uses `@vitejs/plugin-react-swc` for faster builds, but no `.swcrc` configuration; uses defaults which may not match team standards
- Files: `vite.config.ts` line 2: `import react from "@vitejs/plugin-react-swc"`
- Impact: SWC may produce different output than expected; debugging transpilation issues harder without explicit config
- Fix approach: Create `.swcrc` file with explicit compiler options matching TypeScript config intent

**Zustand Not Installed:**
- Issue: CLAUDE.md specifies Zustand for UI state but it's not in package.json dependencies
- Files: `package.json` - missing Zustand
- Impact: Cannot implement UI state management as planned
- Migration plan: Add `zustand@^4.x` to devDependencies; create `client/lib/store.ts`

## Missing Critical Features

**No Keyboard Shortcuts:**
- Issue: CLAUDE.md specifies keyboard shortcuts (Cmd+K, Cmd+1/2/3, Cmd+D, Cmd+\, /, Esc) but Command Palette component doesn't exist and shortcuts aren't wired
- Files: No keyboard event handlers anywhere; Sidebar has `<span>⌘K</span>` text only (line 99)
- Impact: Promised productivity features unavailable; users must click navigation instead of using keyboard
- Fix approach: Create CommandPalette component with cmdk library (already in package.json), implement useKeyboardShortcuts hook, integrate into AppShell

**No Document Preview:**
- Issue: CLAUDE.md specifies `/document/:id` with split-view (65% preview + 35% sidebar), but no preview component exists
- Files: No /document/:id route implemented; no PDF viewer or document rendering
- Impact: Users can't actually view their documents within Cortex
- Fix approach: Implement DocumentPreview page; integrate react-pdf for PDF, custom renderers for DOCX/text, image preview for photos

**No Search Implementation:**
- Issue: CLAUDE.md emphasizes semantic search as core feature, but `/search` route shows placeholder; no search algorithm or vector query implementation
- Files: `client/pages/Placeholder.tsx` (search route unimplemented), no search service layer
- Impact: "Find anything" promise unfulfilled
- Fix approach: Create Search page with filters, results, preview panel; implement search service that calls Tauri `search_documents` command with vector query

**No Onboarding Wizard:**
- Issue: CLAUDE.md specifies `/onboarding` with 4-step wizard (Welcome → Folders → Scanning → Ready), but only Placeholder exists
- Files: `client/pages/Placeholder.tsx` (onboarding unimplemented)
- Impact: New users get no guidance; no folder selection UI
- Fix approach: Create Onboarding page with steps, folder picker, progress indicator, completion state

## Configuration Gaps

**No Logger Configuration:**
- Issue: CLAUDE.md mentions "Activity feed" and analytics, but no logging library configured; only `console.*` available
- Files: No winston, pino, or other logger setup
- Impact: Debugging server-side issues difficult; no log file for post-mortem analysis
- Fix approach: Add pino or winston to Rust backend; implement structured logging for indexing, clustering, and search operations

**No Error Tracking:**
- Issue: No Sentry, LogRocket, or similar error tracking; crashes/bugs go unnoticed
- Files: No error tracking service
- Impact: Production issues unknown until user reports them
- Fix approach: Add sentry-tauri or OpenTelemetry integration; send anonymized crash reports (with user consent)

**Build Configuration Incomplete:**
- Issue: `vite.config.ts` denies access to `server/**` but Netlify serverless functions in `netlify/functions/` are disconnected from main server
- Files: `vite.config.ts` lines 11-13 (FS allowlist), `netlify/functions/api.ts` (unused), `netlify.toml` (minimal)
- Impact: Unclear if app should target Netlify Functions (serverless), Express (local dev), or Tauri backend; build artifacts unclear
- Fix approach: Clarify deployment target; if desktop app, remove Netlify config; if web app, remove Express/Tauri references

## Testing Coverage Gaps

**Dashboard Statistics Not Tested:**
- What's not tested: Hardcoded stats display (3.9K documents, 24 spaces, 1.2G index) have no assertions
- Files: `client/pages/Index.tsx` lines 15-40 (stat data), no test file
- Risk: Display logic changes silently break without notice
- Priority: Medium - visual regression won't break app functionality but affects user trust

**Recent Documents List Not Tested:**
- What's not tested: Mock recent documents rendering, date formatting, space breadcrumbs
- Files: `client/pages/Index.tsx` lines 42-167, no test file
- Risk: Layout breaks with long filenames or special characters; date formatting incorrect in different locales
- Priority: Medium - important for user experience but not core functionality

**Navigation Links Not Tested:**
- What's not tested: Active link highlighting in Sidebar, space navigation, route transitions
- Files: `client/components/layout/Sidebar.tsx` (isActive logic), no test file
- Risk: Wrong link highlighted as active; navigation fails silently
- Priority: High - broken navigation destroys usability

**Theme Switching Not Tested:**
- What's not tested: Dark/light mode toggle, theme persistence, design token application
- Files: `client/components/layout/TopBar.tsx` (theme toggle), no test file
- Risk: Theme toggle doesn't save preference; colors don't update in some components
- Priority: Medium - cosmetic but affects user experience

---

*Concerns audit: 2026-02-27*

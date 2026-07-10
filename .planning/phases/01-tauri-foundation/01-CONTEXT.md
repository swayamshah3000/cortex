# Phase 1: Tauri Foundation - Context

**Gathered:** 2026-02-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Convert existing React web app to Tauri 2 desktop shell. Add src-tauri/ alongside existing client/, remove Express server, establish typed IPC contracts with AppError enum and spawn_blocking patterns, initialize RuVector with multi-collection support and metadata filtering. All IPC commands are stubs returning mock data until Phase 2+ builds real backends.

</domain>

<decisions>
## Implementation Decisions

### Migration approach
- Wrap existing React frontend in Tauri 2 WebView — client/ code stays intact
- Remove Express server entirely (clean break, not dual-mode)
- Upgrade React 18 → 19 and TailwindCSS 3 → 4 as part of this phase
- Keep pnpm as package manager (already configured, lockfile exists)

### IPC contract naming
- Domain-prefixed command names matching CLAUDE.md spec: `search_documents`, `get_spaces`, `add_watched_folder`, `get_stats`, etc.
- All commands defined in CLAUDE.md's "RuVector Integration Points" section are the target IPC surface

### Data storage
- Index size must be visible in Settings > Storage with option to clear/rebuild

### Dev workflow
- Frontend must still run standalone without Tauri (pnpm dev) — hooks fall back to mock data
- Unit tests for core Rust code: AppError serialization, IPC command stubs, RuVector initialization
- Dual-mode hooks: detect Tauri runtime → use invoke(), otherwise → use mock data

### Claude's Discretion
- Server/ directory disposition (delete or archive — evaluate if any code is reusable)
- RuVector storage location (standard app data dir recommended by Tauri conventions)
- First-launch behavior before onboarding exists (Phase 4 builds real onboarding)
- IPC stub behavior (return mock data vs "not implemented" errors)
- IPC error granularity (typed enum vs simple messages — evolve as needed)
- Tauri event system setup timing (now vs Phase 2)
- Embedding model switch strategy (separate collections vs re-index)
- Developer onboarding documentation approach
- CI/CD setup timing

</decisions>

<specifics>
## Specific Ideas

- "Can't we use the existing frontend and wrap it up in Tauri?" — Yes, that's exactly the approach. Existing React frontend is preserved, wrapped in Tauri WebView.
- The CLAUDE.md spec defines all Tauri IPC commands as the target contract surface — use that as the canonical reference for command signatures.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-tauri-foundation*
*Context gathered: 2026-02-27*

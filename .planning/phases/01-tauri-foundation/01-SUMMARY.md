---
phase: 01-tauri-foundation
plan: "01"
subsystem: infra
tags: [tauri, rust, vite, desktop, scaffold]

# Dependency graph
requires: []
provides:
  - Tauri 2 desktop shell compiling and ready for development
  - src-tauri/ with Cargo.toml, build.rs, main.rs, lib.rs, tauri.conf.json
  - Cortex window configured (1400x900, min 900x600)
  - Express server removed, project renamed to cortex
  - Clean pnpm-only frontend with vite build output to dist/
affects:
  - 02-tauri-foundation (IPC commands need src-tauri/ scaffold)
  - 03-tauri-foundation (file watcher depends on clean Rust foundation)
  - 04-tauri-foundation (RuVector integration needs Cargo.toml ready)

# Tech tracking
tech-stack:
  added:
    - "@tauri-apps/cli@2 (Tauri CLI dev dependency)"
    - "tauri@2 (Rust crate)"
    - "tauri-build@2 (Rust build dep)"
    - "serde + serde_json + tokio (Rust runtime deps)"
  patterns:
    - "src-tauri/src/lib.rs exports pub fn run() called from main.rs"
    - "Tauri window config in tauri.conf.json (productName, identifier, window size)"
    - "capabilities/default.json with core:default permissions"
    - "dist/ as frontend output (matches Tauri frontendDist)"

key-files:
  created:
    - src-tauri/Cargo.toml
    - src-tauri/build.rs
    - src-tauri/src/main.rs
    - src-tauri/src/lib.rs
    - src-tauri/tauri.conf.json
    - src-tauri/capabilities/default.json
    - src-tauri/icons/ (32x32.png, 128x128.png, 128x128@2x.png, icon.icns, icon.ico)
  modified:
    - package.json (name=cortex, removed server deps, added tauri script)
    - pnpm-lock.yaml (lockfile cleaned)
    - vite.config.ts (removed express plugin, port changed to 5173, outDir=dist)
    - .gitignore (added src-tauri/target/ and src-tauri/gen/schemas/)

key-decisions:
  - "Manually scaffolded src-tauri/ instead of using interactive tauri init (non-interactive environment)"
  - "Icons generated using ImageMagick with RGBA color type (required by Tauri generate_context! macro)"
  - "Vite outDir changed from dist/spa to dist/ to match tauri.conf.json frontendDist"
  - "Dev server port set to 5173 (Vite default, matches tauri.conf.json devUrl)"

patterns-established:
  - "Tauri pattern: main.rs calls cortex_lib::run(), lib.rs owns Builder::default()"
  - "Build flow: pnpm dev (frontend standalone) | pnpm tauri dev (full desktop app)"

requirements-completed: [TAURI-01, TAURI-02]

# Metrics
duration: 5min
completed: 2026-02-27
---

# Phase 1 Plan 01: Tauri 2 Scaffold and Express Removal Summary

**Tauri 2 desktop shell scaffolded with Cortex window config (1400x900), Express server removed, project cleaned to pnpm-only Vite frontend**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-27T13:25:56Z
- **Completed:** 2026-02-27T13:31:25Z
- **Tasks:** 2
- **Files modified:** 10 (4 created, 7 new in src-tauri/, 3 modified, 7 deleted)

## Accomplishments

- Tauri 2 scaffold in src-tauri/ with Cargo.toml, main.rs, lib.rs, build.rs — `cargo check` passes
- Cortex window configured: title "Cortex", 1400x900, min 900x600, identifier com.cortex.app
- Removed Express server (server/ dir), netlify, .dockerignore, vite.config.server.ts entirely
- package.json renamed to cortex, server deps removed, tauri script added
- vite.config.ts cleaned (removed express plugin, set port 5173, outDir=dist)
- RGBA icons generated for Tauri icon requirements (MacOS icns, ico, 3 PNG sizes)

## Task Commits

Each task was committed atomically:

1. **Task 01.1: Initialize src-tauri/ with Tauri 2** - `38a12ea` (feat)
2. **Task 01.1: Add Tauri build artifacts to .gitignore** - `d06e414` (chore — deviation fix)
3. **Task 01.2: Remove Express server and deployment artifacts** - `d5a6f24` (feat)

## Files Created/Modified

- `src-tauri/Cargo.toml` - Package name=cortex, lib/cdylib/staticlib crate types, tauri 2 + serde + tokio deps
- `src-tauri/build.rs` - Calls tauri_build::build()
- `src-tauri/src/main.rs` - windows_subsystem="windows" guard, calls cortex_lib::run()
- `src-tauri/src/lib.rs` - pub fn run() bootstraps tauri::Builder::default()
- `src-tauri/tauri.conf.json` - Cortex window 1400x900, devUrl localhost:5173, frontendDist ../dist
- `src-tauri/capabilities/default.json` - core:default permissions for main window
- `src-tauri/icons/` - 5 icon files (32x32, 128x128, 128x128@2x, icns, ico) in RGBA format
- `package.json` - Renamed to cortex, removed 6 server deps, added tauri script, simplified build
- `vite.config.ts` - Removed express middleware plugin, port 5173, outDir dist/
- `.gitignore` - Added src-tauri/target/ and src-tauri/gen/schemas/ exclusions

## Decisions Made

- Manually scaffolded src-tauri/ (tauri init is interactive, unsuitable for automated execution)
- Icons use ImageMagick with explicit RGBA color type (-define png:color-type=6) because Tauri's generate_context! macro validates PNG format
- Build output changed from dist/spa to dist/ to match tauri.conf.json frontendDist setting
- Dev server stays on port 5173 (Vite default) matching tauri.conf.json devUrl

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added src-tauri/target/ to .gitignore immediately after first commit**
- **Found during:** Task 01.1 (commit of src-tauri/ scaffold)
- **Issue:** cargo check produced compiled artifacts in target/ which got staged accidentally in the first commit (3180 files)
- **Fix:** Added src-tauri/target/ and src-tauri/gen/schemas/ to .gitignore, ran git rm --cached to untrack them
- **Files modified:** .gitignore
- **Verification:** git status confirmed target/ no longer tracked
- **Committed in:** d06e414 (cleanup commit immediately after task 01.1)

**2. [Rule 3 - Blocking] Fixed vite.config.ts to remove Express server import**
- **Found during:** Task 01.2 (cleanup of server artifacts)
- **Issue:** vite.config.ts imported from ./server (createServer) and had express middleware plugin; after deleting server/, this caused TypeScript error TS2307 and would break pnpm dev
- **Fix:** Rewrote vite.config.ts to remove express plugin, set port 5173, outDir=dist
- **Files modified:** vite.config.ts
- **Verification:** pnpm run typecheck no longer shows vite.config.ts error; pnpm build succeeds
- **Committed in:** d5a6f24 (part of Task 01.2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 3 - Blocking)
**Impact on plan:** Both fixes required for correctness. Target/ gitignore prevents build artifact pollution. vite.config.ts fix was essential after deleting the server it imported from.

## Issues Encountered

- Icons generated without RGBA format initially caused `cargo check` to fail with "icon is not RGBA" from Tauri's generate_context! macro. Fixed by regenerating with `-type TrueColorAlpha -define png:color-type=6` flags.

## User Setup Required

None - no external service configuration required. `pnpm tauri dev` ready to test desktop window.

## Next Phase Readiness

- src-tauri/ scaffold is ready for IPC command additions (Plan 02)
- cargo check passes in under 2 seconds (cached)
- Frontend build (pnpm build) verified working with Vite 7
- Concern: Pre-existing TypeScript errors in client/components/layout/Sidebar.tsx (Lucide prop type mismatch) — these are out of scope for this plan but should be addressed before Plan 05 (final UI wiring)

---
*Phase: 01-tauri-foundation*
*Completed: 2026-02-27*

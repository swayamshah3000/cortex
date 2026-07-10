---
wave: 1
depends_on: []
requirements: [TAURI-01, TAURI-02]
files_modified:
  - src-tauri/Cargo.toml
  - src-tauri/build.rs
  - src-tauri/tauri.conf.json
  - src-tauri/capabilities/default.json
  - src-tauri/src/main.rs
  - src-tauri/src/lib.rs
  - src-tauri/icons/
  - package.json
  - vite.config.ts
  - index.html
autonomous: true
---

# Plan 01: Tauri 2 Scaffold and Express Removal

## Goal

The Tauri 2 desktop shell compiles and opens a window rendering the existing React frontend. The Express server directory is removed. `pnpm tauri dev` launches the app; `pnpm dev` still runs the frontend standalone.

## Context

- Existing React frontend lives in `client/` with entry point at `index.html` (project root).
- `server/` contains Express code (`index.ts`, `routes/`, `node-build.ts`) — confirmed no reusable logic; delete entirely.
- Vite dev server runs on default port 5173.
- Package manager is pnpm (locked decision).
- Related files to remove: `vite.config.server.ts`, `netlify.toml`, `netlify/`, `.dockerignore`, server-related deps (`express`, `dotenv`, `cors`, `serverless-http`, `@types/express`, `@types/cors`).

## Tasks

<task id="01.1" effort="M">
<title>Initialize src-tauri/ with Tauri 2</title>
<detail>
Run `pnpm add -D @tauri-apps/cli` then `pnpm tauri init` to scaffold `src-tauri/`.

After scaffold, edit the generated files:

**src-tauri/tauri.conf.json** — set:
- `productName`: `"Cortex"`
- `version`: `"0.1.0"`
- `identifier`: `"com.cortex.app"`
- `build.beforeDevCommand`: `"pnpm dev"`
- `build.devUrl`: `"http://localhost:5173"`
- `build.beforeBuildCommand`: `"pnpm build:client"`
- `build.frontendDist`: `"../dist"`
- `app.windows[0]`: `{ "title": "Cortex", "width": 1400, "height": 900, "minWidth": 900, "minHeight": 600 }`

**src-tauri/Cargo.toml** — set:
- `package.name` = `"cortex"`
- `package.version` = `"0.1.0"`
- `package.edition` = `"2021"`
- `[lib]` section: `name = "cortex_lib"`, `crate-type = ["lib", "cdylib", "staticlib"]`
- Dependencies: `tauri = { version = "2", features = [] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `tokio = { version = "1", features = ["full"] }`
- Build deps: `tauri-build = { version = "2", features = [] }`
- Do NOT add ruvector deps yet (that's Plan 04).

**src-tauri/src/main.rs** — minimal Tauri app entry:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    cortex_lib::run();
}
```

**src-tauri/src/lib.rs** — minimal run function:
```rust
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**src-tauri/build.rs**:
```rust
fn main() {
    tauri_build::build();
}
```

Verify `pnpm tauri dev` opens a window showing the React frontend.
</detail>
</task>

<task id="01.2" effort="S">
<title>Remove Express server and deployment artifacts</title>
<detail>
Delete these files and directories entirely:
- `server/` (directory)
- `vite.config.server.ts`
- `netlify.toml`
- `netlify/` (directory)
- `.dockerignore`

Edit `package.json`:
- Remove `dependencies`: `express`, `dotenv` (keep `zod` — used by forms)
- Remove `devDependencies`: `cors`, `serverless-http`, `@types/express`, `@types/cors`
- Remove `pkg` section entirely (server asset config)
- Update `scripts`:
  - Remove `build:server`, `start`
  - Change `build` to just `vite build` (no server build step)
  - Add `"tauri": "tauri"` script
- Change `name` from `"fusion-starter"` to `"cortex"`

Run `pnpm install` to clean the lockfile.

Verify `pnpm dev` still starts the Vite dev server and the frontend renders in browser at localhost:5173.
</detail>
</task>

## Verification

```bash
# 1. Tauri compiles
cd src-tauri && cargo check

# 2. Frontend still works standalone
pnpm dev &
sleep 3
curl -s http://localhost:5173 | grep -q "Cortex" && echo "PASS" || echo "FAIL"
kill %1

# 3. Server artifacts gone
test ! -d server && test ! -f vite.config.server.ts && test ! -f netlify.toml && echo "PASS" || echo "FAIL"

# 4. Package.json clean
! grep -q '"express"' package.json && echo "PASS" || echo "FAIL"
```

## must_haves

- [ ] `src-tauri/` directory exists with valid `Cargo.toml`, `tauri.conf.json`, `main.rs`, `lib.rs`, `build.rs`
- [ ] `cargo check` in `src-tauri/` succeeds (Rust compiles)
- [ ] `server/` directory does not exist
- [ ] `vite.config.server.ts`, `netlify.toml`, `netlify/`, `.dockerignore` do not exist
- [ ] `package.json` has no `express`, `dotenv`, `cors`, `serverless-http` dependencies
- [ ] `package.json` name is `"cortex"`
- [ ] `pnpm dev` starts frontend standalone at localhost:5173
- [ ] `pnpm tauri dev` launches a desktop window rendering the React frontend

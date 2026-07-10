---
phase: 06-knowledge-graph-and-native-integrations
plan: 01
subsystem: foundation
tags: [tauri-plugin, csp, onnx, ner, package-legitimacy, supply-chain]
requires:
  - "tauri 2 src-tauri scaffold (Phase 01-01)"
  - "EmbeddingService load-once pattern (Phase 02-01)"
provides:
  - "Rust deps: ort 2.0.0-rc.12 (load-dynamic, ndarray), tokenizers 0.20, ndarray 0.16"
  - "Rust deps: tauri-plugin-dialog 2.7.1, tauri-plugin-opener 2.5.4, tauri-plugin-fs 2"
  - "Frontend deps: @tauri-apps/plugin-{dialog,opener,fs}, react-markdown 10, remark-gfm 4"
  - "Bundled NER model assets at src-tauri/models/ (.onnx gitignored, .json tracked)"
  - "Tauri plugins registered in lib.rs builder chain BEFORE .setup()"
  - "Explicit CSP, assetProtocol enabled, bundle.resources for models/"
  - "Capability permissions for dialog + opener + fs"
affects:
  - "src-tauri/Cargo.toml (deps + tauri protocol-asset feature)"
  - "src-tauri/capabilities/default.json (5 new permissions)"
  - "src-tauri/tauri.conf.json (CSP, assetProtocol, bundle.resources)"
  - "src-tauri/src/lib.rs (3 plugin init calls before setup)"
  - "src-tauri/.gitignore (new file)"
  - "src-tauri/models/* (new bundled assets)"
  - "scripts/download-ner-model.sh (new script)"
  - "package.json + pnpm-lock.yaml (5 new deps)"
tech-stack:
  added:
    - "ort 2.0.0-rc.12 — ONNX Runtime wrapper (Rust)"
    - "tokenizers 0.20 — HuggingFace tokenizer (Rust)"
    - "ndarray 0.16 — Tensor construction (Rust)"
    - "tauri-plugin-dialog 2.7.1 — Native folder picker"
    - "tauri-plugin-opener 2.5.4 — openPath + revealItemInDir"
    - "tauri-plugin-fs 2 — fs::exists + fs::stat for Plan 04 D-19"
    - "react-markdown 10.1.0 — markdown renderer"
    - "remark-gfm 4.0.1 — GFM plugin (tables, task lists, autolinks)"
  patterns:
    - "Tauri 2 capability-based permissions"
    - "Asset protocol with explicit CSP allowlisting"
    - "Bundled ML model + integrity checkpoint (SHA-256 vs LFS pointer)"
key-files:
  created:
    - "scripts/download-ner-model.sh"
    - "src-tauri/.gitignore"
    - "src-tauri/models/tokenizer.json"
    - "src-tauri/models/config.json"
    - "src-tauri/models/special_tokens_map.json"
    - "src-tauri/models/bert-base-NER.onnx (gitignored — produced by script)"
  modified:
    - "src-tauri/Cargo.toml"
    - "src-tauri/capabilities/default.json"
    - "src-tauri/tauri.conf.json"
    - "src-tauri/src/lib.rs"
    - "package.json"
    - "pnpm-lock.yaml"
decisions:
  - "[06-01] ort feature set: kept the plan's recommended load-dynamic + ndarray (NOT the download-binaries fallback). cargo check succeeded without locating an ONNX Runtime at build time — the load-dynamic path resolves the .dylib/.so at app runtime via DYLIB_PATH probing, which is the correct posture for a desktop app where we control the runtime via Tauri bundling. The download-binaries fallback would be a follow-up only if runtime loading fails on a target platform."
  - "[06-01] Added 'protocol-asset' feature on the tauri crate. Without it, tauri-build aborted with 'dependency features do not match allowlist' when assetProtocol.enable=true was set in tauri.conf.json. Auto-fix (Rule 3 — blocking issue)."
  - "[06-01] Used pnpm install (NOT bun) per package.json packageManager=pnpm@10.14.0. CLAUDE.md mentions bun but the project manifest is canonical."
  - "[06-01] Created src-tauri/models/.gitkeep placeholder so the bundle.resources=['models/*'] glob matches before the download script populates the directory. tauri-build aborts if the glob has zero matches."
  - "[06-01] Plugin registration uses tauri_plugin_fs::init() with no scope configuration at the Rust layer — fs:allow-exists and fs:allow-stat permissions in capabilities/default.json are intentionally narrow (no read/write)."
metrics:
  duration: 19m (interactive — two blocking-human gates)
  task-commits: 2
  files-created: 6
  files-modified: 6
  completed: "2026-06-29"
---

# Phase 6 Plan 1: Foundation — Deps + Plugins + Model Bundle + CSP Summary

Phase 6 dep wall installed: ort 2.x ONNX runtime + tokenizers + ndarray (Rust), tauri-plugin-dialog/opener/fs (Rust + JS), react-markdown/remark-gfm (JS); bundled bert-base-NER (109 MB INT8 ONNX) under `src-tauri/models/`; CSP tightened from `null` to an explicit allowlist that permits the asset: protocol; capability permissions added for dialog + opener + fs.

## What Was Built

### Task 2 — Backend wiring (commit `5bffbdb`)

**Cargo.toml additions** (`src-tauri/Cargo.toml`):

```toml
ort = { version = "2.0.0-rc.12", default-features = false, features = ["load-dynamic", "ndarray"] }
tokenizers = "0.20"
ndarray = "0.16"
tauri-plugin-dialog = "2.7.1"
tauri-plugin-opener = "2.5.4"
tauri-plugin-fs = "2"
```

Also enabled `protocol-asset` on the tauri crate (auto-fix; see Deviations).

**Capability permissions** (`src-tauri/capabilities/default.json`): added five identifiers after `core:default`:

- `dialog:allow-open` (folder picker)
- `opener:allow-open-path` (open file in default app)
- `opener:allow-reveal-item-in-dir` (Reveal in Finder)
- `fs:allow-exists` (Plan 04 D-19 client-side path validation)
- `fs:allow-stat` (Plan 04 D-19 directory-vs-file check)

**Tauri config** (`src-tauri/tauri.conf.json`):

- Replaced `security.csp: null` with the explicit CSP from RESEARCH §Pattern 2 — `default-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data:; frame-src 'self' asset: http://asset.localhost; object-src 'self' asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'`.
- Added `security.assetProtocol = { enable: true, scope: ["**"] }`.
- Added `bundle.resources = ["models/*"]` so the ML model bundle ships inside the app.

**Plugin registration** (`src-tauri/src/lib.rs` line 29 onwards, BEFORE `.setup`):

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_fs::init())
    .setup(|app| { /* ... */ })
```

**`src-tauri/.gitignore` (new file)** — ignores `target/`, `gen/`, and the large `models/*.onnx` binary (tokenizer.json + config.json + special_tokens_map.json stay tracked because they are small JSON).

### Task 3 — Model assets + frontend deps (commit `22193af`)

**`scripts/download-ner-model.sh`** — idempotent reproducible downloader:

- Downloads `onnx/model_quantized.onnx` from `https://huggingface.co/Xenova/bert-base-NER/resolve/main/` and saves it as `src-tauri/models/bert-base-NER.onnx`.
- Also fetches `tokenizer.json`, `config.json`, and `special_tokens_map.json` (defensive — per RESEARCH Pitfall 8).
- Skips already-present files (idempotent re-runs).
- Prints SHA-256 of the .onnx for integrity verification.
- `chmod +x` set.

**`src-tauri/models/` contents** (verified after running the script):

| File | Size | Tracked |
|------|------|---------|
| `bert-base-NER.onnx` | 108,952,255 B (~104 MB) | gitignored |
| `tokenizer.json` | 668,923 B | tracked |
| `config.json` | 999 B | tracked |
| `special_tokens_map.json` | 125 B | tracked |
| `.gitkeep` | 0 B | tracked |

`config.json` id2label confirmed — 9 NER labels: `O / B-MISC / I-MISC / B-PER / I-PER / B-ORG / I-ORG / B-LOC / I-LOC`.

**Frontend deps** (`package.json` + `pnpm-lock.yaml`):

```json
"dependencies": {
  "@tauri-apps/plugin-dialog": "^2.7.1",
  "@tauri-apps/plugin-fs": "^2.0.0",
  "@tauri-apps/plugin-opener": "^2.5.4",
  "react-markdown": "^10.1.0",
  "remark-gfm": "^4.0.1"
}
```

`pnpm install` exited 0; the lockfile now records the 5 new direct deps (plus their transitive trees).

## Model Integrity Verification

Per the Task 4 checkpoint (gate=blocking-human, Threat ID T-06-MODEL):

**Local SHA-256 of `src-tauri/models/bert-base-NER.onnx`:**

```
caaee70a5518ec7f9e46e5308fcc9263a8c227703a9ce46cf61c69a552349648
```

**File size:** 108,952,255 bytes.

**Status:** User verified the SHA-256 matches the HuggingFace LFS pointer at `https://huggingface.co/Xenova/bert-base-NER/blob/main/onnx/model_quantized.onnx`. T-06-MODEL is mitigated.

## Deferred Smoke Test

The plan's Task 4 also asks for a `cargo tauri dev` smoke (launch window → check no missing-plugin panics, no CSP-violation console messages → close). This was **deferred to be run by the user after merge**, with explicit user approval, because:

- `beforeDevCommand=pnpm dev` spawns vite which resolves paths from the project root; running it inside a separate executor environment risks false positives unrelated to the plan's correctness.
- The error surface the smoke targets (plugin-init panics, malformed CSP) is cleanly observable post-merge via `pnpm tauri dev`.
- `cargo check` (Task 2 verify) already exited 0 — Rust deps compile, plugin registrations are wired before `.setup`, capabilities parse cleanly.

If the user encounters startup errors after merge, the most likely candidates are:

1. ort runtime loading — `load-dynamic` requires the OS to find an ONNX Runtime shared library. If `libonnxruntime.dylib` is not present, switch the Cargo.toml `ort` features to `["download-binaries", "ndarray"]` (the documented fallback in RESEARCH §Environment Availability).
2. CSP false positive — if the dev console shows a Content-Security-Policy violation on a resource we expect to allow, the CSP string in `tauri.conf.json` may need an additional directive (e.g., `connect-src` for `localhost:5173` HMR).

## Accepted Security Tradeoffs

Two threats from the plan's `<threat_model>` are documented here as accepted (per the plan's `mitigate` / `accept` dispositions):

**T-06-AP — Asset protocol scope `["**"]`** (Information Disclosure): The renderer can fetch any local file via `asset://`. **Accepted** because Cortex indexes user-chosen folders; the user has already trusted the app with those paths. Defence-in-depth: react-markdown defaults escape HTML so malicious .md files cannot inject `<img src="asset://...">` requests that load arbitrary local files.

**T-06-OPN — `opener:allow-open-path` universal permission** (Elevation of Privilege): The opener plugin can call shell handlers on any path. **Accepted** because Plan 04 wires `openPath` calls only against `doc.path` values originating in watched-folder ingestion (user-trusted source). The alternative — per-path glob allowlists — is impractical for a tool whose value proposition is "find anything" across user-chosen folders.

The remaining threats from the plan's register are now mitigated:

- **T-06-SC** (Tampering, npm/cargo installs) — Task 1 blocking-human gate (user approved all 11 packages + model source via registry inspection).
- **T-06-MODEL** (Tampering, ONNX model) — Task 4 blocking-human gate (user verified SHA-256 matches HuggingFace LFS pointer).
- **T-06-CSP** (Information Disclosure / XSS) — explicit CSP replaces `csp: null`.
- **T-06-PERM-DIAG** (Spoofing, dialog) — `dialog:allow-open` permission added.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking issue] Added `protocol-asset` feature to the `tauri` crate**

- **Found during:** Task 2, after first `cargo check` run.
- **Issue:** `tauri-build` aborted with `The 'tauri' dependency features on the 'Cargo.toml' file does not match the allowlist defined under 'tauri.conf.json'. Please run 'tauri dev' or 'tauri build' or add the 'protocol-asset' feature.` This happens because enabling `security.assetProtocol.enable=true` in `tauri.conf.json` requires the `tauri` crate to be compiled with the `protocol-asset` feature. The plan's RESEARCH section showed the assetProtocol JSON but did not call out the Cargo.toml side of the feature gate.
- **Fix:** Changed `tauri = { version = "2", features = [] }` → `tauri = { version = "2", features = ["protocol-asset"] }`.
- **Files modified:** `src-tauri/Cargo.toml`
- **Commit:** `5bffbdb`

**2. [Rule 3 — Blocking issue] Created `src-tauri/models/.gitkeep`**

- **Found during:** Task 2, after second `cargo check` run.
- **Issue:** `tauri-build` aborted with `glob pattern models/* path not found or didn't match any files.` Tauri validates `bundle.resources` globs at build time and refuses to compile if a glob has zero matches.
- **Fix:** Created an empty `src-tauri/models/.gitkeep` so the glob matches at least one file. The actual model assets are populated by Task 3's `scripts/download-ner-model.sh`.
- **Files modified:** `src-tauri/models/.gitkeep`
- **Commit:** `5bffbdb`

### Auth Gates

No authentication gates required. HuggingFace public model, public npm registry, public crates.io.

### Checkpoint Outcomes

- **Task 1** (package legitimacy): User approved after reviewing all 11 registry pages + the HuggingFace model page. No package substitutions requested.
- **Task 4** (model SHA + tauri-dev smoke): User approved after verifying SHA-256 match; `tauri dev` smoke deferred to post-merge by mutual agreement.

## Verification Results

End-of-plan checks (all run from project root):

- `cd src-tauri && cargo check` — exit 0 (5 pre-existing warnings in `watcher/worker.rs` and `ruvector-attention`, none from this plan's changes)
- `pnpm install` — exit 0; 5 new direct deps added; lockfile updated; peer warnings about react 19 vs deps wanting react 18 are pre-existing.
- All 4 model files present in `src-tauri/models/` (3 tracked JSON + 1 gitignored .onnx).
- All 5 new permission identifiers present in `capabilities/default.json` after `core:default`.
- `tauri.conf.json` `security.csp` is a non-null string containing `asset: http://asset.localhost`.
- `tauri.conf.json` `security.assetProtocol = { enable: true, scope: ["**"] }`.
- `tauri.conf.json` `bundle.resources` includes `"models/*"`.
- `src-tauri/src/lib.rs` registers `tauri_plugin_dialog::init()`, `tauri_plugin_opener::init()`, `tauri_plugin_fs::init()` at lines 29–31, BEFORE the `.setup(` call at line 32 (line ordering verified).
- `src-tauri/.gitignore` contains `src-tauri/models/*.onnx`.

## Commits

| # | Hash | Subject |
|---|------|---------|
| 1 | `5bffbdb` | feat(06-01): add Rust deps + Tauri plugin registration + CSP + asset protocol |
| 2 | `22193af` | feat(06-01): download NER model bundle + add frontend Tauri plugin deps |
| 3 | (this) | docs(06-01): SUMMARY.md |

## Known Stubs

None. This is a pure-foundation plan; no UI surface or runtime code paths are added that would need stub data.

## Threat Flags

No new security-relevant surface introduced beyond what is in the plan's `<threat_model>`. The bundled NER model + asset protocol + opener plugin are exactly the surface the plan declared; both blocking-human gates verified them.

## Self-Check: PASSED

Files verified to exist on disk:

- `FOUND: scripts/download-ner-model.sh` (executable)
- `FOUND: src-tauri/.gitignore`
- `FOUND: src-tauri/models/bert-base-NER.onnx` (108,952,255 B; gitignored)
- `FOUND: src-tauri/models/tokenizer.json` (668,923 B)
- `FOUND: src-tauri/models/config.json` (999 B)
- `FOUND: src-tauri/models/special_tokens_map.json` (125 B)
- `FOUND: src-tauri/Cargo.toml` (modified — 6 new deps + protocol-asset)
- `FOUND: src-tauri/capabilities/default.json` (modified — 5 new permissions)
- `FOUND: src-tauri/tauri.conf.json` (modified — CSP, assetProtocol, bundle.resources)
- `FOUND: src-tauri/src/lib.rs` (modified — 3 plugin init calls before .setup)
- `FOUND: package.json` (modified — 5 new deps)
- `FOUND: pnpm-lock.yaml` (modified)

Commits verified in git log:

- `FOUND: 5bffbdb` (Task 2)
- `FOUND: 22193af` (Task 3)

# Deferred Items — Phase 11.6

Items discovered during execution that are out of scope for the current plan
and were NOT auto-fixed per the executor's scope-boundary rule.

## 11.6-02 (OntologyStore)

- **Concurrent-agent breakage in `src/search/query.rs` / `src/commands/documents.rs`**
  (observed during Task 1/2 `cargo check` verification, 2026-07-10): another
  agent (Plan 11.8-05, hyperbolic HNSW space scoping) has an in-progress,
  uncommitted edit to `search_documents_impl` in `src/search/query.rs` that
  adds 3 new parameters (`hyp_index`, `hyp_id_to_space`, `space_manager`).
  The call site in `src/commands/documents.rs:84` has not yet been updated to
  match, producing `error[E0061]: this function takes 8 arguments but 5
  arguments were supplied`. This breaks whole-crate `cargo check` / `cargo
  test --lib` (including unrelated modules like `graph::ontology_store`,
  since Rust compiles the whole crate).
  - **Files affected:** `src-tauri/src/search/query.rs` (uncommitted, modified
    by a sibling agent), `src-tauri/src/commands/documents.rs` (not yet
    touched by that sibling agent).
  - **Not fixed here:** Neither file is in this plan's `files_modified` list
    (`src-tauri/src/graph/ontology_store.rs`, `src-tauri/src/graph/mod.rs`).
    Editing `commands/documents.rs` to patch the call site would risk
    conflicting with the other agent's own in-progress commit for 11.8-05.
  - **Verification workaround used:** `cargo test --lib graph::ontology_store
    -- --test-threads=1` was run and passed (20/20) at a point in time before
    this breakage appeared (isolated re-run of just the new module's test
    target). `cargo check` on the whole crate could not be used as the final
    gate because of this unrelated, transient concurrent-edit state.
  - **Expected resolution:** the sibling agent (11.8-05) completes and commits
    its own change to `commands/documents.rs`, restoring a green whole-crate
    build. No action needed from 11.6-02.

## 11.6-03 (Entity normalizer)

- **Cross-plan commit contamination (shared working directory, no worktree
  isolation):** Task 2's `git commit` (hash `df90c0d`) unintentionally
  included two files staged by a concurrently-running sibling agent working
  on Plan 11.7-04 (Chat streaming) — `src-tauri/src/ai/openai.rs` (+58 lines,
  `build_codex_stream_request`) and `src-tauri/src/ai/stream.rs` (+448
  lines, streaming implementation) — plus that agent's own
  `.planning/phases/11.7-rag-chat/deferred-items.md`. This repo is checked
  out as a plain working directory (`.git` is a directory, not a worktree
  file), so `git add <specific-path>` only stages the paths named, but a
  concurrent agent had ALREADY run its own `git add` on those two files
  moments earlier in the shared index; my subsequent `git commit` (which
  commits the full index, not just the paths passed to my `git add`) swept
  them in under the 11.6-03 commit message.
  - **Verified non-destructive:** `cargo test --lib ai::stream` passes (8/8)
    against the state at `df90c0d` — the sibling agent's work was complete
    and self-consistent when captured, not a half-finished edit. No code was
    lost or corrupted.
  - **Not reverted here:** Splitting `df90c0d` after the fact (e.g.
    `git reset --soft` + selective re-commit) risks colliding with the
    11.7-04 agent's own in-flight commit of the same content, potentially
    producing a duplicate/conflicting commit or losing attribution entirely.
    Per the destructive-git-prohibition guidance, no history rewriting was
    attempted.
  - **Expected resolution:** the 11.7-04 agent's own commit (if/when made)
    will either be a no-op (content already present, git recognizes no
    diff) or the orchestrator/user can `git commit --amend` /
    cherry-pick-split `df90c0d` by hand if clean attribution is required.
    Flagging here for phase-level review — no functional action needed.

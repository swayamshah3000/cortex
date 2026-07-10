# Deferred Items — Phase 11.7 (RAG Chat)

## Out-of-scope build instability observed during Plan 11.7-04

During execution of Plan 11.7-04 (streaming AI service), `cargo test --lib`
transiently failed to compile due to unrelated in-progress changes in
`src-tauri/src/search/query.rs` and `src-tauri/src/commands/documents.rs`
(a `search_documents_impl` signature change from a concurrent Plan 11.8-05
session, missing an updated call site). These files are not touched by
Plan 11.7-04 and are out of scope per the SCOPE BOUNDARY rule.

The issue self-resolved (the concurrent session finished updating the call
site) before Plan 11.7-04 completed, and `cargo check --lib` / `cargo test --lib`
both passed cleanly afterward. No action taken — logged for traceability only.

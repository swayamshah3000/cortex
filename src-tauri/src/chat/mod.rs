//! Chat with Your Docs (Phase 11.7).
//!
//! `session_store` owns the JSON-sidecar persistence layer (chat_sessions.json).
//! `engine` orchestrates the RAG pipeline (retrieval, chunking, prompt assembly,
//! streaming). `commands` exposes the IPC surface consumed by the frontend.

pub mod session_store;
pub mod engine;
pub mod commands;

//! Saved Searches feature module.
//!
//! `store` owns the JSON-sidecar persistence layer (`saved_searches.json`).
//! `commands` provides the four IPC commands (Plan 04): save/delete/get/counts.

pub mod store;
pub mod commands;

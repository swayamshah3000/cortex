//! Persistent store for Chat with Your Docs sessions (Phase 11.7).
//!
//! Writes `{app_data_dir}/chat_sessions.json` — mirrors the JSON-sidecar
//! pattern used by `saved_searches/store.rs` for `saved_searches.json`.
//!
//! # Schema (D-12 from 11.7-CONTEXT.md)
//! ```json
//! {
//!   "sessions": [
//!     { "id": "cs-uuid", "title": "When did I buy Unit 204?",
//!       "createdAt": "2026-07-09T10:00:00Z", "updatedAt": "2026-07-09T10:05:00Z",
//!       "messages": [
//!         { "id": "mid-uuid", "role": "user", "content": "...", "citations": null, "createdAt": "..." }
//!       ] }
//!   ]
//! }
//! ```
//!
//! # Thread-safety (T-11.7-05)
//! `load` / `save` are synchronous. Callers in Plan 05 wrap this in
//! `Arc<tokio::sync::Mutex<ChatSessionStore>>` inside `AppState` — same
//! pattern as `SavedSearchStore`. All writes serialize through the mutex.
//!
//! # Error resilience (T-11.7-03)
//! `load` never panics. Any I/O or JSON parse error silently returns the
//! `Default` (empty store). The worst-case outcome is empty chat history,
//! not a crash. Mirror of SavedSearchStore / SpaceLabelCache resilience
//! contract (D-05 / T-11-04 in the earlier phase).

use crate::types::{ChatMessage, ChatSession};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// In-memory mirror of `{app_data_dir}/chat_sessions.json`.
///
/// Arc<tokio::sync::Mutex<ChatSessionStore>> wrapping is done by Plan 05 at
/// the AppState layer — not here.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionStore {
    /// Ordered list of chat sessions. Insertion order is preserved; the
    /// store does NOT enforce most-recently-updated-first ordering — sort
    /// in the IPC layer if needed.
    pub sessions: Vec<ChatSession>,
}

impl ChatSessionStore {
    /// Load the store from `{app_data_dir}/chat_sessions.json`.
    ///
    /// Returns `Default::default()` (empty store) on any I/O or JSON parse
    /// error — never panics (T-11.7-03 mitigation).
    pub fn load(app_data_dir: &Path) -> Self {
        let path = app_data_dir.join("chat_sessions.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist the store to `{app_data_dir}/chat_sessions.json`.
    ///
    /// Creates the file if it does not exist; overwrites if it does.
    ///
    /// # Concurrency note (T-11.7-05)
    /// Wrap in `Arc<Mutex<>>` at the call site (Plan 05 AppState) to prevent
    /// concurrent writes tearing the file.
    pub fn save(&self, app_data_dir: &Path) -> std::io::Result<()> {
        let path = app_data_dir.join("chat_sessions.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// Read accessor — returns the ChatSession with the given `id`, if present.
    pub fn get(&self, id: &str) -> Option<&ChatSession> {
        self.sessions.iter().find(|s| s.id == id)
    }

    /// Returns the full slice of chat sessions (all entries).
    pub fn all(&self) -> &[ChatSession] {
        &self.sessions
    }

    /// Write accessor — inserts `session` if its `id` is absent; replaces
    /// in-place if the `id` already exists (upsert semantics).
    pub fn insert(&mut self, session: ChatSession) {
        if let Some(pos) = self.sessions.iter().position(|s| s.id == session.id) {
            self.sessions[pos] = session;
        } else {
            self.sessions.push(session);
        }
    }

    /// Remove the chat session with the given `id`.
    ///
    /// Returns `true` if a removal happened, `false` if no entry matched.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.sessions.len();
        self.sessions.retain(|s| s.id != id);
        self.sessions.len() < before
    }

    /// Creates a new `ChatSession` with `id`, `title`, `created_at = now_iso`,
    /// `updated_at = now_iso`, and empty `messages`. Inserts it into the
    /// store (upsert) and returns a clone.
    ///
    /// The caller is responsible for generating `id` (e.g.
    /// `format!("cs-{}", Uuid::new_v4())`) and `now_iso` (e.g.
    /// `chrono::Utc::now().to_rfc3339()`) so the store stays pure and
    /// testable.
    pub fn create_session(&mut self, id: String, title: String, now_iso: String) -> ChatSession {
        let session = ChatSession {
            id,
            title,
            created_at: now_iso.clone(),
            updated_at: now_iso,
            messages: Vec::new(),
        };
        self.insert(session.clone());
        session
    }

    /// Appends `msg` into the session identified by `session_id` and updates
    /// its `updated_at` to `now_iso`.
    ///
    /// Returns `true` if the session was found and updated, `false`
    /// otherwise.
    pub fn append_message(&mut self, session_id: &str, msg: ChatMessage, now_iso: String) -> bool {
        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == session_id) {
            session.messages.push(msg);
            session.updated_at = now_iso;
            true
        } else {
            false
        }
    }

    /// Renames the session identified by `id` to `new_title`.
    ///
    /// `new_title` is trimmed of leading/trailing whitespace. If the
    /// trimmed title is empty, returns `Err` without mutating the store.
    /// Otherwise returns `Ok(true)` if the session was found and renamed,
    /// `Ok(false)` if no session matched `id`.
    pub fn rename_session(&mut self, id: &str, new_title: String) -> Result<bool, String> {
        let trimmed = new_title.trim();
        if trimmed.is_empty() {
            return Err("title must not be empty".to_string());
        }
        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == id) {
            session.title = trimmed.to_string();
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChatRole;
    use tempfile::TempDir;

    // Helper: build a ChatSession with the given id/title and no messages.
    fn make_session(id: &str, title: &str) -> ChatSession {
        ChatSession {
            id: id.to_string(),
            title: title.to_string(),
            created_at: "2026-07-09T10:00:00Z".to_string(),
            updated_at: "2026-07-09T10:00:00Z".to_string(),
            messages: vec![],
        }
    }

    // Helper: build a ChatMessage with the given id/role/content, no citations.
    fn make_message(id: &str, role: ChatRole, content: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role,
            content: content.to_string(),
            citations: None,
            created_at: "2026-07-09T10:01:00Z".to_string(),
        }
    }

    /// Test 1: `ChatSessionStore::default()` produces an empty `sessions` vec.
    #[test]
    fn test_default_empty() {
        let store = ChatSessionStore::default();
        assert!(store.sessions.is_empty(), "Default store must be empty");
    }

    /// Test 2: `load()` on a nonexistent path returns empty default without
    /// panicking (T-11.7-03 resilience contract).
    #[test]
    fn test_load_missing_returns_default() {
        let store = ChatSessionStore::load(Path::new("/nonexistent/path/cortex-test-11-7-02"));
        assert!(
            store.sessions.is_empty(),
            "Load from missing path must return empty default, not panic"
        );
    }

    /// Test 3: Round-trip preserves every field (id, title, created_at,
    /// updated_at, and every ChatMessage inside sessions).
    #[test]
    fn test_roundtrip_preserves_all_fields() {
        let dir = TempDir::new().unwrap();

        let mut s1 = make_session("cs-001", "Property Tax Question");
        s1.messages.push(make_message("mid-001", ChatRole::User, "When did I buy Unit 204?"));
        s1.messages.push(ChatMessage {
            id: "mid-002".to_string(),
            role: ChatRole::Assistant,
            content: "You bought it in 2004 per the deed.".to_string(),
            citations: Some(vec![crate::types::Citation {
                index: 1,
                doc_id: "doc-1".to_string(),
                doc_title: "Deed.pdf".to_string(),
                chunk_start: 0,
                chunk_end: 100,
            }]),
            created_at: "2026-07-09T10:02:00Z".to_string(),
        });
        s1.updated_at = "2026-07-09T10:02:00Z".to_string();

        let s2 = make_session("cs-002", "Empty Session");

        let mut store = ChatSessionStore::default();
        store.insert(s1.clone());
        store.insert(s2.clone());

        store.save(dir.path()).unwrap();
        let loaded = ChatSessionStore::load(dir.path());

        // --- s1 field-by-field assertions ---
        let r1 = loaded.get("cs-001").expect("cs-001 must survive round-trip");
        assert_eq!(r1.id, s1.id, "id");
        assert_eq!(r1.title, s1.title, "title");
        assert_eq!(r1.created_at, s1.created_at, "created_at");
        assert_eq!(r1.updated_at, s1.updated_at, "updated_at");
        assert_eq!(r1.messages.len(), s1.messages.len(), "messages.len");
        assert_eq!(r1.messages[0].id, s1.messages[0].id, "messages[0].id");
        assert_eq!(r1.messages[0].role, s1.messages[0].role, "messages[0].role");
        assert_eq!(r1.messages[0].content, s1.messages[0].content, "messages[0].content");
        assert_eq!(r1.messages[0].citations, s1.messages[0].citations, "messages[0].citations");
        assert_eq!(r1.messages[0].created_at, s1.messages[0].created_at, "messages[0].created_at");
        assert_eq!(r1.messages[1].id, s1.messages[1].id, "messages[1].id");
        assert_eq!(r1.messages[1].role, s1.messages[1].role, "messages[1].role");
        assert_eq!(r1.messages[1].content, s1.messages[1].content, "messages[1].content");
        assert_eq!(r1.messages[1].citations, s1.messages[1].citations, "messages[1].citations");
        assert_eq!(r1.messages[1].created_at, s1.messages[1].created_at, "messages[1].created_at");

        // --- s2 field-by-field assertions ---
        let r2 = loaded.get("cs-002").expect("cs-002 must survive round-trip");
        assert_eq!(r2.id, s2.id, "id s2");
        assert_eq!(r2.title, s2.title, "title s2");
        assert_eq!(r2.created_at, s2.created_at, "created_at s2");
        assert_eq!(r2.updated_at, s2.updated_at, "updated_at s2");
        assert!(r2.messages.is_empty(), "messages s2 must be empty");
    }

    /// Test 4: Pre-writing garbage to `chat_sessions.json` and calling
    /// `load()` returns empty default (no panic).
    #[test]
    fn test_malformed_json_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("chat_sessions.json");
        std::fs::write(&path, b"{{{not valid JSON!!!}}}").unwrap();

        let store = ChatSessionStore::load(dir.path());
        assert!(
            store.sessions.is_empty(),
            "Malformed JSON must yield empty default, not panic"
        );
    }

    /// Test 5: `save()` twice with different content — second `load()`
    /// returns only the second content.
    #[test]
    fn test_save_overwrites() {
        let dir = TempDir::new().unwrap();

        let mut store_v1 = ChatSessionStore::default();
        store_v1.insert(make_session("cs-v1", "First Session"));
        store_v1.save(dir.path()).unwrap();

        let mut store_v2 = ChatSessionStore::default();
        store_v2.insert(make_session("cs-v2", "Second Session"));
        store_v2.save(dir.path()).unwrap();

        let loaded = ChatSessionStore::load(dir.path());
        assert_eq!(
            loaded.sessions.len(),
            1,
            "Only the second save's single entry must be present"
        );
        assert_eq!(loaded.sessions[0].id, "cs-v2", "Loaded entry must be from the second save");
        assert!(
            loaded.get("cs-v1").is_none(),
            "First save's entry must not appear after second save overwrites"
        );
    }

    /// Test 6: File is written at `{tmp_dir}/chat_sessions.json` exactly
    /// (not a subdirectory).
    #[test]
    fn test_file_at_correct_path() {
        let dir = TempDir::new().unwrap();
        let mut store = ChatSessionStore::default();
        store.insert(make_session("cs-path-check", "Path Check"));

        store.save(dir.path()).unwrap();

        let expected = dir.path().join("chat_sessions.json");
        assert!(
            expected.exists(),
            "Expected file at {}, but it does not exist",
            expected.display()
        );
    }

    /// Test 7: `create_session(title, id)` returns a new ChatSession with
    /// empty `messages`, correct id/title, and inserts it into the store.
    #[test]
    fn test_create_session() {
        let mut store = ChatSessionStore::default();
        let session = store.create_session(
            "cs-new".to_string(),
            "New Session".to_string(),
            "2026-07-09T11:00:00Z".to_string(),
        );

        assert_eq!(session.id, "cs-new");
        assert_eq!(session.title, "New Session");
        assert_eq!(session.created_at, "2026-07-09T11:00:00Z");
        assert_eq!(session.updated_at, "2026-07-09T11:00:00Z");
        assert!(session.messages.is_empty());

        let stored = store.get("cs-new").expect("session must be inserted into store");
        assert_eq!(stored.id, "cs-new");
        assert_eq!(stored.title, "New Session");
    }

    /// Test 8: `append_message(session_id, msg)` pushes the message into the
    /// correct session's `messages` vec, updates `updated_at`, and returns
    /// `true` when the session exists / `false` when it does not.
    #[test]
    fn test_append_message() {
        let mut store = ChatSessionStore::default();
        store.create_session(
            "cs-append".to_string(),
            "Append Test".to_string(),
            "2026-07-09T11:00:00Z".to_string(),
        );

        let msg = make_message("mid-append-1", ChatRole::User, "Hello?");
        let found = store.append_message("cs-append", msg.clone(), "2026-07-09T11:05:00Z".to_string());
        assert!(found, "append_message must return true when session exists");

        let session = store.get("cs-append").unwrap();
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].id, "mid-append-1");
        assert_eq!(session.updated_at, "2026-07-09T11:05:00Z", "updated_at must be bumped");

        let missing = store.append_message("cs-does-not-exist", msg, "2026-07-09T11:06:00Z".to_string());
        assert!(!missing, "append_message must return false when session absent");
    }

    /// Test 9: `rename_session(id, new_title)` updates the title and returns
    /// `true`/`false`. Title trimming: leading/trailing whitespace
    /// collapsed; empty new_title returns Err.
    #[test]
    fn test_rename_session() {
        let mut store = ChatSessionStore::default();
        store.insert(make_session("cs-rename", "Original Title"));

        let result = store.rename_session("cs-rename", "  Trimmed Title  ".to_string());
        assert_eq!(result, Ok(true), "rename_session must return Ok(true) when found");
        assert_eq!(store.get("cs-rename").unwrap().title, "Trimmed Title", "title must be trimmed");

        let missing = store.rename_session("cs-does-not-exist", "New Title".to_string());
        assert_eq!(missing, Ok(false), "rename_session must return Ok(false) when absent");

        let empty = store.rename_session("cs-rename", "   ".to_string());
        assert!(empty.is_err(), "rename_session must return Err for empty/whitespace-only title");
        assert_eq!(
            store.get("cs-rename").unwrap().title,
            "Trimmed Title",
            "title must remain unchanged after rejected empty rename"
        );
    }

    /// Test 10: `remove(id)` returns `true` when removed, `false` when
    /// absent.
    #[test]
    fn test_remove_returns_bool() {
        let mut store = ChatSessionStore::default();
        store.insert(make_session("cs-rm", "To Remove"));

        assert!(store.remove("cs-rm"), "remove must return true when entry exists");
        assert!(!store.remove("cs-rm"), "remove must return false when entry already gone");
        assert!(store.sessions.is_empty(), "Store must be empty after removal");
    }
}

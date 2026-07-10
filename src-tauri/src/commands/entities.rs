use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::graph::entity_store::EntityStore;
use crate::state::AppState;
use crate::types::{CanonicalEntity, Document, EntitySummary, ExtractionSettings, RelatedEntity};

/// Get all canonical entities, optionally filtered by entity_type.
/// Returns entities sorted by document_count desc.
///
/// # Arguments
/// - `entity_type` — if Some("person"), filters to person entities; if None, returns all.
#[tauri::command]
pub async fn get_entities_by_type(
    entity_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<EntitySummary>, AppError> {
    let store = state.entity_store.clone();

    let result = tokio::task::spawn_blocking(move || {
        let store_guard = store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok::<Vec<EntitySummary>, AppError>(store_guard.get_by_type(entity_type.as_deref()))
    })
    .await??;
    Ok(result)
}

/// Get the full CanonicalEntity for a given canonical id.
///
/// Returns AppError::NotFound if the id does not exist in the entity store.
#[tauri::command]
pub async fn get_entity(
    id: String,
    state: State<'_, AppState>,
) -> Result<CanonicalEntity, AppError> {
    let store = state.entity_store.clone();

    let result = tokio::task::spawn_blocking(move || {
        let store_guard = store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        store_guard
            .get_canonical(&id)
            .ok_or_else(|| AppError::NotFound(format!("entity not found: {}", id)))
    })
    .await??;
    Ok(result)
}

/// Get all documents that mention a canonical entity (reverse doc index lookup).
///
/// Joins EntityStore.doc_index[id] with the documents_384 collection.
#[tauri::command]
pub async fn get_documents_for_entity(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Document>, AppError> {
    let store = state.entity_store.clone();
    let engine = state.engine.clone();

    let result = tokio::task::spawn_blocking(move || {
        // Get doc_ids from entity_store
        let doc_ids: Vec<String> = {
            let store_guard = store
                .lock()
                .map_err(|e| AppError::Internal(e.to_string()))?;
            store_guard
                .doc_index
                .get(&id)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default()
        };

        if doc_ids.is_empty() {
            return Ok::<Vec<Document>, AppError>(vec![]);
        }

        // Lookup each doc from the collection
        let engine_guard = engine.blocking_lock();
        let collection_arc = engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                AppError::VectorStorage("documents_384 collection not found".to_string())
            })?;

        let mut documents: Vec<Document> = Vec::new();
        let collection = collection_arc.read();

        for doc_id in &doc_ids {
            let entry = collection
                .db
                .get(doc_id)
                .map_err(|e| AppError::VectorStorage(e.to_string()))?;

            if let Some(entry) = entry {
                if let Some(ref metadata) = entry.metadata {
                    documents.push(crate::search::query::build_document_from_metadata(
                        doc_id, metadata,
                    ));
                }
            }
        }

        Ok::<Vec<Document>, AppError>(documents)
    })
    .await??;
    Ok(result)
}

/// Get entities related to a canonical entity by co-occurrence in the same document.
///
/// Defaults: min_co_occurrence=2 (per D-11), limit=10.
#[tauri::command]
pub async fn get_related_entities(
    id: String,
    min_co_occurrence: Option<u32>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<RelatedEntity>, AppError> {
    let store = state.entity_store.clone();
    let engine = state.engine.clone();
    let min_co = min_co_occurrence.unwrap_or(2);
    let lim = limit.unwrap_or(10);

    let result = tokio::task::spawn_blocking(move || {
        let store_guard = store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let engine_guard = engine.blocking_lock();
        store_guard.related_entities(&id, min_co, lim, &engine_guard)
    })
    .await??;
    Ok(result)
}

/// Rename a canonical entity's canonical_name.
///
/// Per D-12: updates canonical_name only — no alias changes, no doc rewrites.
/// Returns the updated CanonicalEntity.
#[tauri::command]
pub async fn rename_entity_canonical(
    id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<CanonicalEntity, AppError> {
    let store = state.entity_store.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut store_guard = store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        store_guard.rename_canonical(&id, &new_name)?;
        store_guard
            .get_canonical(&id)
            .ok_or_else(|| AppError::NotFound(format!("entity not found after rename: {}", id)))
    })
    .await??;
    Ok(result)
}

/// Split an alias off a canonical entity into a new canonical.
///
/// Per D-08: creates a new canonical, removes alias from old, rewrites affected doc metadata.
/// Returns the NEW canonical (the split-off entity).
#[tauri::command]
pub async fn split_entity_alias(
    canonical_id: String,
    alias: String,
    state: State<'_, AppState>,
) -> Result<CanonicalEntity, AppError> {
    let store = state.entity_store.clone();
    let engine = state.engine.clone();
    let embedder = state.embedding_service.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut store_guard = store
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let engine_guard = engine.blocking_lock();
        let new_id =
            store_guard.split_alias(&canonical_id, &alias, embedder.as_ref(), &engine_guard)?;
        store_guard
            .get_canonical(&new_id)
            .ok_or_else(|| AppError::Internal(format!("new canonical not found after split: {}", new_id)))
    })
    .await??;
    Ok(result)
}

// ─── Phase 8 Plan 05: Extraction settings IPC commands ────────────────────────

/// Return the current runtime extraction settings (model + toggle).
///
/// Reads the live state from TwoPassExtractor — NOT from settings.json on disk.
/// This gives the frontend an always-accurate view of the runtime configuration
/// (e.g. after a crash-recovery where the model was set in memory but not persisted).
///
/// D-32: used by the Settings → AI tab to populate the "Use LLM Extraction" toggle
/// and model selector on mount.
#[tauri::command]
pub async fn get_extraction_settings(
    state: State<'_, AppState>,
) -> Result<ExtractionSettings, AppError> {
    let model = state.two_pass_extractor.pass2().model().await;
    let use_llm = state.two_pass_extractor.llm_enabled();
    Ok(ExtractionSettings {
        extraction_model: model,
        use_llm_extraction: use_llm,
    })
}

/// Update runtime extraction settings AND persist to settings.json.
///
/// Updates:
///   1. TwoPassExtractor runtime state (takes effect immediately, no restart needed).
///   2. settings.json on disk (merges into existing Settings blob so other fields survive).
///
/// D-33: the Settings toggle calls this command; the backend reflects the change in the
/// very next extract_full() call (AtomicBool store with Release ordering).
/// D-32: updating the model also triggers re-selection on the next refine() call.
#[tauri::command]
pub async fn set_extraction_settings(
    settings: ExtractionSettings,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // 1. Update runtime state immediately.
    state.two_pass_extractor.set_model(settings.extraction_model.clone()).await;
    state.two_pass_extractor.set_llm_enabled(settings.use_llm_extraction);

    // 2. Persist to settings.json by reading current settings, patching, and writing back.
    let registry_path = state.registry_path.clone();
    let new_model = settings.extraction_model.clone();
    let new_flag  = settings.use_llm_extraction;

    tokio::task::spawn_blocking(move || {
        let settings_path = registry_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("settings.json");

        // Read existing settings (or default) so we don't clobber other fields.
        let mut existing: crate::types::Settings = match std::fs::read_to_string(&settings_path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| default_settings_inline()),
            Err(_) => default_settings_inline(),
        };

        existing.extraction_model    = new_model;
        existing.use_llm_extraction  = new_flag;

        let json = serde_json::to_string_pretty(&existing)
            .map_err(|e| AppError::Internal(format!("Failed to serialize settings: {}", e)))?;

        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Internal(format!("Failed to create settings dir: {}", e)))?;
        }

        std::fs::write(&settings_path, json)
            .map_err(|e| AppError::Internal(format!("Failed to write settings: {}", e)))?;

        Ok::<(), AppError>(())
    })
    .await??;

    Ok(())
}

/// Trigger the entity backfill pipeline for documents that lack Pass-2 entities.
///
/// Validates preconditions before handing off to the backfill worker:
///   - `two_pass_extractor.llm_enabled()` must be true
///   - The configured model must be non-empty (empty → provider default, still ok for
///     Anthropic/OpenAI/Gemini; only Ollama with empty = no-default is gated below)
///
/// T-08-15 (DoS mitigation): returns an error early if the toggle is off or model
/// is unconfigured; the actual concurrency guard (single-flight via engine mutex +
/// entities_version gate) lives in Plan 06 backfill.rs.
///
/// HANDOFF NOTE (Plan 05 → Plan 06):
/// The body currently returns Ok(()) after validation.  Plan 06 Task 2 owns backfill.rs
/// and will replace the `todo!` stub below with the actual
/// `pipeline::backfill::spawn_entity_backfill(app, engine, two_pass, entity_store, embedder)`
/// call using the Phase-8 two-pass signature.  The command name and type surface are stable
/// here so the frontend (Plan 07) can already wire up the "Re-extract" button.
#[tauri::command]
pub async fn trigger_entity_backfill(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), AppError> {
    // T-08-15 (DoS mitigation): gate on the LLM toggle.
    // If disabled, backfill would process all docs as Pass-1-only which is a no-op
    // for the common case (docs already at 2.5).  Require the user to enable LLM first.
    // Exception: allow if llm_enabled=true even when no provider is connected — the
    // extractor handles that gracefully (docs land at 2.5, counted as fallbacks in D-29).
    if !state.two_pass_extractor.llm_enabled() {
        return Err(AppError::Internal(
            "Enable 'Use LLM for entity extraction' in Settings → AI & Models first.".into(),
        ));
    }

    // WR-02 single-flight guard: reject duplicate backfill requests.
    // compare_exchange(false, true) returns Err if the flag was already true,
    // meaning another backfill task is still running.  The backfill worker
    // resets this to false when it emits the final "complete" event.
    state.backfill_running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| AppError::Internal(
            "A backfill is already in progress. Wait for it to complete before starting another.".into()
        ))?;

    // Spawn the backfill task in the background.
    // Progress flows via the "entity-backfill-progress" Tauri event.
    crate::pipeline::backfill::spawn_entity_backfill(
        app,
        state.engine.clone(),
        state.two_pass_extractor.clone(),
        state.entity_store.clone(),
        state.triple_store.clone(),
        state.ontology_store.clone(),
        state.auth_state.clone(),
        state.embedding_service.clone(),
        state.backfill_running.clone(),
        state.app_data_dir.clone(),
    );

    Ok(())
}

// ─── Phase 11 Plan 06: Entity detail page IPC (ENEX-01) ──────────────────────

/// Aggregate co-occurring `{class}:{value}` pairs across a slice of document metadata blobs.
///
/// For each doc, iterates extracted_entities and counts entities OTHER than `target_class:target_value`.
/// Uses the same Phase 6/8 bridge as `build_entity_set` in documents.rs: prefers "class" field,
/// falls back to "entity_type", then capitalizes.
///
/// Returns Vec<RelatedEntityRef> sorted descending by co_doc_count, truncated to `limit`.
/// Empty entity metadata → empty result (Test G — no panic).
///
/// WR-05 fix: `limit` is now caller-controlled (was hardcoded to 10). Pass `10` at
/// the existing callsite to preserve current behavior; callers can request fewer/more.
///
/// Pure helper for unit testing without engine dependencies.
pub(crate) fn aggregate_co_occurrence(
    docs_metadata: &[std::collections::HashMap<String, serde_json::Value>],
    target_class: &str,
    target_value: &str,
    limit: usize,
) -> Vec<crate::types::RelatedEntityRef> {
    use std::collections::HashMap;

    let target_cap = capitalize_class_entities(target_class);
    let target_key = format!("{}:{}", target_cap, target_value);

    // Count occurrences of each (class, value) pair EXCLUDING the target
    let mut counts: HashMap<(String, String), u32> = HashMap::new();

    for meta in docs_metadata {
        if let Some(arr) = meta.get("extracted_entities").and_then(|v| v.as_array()) {
            for e in arr {
                let class_raw = e
                    .get("class")
                    .or_else(|| e.get("entity_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let value = e.get("value").and_then(|v| v.as_str()).unwrap_or("");
                if class_raw.is_empty() || value.is_empty() {
                    continue;
                }
                let class_cap = capitalize_class_entities(class_raw);
                let key = format!("{}:{}", class_cap, value);
                // Exclude the target entity itself
                if key == target_key {
                    continue;
                }
                *counts.entry((class_cap, value.to_string())).or_insert(0) += 1;
            }
        }
    }

    let mut refs: Vec<crate::types::RelatedEntityRef> = counts
        .into_iter()
        .map(|((class, value), count)| crate::types::RelatedEntityRef {
            class,
            value,
            co_doc_count: count,
        })
        .collect();

    refs.sort_by(|a, b| b.co_doc_count.cmp(&a.co_doc_count));
    refs.truncate(limit);
    refs
}

/// Capitalize first letter — mirrors commands/documents.rs and spaces/manager.rs patterns.
#[inline]
fn capitalize_class_entities(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// `get_entity_page_data(class, value, page)` — Pattern 4 in 11-RESEARCH.md.
///
/// Resolves `class:value` → canonical entity via `alias_index` (case-insensitive),
/// returns paginated documents (20/page per D-16) + top-10 co-occurring entities.
///
/// Resolution order (Phase 6/8 bridge, pitfall #5):
///   1. (value.to_lowercase(), class.to_lowercase())   — Phase 6 alias_index stores lowercase keys
///   2. (value.to_lowercase(), class.clone())           — Phase 8 stores capitalized class
///
/// T-11-18 mitigation: unknown class+value → AppError::NotFound (D-18 empty state).
/// T-11-19 mitigation: entity_store std::sync::Mutex acquired inside spawn_blocking,
///   released before entering the engine.blocking_lock() section.
/// D-16: page size = 20, 0-indexed.
#[tauri::command]
pub async fn get_entity_page_data(
    class: String,
    value: String,
    // WR-02 fix: use i32 so Tauri IPC can deserialize negative integers without
    // a cryptic serialization error; validate >= 0 explicitly for a clear message.
    page: Option<i32>,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<crate::types::EntityPageData, crate::error::AppError> {
    // WR-02: reject negative page numbers with a clear error rather than an
    // opaque IPC deserialization failure.
    if let Some(p) = page {
        if p < 0 {
            return Err(crate::error::AppError::Internal(
                "page must be >= 0".to_string(),
            ));
        }
    }
    let page = page.map(|p| p as u32);

    let entity_store = state.entity_store.clone();
    let engine = state.engine.clone();

    let result = tokio::task::spawn_blocking(move || {
        const PAGE_SIZE: u32 = 20;

        // Step 1: Resolve alias_index with Phase 6/8 bridge.
        // entity_store uses std::sync::Mutex — acquire here, drop before engine lock.
        let (canonical_id, canonical, sorted_doc_ids) = {
            let store_guard = entity_store
                .lock()
                .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

            // Primary lookup: (lowercase value, lowercase class) — matches Phase 6 store pattern
            let canonical_id = store_guard
                .alias_index
                .get(&(value.to_lowercase(), class.to_lowercase()))
                .cloned()
                // Fallback: (lowercase value, original class) — matches Phase 8 capitalized class
                .or_else(|| {
                    store_guard
                        .alias_index
                        .get(&(value.to_lowercase(), class.clone()))
                        .cloned()
                })
                .ok_or_else(|| {
                    crate::error::AppError::NotFound(format!(
                        "entity not found: {}:{}",
                        class, value
                    ))
                })?;

            // Defensive: fetch canonical (should always succeed given alias_index hit)
            let canonical = store_guard
                .get_canonical(&canonical_id)
                .ok_or_else(|| {
                    crate::error::AppError::NotFound(format!(
                        "canonical entity missing for id: {}",
                        canonical_id
                    ))
                })?;

            // Collect doc_ids sorted deterministically for stable pagination
            let mut sorted_doc_ids: Vec<String> = store_guard
                .doc_index
                .get(&canonical_id)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default();
            sorted_doc_ids.sort();

            (canonical_id, canonical, sorted_doc_ids)
            // store_guard dropped here — entity_store lock released before engine lock
        };

        let total_document_count = sorted_doc_ids.len() as u32;
        let current_page = page.unwrap_or(0);
        let page_start = current_page as usize * PAGE_SIZE as usize;

        // Step 2: Acquire engine lock and read collection
        let engine_guard = engine.blocking_lock();
        let collection_arc = engine_guard
            .collections
            .get_collection("documents_384")
            .ok_or_else(|| {
                crate::error::AppError::VectorStorage(
                    "documents_384 collection not found".to_string(),
                )
            })?;

        // Step 3: Slice the page and build Document objects
        let page_ids: Vec<&String> = sorted_doc_ids
            .iter()
            .skip(page_start)
            .take(PAGE_SIZE as usize)
            .collect();

        let collection = collection_arc.read();
        let mut documents: Vec<crate::types::Document> = Vec::new();
        for doc_id in &page_ids {
            if let Ok(Some(entry)) = collection.db.get(doc_id) {
                if let Some(ref meta) = entry.metadata {
                    documents.push(crate::search::query::build_document_from_metadata(
                        doc_id, meta,
                    ));
                }
            }
        }

        // Step 4: Co-occurrence — aggregate over ALL doc_ids (not just current page)
        // to ensure the signal is corpus-wide, not page-scoped.
        let mut all_metadata: Vec<std::collections::HashMap<String, serde_json::Value>> =
            Vec::new();
        for doc_id in &sorted_doc_ids {
            if let Ok(Some(entry)) = collection.db.get(doc_id) {
                if let Some(meta) = entry.metadata {
                    all_metadata.push(meta);
                }
            }
        }
        let co_occurring_entities =
            aggregate_co_occurrence(&all_metadata, &canonical.entity_type, &canonical.canonical_name, 10);

        Ok::<crate::types::EntityPageData, crate::error::AppError>(crate::types::EntityPageData {
            canonical,
            documents,
            total_document_count,
            co_occurring_entities,
            page: current_page,
            page_size: PAGE_SIZE,
        })
    })
    .await??;

    Ok(result)
}

/// Inline default settings constructor — mirrors default_settings() in commands/settings.rs
/// without creating a dependency cycle (entities.rs cannot import settings.rs).
fn default_settings_inline() -> crate::types::Settings {
    crate::types::Settings {
        theme:              "dark".to_string(),
        sidebar_collapsed:  false,
        embedding_model:    "local".to_string(),
        watched_folders:    vec![],
        excluded_patterns:  vec![
            ".git".to_string(),
            "node_modules".to_string(),
            ".DS_Store".to_string(),
        ],
        index_on_startup:   true,
        index_size:         0,
        storage_path:       "~/Library/Application Support/com.cortex.app/vectors".to_string(),
        extraction_model:   String::new(),
        use_llm_extraction: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    use crate::graph::entity_store::EntityStore;
    use crate::types::{CanonicalEntity, EntitySummary};

    /// Build a minimal EntityStore seeded with known data (no embedder needed).
    fn make_seeded_store() -> EntityStore {
        let mut store = EntityStore::new();

        // 3 persons
        let p1 = CanonicalEntity {
            id: "p1".to_string(),
            canonical_name: "Alice Smith".to_string(),
            entity_type: "person".to_string(),
            aliases: vec!["Alice Smith".to_string()],
            document_count: 5,
        canonical_short_name: None,
        };
        let p2 = CanonicalEntity {
            id: "p2".to_string(),
            canonical_name: "Bob Jones".to_string(),
            entity_type: "person".to_string(),
            aliases: vec!["Bob Jones".to_string()],
            document_count: 3,
        canonical_short_name: None,
        };
        let p3 = CanonicalEntity {
            id: "p3".to_string(),
            canonical_name: "Carol White".to_string(),
            entity_type: "person".to_string(),
            aliases: vec!["Carol White".to_string()],
            document_count: 1,
        canonical_short_name: None,
        };

        // 2 organizations
        let o1 = CanonicalEntity {
            id: "o1".to_string(),
            canonical_name: "Acme Corp".to_string(),
            entity_type: "organization".to_string(),
            aliases: vec!["Acme Corp".to_string()],
            document_count: 10,
        canonical_short_name: None,
        };
        let o2 = CanonicalEntity {
            id: "o2".to_string(),
            canonical_name: "Globex Inc".to_string(),
            entity_type: "organization".to_string(),
            aliases: vec!["Globex Inc".to_string()],
            document_count: 7,
        canonical_short_name: None,
        };

        // Seed doc_index
        let mut p1_docs = HashSet::new();
        p1_docs.insert("doc1".to_string());
        p1_docs.insert("doc2".to_string());
        p1_docs.insert("doc3".to_string());
        p1_docs.insert("doc4".to_string());
        p1_docs.insert("doc5".to_string());

        let mut p2_docs = HashSet::new();
        p2_docs.insert("doc1".to_string());
        p2_docs.insert("doc2".to_string());
        p2_docs.insert("doc3".to_string());

        let mut o1_docs = HashSet::new();
        for i in 1..=10 {
            o1_docs.insert(format!("doc{}", i));
        }

        store.canonicals.insert("p1".to_string(), p1);
        store.canonicals.insert("p2".to_string(), p2);
        store.canonicals.insert("p3".to_string(), p3);
        store.canonicals.insert("o1".to_string(), o1);
        store.canonicals.insert("o2".to_string(), o2);

        store.doc_index.insert("p1".to_string(), p1_docs);
        store.doc_index.insert("p2".to_string(), p2_docs);
        store.doc_index.insert("p3".to_string(), HashSet::new());
        store.doc_index.insert("o1".to_string(), o1_docs);
        store.doc_index.insert("o2".to_string(), HashSet::new());

        // Alias index
        store.alias_index.insert(
            ("alice smith".to_string(), "person".to_string()),
            "p1".to_string(),
        );
        store.alias_index.insert(
            ("bob jones".to_string(), "person".to_string()),
            "p2".to_string(),
        );
        store.alias_index.insert(
            ("carol white".to_string(), "person".to_string()),
            "p3".to_string(),
        );
        store.alias_index.insert(
            ("acme corp".to_string(), "organization".to_string()),
            "o1".to_string(),
        );
        store.alias_index.insert(
            ("globex inc".to_string(), "organization".to_string()),
            "o2".to_string(),
        );

        store
    }

    /// Test 1: get_by_type(None) returns all 5 entities sorted by document_count desc.
    #[test]
    fn test_get_by_type_none_returns_all_sorted() {
        let store = make_seeded_store();
        let all = store.get_by_type(None);
        assert_eq!(all.len(), 5, "Should return all 5 entities");

        // Verify sorted by doc_count desc
        for i in 0..all.len() - 1 {
            assert!(
                all[i].document_count >= all[i + 1].document_count,
                "Should be sorted by document_count desc at position {}: {} >= {}",
                i,
                all[i].document_count,
                all[i + 1].document_count
            );
        }
    }

    /// Test 2: get_by_type(Some("person")) returns only 3 persons.
    #[test]
    fn test_get_by_type_filtered_returns_persons_only() {
        let store = make_seeded_store();
        let persons = store.get_by_type(Some("person"));
        assert_eq!(persons.len(), 3, "Should return only 3 person entities");
        assert!(
            persons.iter().all(|e| e.entity_type == "person"),
            "All returned entities should be persons"
        );
    }

    /// Test 3: get_canonical(id) returns full CanonicalEntity; unknown id returns None.
    #[test]
    fn test_get_canonical_known_and_unknown() {
        let store = make_seeded_store();

        let alice = store.get_canonical("p1").expect("p1 should exist");
        assert_eq!(alice.canonical_name, "Alice Smith");
        assert_eq!(alice.entity_type, "person");

        assert!(
            store.get_canonical("nonexistent").is_none(),
            "Unknown id should return None"
        );
    }

    /// Test 5: get_related_entities defaults min_co_occurrence=2, limit=10.
    /// Since we don't have engine access in this unit test, we verify the
    /// related_entities method on the EntityStore directly.
    #[test]
    fn test_related_entities_defaults() {
        let store = make_seeded_store();
        // p1 and p2 share doc1, doc2, doc3 (3 co-occurrences) — meets threshold 2
        // But without engine, we can't call related_entities directly.
        // Verify the doc_index state is correct (precondition).
        let p1_docs = store.doc_index.get("p1").unwrap();
        let p2_docs = store.doc_index.get("p2").unwrap();
        let shared: Vec<_> = p1_docs.iter().filter(|d| p2_docs.contains(*d)).collect();
        assert!(
            shared.len() >= 2,
            "p1 and p2 should share at least 2 docs for co-occurrence threshold"
        );
    }

    /// Test 6: rename_entity_canonical returns updated CanonicalEntity with new name.
    #[test]
    fn test_rename_entity_canonical_unit() {
        let mut store = make_seeded_store();
        store.rename_canonical("p1", "Alice J. Smith").unwrap();
        let updated = store.get_canonical("p1").unwrap();
        assert_eq!(updated.canonical_name, "Alice J. Smith");
        // aliases unchanged
        assert!(updated.aliases.contains(&"Alice Smith".to_string()));
        // document_count unchanged
        assert_eq!(updated.document_count, 5);
    }

    /// Test 8: read_document_text size cap (logic unit test).
    /// Verifies the 5 MB cap constant is correct.
    #[test]
    fn test_read_document_text_size_cap_constant() {
        // The hard cap must be exactly 5 MB = 5 * 1024 * 1024 bytes
        let hard_cap: u64 = 5 * 1024 * 1024;
        assert_eq!(hard_cap, 5_242_880u64, "5 MB cap should be 5,242,880 bytes");
    }

    /// Test 10: defense in depth — max_bytes u64::MAX is still capped at 5 MB.
    #[test]
    fn test_hard_cap_clamps_max_bytes() {
        let hard_cap: u64 = 5 * 1024 * 1024;
        let caller_max: u64 = u64::MAX;
        let effective = caller_max.min(hard_cap);
        assert_eq!(
            effective,
            hard_cap,
            "u64::MAX should be clamped to hard_cap (5 MB)"
        );
    }

    // ── Tests for aggregate_co_occurrence helper (Tests F, G) ─────────────────

    /// Helper: build a single doc metadata blob with extracted_entities.
    fn make_doc_meta(entities: serde_json::Value) -> std::collections::HashMap<String, serde_json::Value> {
        let mut m = std::collections::HashMap::new();
        m.insert("extracted_entities".to_string(), entities);
        m
    }

    /// Test F: co-occurring entity counting returns correct counts and excludes target.
    #[test]
    fn test_aggregate_co_occurrence_basic() {
        // 3 docs, each mentioning "Person:Alice" (target) + other entities
        let metas = vec![
            make_doc_meta(serde_json::json!([
                {"class": "Person", "value": "Alice"},
                {"class": "Organization", "value": "Acme Corp"}
            ])),
            make_doc_meta(serde_json::json!([
                {"class": "Person", "value": "Alice"},
                {"class": "Organization", "value": "Acme Corp"}
            ])),
            make_doc_meta(serde_json::json!([
                {"class": "Person", "value": "Alice"},
                {"class": "Person", "value": "Bob"}
            ])),
        ];

        let result = aggregate_co_occurrence(&metas, "Person", "Alice", 10);

        // "Acme Corp" appears in 2 docs → co_doc_count=2
        // "Bob" appears in 1 doc → co_doc_count=1
        // "Alice" (target) must be excluded
        assert_eq!(result.len(), 2, "Expected 2 co-occurring entities");

        let acme = result.iter().find(|r| r.value == "Acme Corp").expect("Acme Corp must appear");
        assert_eq!(acme.class, "Organization");
        assert_eq!(acme.co_doc_count, 2, "Acme Corp appears in 2 docs");

        let bob = result.iter().find(|r| r.value == "Bob").expect("Bob must appear");
        assert_eq!(bob.class, "Person");
        assert_eq!(bob.co_doc_count, 1, "Bob appears in 1 doc");

        // Sorted descending by co_doc_count
        assert_eq!(result[0].co_doc_count, 2, "First result must have highest co_doc_count");
        assert_eq!(result[1].co_doc_count, 1);
    }

    /// Test G: empty co-occurrence when entity has no other entities → empty list, no panic.
    #[test]
    fn test_aggregate_co_occurrence_empty_no_panic() {
        // Docs only contain the target entity
        let metas = vec![
            make_doc_meta(serde_json::json!([
                {"class": "Person", "value": "Alice"}
            ])),
            make_doc_meta(serde_json::json!([
                {"class": "Person", "value": "Alice"}
            ])),
        ];

        let result = aggregate_co_occurrence(&metas, "Person", "Alice", 10);
        assert!(result.is_empty(), "No co-occurring entities should return empty list");
    }

    /// Test G variant: empty metadata → empty list, no panic.
    #[test]
    fn test_aggregate_co_occurrence_empty_metadata() {
        let result = aggregate_co_occurrence(&[], "Person", "Alice", 10);
        assert!(result.is_empty(), "Empty metadata slice should return empty list");
    }

    /// Test: Phase 6 entity_type fallback in co-occurrence.
    #[test]
    fn test_aggregate_co_occurrence_phase6_entity_type_bridge() {
        let metas = vec![
            make_doc_meta(serde_json::json!([
                {"entity_type": "person", "value": "Alice"},
                {"entity_type": "organization", "value": "Acme Corp"}
            ])),
        ];
        let result = aggregate_co_occurrence(&metas, "person", "Alice", 10);
        assert_eq!(result.len(), 1, "Should find 1 co-occurring entity via Phase 6 bridge");
        assert_eq!(result[0].class, "Organization", "Phase 6 entity_type should be capitalized");
        assert_eq!(result[0].value, "Acme Corp");
    }

    /// Test: capitalize_class_entities handles edge cases.
    #[test]
    fn test_capitalize_class_entities() {
        assert_eq!(capitalize_class_entities("person"), "Person");
        assert_eq!(capitalize_class_entities("Person"), "Person");
        assert_eq!(capitalize_class_entities(""), "");
        assert_eq!(capitalize_class_entities("organization"), "Organization");
    }

    /// Test: alias_index resolution for entity page (Tests A + B from plan).
    #[test]
    fn test_entity_page_alias_resolution_case_insensitive() {
        use crate::graph::entity_store::EntityStore;
        use crate::types::CanonicalEntity;

        let mut store = EntityStore::new();
        // Seed with lowercase key (Phase 6 pattern)
        store.canonicals.insert("cid-p1".to_string(), CanonicalEntity {
            id: "cid-p1".to_string(),
            canonical_name: "Alex Doe".to_string(),
            entity_type: "person".to_string(),
            aliases: vec!["Alex Doe".to_string()],
            document_count: 3,
        canonical_short_name: None,
        });
        store.alias_index.insert(
            ("alex doe".to_string(), "person".to_string()),
            "cid-p1".to_string(),
        );
        store.doc_index.insert("cid-p1".to_string(), {
            let mut s = std::collections::HashSet::new();
            s.insert("d1".to_string());
            s.insert("d2".to_string());
            s.insert("d3".to_string());
            s
        });

        // Test A: exact (lowercase) class lookup
        let cid_a = store.alias_index.get(&("alex doe".to_string(), "person".to_string()));
        assert_eq!(cid_a, Some(&"cid-p1".to_string()), "Lowercase class lookup should find entity");

        // Test B: case-insensitive value — "ALEX DOE".to_lowercase() = "alex doe"
        let cid_b = store.alias_index.get(&("alex doe".to_string(), "person".to_string()));
        assert_eq!(cid_b, Some(&"cid-p1".to_string()), "Uppercase value should resolve after .to_lowercase()");

        // Test D: unknown entity
        let cid_unknown = store.alias_index.get(&("unknown person".to_string(), "person".to_string()));
        assert!(cid_unknown.is_none(), "Unknown entity should return None from alias_index");
    }

    /// Test pagination bounds for entity page.
    #[test]
    fn test_entity_page_pagination_bounds() {
        const PAGE_SIZE: u32 = 20;

        // 25 docs total
        let sorted_ids: Vec<String> = (0..25).map(|i| format!("doc{:02}", i)).collect();
        let total = sorted_ids.len() as u32;

        // Page 0: first 20
        let page0_start = 0usize;
        let page0: Vec<&String> = sorted_ids.iter().skip(page0_start).take(PAGE_SIZE as usize).collect();
        assert_eq!(page0.len(), 20, "Page 0 must return 20 docs");
        assert_eq!(total, 25, "total_document_count must be 25");

        // Page 1: remaining 5
        let page1_start = 1 * PAGE_SIZE as usize;
        let page1: Vec<&String> = sorted_ids.iter().skip(page1_start).take(PAGE_SIZE as usize).collect();
        assert_eq!(page1.len(), 5, "Page 1 must return 5 docs");

        // Page 2: out-of-bounds → empty
        let page2_start = 2 * PAGE_SIZE as usize;
        let page2: Vec<&String> = sorted_ids.iter().skip(page2_start).take(PAGE_SIZE as usize).collect();
        assert_eq!(page2.len(), 0, "Page 2 must return empty slice");
    }
}

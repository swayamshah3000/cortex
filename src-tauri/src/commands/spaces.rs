use std::collections::HashMap;
use tauri::State;
use crate::error::AppError;
use crate::search::query::build_document_from_metadata;
use crate::spaces::label_cache::SpaceLabelEntry;
use crate::state::AppState;
use crate::types::*;

#[tauri::command]
pub async fn get_spaces(
    state: State<'_, AppState>,
) -> Result<Vec<Space>, AppError> {
    let space_mgr = state.space_manager.clone();
    let guard = space_mgr.lock().await;
    Ok(guard.get_spaces())
}

#[tauri::command]
pub async fn get_space_documents(
    space_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Document>, AppError> {
    let space_mgr = state.space_manager.clone();
    let engine = state.engine.clone();

    let doc_ids = {
        let space_guard = space_mgr.lock().await;
        space_guard.get_space_documents(&space_id)
    };

    let engine_guard = engine.lock().await;
    let collection_arc = engine_guard
        .collections
        .get_collection("documents_384")
        .ok_or_else(|| {
            AppError::VectorStorage("documents_384 collection not found".to_string())
        })?;
    let collection = collection_arc.read();

    let mut documents: Vec<Document> = Vec::new();
    for id in doc_ids {
        let entry = collection
            .db
            .get(&id)
            .map_err(|e| AppError::VectorStorage(e.to_string()))?;
        if let Some(entry) = entry {
            if let Some(ref metadata) = entry.metadata {
                documents.push(build_document_from_metadata(&id, metadata));
            }
        }
    }

    Ok(documents)
}

#[tauri::command]
pub async fn move_document_to_space(
    doc_id: String,
    space_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let space_mgr = state.space_manager.clone();
    let activity_log = state.activity_log.clone();

    let (space_name, _) = {
        let mut guard = space_mgr.lock().await;

        let name = guard
            .get_space_data(&space_id)
            .map(|sd| sd.space.name.clone())
            .unwrap_or_else(|| space_id.clone());

        guard.move_document(&doc_id, &space_id)?;
        (name, ())
    };

    // Record activity (std::sync::Mutex — brief lock, no await)
    if let Ok(mut log) = activity_log.lock() {
        log.record("moved", &format!("{} -> {}", doc_id, space_name));
    }

    Ok(())
}

#[tauri::command]
pub async fn recluster_spaces(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    auth: State<'_, crate::auth::AuthState>,
) -> Result<Vec<Space>, AppError> {
    let engine = state.engine.clone();
    let space_mgr = state.space_manager.clone();
    let doc_graph = state.doc_graph.clone();
    let cache_arc = state.space_label_cache.clone();
    let app_data_dir = state.app_data_dir.clone();
    // Read extraction model from settings.json (same pattern as Phase 8)
    let settings_path = state.app_data_dir.join("settings.json");
    let model: String = std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str::<crate::types::Settings>(&s).ok())
        .map(|s| s.extraction_model)
        .unwrap_or_default();
    // AuthState is Clone; shallow clone for the async recluster call
    let auth_clone = (*auth.inner()).clone();

    // Phase 9: recluster is now async (LlmSpaceLabeler calls ai_request).
    let engine_guard = engine.lock().await;
    let mut cache_guard = cache_arc.lock().await;
    let mut space_guard = space_mgr.lock().await;

    let spaces = space_guard
        .recluster(
            &engine_guard,
            &auth_clone,
            &model,
            &app_handle,
            &mut cache_guard,
            &app_data_dir,
        )
        .await?;

    // Rebuild document graph after recluster (std::sync::Mutex — held in an
    // inner scope so the guard drops before we await hyperbolic-index rebuild).
    let top_level_spaces = {
        let mut graph_guard = doc_graph
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        graph_guard.build_edges(&engine_guard, &space_guard)?;
        space_guard.get_top_level_space_data()
    };

    // Phase 10: rebuild hyperbolic secondary index over top-level Space centroids.
    // graph_guard is dropped above so the future stays Send across this await.
    crate::spaces::hyp_index::rebuild_hyp_index(
        &top_level_spaces,
        &state.hyp_index,
        &state.hyp_id_to_space,
    )
    .await;

    Ok(spaces)
}

/// Phase 9: Return the full space label cache as a HashMap keyed by space_id.
/// Frontend hook `useSpaceLabels()` consumes this.
#[tauri::command]
pub async fn get_space_labels(
    state: State<'_, AppState>,
) -> Result<HashMap<String, SpaceLabelEntry>, AppError> {
    let cache_arc = state.space_label_cache.clone();
    let guard = cache_arc.lock().await;
    Ok(guard.labels.clone())
}

/// Phase 9: Manually rename a Space label. Sets `user_locked = true` so
/// subsequent LLM re-labels skip this space. Persists cache and mirrors
/// the change into the in-memory SpaceManager so `get_spaces` reflects the
/// new name immediately (without waiting for the next recluster).
#[tauri::command]
pub async fn rename_space_label(
    space_id: String,
    label: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<SpaceLabelEntry, AppError> {
    let cache_arc = state.space_label_cache.clone();
    let space_arc = state.space_manager.clone();
    let app_data_dir = state.app_data_dir.clone();

    let entry = {
        let mut guard = cache_arc.lock().await;
        let existing = guard.labels.get(&space_id).cloned();
        let mut entry = existing.unwrap_or(SpaceLabelEntry {
            fingerprint: String::new(),
            label: String::new(),
            description: String::new(),
            canonical_entity_hint: None,
            generated_at: String::new(),
            user_locked: false,
            parent_id: None,
            depth: 0,
        });
        entry.label = label.clone();
        if let Some(d) = description.clone() {
            entry.description = d;
        }
        entry.user_locked = true;
        guard.labels.insert(space_id.clone(), entry.clone());
        guard
            .save(&app_data_dir)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        entry
    }; // cache lock released before acquiring space_manager lock

    // Mirror to in-memory SpaceManager so get_spaces reflects new label immediately.
    let mut space_guard = space_arc.lock().await;
    space_guard.update_space_label(
        &space_id,
        label,
        description,
        true, // user_locked = true
    );

    Ok(entry)
}

/// Phase 9: Clear the manual user-lock on a Space so LLM re-labeling
/// resumes on next recluster. Mirrors the change into the in-memory
/// SpaceManager so `get_spaces` reflects `user_locked = false` immediately
/// (without waiting for the next recluster).
#[tauri::command]
pub async fn clear_space_override(
    space_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let cache_arc = state.space_label_cache.clone();
    let space_arc = state.space_manager.clone();
    let app_data_dir = state.app_data_dir.clone();

    {
        let mut cache_guard = cache_arc.lock().await;
        if let Some(entry) = cache_guard.labels.get_mut(&space_id) {
            entry.user_locked = false;
            cache_guard
                .save(&app_data_dir)
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
    } // cache lock released before acquiring space_manager lock

    // Mirror to in-memory SpaceManager so get_spaces returns current state.
    // Read the current name and description first (cloning avoids borrow conflict),
    // then call update_space_label with the same name but user_locked = false.
    let mut space_guard = space_arc.lock().await;
    let current = space_guard
        .get_space_data(&space_id)
        .map(|sd| (sd.space.name.clone(), sd.space.description.clone()));
    if let Some((current_name, current_desc)) = current {
        space_guard.update_space_label(
            &space_id,
            current_name,
            current_desc,
            false, // user_locked = false
        );
    }

    Ok(())
}

/// Phase 9: Force-relabel a single Space via LLM, bypassing the fingerprint
/// cache-hit path. Removes the cache entry and calls recluster (which re-labels
/// all changed/missing spaces). Only affects the target space label because
/// other spaces retain their cache entries.
#[tauri::command]
pub async fn trigger_relabel(
    space_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    auth: State<'_, crate::auth::AuthState>,
) -> Result<(), AppError> {
    // Drop the cache entry so recluster treats this space as "changed"
    {
        let cache_arc = state.space_label_cache.clone();
        let app_data_dir = state.app_data_dir.clone();
        let mut guard = cache_arc.lock().await;
        guard.remove(&space_id);
        guard
            .save(&app_data_dir)
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }
    // Trigger a full recluster (labels the removed one)
    let _ = recluster_spaces(app_handle, state, auth).await?;
    Ok(())
}

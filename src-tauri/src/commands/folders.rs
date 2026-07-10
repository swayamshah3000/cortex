use tauri::State;
use tauri::Emitter;
use crate::error::AppError;
use crate::state::{AppState, WatcherCommand};
use crate::types::*;
use crate::watcher::worker::IndexProgress;

#[tauri::command]
pub async fn add_watched_folder(
    path: String,
    state: State<'_, AppState>,
) -> Result<WatchedFolder, AppError> {
    let config = {
        let mut registry = state.registry.lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let config = registry.add_folder(path.clone());
        registry.save(&state.registry_path)?;
        config
    };

    // Notify watcher task to start watching
    state.watcher_tx.send(WatcherCommand::AddFolder {
        path,
        folder_id: config.id.clone(),
    }).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(WatchedFolder {
        id: config.id,
        path: config.path,
        document_count: config.document_count,
        last_scan: config.last_scan.unwrap_or_else(|| "never".to_string()),
        status: if config.is_paused { "paused".to_string() } else { "watching".to_string() },
    })
}

#[tauri::command]
pub async fn remove_watched_folder(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let path = {
        let mut registry = state.registry.lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let folder_path = registry.folders.get(&id)
            .map(|c| c.path.clone())
            .ok_or_else(|| AppError::NotFound(format!("Folder {id} not found")))?;
        registry.remove_folder(&id);
        registry.save(&state.registry_path)?;
        folder_path
    };

    // Notify watcher task to stop watching
    state.watcher_tx.send(WatcherCommand::RemoveFolder {
        folder_id: id,
        path,
    }).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(())
}

#[tauri::command]
pub async fn trigger_scan(
    folder_id: String,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ScanProgress, AppError> {
    let folder_config = {
        let registry = state.registry.lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        registry.folders.get(&folder_id).cloned()
            .ok_or_else(|| AppError::NotFound(format!("Folder {folder_id} not found")))?
    };

    let engine = state.engine.clone();
    let embedding_service = state.embedding_service.clone();
    let two_pass = state.two_pass_extractor.clone();
    let indexer = state.indexer.clone();
    let registry = state.registry.clone();
    let state_registry_path = state.registry_path.clone();
    let activity_log = state.activity_log.clone();
    let entity_store = state.entity_store.clone();
    let triple_store = state.triple_store.clone();
    let ontology_store = state.ontology_store.clone();
    let auth_state = state.auth_state.clone();
    let app_data_dir = state.app_data_dir.clone();
    let backfill_running = state.backfill_running.clone();
    let fid = folder_id.clone();

    // Spawn background scan task — returns immediately, progress via events
    tauri::async_runtime::spawn(async move {
        let folder_path = std::path::Path::new(&folder_config.path);
        if let Ok(entries) = walk_dir_recursive(folder_path) {
            for file_path in entries {
                let ext = file_path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");

                // Check type enabled and exclusions
                let (excluded, type_ok) = {
                    let reg = registry.lock().unwrap();
                    (reg.is_excluded(&fid, &file_path), reg.is_type_enabled(&fid, ext))
                };
                if excluded || !type_ok {
                    continue;
                }

                let path_str = file_path.to_string_lossy().to_string();
                let _ = app_handle.emit("index-progress", IndexProgress {
                    file_path: path_str.clone(),
                    status: "indexing".to_string(),
                    doc_id: None,
                    error: None,
                    folder_id: Some(fid.clone()),
                });

                let eng = engine.clone();
                let emb = embedding_service.clone();
                let tp = two_pass.clone();
                let idx = indexer.clone();
                let fp = file_path.clone();
                let es = entity_store.clone();
                let embedder = emb.clone();

                let result = tokio::task::spawn_blocking(move || {
                    let engine_guard = eng.blocking_lock();
                    idx.index_file(&fp, &engine_guard, &emb, &tp, es, embedder)
                }).await;

                match result {
                    Ok(Ok(doc_id)) => {
                        let _ = app_handle.emit("index-progress", IndexProgress {
                            file_path: path_str.clone(),
                            status: "indexed".to_string(),
                            doc_id: Some(doc_id),
                            error: None,
                            folder_id: Some(fid.clone()),
                        });
                        // Record activity
                        if let Ok(mut log) = activity_log.lock() {
                            let file_name = std::path::Path::new(&path_str)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&path_str);
                            log.record("indexed", file_name);
                        }
                        // Bump the folder's document_count so the Watched Folders UI
                        // reflects live progress instead of showing "0 documents".
                        // Registry lock is std::sync::Mutex — brief hold, no await inside.
                        if let Ok(mut reg) = registry.lock() {
                            reg.increment_doc_count(&fid);
                        }
                    }
                    Ok(Err(e)) => {
                        let _ = app_handle.emit("index-progress", IndexProgress {
                            file_path: path_str,
                            status: "error".to_string(),
                            doc_id: None,
                            error: Some(e.to_string()),
                            folder_id: Some(fid.clone()),
                        });
                    }
                    Err(e) => {
                        let _ = app_handle.emit("index-progress", IndexProgress {
                            file_path: path_str,
                            status: "error".to_string(),
                            doc_id: None,
                            error: Some(e.to_string()),
                            folder_id: Some(fid.clone()),
                        });
                    }
                }
            }
        }

        // Stamp last_scan + persist registry so Watched Folders UI shows
        // "Last scan: X min ago" and the counter survives an app restart.
        {
            let registry_path_for_save = state_registry_path.clone();
            if let Ok(mut reg) = registry.lock() {
                reg.mark_scan_complete(&fid);
                // Best-effort save; ignore errors so scan-complete still fires.
                let _ = reg.save(&registry_path_for_save);
            }
        }

        // Emit scan complete
        let _ = app_handle.emit("index-progress", IndexProgress {
            file_path: folder_config.path,
            status: "complete".to_string(),
            doc_id: None,
            error: None,
            folder_id: Some(fid),
        });

        // Auto-trigger Pass 2 backfill on freshly indexed docs (v2.5 → v3.0).
        // Live index leaves docs at PASS1_ONLY_VERSION; boot-time backfill can
        // only see docs that existed on startup. Without this, newly indexed
        // docs stay at v2.5 until user manually clicks "Re-extract entities".
        // Single-flight guard prevents concurrent backfills.
        use std::sync::atomic::Ordering;
        if backfill_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            crate::pipeline::backfill::spawn_entity_backfill(
                app_handle,
                engine,
                two_pass,
                entity_store,
                triple_store,
                ontology_store,
                auth_state,
                embedding_service,
                backfill_running,
                app_data_dir,
            );
        }
    });

    Ok(ScanProgress {
        folder_id,
        total_files: 0, // Actual count comes via events
        processed_files: 0,
        status: "scanning".to_string(),
    })
}

#[tauri::command]
pub async fn get_watched_folders(
    state: State<'_, AppState>,
) -> Result<Vec<WatchedFolder>, AppError> {
    let registry = state.registry.lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let folders: Vec<WatchedFolder> = registry.folders.values().map(|config| {
        WatchedFolder {
            id: config.id.clone(),
            path: config.path.clone(),
            document_count: config.document_count,
            last_scan: config.last_scan.clone().unwrap_or_else(|| "never".to_string()),
            status: if config.is_paused { "paused".to_string() } else { "watching".to_string() },
        }
    }).collect();

    Ok(folders)
}

/// Walk a directory tree iteratively, returning all file paths.
///
/// Uses an explicit stack (BFS) instead of recursion so deep directory trees
/// (e.g. `~/private` w/ 10k+ nested files) do not blow the Tokio worker's
/// 2 MB stack. Skips symlinks entirely so symlink cycles (a common Dropbox
/// pattern where a subdir links back to a shared folder) cannot cause an
/// infinite walk.
fn walk_dir_recursive(dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    use std::collections::HashSet;
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    // Track canonicalized dirs already visited so hard-linked or duplicate
    // mount points (rarer than symlinks but possible) do not cause a re-walk.
    let mut visited: HashSet<std::path::PathBuf> = HashSet::new();
    let mut stack: Vec<std::path::PathBuf> = Vec::new();
    stack.push(dir.to_path_buf());

    while let Some(current) = stack.pop() {
        // Canonicalize to dedup + short-circuit cycles. Fallback to raw path
        // if canonicalize fails (e.g. permission-denied) so we still descend.
        let canon = current.canonicalize().unwrap_or_else(|_| current.clone());
        if !visited.insert(canon) {
            continue;
        }

        let read = match std::fs::read_dir(&current) {
            Ok(r) => r,
            Err(_) => continue, // skip unreadable dirs, don't abort the whole walk
        };
        for entry in read.flatten() {
            let path = entry.path();
            // Skip symlinks entirely — cheapest way to avoid symlink cycles
            // and phantom "deep" trees.
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }
    Ok(files)
}

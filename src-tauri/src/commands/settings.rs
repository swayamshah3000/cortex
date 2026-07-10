use std::fs;
use std::path::PathBuf;
use tauri::State;
use crate::error::AppError;
use crate::state::AppState;
use crate::types::*;

/// Returns the path to the settings JSON file in the app data directory.
/// Derives the app data dir from the registry_path (which is {app_data_dir}/watcher-registry.json).
fn settings_path(state: &AppState) -> PathBuf {
    state
        .registry_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("settings.json")
}

/// Default settings returned when no settings file exists yet.
///
/// Phase 8 additions (D-33):
/// - `extraction_model`: empty string — TwoPassExtractor resolves the per-provider
///   default (Haiku 4.5 / gpt-5-mini / gemini-2.5-flash / Ollama model) at request time.
/// - `use_llm_extraction`: true — Pass 2 LLM refinement is on by default when a provider
///   is connected. Privacy-strict users can toggle it off in Settings → AI.
fn default_settings() -> Settings {
    Settings {
        theme: "dark".to_string(),
        sidebar_collapsed: false,
        embedding_model: "local".to_string(),
        watched_folders: vec![],
        excluded_patterns: vec![
            ".git".to_string(),
            "node_modules".to_string(),
            ".DS_Store".to_string(),
        ],
        index_on_startup: true,
        index_size: 0,
        storage_path: "~/Library/Application Support/com.cortex.app/vectors".to_string(),
        extraction_model: String::new(),
        use_llm_extraction: true,
    }
}

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<Settings, AppError> {
    let path = settings_path(&state);

    let result = tokio::task::spawn_blocking(move || {
        match fs::read_to_string(&path) {
            Ok(contents) => {
                match serde_json::from_str::<Settings>(&contents) {
                    Ok(settings) => Ok::<Settings, AppError>(settings),
                    Err(_) => Ok(default_settings()),
                }
            }
            Err(_) => Ok(default_settings()),
        }
    })
    .await??;
    Ok(result)
}

#[tauri::command]
pub async fn update_settings(
    settings: Settings,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let path = settings_path(&state);

    tokio::task::spawn_blocking(move || {
        let json = serde_json::to_string_pretty(&settings)
            .map_err(|e| AppError::Internal(format!("Failed to serialize settings: {}", e)))?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::Internal(format!("Failed to create settings dir: {}", e)))?;
        }

        fs::write(&path, json)
            .map_err(|e| AppError::Internal(format!("Failed to write settings: {}", e)))?;

        Ok::<(), AppError>(())
    })
    .await??;
    Ok(())
}

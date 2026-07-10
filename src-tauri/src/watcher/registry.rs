use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::AppError;

/// Configuration for a single watched folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedFolderConfig {
    pub id: String,
    pub path: String,
    pub enabled_types: Vec<String>,
    pub excluded_patterns: Vec<String>,
    pub is_paused: bool,
    pub document_count: u32,
    pub last_scan: Option<String>, // ISO 8601
}

/// Registry of all watched folders — persists to JSON on disk.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WatcherRegistry {
    pub folders: HashMap<String, WatchedFolderConfig>,
}

impl WatcherRegistry {
    /// Load registry from JSON file. Returns empty registry on any error (first run).
    pub fn load(registry_path: &PathBuf) -> Self {
        std::fs::read_to_string(registry_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist registry to JSON file.
    pub fn save(&self, registry_path: &PathBuf) -> Result<(), AppError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        std::fs::write(registry_path, json)?;
        Ok(())
    }

    /// Add a new folder with default configuration.
    pub fn add_folder(&mut self, path: String) -> WatchedFolderConfig {
        let id = uuid::Uuid::new_v4().to_string();
        let config = WatchedFolderConfig {
            id: id.clone(),
            path,
            enabled_types: vec![
                "pdf".into(), "docx".into(), "txt".into(), "md".into(),
                "xlsx".into(), "csv".into(), "xls".into(), "ods".into(),
            ],
            excluded_patterns: vec![
                "node_modules".into(), ".git".into(), "target".into(),
                "__pycache__".into(), ".DS_Store".into(),
            ],
            is_paused: false,
            document_count: 0,
            last_scan: None,
        };
        self.folders.insert(id, config.clone());
        config
    }

    /// Remove a folder. Returns true if it existed.
    pub fn remove_folder(&mut self, folder_id: &str) -> bool {
        self.folders.remove(folder_id).is_some()
    }

    /// Increment the folder's document_count by 1 (idempotent-friendly per-doc call).
    /// No-op if folder was removed mid-scan.
    pub fn increment_doc_count(&mut self, folder_id: &str) {
        if let Some(cfg) = self.folders.get_mut(folder_id) {
            cfg.document_count = cfg.document_count.saturating_add(1);
        }
    }

    /// Overwrite the folder's document_count (used when we know the exact total, e.g. after a full scan).
    pub fn set_doc_count(&mut self, folder_id: &str, count: u32) {
        if let Some(cfg) = self.folders.get_mut(folder_id) {
            cfg.document_count = count;
        }
    }

    /// Stamp the folder's `last_scan` with the current UTC time (RFC 3339).
    pub fn mark_scan_complete(&mut self, folder_id: &str) {
        if let Some(cfg) = self.folders.get_mut(folder_id) {
            cfg.last_scan = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    /// Returns true if the given path should be excluded from the folder's watch scope.
    pub fn is_excluded(&self, folder_id: &str, path: &Path) -> bool {
        let config = match self.folders.get(folder_id) {
            Some(c) => c,
            None => return false,
        };
        // Hidden files (starting with '.')
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                return true;
            }
        }
        // Check each path component against exclusion patterns (substring match)
        let path_str = path.to_string_lossy();
        for pattern in &config.excluded_patterns {
            if path_str.contains(pattern.as_str()) {
                return true;
            }
        }
        false
    }

    /// Returns true if the file extension is enabled for indexing in this folder.
    pub fn is_type_enabled(&self, folder_id: &str, ext: &str) -> bool {
        let config = match self.folders.get(folder_id) {
            Some(c) => c,
            None => return false,
        };
        if config.enabled_types.is_empty() {
            return true;
        }
        config.enabled_types.iter().any(|t| t == &ext.to_lowercase())
    }

    /// Find which watched folder a file belongs to (prefix match on path).
    pub fn find_folder_for_path(&self, file_path: &Path) -> Option<&WatchedFolderConfig> {
        self.folders.values().find(|config| {
            file_path.starts_with(&config.path)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let path = PathBuf::from("/tmp/nonexistent_registry_xyz_12345.json");
        let registry = WatcherRegistry::load(&path);
        assert!(registry.folders.is_empty());
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        let mut registry = WatcherRegistry::default();
        registry.add_folder("/home/user/Documents".to_string());
        registry.save(&path).unwrap();

        let loaded = WatcherRegistry::load(&path);
        assert_eq!(loaded.folders.len(), 1);
        let folder = loaded.folders.values().next().unwrap();
        assert_eq!(folder.path, "/home/user/Documents");
    }

    #[test]
    fn test_add_folder_creates_with_defaults() {
        let mut registry = WatcherRegistry::default();
        let config = registry.add_folder("/tmp/docs".to_string());
        assert_eq!(config.path, "/tmp/docs");
        assert!(!config.is_paused);
        assert_eq!(config.document_count, 0);
        assert!(config.last_scan.is_none());
        assert!(config.enabled_types.contains(&"pdf".to_string()));
        assert!(config.excluded_patterns.contains(&"node_modules".to_string()));
        assert_eq!(registry.folders.len(), 1);
    }

    #[test]
    fn test_remove_folder_returns_correct_bool() {
        let mut registry = WatcherRegistry::default();
        let config = registry.add_folder("/tmp/docs".to_string());
        assert!(registry.remove_folder(&config.id));
        assert!(!registry.remove_folder(&config.id));
        assert!(registry.folders.is_empty());
    }

    #[test]
    fn test_is_excluded() {
        let mut registry = WatcherRegistry::default();
        let config = registry.add_folder("/tmp/project".to_string());
        let id = &config.id.clone();

        // .git directory component should be excluded
        assert!(registry.is_excluded(id, Path::new("/tmp/project/.git/config")));
        // node_modules should be excluded
        assert!(registry.is_excluded(id, Path::new("/tmp/project/node_modules/pkg/index.js")));
        // Hidden file should be excluded
        assert!(registry.is_excluded(id, Path::new("/tmp/project/.hidden_file")));
        // Normal source file should NOT be excluded
        assert!(!registry.is_excluded(id, Path::new("/tmp/project/src/main.rs")));
    }

    #[test]
    fn test_is_type_enabled() {
        let mut registry = WatcherRegistry::default();
        let config = registry.add_folder("/tmp/docs".to_string());
        let id = &config.id.clone();

        assert!(registry.is_type_enabled(id, "pdf"));
        assert!(registry.is_type_enabled(id, "PDF")); // case-insensitive
        assert!(registry.is_type_enabled(id, "docx"));
        assert!(!registry.is_type_enabled(id, "exe"));
        assert!(!registry.is_type_enabled(id, "dll"));
    }

    #[test]
    fn test_find_folder_for_path() {
        let mut registry = WatcherRegistry::default();
        registry.add_folder("/tmp/documents".to_string());
        registry.add_folder("/tmp/downloads".to_string());

        let result = registry.find_folder_for_path(Path::new("/tmp/documents/report.pdf"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, "/tmp/documents");

        let result2 = registry.find_folder_for_path(Path::new("/home/user/other.txt"));
        assert!(result2.is_none());
    }
}

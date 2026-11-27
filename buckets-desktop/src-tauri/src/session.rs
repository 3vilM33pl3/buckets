// SPDX-License-Identifier: MIT
//! Which repository is open, and the app's own settings.
//!
//! The buckets library keeps one global database connection and reads the
//! working directory from the process, so a window works on one repository
//! at a time. Opening another one saves it as the most recent and restarts
//! the app.

use std::fs;
use std::path::{Path, PathBuf};

use buckets::utils::utils::find_bucket_repo;
use serde::{Deserialize, Serialize};

use crate::model::RepoInfo;

const MAX_RECENT: usize = 8;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub recent: Vec<PathBuf>,
    /// Path to the `buckets` CLI. Found automatically when empty.
    pub cli_path: Option<PathBuf>,
    /// Run the semantic duplicate check when adding expectations (downloads
    /// a ~90 MB model the first time).
    pub similarity_check: bool,
    /// `dark`, `light` or `system`.
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent: Vec::new(),
            cli_path: None,
            similarity_check: false,
            theme: "dark".to_string(),
        }
    }
}

fn settings_file() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("buckets-desktop").join("settings.json"))
}

impl Settings {
    pub fn load() -> Self {
        settings_file()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = settings_file().ok_or("No config directory on this system")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, text).map_err(|e| e.to_string())
    }

    pub fn remember(&mut self, root: &Path) {
        self.recent.retain(|p| p != root);
        self.recent.insert(0, root.to_path_buf());
        self.recent.truncate(MAX_RECENT);
    }
}

/// The repository root (the folder holding `.buckets`) for any path inside it.
pub fn repo_root_for(path: &Path) -> Option<PathBuf> {
    let buckets_dir = find_bucket_repo(path)?;
    buckets_dir.parent().map(Path::to_path_buf)
}

pub struct Session {
    pub root: PathBuf,
}

impl Session {
    /// Make `root` the working directory and connect to its database.
    pub fn open(root: &Path) -> Result<Self, String> {
        std::env::set_current_dir(root)
            .map_err(|e| format!("Cannot enter {}: {}", root.display(), e))?;
        // Bootstrapping builds its own Tokio runtime, so run it on a plain
        // thread rather than inside Tauri's.
        std::thread::spawn(buckets::bootstrap::bootstrap_database)
            .join()
            .map_err(|_| "Database bootstrap panicked".to_string())?
            .map_err(|e| e.to_string())?;
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    pub fn info(&self) -> RepoInfo {
        RepoInfo {
            name: self
                .root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| self.root.display().to_string()),
            root: self.root.display().to_string(),
            remote: remote_path(&self.root),
        }
    }
}

#[derive(Deserialize)]
struct RemoteFile {
    path: PathBuf,
}

/// The remote store set with `buckets remote set`, if any.
pub fn remote_path(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join(".buckets").join("remote.toml")).ok()?;
    let remote: RemoteFile = toml::from_str(&text).ok()?;
    Some(remote.path.display().to_string())
}

//! Persistent list of recently opened files. Stored as JSON under the
//! platform config dir so it survives across runs.

use std::path::{Path, PathBuf};

const MAX_RECENT: usize = 8;
const FILE_NAME: &str = "recent.json";

fn config_path() -> Option<PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("table-rs");
    Some(p.join(FILE_NAME))
}

pub fn load() -> Vec<PathBuf> {
    let Some(path) = config_path() else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    serde_json::from_slice::<Vec<PathBuf>>(&bytes).unwrap_or_default()
}

pub fn push(paths: &mut Vec<PathBuf>, added: &Path) {
    paths.retain(|p| p != added);
    paths.insert(0, added.to_path_buf());
    paths.truncate(MAX_RECENT);
}

pub fn save(paths: &[PathBuf]) -> std::io::Result<()> {
    let Some(path) = config_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(paths).unwrap_or_else(|_| b"[]".to_vec());
    std::fs::write(path, json)
}

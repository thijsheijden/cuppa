use std::path::PathBuf;

use directories::ProjectDirs;

/// Get the platform-appropriate data directory for Cuppa.
/// Creates it if it doesn't exist.
pub fn data_dir() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("com", "cuppa", "cuppa")?;
    let data_dir = dirs.data_dir().to_path_buf();
    std::fs::create_dir_all(&data_dir).ok()?;
    Some(data_dir)
}

/// Get the path to the database file as a String.
pub fn db_path() -> String {
    data_dir()
        .map(|d| d.join("cuppa.db"))
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "cuppa.db".to_string())
}

/// Get the path to the sync log directory.
pub fn sync_log_dir() -> Option<PathBuf> {
    let dir = data_dir()?.join("sync-logs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

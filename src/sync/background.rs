use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    Idle,
    Pulling,
    Applying,
    Done,
    Error,
}

impl SyncStatus {
    pub fn message(&self) -> &'static str {
        match self {
            SyncStatus::Idle => "",
            SyncStatus::Pulling => "Syncing: pulling remote logs...",
            SyncStatus::Applying => "Syncing: applying changes...",
            SyncStatus::Done => "Sync: up to date",
            SyncStatus::Error => "Sync: error",
        }
    }
}

/// Shared state for the background sync process.
#[derive(Debug, Clone)]
pub struct BackgroundSyncState {
    pub status: SyncStatus,
    pub message: String,
}

impl BackgroundSyncState {
    pub fn new() -> Self {
        Self {
            status: SyncStatus::Idle,
            message: String::new(),
        }
    }
}

/// Manages the background sync process at app startup.
pub struct BackgroundSync {
    state: Arc<Mutex<BackgroundSyncState>>,
}

impl BackgroundSync {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(BackgroundSyncState::new())),
        }
    }

    pub fn state(&self) -> Arc<Mutex<BackgroundSyncState>> {
        Arc::clone(&self.state)
    }

    /// Spawn the background sync task on the given runtime handle.
    /// This performs a git pull and updates the shared state.
    pub fn spawn(&self, runtime: &tokio::runtime::Runtime, log_dir: std::path::PathBuf) {
        let state = Arc::clone(&self.state);

        runtime.spawn(async move {
            // Check if a remote URL is configured in settings before doing anything
            let remote_url = match crate::repository::setting::SettingRepository::new() {
                Ok(repo) => repo.get_sync_remote_url().unwrap_or(None),
                Err(_) => None,
            };

            // No remote configured in settings — nothing to do, stay silent
            if remote_url.is_none() {
                return;
            }
            let remote_url = remote_url.unwrap();

            // Update status: pulling
            {
                let mut s = state.lock().await;
                s.status = SyncStatus::Pulling;
                s.message = "Pulling remote logs...".to_string();
            }

            // If no git repo exists yet, initialise one and add the remote
            if !log_dir.join(".git").exists() {
                {
                    let mut s = state.lock().await;
                    s.message = "Git: init repo...".to_string();
                }

                let output = Command::new("git")
                    .args(["init"])
                    .current_dir(&log_dir)
                    .output();

                if let Ok(output) = output {
                    if !output.status.success() {
                        let mut s = state.lock().await;
                        s.status = SyncStatus::Error;
                        s.message = format!("git init failed: {}", String::from_utf8_lossy(&output.stderr));
                        return;
                    }
                } else {
                    let mut s = state.lock().await;
                    s.status = SyncStatus::Error;
                    s.message = "git init failed".to_string();
                    return;
                }

                // Configure git user for commits
                let _ = Command::new("git")
                    .args(["config", "user.email", "sync@cuppa.app"])
                    .current_dir(&log_dir)
                    .output();
                let _ = Command::new("git")
                    .args(["config", "user.name", "Cuppa Sync"])
                    .current_dir(&log_dir)
                    .output();

                // Add the remote
                let output = Command::new("git")
                    .args(["remote", "add", "origin", &remote_url])
                    .current_dir(&log_dir)
                    .output();

                if let Ok(output) = output {
                    if !output.status.success() {
                        let mut s = state.lock().await;
                        s.status = SyncStatus::Error;
                        s.message = format!("git remote add failed: {}", String::from_utf8_lossy(&output.stderr));
                        return;
                    }
                } else {
                    let mut s = state.lock().await;
                    s.status = SyncStatus::Error;
                    s.message = "git remote add failed".to_string();
                    return;
                }

                // Create initial commit so we have a branch to pull into
                let output = Command::new("git")
                    .args(["commit", "--allow-empty", "-m", "Initial commit"])
                    .current_dir(&log_dir)
                    .output();

                if let Ok(output) = output {
                    if !output.status.success() {
                        let mut s = state.lock().await;
                        s.status = SyncStatus::Error;
                        s.message = format!("git initial commit failed: {}", String::from_utf8_lossy(&output.stderr));
                        return;
                    }
                }
            }

            // Check if remote exists
            let remote_check = Command::new("git")
                .args(["remote", "get-url", "origin"])
                .current_dir(&log_dir)
                .output();

            match remote_check {
                Ok(output) if !output.status.success() => {
                    let mut s = state.lock().await;
                    s.status = SyncStatus::Error;
                    s.message = "Remote not found".to_string();
                    return;
                }
                Err(_) => {
                    let mut s = state.lock().await;
                    s.status = SyncStatus::Error;
                    s.message = "Failed to check remote".to_string();
                    return;
                }
                _ => {}
            }

            // Perform git pull
            let pull_result = Command::new("git")
                .args(["pull", "origin", "main", "--ff-only"])
                .current_dir(&log_dir)
                .output();

            match pull_result {
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    if output.status.success() {
                        let mut s = state.lock().await;
                        s.status = SyncStatus::Applying;
                        // git pull writes progress to stderr even on success;
                        // use stdout for the actual result message.
                        s.message = if stdout.contains("Already up to date") {
                            "Already up to date".to_string()
                        } else if !stdout.trim().is_empty() {
                            format!("Pulled changes: {}", stdout.trim())
                        } else if !stderr.trim().is_empty() {
                            format!("Pulled changes: {}", stderr.trim())
                        } else {
                            "Pulled changes".to_string()
                        };
                    } else {
                        let mut s = state.lock().await;
                        s.status = SyncStatus::Error;
                        s.message = format!("Pull failed: {}", stderr.trim());
                        return;
                    }
                }
                Err(e) => {
                    let mut s = state.lock().await;
                    s.status = SyncStatus::Error;
                    s.message = format!("Pull error: {}", e);
                    return;
                }
            }

            // Wait a moment so the user can see the "Applying" message,
            // then clear it after 2.5 seconds.
            sleep(Duration::from_millis(2500)).await;
            {
                let mut s = state.lock().await;
                s.status = SyncStatus::Idle;
                s.message.clear();
            }
        });
    }
}

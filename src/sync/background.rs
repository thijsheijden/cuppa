use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::sync::git::GitRepo;

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

            // Open or init the git repo, adding the remote if it's fresh
            let repo = match GitRepo::open_or_init(&log_dir, Some(&remote_url)) {
                Ok(repo) => repo,
                Err(e) => {
                    let mut s = state.lock().await;
                    s.status = SyncStatus::Error;
                    s.message = format!("Git repo init failed: {}", e);
                    return;
                }
            };

            // Check if remote exists
            let has_remote = match repo.has_remote() {
                Ok(has) => has,
                Err(_) => {
                    let mut s = state.lock().await;
                    s.status = SyncStatus::Error;
                    s.message = "Remote not found".to_string();
                    return;
                }
            };

            if !has_remote {
                let mut s = state.lock().await;
                s.status = SyncStatus::Error;
                s.message = "Remote not configured".to_string();
                return;
            }

            // Perform git pull
            {
                let mut s = state.lock().await;
                s.status = SyncStatus::Applying;
                s.message = "Pulling from origin...".to_string();
            }

            match repo.pull() {
                Ok(_) => {
                    let mut s = state.lock().await;
                    s.message = "Pulled changes".to_string();
                }
                Err(e) => {
                    let mut s = state.lock().await;
                    s.status = SyncStatus::Error;
                    s.message = format!("Pull failed: {}", e);
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

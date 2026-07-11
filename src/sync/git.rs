use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct GitError {
    message: String,
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for GitError {}

impl From<std::io::Error> for GitError {
    fn from(e: std::io::Error) -> Self {
        GitError {
            message: format!("io error: {}", e),
        }
    }
}

impl From<String> for GitError {
    fn from(s: String) -> Self {
        GitError { message: s }
    }
}

impl From<&str> for GitError {
    fn from(s: &str) -> Self {
        GitError {
            message: s.to_string(),
        }
    }
}

/// Manages a git repository in a given directory for syncing log files.
/// Handles initialization, remote setup, and adopting remote history.
pub struct GitRepo {
    dir: PathBuf,
}

impl GitRepo {
    /// Open an existing git repo at `dir`, or create one if it doesn't exist.
    /// If `remote_url` is provided and the repo is fresh, the remote is added
    /// and its history is adopted (or an initial commit is created if the
    /// remote is empty).
    pub fn open_or_init(
        dir: impl AsRef<Path>,
        remote_url: Option<&str>,
    ) -> Result<Self, GitError> {
        let dir = dir.as_ref().to_path_buf();

        if !dir.join(".git").exists() {
            Self::init_fresh(&dir, remote_url)?;
        }

        Ok(Self { dir })
    }

    /// Initialize a fresh git repo at `dir`.
    fn init_fresh(dir: &Path, remote_url: Option<&str>) -> Result<(), GitError> {
        // git init
        let output = Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git init failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        // Configure git user (local to this repo)
        let _ = Command::new("git")
            .args(["config", "user.email", "sync@cuppa.app"])
            .current_dir(dir)
            .output()?;
        let _ = Command::new("git")
            .args(["config", "user.name", "Cuppa Sync"])
            .current_dir(dir)
            .output()?;

        // If a remote URL is provided, add it and adopt its history
        if let Some(url) = remote_url {
            let output = Command::new("git")
                .args(["remote", "add", "origin", url])
                .current_dir(dir)
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "git remote add failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }

            // Fetch to check if remote has history
            let fetch_result = Command::new("git")
                .args(["fetch", "origin", "main"])
                .current_dir(dir)
                .output()?;

            if fetch_result.status.success() {
                // Remote has history — adopt it
                let output = Command::new("git")
                    .args(["reset", "--hard", "origin/main"])
                    .current_dir(dir)
                    .output()?;
                if !output.status.success() {
                    return Err(format!(
                        "git reset failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )
                    .into());
                }
            } else {
                // Remote is empty — create initial commit so we have a branch to push to
                let output = Command::new("git")
                    .args(["commit", "--allow-empty", "-m", "Initial commit"])
                    .current_dir(dir)
                    .output()?;
                if !output.status.success() {
                    return Err(format!(
                        "git initial commit failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )
                    .into());
                }
            }
        } else {
            // No remote — create initial commit for local-only repo
            let output = Command::new("git")
                .args(["commit", "--allow-empty", "-m", "Initial commit"])
                .current_dir(dir)
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "git initial commit failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
        }

        Ok(())
    }

    /// Check if a remote named `origin` is configured.
    pub fn has_remote(&self) -> Result<bool, GitError> {
        let output = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(&self.dir)
            .output()?;
        Ok(output.status.success())
    }

    /// Add a remote named `origin`.
    pub fn add_remote(&self, url: &str) -> Result<(), GitError> {
        let output = Command::new("git")
            .args(["remote", "add", "origin", url])
            .current_dir(&self.dir)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git remote add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    }

    /// Update the URL of the existing `origin` remote.
    pub fn set_remote_url(&self, url: &str) -> Result<(), GitError> {
        let output = Command::new("git")
            .args(["remote", "set-url", "origin", url])
            .current_dir(&self.dir)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git remote set-url failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    }

    /// Stage all changes.
    pub fn add_all(&self) -> Result<(), GitError> {
        let output = Command::new("git")
            .args(["add", "."])
            .current_dir(&self.dir)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    }

    /// Check if there are staged changes to commit.
    pub fn has_staged_changes(&self) -> Result<bool, GitError> {
        let output = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(&self.dir)
            .output()?;
        Ok(!output.status.success())
    }

    /// Commit staged changes with the given message.
    pub fn commit(&self, message: &str) -> Result<(), GitError> {
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.dir)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git commit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    }

    /// Push current branch to origin/main.
    pub fn push(&self) -> Result<(), GitError> {
        let output = Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&self.dir)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!(
                "git push failed: stderr='{}' stdout='{}'",
                stderr, stdout
            )
            .into());
        }
        Ok(())
    }

    /// Pull from origin/main with --ff-only.
    pub fn pull(&self) -> Result<(), GitError> {
        let output = Command::new("git")
            .args(["pull", "origin", "main", "--ff-only"])
            .current_dir(&self.dir)
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git pull failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    }

    /// Get the repository directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

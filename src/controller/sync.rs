use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    prelude::*,
};

use crate::controller::error_screen::ErrorScreen;
use crate::controller::popover::PopoverScreen;
use crate::controller::screen::Screen;
use crate::controller::syncing::SyncingScreen;
use crate::repository::setting::SettingRepository;
use crate::sync::git::GitRepo;
use crate::sync::log::SyncLog;
use crate::sync::ops::PendingOp;

pub struct SyncController {
    sync_log: Rc<RefCell<SyncLog>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncResult {
    Proceed,
    Stay,
}

impl SyncController {
    pub fn new(sync_log: Rc<RefCell<SyncLog>>) -> Self {
        Self { sync_log }
    }

    /// Read missing log entries from the sync repository that have not yet been
    /// applied to the local database. Returns a list of `(sequence_number, PendingOp)`
    /// pairs, ordered by sequence number, ready to be executed.
    pub fn read_missing_logs(&self,
    ) -> Result<Vec<(u64, PendingOp)>, Box<dyn std::error::Error>> {
        let last_seq = {
            let settings = SettingRepository::new()?;
            settings.get_sync_last_seq()?
        };

        let ops = {
            let sync_log = self.sync_log.borrow();
            sync_log.read_missing(last_seq)?
        };

        Ok(ops)
    }

    /// Attempt to sync and return whether the app should proceed to close or stay open.
    /// Renders the provided `render_app` callback between sync steps so the UI stays responsive.
    pub fn try_sync_and_close<F>(
        &self,
        terminal: &mut Terminal<impl Backend>,
        mut render_app: F,
    ) -> std::io::Result<SyncResult>
    where
        F: FnMut(&mut Frame),
    {
        // Only sync if there are operations to export
        if self.sync_log.borrow().session_count() == 0 {
            return Ok(SyncResult::Proceed);
        }

        let mut syncing = SyncingScreen::new();

        // Show initial syncing popover
        self.draw_with_syncing(terminal, &mut render_app, &syncing)?;

        // Perform the export
        syncing.add_message("Writing log files...".to_string());
        self.draw_with_syncing(terminal, &mut render_app, &syncing)?;

        let export_result = {
            let mut sync_log = self.sync_log.borrow_mut();
            sync_log.export()
        };
        if let Err(e) = export_result {
            syncing.add_message(format!("Export failed: {}", e));
            self.draw_with_syncing(terminal, &mut render_app, &syncing)?;
            let msg = format!("Export failed: {}", e);
            return self.show_sync_error_and_wait(terminal, render_app, &msg);
        }

        syncing.add_message("Export done.".to_string());
        self.draw_with_syncing(terminal, &mut render_app, &syncing)?;

        // Commit and push to git
        syncing.add_message("Git: checking remote...".to_string());
        self.draw_with_syncing(terminal, &mut render_app, &syncing)?;

        if let Err(e) = self.git_commit_and_push(&mut syncing, terminal, &mut render_app) {
            let msg = format!("Git sync failed: {}", e);
            return self.show_sync_error_and_wait(terminal, render_app, &msg);
        }

        syncing.add_message("Sync complete.".to_string());
        self.draw_with_syncing(terminal, &mut render_app, &syncing)?;

        Ok(SyncResult::Proceed)
    }

    fn draw_with_syncing<F>(
        &self,
        terminal: &mut Terminal<impl Backend>,
        render_app: &mut F,
        syncing: &SyncingScreen,
    ) -> std::io::Result<()>
    where
        F: FnMut(&mut Frame),
    {
        terminal.draw(|frame| {
            render_app(frame);
            syncing.render(frame);
        })?;
        Ok(())
    }

    fn show_sync_error_and_wait<F>(
        &self,
        terminal: &mut Terminal<impl Backend>,
        mut render_app: F,
        error_message: &str,
    ) -> std::io::Result<SyncResult>
    where
        F: FnMut(&mut Frame),
    {
        let error_screen = ErrorScreen::new(error_message.to_string());
        let popover = PopoverScreen::new(Box::new(error_screen), 60, 12);

        // We need to track the popover state ourselves since we don't own the screen stack
        let mut showing_error = true;

        // Wait for user to dismiss the error popup
        loop {
            terminal.draw(|frame| {
                render_app(frame);
                if showing_error {
                    popover.render(frame);
                }
            })?;

            if let Event::Key(key) = event::read()? {
                if key.kind == ratatui::crossterm::event::KeyEventKind::Press {
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                        // Even on Ctrl+C, require dismissing the error first
                        continue;
                    }

                    // Simple input handling for the error popover
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter => {
                            return Ok(SyncResult::Stay);
                        }
                        KeyCode::Char('q') => {
                            return Ok(SyncResult::Stay);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn git_commit_and_push<F>(
        &self,
        syncing: &mut SyncingScreen,
        terminal: &mut Terminal<impl Backend>,
        render_app: &mut F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&mut Frame),
    {
        let dir = self.sync_log.borrow().dir().to_path_buf();

        syncing.add_message("Git: opening repo...".to_string());
        self.draw_with_syncing(terminal, render_app, syncing)?;

        // Open or init repo
        let remote_url: Option<String> = if dir.join(".git").exists() {
            None
        } else {
            let settings = SettingRepository::new()?;
            settings.get_sync_remote_url()?
        };
        let remote_url_ref = remote_url.as_deref();

        let repo = GitRepo::open_or_init(&dir, remote_url_ref)?;

        syncing.add_message("Git: repo ready.".to_string());
        self.draw_with_syncing(terminal, render_app, syncing)?;

        // Check if remote exists
        if !repo.has_remote()? {
            syncing.add_message("Git: reading remote URL from settings...".to_string());
            self.draw_with_syncing(terminal, render_app, syncing)?;

            let settings = SettingRepository::new()?;
            if let Some(url) = settings.get_sync_remote_url()? {
                syncing.add_message(format!("Git: adding remote {}...", url));
                self.draw_with_syncing(terminal, render_app, syncing)?;
                repo.add_remote(&url)?;
            } else {
                syncing.add_message("Git: no remote configured, skipping.".to_string());
                self.draw_with_syncing(terminal, render_app, syncing)?;
                return Ok(()); // No remote configured, skip
            }
        }

        syncing.add_message("Git: skip fetch (no concurrent use).".to_string());
        self.draw_with_syncing(terminal, render_app, syncing)?;

        // Stage all changes
        syncing.add_message("Git: staging changes...".to_string());
        self.draw_with_syncing(terminal, render_app, syncing)?;
        repo.add_all()?;

        // Check if there are changes to commit
        if repo.has_staged_changes()? {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let message = format!("Logs update {}", now);

            syncing.add_message(format!("Git: committing '{}'...", message));
            self.draw_with_syncing(terminal, render_app, syncing)?;
            repo.commit(&message)?;
        } else {
            syncing.add_message("Git: nothing to commit.".to_string());
            self.draw_with_syncing(terminal, render_app, syncing)?;
        }

        syncing.add_message("Git: pushing to origin...".to_string());
        self.draw_with_syncing(terminal, render_app, syncing)?;
        repo.push()?;

        syncing.add_message("Git: push done.".to_string());
        self.draw_with_syncing(terminal, render_app, syncing)?;

        Ok(())
    }
}

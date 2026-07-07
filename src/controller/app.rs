use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    prelude::*,
};

use std::cell::RefCell;
use std::rc::Rc;

use crate::controller::error_screen::ErrorScreen;
use crate::controller::popover::PopoverScreen;
use crate::controller::screen::{AppAction, Screen};
use crate::controller::syncing::SyncingScreen;
use crate::sync::log::SyncLog;

pub struct AppController {
    screen_stack: Vec<Box<dyn Screen>>,
    sync_log: Rc<RefCell<SyncLog>>,
}

impl AppController {
    pub fn new() -> std::io::Result<Self> {
        let sync_log = Rc::new(RefCell::new(
            SyncLog::new("/Users/thijsheijden/Developer/rust/cuppa/synced-logfiles")?
        ));
        Ok(Self {
            screen_stack: Vec::new(),
            sync_log,
        })
    }

    pub fn push_screen(&mut self, screen: Box<dyn Screen>) {
        self.screen_stack.push(screen);
    }

    pub fn run(&mut self, mut terminal: Terminal<impl Backend>) -> std::io::Result<()> {
        loop {
            terminal.draw(|frame| {
                self.render(frame);
            })?;

            if let Event::Key(key) = event::read()? {
                if key.kind == ratatui::crossterm::event::KeyEventKind::Press {
                    // Global Ctrl+C shortcut
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                        match self.try_sync_and_close(&mut terminal)? {
                            CloseAction::Proceed => return Ok(()),
                            CloseAction::Stay => continue,
                        }
                    }

                    let action = self.handle_input(key);
                    match action {
                        AppAction::Quit => {
                            match self.try_sync_and_close(&mut terminal)? {
                                CloseAction::Proceed => return Ok(()),
                                CloseAction::Stay => continue,
                            }
                        }
                        AppAction::Continue => {}
                        AppAction::PushScreen(screen) => self.push_screen(screen),
                        AppAction::PopScreen => {
                            if self.screen_stack.len() > 1 {
                                self.screen_stack.pop();
                            }
                            // Refresh home screen data after closing a popup
                            if let Some(screen) = self.screen_stack.last_mut() {
                                let _ = screen.handle_input(KeyEvent::from(KeyCode::F(5)));
                            }
                        }
                    }
                }
            }
        }
    }

    fn try_sync_and_close(&mut self, terminal: &mut Terminal<impl Backend>) -> std::io::Result<CloseAction> {
        // Only sync if there are operations to export
        if self.sync_log.borrow().session_count() == 0 {
            return Ok(CloseAction::Proceed);
        }

        let mut syncing = SyncingScreen::new();

        // Show initial syncing popover
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        // Perform the export
        syncing.add_message("Writing log files...".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        let export_result = {
            let mut sync_log = self.sync_log.borrow_mut();
            sync_log.export()
        };
        if let Err(e) = export_result {
            syncing.add_message(format!("Export failed: {}", e));
            terminal.draw(|frame| {
                self.render(frame);
                syncing.render(frame);
            })?;
            let msg = format!("Export failed: {}", e);
            return self.show_sync_error_and_wait(terminal, &msg);
        }

        syncing.add_message("Export done.".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        // Commit and push to git
        syncing.add_message("Git: checking remote...".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        if let Err(e) = self.git_commit_and_push(&mut syncing, terminal) {
            let msg = format!("Git sync failed: {}", e);
            return self.show_sync_error_and_wait(terminal, &msg);
        }

        syncing.add_message("Sync complete.".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        Ok(CloseAction::Proceed)
    }

    fn show_sync_error_and_wait(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        error_message: &str,
    ) -> std::io::Result<CloseAction> {
        let error_screen = ErrorScreen::new(error_message.to_string());
        let popover = PopoverScreen::new(Box::new(error_screen), 60, 12);
        self.push_screen(Box::new(popover));

        // Wait for user to dismiss the error popup
        loop {
            terminal.draw(|frame| {
                self.render(frame);
            })?;

            if let Event::Key(key) = event::read()? {
                if key.kind == ratatui::crossterm::event::KeyEventKind::Press {
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                        // Even on Ctrl+C, require dismissing the error first
                        continue;
                    }

                    let action = self.handle_input(key);
                    match action {
                        AppAction::PopScreen => {
                            self.screen_stack.pop();
                            return Ok(CloseAction::Stay);
                        }
                        AppAction::Quit => {
                            // If they press q on the error screen, dismiss it and stay in app
                            self.screen_stack.pop();
                            return Ok(CloseAction::Stay);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn git_commit_and_push(
        &self,
        syncing: &mut SyncingScreen,
        terminal: &mut Terminal<impl Backend>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};

        let dir = self.sync_log.borrow().dir().to_path_buf();

        syncing.add_message("Git: opening repo...".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        // Open or init repo using git CLI
        if !dir.join(".git").exists() {
            syncing.add_message("Git: init repo...".to_string());
            terminal.draw(|frame| {
                self.render(frame);
                syncing.render(frame);
            })?;
            let output = Command::new("git")
                .args(["init"])
                .current_dir(&dir)
                .output()?;
            if !output.status.success() {
                return Err(format!("git init failed: {}", String::from_utf8_lossy(&output.stderr)).into());
            }

            // Create initial commit
            let output = Command::new("git")
                .args(["config", "user.email", "sync@cuppa.app"])
                .current_dir(&dir)
                .output()?;
            let output = Command::new("git")
                .args(["config", "user.name", "Cuppa Sync"])
                .current_dir(&dir)
                .output()?;
            let output = Command::new("git")
                .args(["commit", "--allow-empty", "-m", "Initial commit"])
                .current_dir(&dir)
                .output()?;
            if !output.status.success() {
                return Err(format!("git initial commit failed: {}", String::from_utf8_lossy(&output.stderr)).into());
            }
        }

        syncing.add_message("Git: repo ready.".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        // Check if remote exists
        let remote_check = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(&dir)
            .output()?;

        if !remote_check.status.success() {
            syncing.add_message("Git: reading remote URL from settings...".to_string());
            terminal.draw(|frame| {
                self.render(frame);
                syncing.render(frame);
            })?;

            let db = crate::repository::connection::DbConnection::open("cuppa.db")?;
            let settings = crate::repository::setting::SettingRepository::new(db)?;
            if let Some(url) = settings.get_sync_remote_url()? {
                syncing.add_message(format!("Git: adding remote {}...", url));
                terminal.draw(|frame| {
                    self.render(frame);
                    syncing.render(frame);
                })?;
                let output = Command::new("git")
                    .args(["remote", "add", "origin", &url])
                    .current_dir(&dir)
                    .output()?;
                if !output.status.success() {
                    return Err(format!("git remote add failed: {}", String::from_utf8_lossy(&output.stderr)).into());
                }
            } else {
                syncing.add_message("Git: no remote configured, skipping.".to_string());
                terminal.draw(|frame| {
                    self.render(frame);
                    syncing.render(frame);
                })?;
                return Ok(()); // No remote configured, skip
            }
        }

        syncing.add_message("Git: skip fetch (no concurrent use).".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        // Stage all changes
        syncing.add_message("Git: staging changes...".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        let output = Command::new("git")
            .args(["add", "."])
            .current_dir(&dir)
            .output()?;
        if !output.status.success() {
            return Err(format!("git add failed: {}", String::from_utf8_lossy(&output.stderr)).into());
        }

        // Check if there are changes to commit
        let output = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(&dir)
            .output()?;
        let has_changes = !output.status.success();

        if has_changes {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let message = format!("Logs update {}", now);

            syncing.add_message(format!("Git: committing '{}'...", message));
            terminal.draw(|frame| {
                self.render(frame);
                syncing.render(frame);
            })?;

            let output = Command::new("git")
                .args(["commit", "-m", &message])
                .current_dir(&dir)
                .output()?;
            if !output.status.success() {
                return Err(format!("git commit failed: {}", String::from_utf8_lossy(&output.stderr)).into());
            }
        } else {
            syncing.add_message("Git: nothing to commit.".to_string());
            terminal.draw(|frame| {
                self.render(frame);
                syncing.render(frame);
            })?;
        }

        syncing.add_message("Git: pushing to origin...".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        let output = Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&dir)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("git push failed: stderr='{}' stdout='{}'", stderr, stdout).into());
        }

        syncing.add_message("Git: push done.".to_string());
        terminal.draw(|frame| {
            self.render(frame);
            syncing.render(frame);
        })?;

        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        if let Some(screen) = self.screen_stack.last() {
            screen.render(frame);
        }
    }

    fn handle_input(&mut self, key: KeyEvent) -> AppAction {
        if let Some(screen) = self.screen_stack.last_mut() {
            let action = screen.handle_input(key);
            if action != AppAction::Continue {
                return action;
            }
        }

        // Fallback: no screen handled it, check app-level shortcuts
        match key.code {
            KeyCode::Char('q') => {
                if self.screen_stack.len() > 1 {
                    AppAction::PopScreen
                } else {
                    AppAction::Quit
                }
            }
            _ => AppAction::Continue,
        }
    }

    /// Get a shared clone of the sync log.
    pub fn sync_log(&self) -> Rc<RefCell<SyncLog>> {
        Rc::clone(&self.sync_log)
    }
}

enum CloseAction {
    Proceed,
    Stay,
}

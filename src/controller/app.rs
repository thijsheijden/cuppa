use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    prelude::*,
};

use std::cell::RefCell;
use std::rc::Rc;

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
                        self.show_syncing_and_export(&mut terminal)?;
                        return Ok(());
                    }

                    let action = self.handle_input(key);
                    match action {
                        AppAction::Quit => {
                            self.show_syncing_and_export(&mut terminal)?;
                            return Ok(());
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

    fn show_syncing_and_export(&mut self, terminal: &mut Terminal<impl Backend>) -> std::io::Result<()> {
        // Only show syncing if there are operations to export
        if self.sync_log.borrow().session_count() == 0 {
            return Ok(());
        }

        // Render the syncing popover
        terminal.draw(|frame| {
            self.render(frame);
            // Render syncing screen on top
            let syncing = SyncingScreen::new();
            syncing.render(frame);
        })?;

        // Perform the export
        let _ = self.sync_log.borrow_mut().export();

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

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    prelude::*,
};

use std::cell::RefCell;
use std::rc::Rc;

use crate::controller::screen::{AppAction, Screen};
use crate::controller::sync::{SyncController, SyncResult};
use crate::sync::log::SyncLog;

pub struct AppController {
    screen_stack: Vec<Box<dyn Screen>>,
    sync_log: Rc<RefCell<SyncLog>>,
    sync_controller: SyncController,
}

impl AppController {
    pub fn new() -> std::io::Result<Self> {
        let sync_log = Rc::new(RefCell::new(
            SyncLog::new("/Users/thijsheijden/Developer/rust/cuppa/synced-logfiles")?
        ));
        let sync_controller = SyncController::new(Rc::clone(&sync_log));
        Ok(Self {
            screen_stack: Vec::new(),
            sync_log,
            sync_controller,
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
        let render_app = |frame: &mut Frame| {
            self.render(frame);
        };
        match self.sync_controller.try_sync_and_close(terminal, render_app)? {
            SyncResult::Proceed => Ok(CloseAction::Proceed),
            SyncResult::Stay => Ok(CloseAction::Stay),
        }
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

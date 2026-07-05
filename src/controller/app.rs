use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    prelude::*,
};

use crate::controller::screen::{AppAction, Screen};

pub struct AppController {
    screen_stack: Vec<Box<dyn Screen>>,
}

impl AppController {
    pub fn new() -> Self {
        Self {
            screen_stack: Vec::new(),
        }
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
                if key.kind == KeyEventKind::Press {
                    let action = self.handle_input(key.code);
                    match action {
                        AppAction::Quit => return Ok(()),
                        AppAction::Continue => {}
                        AppAction::PushScreen(screen) => self.push_screen(screen),
                        AppAction::PopScreen => {
                            if self.screen_stack.len() > 1 {
                                self.screen_stack.pop();
                            }
                            // Refresh home screen data after closing a popup
                            if let Some(screen) = self.screen_stack.last_mut() {
                                let _ = screen.handle_input(KeyCode::F(5));
                            }
                        }
                    }
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        if let Some(screen) = self.screen_stack.last() {
            screen.render(frame);
        }
    }

    fn handle_input(&mut self, key: KeyCode) -> AppAction {
        if let Some(screen) = self.screen_stack.last_mut() {
            let action = screen.handle_input(key);
            if action != AppAction::Continue {
                return action;
            }
        }

        // Fallback: no screen handled it, check app-level shortcuts
        match key {
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
}

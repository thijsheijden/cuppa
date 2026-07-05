use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub trait Screen {
    fn render(&self, frame: &mut ratatui::Frame);
    fn handle_input(&mut self, key: KeyEvent) -> AppAction;
}

pub enum AppAction {
    Continue,
    Quit,
    PushScreen(Box<dyn Screen>),
    PopScreen,
}

impl PartialEq for AppAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AppAction::Continue, AppAction::Continue) => true,
            (AppAction::Quit, AppAction::Quit) => true,
            (AppAction::PopScreen, AppAction::PopScreen) => true,
            _ => false,
        }
    }
}

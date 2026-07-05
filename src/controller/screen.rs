pub trait Screen {
    fn render(&self, frame: &mut ratatui::Frame);
    fn handle_input(&mut self, key: ratatui::crossterm::event::KeyCode) -> AppAction;
}

pub enum AppAction {
    Continue,
    Quit,
    PushScreen(Box<dyn Screen>),
    PopScreen,
}

use duckdb::Result as DuckResult;

use crate::controller::screen::{AppAction, Screen};
use crate::view::home;

pub struct HomeController {
    pub current_caffeine_level: f64,
    pub today_total_caffeine: i32,
    pub half_life_hours: f64,
}

impl HomeController {
    pub fn new(db: crate::repository::connection::DbConnection) -> DuckResult<Self> {
        let repo = crate::repository::duckdb::DrinkRepository::new(db)?;
        let current_caffeine_level = repo.current_caffeine_level()?;
        let today_total_caffeine = repo.get_today_total_caffeine()?;

        Ok(Self {
            current_caffeine_level,
            today_total_caffeine,
            half_life_hours: crate::repository::duckdb::CAFFEINE_HALF_LIFE_HOURS,
        })
    }
}

impl Screen for HomeController {
    fn render(&self, frame: &mut ratatui::Frame) {
        home::render(frame, self);
    }

    fn handle_input(&mut self, key: ratatui::crossterm::event::KeyCode) -> AppAction {
        if key == ratatui::crossterm::event::KeyCode::Char('q') {
            return AppAction::Quit;
        }
        AppAction::Continue
    }
}

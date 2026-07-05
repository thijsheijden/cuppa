use duckdb::Result as DuckResult;

use crate::controller::add_drink::AddDrinkScreen;
use crate::controller::popover::PopoverScreen;
use crate::controller::screen::{AppAction, Screen};
use crate::repository::connection::DbConnection;

pub struct HomeController {
    pub current_caffeine_level: f64,
    pub today_total_caffeine: i32,
    pub half_life_hours: f64,
}

impl HomeController {
    pub fn new(db: DbConnection) -> DuckResult<Self> {
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
        crate::view::home::render(frame, self);
    }

    fn handle_input(&mut self, key: ratatui::crossterm::event::KeyCode) -> AppAction {
        match key {
            ratatui::crossterm::event::KeyCode::Char('q') => AppAction::Quit,
            ratatui::crossterm::event::KeyCode::Char('a') => {
                match AddDrinkScreen::new() {
                    Ok(add_screen) => {
                        let popover = PopoverScreen::new(Box::new(add_screen), 60, 20);
                        AppAction::PushScreen(Box::new(popover))
                    }
                    Err(_) => AppAction::Continue,
                }
            }
            _ => AppAction::Continue,
        }
    }
}

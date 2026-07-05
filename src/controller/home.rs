use duckdb::Result as DuckResult;

use chrono::{Local, Utc, TimeZone};

use crate::controller::add_drink::AddDrinkScreen;
use crate::controller::popover::PopoverScreen;
use crate::controller::screen::{AppAction, Screen};
use crate::repository::connection::DbConnection;
use crate::repository::duckdb::{DrinkFilter, DrinkRepository};

pub struct HomeController {
    pub current_caffeine_level: f64,
    pub today_total_caffeine: i32,
    pub half_life_hours: f64,
    pub todays_drinks: Vec<(String, String)>,
}

impl HomeController {
    pub fn new(db: DbConnection) -> DuckResult<Self> {
        let repo = DrinkRepository::new(db)?;
        let current_caffeine_level = repo.current_caffeine_level()?;
        let today_total_caffeine = repo.get_today_total_caffeine()?;
        let todays_drinks = Self::load_todays_drinks()?;

        Ok(Self {
            current_caffeine_level,
            today_total_caffeine,
            half_life_hours: crate::repository::duckdb::CAFFEINE_HALF_LIFE_HOURS,
            todays_drinks,
        })
    }

    fn load_todays_drinks() -> DuckResult<Vec<(String, String)>> {
        let db = DbConnection::open("cuppa.db")?;
        let repo = DrinkRepository::new(db)?;

        let today_start = Local::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
            .with_timezone(&Utc);

        let filter = DrinkFilter::new()
            .with_since(today_start)
            .with_limit(8);

        let drinks = repo.get_all_drinks(Some(&filter))?;

        let recent = drinks
            .into_iter()
            .map(|d| {
                let time = d.consumed_at.with_timezone(&Local).format("%H:%M").to_string();
                (time, d.drink_name)
            })
            .collect();

        Ok(recent)
    }

    pub fn refresh(&mut self) -> DuckResult<()> {
        let db = DbConnection::open("cuppa.db")?;
        let repo = DrinkRepository::new(db)?;
        self.current_caffeine_level = repo.current_caffeine_level()?;
        self.today_total_caffeine = repo.get_today_total_caffeine()?;
        self.todays_drinks = Self::load_todays_drinks()?;
        Ok(())
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
            ratatui::crossterm::event::KeyCode::F(5) => {
                let _ = self.refresh();
                AppAction::Continue
            }
            _ => AppAction::Continue,
        }
    }
}

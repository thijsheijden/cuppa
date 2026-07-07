use std::cell::RefCell;
use std::rc::Rc;

use duckdb::Result as DuckResult;

use chrono::{Local, Utc, TimeZone};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::controller::add_drink::AddDrinkScreen;
use crate::controller::popover::PopoverScreen;
use crate::controller::screen::{AppAction, Screen};
use crate::repository::connection::DbConnection;
use crate::repository::duckdb::{DrinkFilter, DrinkRepository};
use crate::repository::setting::SettingRepository;
use crate::sync::log::SyncLog;

pub struct HomeController {
    pub current_caffeine_level: f64,
    pub today_total_caffeine: i32,
    pub half_life_hours: f64,
    pub todays_drinks: Vec<(String, String)>,
    pub caffeine_series: Vec<(String, f64)>,
    pub sleep_time: Option<String>,
    pub current_time: String,
    pub bedtime: String,
    pub bedtime_caffeine_mg: i32,
    sync_log: Rc<RefCell<SyncLog>>,
}

impl HomeController {
    pub fn new(db: DbConnection, sync_log: Rc<RefCell<SyncLog>>) -> DuckResult<Self> {
        let repo = DrinkRepository::with_sync_log(db, Rc::clone(&sync_log))?;
        let current_caffeine_level = repo.current_caffeine_level()?;
        let today_total_caffeine = repo.get_today_total_caffeine()?;
        let todays_drinks = Self::load_todays_drinks()?;
        let caffeine_series = Self::load_caffeine_series()?;
        let sleep_time = Self::load_sleep_time()?;

        let (bedtime, bedtime_caffeine_mg) = Self::load_settings()?;

        let current_time = Local::now().format("%H:%M").to_string();

        Ok(Self {
            current_caffeine_level,
            today_total_caffeine,
            half_life_hours: crate::repository::duckdb::CAFFEINE_HALF_LIFE_HOURS,
            todays_drinks,
            caffeine_series,
            sleep_time,
            current_time,
            bedtime,
            bedtime_caffeine_mg,
            sync_log,
        })
    }

    fn load_settings() -> DuckResult<(String, i32)> {
        let db = DbConnection::open("cuppa.db")?;
        let repo = SettingRepository::new(db)?;
        
        let bedtime = repo
            .get_setting(crate::entity::setting::SETTING_BEDTIME)?
            .map(|s| s.value)
            .unwrap_or_else(|| "23:00".to_string());
        
        let bedtime_caffeine_mg = repo
            .get_setting(crate::entity::setting::SETTING_CAFFEINE_MG_AT_BEDTIME)?
            .and_then(|s| s.as_int())
            .unwrap_or(50);
        
        Ok((bedtime, bedtime_caffeine_mg))
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

    fn load_caffeine_series() -> DuckResult<Vec<(String, f64)>> {
        let db = DbConnection::open("cuppa.db")?;
        let repo = DrinkRepository::new(db)?;
        let series = repo.generate_caffeine_series()?;

        let formatted = series
            .into_iter()
            .map(|(dt, level)| {
                let time = dt.with_timezone(&Local).format("%H:%M").to_string();
                (time, level)
            })
            .collect();

        Ok(formatted)
    }

    fn load_sleep_time() -> DuckResult<Option<String>> {
        let db = DbConnection::open("cuppa.db")?;
        let repo = DrinkRepository::new(db)?;
        match repo.time_until_threshold(50.0)? {
            Some(dt) => {
                let local = dt.with_timezone(&Local);
                let now = Local::now();
                let today = now.date_naive();
                let dt_date = local.date_naive();
                
                let time_str = local.format("%H:%M").to_string();
                let date_str = if dt_date == today {
                    "today".to_string()
                } else if dt_date == today.succ_opt().unwrap_or(today) {
                    "tomorrow".to_string()
                } else {
                    local.format("%a %d %b").to_string()
                };
                
                Ok(Some(format!("{} at {}", date_str, time_str)))
            }
            None => Ok(None),
        }
    }

    pub fn refresh(&mut self) -> DuckResult<()> {
        let db = DbConnection::open("cuppa.db")?;
        let repo = DrinkRepository::with_sync_log(db, Rc::clone(&self.sync_log))?;
        self.current_caffeine_level = repo.current_caffeine_level()?;
        self.today_total_caffeine = repo.get_today_total_caffeine()?;
        self.todays_drinks = Self::load_todays_drinks()?;
        self.caffeine_series = Self::load_caffeine_series()?;
        self.sleep_time = Self::load_sleep_time()?;
        self.current_time = Local::now().format("%H:%M").to_string();
        
        let (bedtime, bedtime_caffeine_mg) = Self::load_settings()?;
        self.bedtime = bedtime;
        self.bedtime_caffeine_mg = bedtime_caffeine_mg;
        
        Ok(())
    }
}

impl Screen for HomeController {
    fn render(&self, frame: &mut ratatui::Frame) {
        crate::view::home::render(frame, self);
    }

    fn handle_input(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Char('q') => AppAction::Quit,
            KeyCode::Char('a') => {
                match AddDrinkScreen::new(Rc::clone(&self.sync_log)) {
                    Ok(add_screen) => {
                        let popover = PopoverScreen::new(Box::new(add_screen), 60, 20);
                        AppAction::PushScreen(Box::new(popover))
                    }
                    Err(_) => AppAction::Continue,
                }
            }
            KeyCode::F(5) => {
                let _ = self.refresh();
                AppAction::Continue
            }
            KeyCode::Char('l') => {
                match crate::controller::drink_log::DrinkLogScreen::new(Rc::clone(&self.sync_log)) {
                    Ok(log_screen) => {
                        let popover = crate::controller::popover::PopoverScreen::new(Box::new(log_screen), 60, 18);
                        AppAction::PushScreen(Box::new(popover))
                    }
                    Err(_) => AppAction::Continue,
                }
            }
            KeyCode::Char('s') => {
                match crate::controller::settings::SettingsScreen::new() {
                    Ok(settings_screen) => {
                        let popover = crate::controller::popover::PopoverScreen::new(Box::new(settings_screen), 50, 16);
                        AppAction::PushScreen(Box::new(popover))
                    }
                    Err(_) => AppAction::Continue,
                }
            }
            _ => AppAction::Continue,
        }
    }
}

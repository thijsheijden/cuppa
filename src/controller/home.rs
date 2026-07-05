use duckdb::Result as DuckResult;

use crate::repository::{
    connection::DbConnection,
    duckdb::{DrinkRepository, CAFFEINE_HALF_LIFE_HOURS},
};

pub struct HomeController {
    pub current_caffeine_level: f64,
    pub today_total_caffeine: i32,
    pub half_life_hours: f64,
}

impl HomeController {
    pub fn new(db: DbConnection) -> DuckResult<Self> {
        let repo = DrinkRepository::new(db)?;
        let current_caffeine_level = repo.current_caffeine_level()?;
        let today_total_caffeine = repo.get_today_total_caffeine()?;

        Ok(Self {
            current_caffeine_level,
            today_total_caffeine,
            half_life_hours: CAFFEINE_HALF_LIFE_HOURS,
        })
    }
}

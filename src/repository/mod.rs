use chrono::{DateTime, Utc};

pub mod connection;
pub mod duckdb;

#[derive(Debug, Clone)]
pub struct DrinkRecord {
    pub id: Option<i64>,
    pub drink_name: String,
    pub caffeine_mg: i32,
    pub consumed_at: DateTime<Utc>,
}

use chrono::{DateTime, Utc, Duration, Local};

use crate::repository::connection::open_db;
use crate::paths::db_path;
use crate::sync::log::SyncLog;
use crate::sync::ops::SyncOp;
use std::cell::RefCell;
use std::rc::Rc;

pub struct DrinkFilter {
    pub limit: Option<usize>,
    pub since: Option<DateTime<Utc>>,
}

impl DrinkFilter {
    pub fn new() -> Self {
        Self {
            limit: None,
            since: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }
}

pub const CAFFEINE_HALF_LIFE_HOURS: f64 = 5.0;

pub struct DrinkRepository {
    sync_log: Option<Rc<RefCell<SyncLog>>>,
}

impl DrinkRepository {
    pub fn new() -> rusqlite::Result<Self> {
        let repo = Self { sync_log: None };
        repo.init_schema()?;
        Ok(repo)
    }

    pub fn with_sync_log(sync_log: Rc<RefCell<SyncLog>>) -> rusqlite::Result<Self> {
        let repo = Self { sync_log: Some(sync_log) };
        repo.init_schema()?;
        Ok(repo)
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        let conn = open_db(db_path())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS drinks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                drink_name TEXT NOT NULL,
                caffeine_mg INTEGER NOT NULL,
                consumed_at TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn add_drink_sync(&self, drink_name: &str, caffeine_mg: i32, consumed_at: DateTime<Utc>) -> rusqlite::Result<()> {
        let conn = open_db(db_path())?;
        conn.execute(
            "INSERT INTO drinks (drink_name, caffeine_mg, consumed_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![drink_name, caffeine_mg, consumed_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn delete_drink_by_name_and_time(&self, drink_name: &str, consumed_at: DateTime<Utc>) -> rusqlite::Result<usize> {
        let conn = open_db(db_path())?;
        let rows = conn.execute(
            "DELETE FROM drinks WHERE drink_name = ?1 AND consumed_at = ?2",
            rusqlite::params![drink_name, consumed_at.to_rfc3339()],
        )?;
        Ok(rows)
    }

    pub fn get_drink_by_name_and_time(&self, drink_name: &str, consumed_at: DateTime<Utc>) -> rusqlite::Result<Option<DrinkRecord>> {
        let conn = open_db(db_path())?;
        let mut stmt = conn.prepare(
            "SELECT id, drink_name, caffeine_mg, consumed_at FROM drinks WHERE drink_name = ?1 AND consumed_at = ?2"
        )?;
        let mut rows = stmt.query(rusqlite::params![drink_name, consumed_at.to_rfc3339()])?;
        if let Some(row) = rows.next()? {
            let consumed_at_str: String = row.get(3)?;
            let consumed_at = DateTime::parse_from_rfc3339(&consumed_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            return Ok(Some(DrinkRecord {
                id: Some(row.get(0)?),
                drink_name: row.get(1)?,
                caffeine_mg: row.get(2)?,
                consumed_at,
            }));
        }
        Ok(None)
    }

    pub fn add_drink(&self, drink_name: &str, caffeine_mg: i32, consumed_at: DateTime<Utc>) -> rusqlite::Result<()> {
        let conn = open_db(db_path())?;
        conn.execute(
            "INSERT INTO drinks (drink_name, caffeine_mg, consumed_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![drink_name, caffeine_mg, consumed_at.to_rfc3339()],
        )?;

        if let Some(ref sync_log) = self.sync_log {
            sync_log.borrow_mut().track(SyncOp::add_drink(
                drink_name.to_string(),
                caffeine_mg,
                consumed_at,
            ));
        }

        Ok(())
    }

    pub fn get_all_drinks(&self, filter: Option<&DrinkFilter>) -> rusqlite::Result<Vec<DrinkRecord>> {
        let conn = open_db(db_path())?;
        let mut sql = "SELECT id, drink_name, caffeine_mg, consumed_at FROM drinks".to_string();
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        let since_str;

        if let Some(filter) = filter {
            if let Some(since) = filter.since {
                sql.push_str(" WHERE consumed_at >= ?1");
                since_str = since.to_rfc3339();
                params.push(&since_str);
            }
        }

        sql.push_str(" ORDER BY consumed_at DESC");

        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params_from_iter(params.iter()))?;

        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            let consumed_at_str: String = row.get(3)?;
            let consumed_at = DateTime::parse_from_rfc3339(&consumed_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            records.push(DrinkRecord {
                id: Some(row.get(0)?),
                drink_name: row.get(1)?,
                caffeine_mg: row.get(2)?,
                consumed_at,
            });
        }

        if let Some(filter) = filter {
            if let Some(limit) = filter.limit {
                records.truncate(limit);
            }
        }

        Ok(records)
    }

    pub fn get_today_total_caffeine(&self) -> rusqlite::Result<i32> {
        let conn = open_db(db_path())?;
        let total: i32 = conn.query_row(
            "SELECT COALESCE(SUM(caffeine_mg), 0) FROM drinks WHERE date(consumed_at) = date('now')",
            [],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    pub fn delete_drink(&self, id: i64) -> rusqlite::Result<usize> {
        let conn = open_db(db_path())?;
        // Get drink info before deleting for sync log
        let drink_info: Option<(String, DateTime<Utc>)> = conn.query_row(
            "SELECT drink_name, consumed_at FROM drinks WHERE id = ?1",
            [id],
            |row| {
                let consumed_at_str: String = row.get(1)?;
                let consumed_at = DateTime::parse_from_rfc3339(&consumed_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok((row.get::<_, String>(0)?, consumed_at))
            }
        ).ok();

        let result = conn.execute("DELETE FROM drinks WHERE id = ?1", [id])?;

        if let (Some(sync_log), Some((name, consumed_at))) = (&self.sync_log, drink_info) {
            sync_log.borrow_mut().track(SyncOp::delete_drink(name, consumed_at));
        }

        Ok(result)
    }

    pub fn generate_caffeine_series(&self) -> rusqlite::Result<Vec<(DateTime<Utc>, f64)>> {
        let now = Utc::now();
        let cutoff = now - Duration::hours(72);
        let drinks = self.get_drinks_since(cutoff)?;

        let start = now - Duration::hours(2);
        let end = now + Duration::hours(8);

        let mut points = Vec::new();
        let mut t = start;

        while t <= end {
            let level = Self::calculate_level_at(&drinks, t);
            points.push((t, level));
            t += Duration::minutes(15);
        }

        Ok(points)
    }

    pub fn current_caffeine_level(&self) -> rusqlite::Result<f64> {
        let now = Utc::now();
        let cutoff = now - Duration::hours(72);
        let drinks = self.get_drinks_since(cutoff)?;
        Ok(Self::calculate_level_at(&drinks, now))
    }

    /// Find the first time in the future when caffeine level drops to 50mg or below.
    /// Returns None if already below 50mg or if it won't drop within 72 hours.
    pub fn time_until_threshold(&self, threshold: f64) -> rusqlite::Result<Option<DateTime<Utc>>> {
        let now = Utc::now();
        let current = self.current_caffeine_level()?;

        if current <= threshold {
            return Ok(None);
        }

        let cutoff = now - Duration::hours(72);
        let drinks = self.get_drinks_since(cutoff)?;

        // Search forward in 15-minute increments up to 72 hours
        let mut t = now + Duration::minutes(15);
        let end = now + Duration::hours(72);

        while t <= end {
            let level = Self::calculate_level_at(&drinks, t);
            if level <= threshold {
                return Ok(Some(t));
            }
            t += Duration::minutes(15);
        }

        Ok(None)
    }

    pub fn get_drinks_paginated(&self, offset: usize, limit: usize) -> rusqlite::Result<Vec<DrinkRecord>> {
        let conn = open_db(db_path())?;
        let mut stmt = conn.prepare(
            "SELECT id, drink_name, caffeine_mg, consumed_at FROM drinks ORDER BY consumed_at DESC LIMIT ?1 OFFSET ?2"
        )?;
        let mut rows = stmt.query(rusqlite::params![limit as i64, offset as i64])?;

        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            let consumed_at_str: String = row.get(3)?;
            let consumed_at = DateTime::parse_from_rfc3339(&consumed_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            records.push(DrinkRecord {
                id: Some(row.get(0)?),
                drink_name: row.get(1)?,
                caffeine_mg: row.get(2)?,
                consumed_at,
            });
        }
        Ok(records)
    }

    pub fn get_drinks_since(&self, since: DateTime<Utc>) -> rusqlite::Result<Vec<DrinkRecord>> {
        let conn = open_db(db_path())?;
        let mut stmt = conn.prepare(
            "SELECT id, drink_name, caffeine_mg, consumed_at FROM drinks WHERE consumed_at >= ?1 ORDER BY consumed_at DESC"
        )?;
        let mut rows = stmt.query(rusqlite::params![since.to_rfc3339()])?;

        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            let consumed_at_str: String = row.get(3)?;
            let consumed_at = DateTime::parse_from_rfc3339(&consumed_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            records.push(DrinkRecord {
                id: Some(row.get(0)?),
                drink_name: row.get(1)?,
                caffeine_mg: row.get(2)?,
                consumed_at,
            });
        }
        Ok(records)
    }

    fn calculate_level_at(drinks: &[DrinkRecord], at: DateTime<Utc>) -> f64 {
        drinks.iter().map(|d| Self::decayed_amount(d.caffeine_mg as f64, d.consumed_at, at)).sum()
    }

    fn decayed_amount(initial_mg: f64, consumed_at: DateTime<Utc>, at: DateTime<Utc>) -> f64 {
        if at < consumed_at {
            return 0.0;
        }
        if at == consumed_at {
            return initial_mg;
        }
        let hours_elapsed = at.signed_duration_since(consumed_at).num_seconds() as f64 / 3600.0;
        initial_mg * 0.5f64.powf(hours_elapsed / CAFFEINE_HALF_LIFE_HOURS)
    }
}

#[derive(Debug, Clone)]
pub struct DrinkRecord {
    pub id: Option<i64>,
    pub drink_name: String,
    pub caffeine_mg: i32,
    pub consumed_at: DateTime<Utc>,
}

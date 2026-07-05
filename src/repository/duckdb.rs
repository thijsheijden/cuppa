use chrono::{DateTime, Utc};
use duckdb::{params, Connection, Result as DuckResult};

use crate::repository::DrinkRecord;

pub struct DrinkRepository {
    conn: Connection,
}

impl DrinkRepository {
    pub fn new(db_path: &str) -> DuckResult<Self> {
        let conn = Connection::open(db_path)?;
        let repo = Self { conn };
        repo.init_schema()?;
        Ok(repo)
    }

    fn init_schema(&self) -> DuckResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS drinks (
                id INTEGER PRIMARY KEY,
                drink_name VARCHAR NOT NULL,
                caffeine_mg INTEGER NOT NULL,
                consumed_at TIMESTAMP WITH TIME ZONE NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn add_drink(&self, drink_name: &str, caffeine_mg: i32, consumed_at: DateTime<Utc>) -> DuckResult<()> {
        self.conn.execute(
            "INSERT INTO drinks (drink_name, caffeine_mg, consumed_at) VALUES (?, ?, ?)",
            params![drink_name, caffeine_mg, consumed_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_all_drinks(&self) -> DuckResult<Vec<DrinkRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, drink_name, caffeine_mg, consumed_at FROM drinks ORDER BY consumed_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            let consumed_at_str: String = row.get(3)?;
            let consumed_at = DateTime::parse_from_rfc3339(&consumed_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(DrinkRecord {
                id: Some(row.get(0)?),
                drink_name: row.get(1)?,
                caffeine_mg: row.get(2)?,
                consumed_at,
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn get_today_total_caffeine(&self) -> DuckResult<i32> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(SUM(caffeine_mg), 0) FROM drinks WHERE consumed_at >= date_trunc('day', current_timestamp)"
        )?;
        let total: i32 = stmt.query_row([], |row| row.get(0))?;
        Ok(total)
    }

    pub fn delete_drink(&self, id: i64) -> DuckResult<usize> {
        self.conn.execute("DELETE FROM drinks WHERE id = ?", params![id])
    }
}

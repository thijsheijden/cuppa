use chrono::{DateTime, Utc, Duration};

use crate::repository::{connection::DbConnection, DrinkRecord};

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

pub struct DrinkRepository {
    db: DbConnection,
}

pub const CAFFEINE_HALF_LIFE_HOURS: f64 = 5.0;

impl DrinkRepository {
    pub fn new(db: DbConnection) -> duckdb::Result<Self> {
        let repo = Self { db };
        repo.init_schema()?;
        Ok(repo)
    }

    fn init_schema(&self) -> duckdb::Result<()> {
        self.db.execute(
            "CREATE SEQUENCE IF NOT EXISTS drinks_id_seq START 1",
            &[],
        )?;
        self.db.execute(
            "CREATE TABLE IF NOT EXISTS drinks (
                id INTEGER NOT NULL DEFAULT nextval('drinks_id_seq'),
                drink_name VARCHAR NOT NULL,
                caffeine_mg INTEGER NOT NULL,
                consumed_at TIMESTAMP WITH TIME ZONE NOT NULL,
                PRIMARY KEY (id)
            )",
            &[],
        )?;
        Ok(())
    }

    pub fn add_drink(&self, drink_name: &str, caffeine_mg: i32, consumed_at: DateTime<Utc>) -> duckdb::Result<()> {
        self.db.execute(
            "INSERT INTO drinks (drink_name, caffeine_mg, consumed_at) VALUES (?, ?, ?)",
            &[&drink_name as &dyn duckdb::ToSql, &caffeine_mg, &consumed_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_all_drinks(&self, filter: Option<&DrinkFilter>) -> duckdb::Result<Vec<DrinkRecord>> {
        let mut sql = "SELECT id, drink_name, caffeine_mg, consumed_at FROM drinks".to_string();
        let mut param_values: Vec<String> = Vec::new();

        if let Some(filter) = filter {
            if let Some(since) = filter.since {
                sql.push_str(" WHERE consumed_at >= ?");
                param_values.push(since.to_rfc3339());
            }
        }

        sql.push_str(" ORDER BY consumed_at DESC");

        let mut stmt = self.db.prepare(&sql)?;
        let row_mapper = |row: &duckdb::Row<'_>| {
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
        };

        let rows = if param_values.is_empty() {
            stmt.query_map([], row_mapper)?
        } else {
            let params: Vec<&dyn duckdb::ToSql> = param_values
                .iter()
                .map(|s| s as &dyn duckdb::ToSql)
                .collect();
            stmt.query_map(&*params, row_mapper)?
        };

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }

        if let Some(filter) = filter {
            if let Some(limit) = filter.limit {
                records.truncate(limit);
            }
        }

        Ok(records)
    }

    pub fn get_today_total_caffeine(&self) -> duckdb::Result<i32> {
        let total: i32 = self.db.query_row(
            "SELECT COALESCE(SUM(caffeine_mg), 0) FROM drinks WHERE consumed_at >= date_trunc('day', current_timestamp)",
            [],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    pub fn delete_drink(&self, id: i64) -> duckdb::Result<usize> {
        self.db.execute("DELETE FROM drinks WHERE id = ?", &[&id as &dyn duckdb::ToSql])
    }

    pub fn current_caffeine_level(&self) -> duckdb::Result<f64> {
        let now = Utc::now();
        let cutoff = now - Duration::hours(72);
        let drinks = self.get_drinks_since(cutoff)?;
        Ok(Self::calculate_level_at(&drinks, now))
    }

    pub fn get_drinks_since(&self, since: DateTime<Utc>) -> duckdb::Result<Vec<DrinkRecord>> {
        let mut stmt = self.db.prepare(
            "SELECT id, drink_name, caffeine_mg, consumed_at FROM drinks WHERE consumed_at >= ? ORDER BY consumed_at DESC"
        )?;
        let rows = stmt.query_map(&[&since.to_rfc3339() as &dyn duckdb::ToSql], |row| {
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

    fn calculate_level_at(drinks: &[DrinkRecord], at: DateTime<Utc>) -> f64 {
        drinks.iter().map(|d| Self::decayed_amount(d.caffeine_mg as f64, d.consumed_at, at)).sum()
    }

    fn decayed_amount(initial_mg: f64, consumed_at: DateTime<Utc>, at: DateTime<Utc>) -> f64 {
        if at <= consumed_at {
            return initial_mg;
        }
        let hours_elapsed = at.signed_duration_since(consumed_at).num_seconds() as f64 / 3600.0;
        initial_mg * 0.5f64.powf(hours_elapsed / CAFFEINE_HALF_LIFE_HOURS)
    }
}

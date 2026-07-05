use duckdb::Result as DuckResult;

use crate::repository::connection::DbConnection;

pub struct DrinkTypeRepository {
    db: DbConnection,
}

impl DrinkTypeRepository {
    pub fn new(db: DbConnection) -> DuckResult<Self> {
        let repo = Self { db };
        repo.init_schema()?;
        repo.seed_defaults()?;
        Ok(repo)
    }

    fn init_schema(&self) -> DuckResult<()> {
        self.db.execute(
            "CREATE TABLE IF NOT EXISTS drink_types (
                key VARCHAR PRIMARY KEY,
                name VARCHAR NOT NULL,
                caffeine_mg INTEGER NOT NULL
            )",
            &[],
        )?;
        Ok(())
    }

    fn seed_defaults(&self) -> DuckResult<()> {
        use crate::entity::ALL;

        for drink in ALL {
            self.db.execute(
                "INSERT OR IGNORE INTO drink_types (key, name, caffeine_mg) VALUES (?, ?, ?)",
                &[
                    &drink.id as &dyn duckdb::ToSql,
                    &drink.name,
                    &drink.caffeine_mg,
                ],
            )?;
        }

        Ok(())
    }

    pub fn add_custom_drink(&self, name: &str, caffeine_mg: i32) -> DuckResult<String> {
        let key = format!("custom_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        self.db.execute(
            "INSERT INTO drink_types (key, name, caffeine_mg) VALUES (?, ?, ?)",
            &[
                &key as &dyn duckdb::ToSql,
                &name,
                &caffeine_mg,
            ],
        )?;
        Ok(key)
    }

    pub fn get_drink_types_sorted_by_consumption(&self) -> duckdb::Result<Vec<(String, String, i32)>> {
        let mut stmt = self.db.prepare(
            "SELECT dt.key, dt.name, dt.caffeine_mg, COUNT(d.id) as consumption_count
             FROM drink_types dt
             LEFT JOIN drinks d ON dt.name = d.drink_name
             GROUP BY dt.key, dt.name, dt.caffeine_mg
             ORDER BY consumption_count DESC, dt.name ASC"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn get_all_drink_types(&self) -> DuckResult<Vec<(String, String, i32)>> {
        let mut stmt = self.db.prepare("SELECT key, name, caffeine_mg FROM drink_types ORDER BY name")?;
        let rows = stmt.query_map([], |row: &duckdb::Row<'_>| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    pub fn get_drink_type_by_key(&self, key: &str) -> DuckResult<Option<(String, String, i32)>> {
        let mut stmt = self.db.prepare("SELECT key, name, caffeine_mg FROM drink_types WHERE key = ?")?;
        let result = stmt.query_row(&[&key as &dyn duckdb::ToSql], |row: &duckdb::Row<'_>| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ))
        });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn delete_drink_type(&self, key: &str) -> DuckResult<usize> {
        self.db.execute("DELETE FROM drink_types WHERE key = ?", &[&key as &dyn duckdb::ToSql])
    }
}

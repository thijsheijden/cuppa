use crate::repository::connection::open_db;
use crate::paths::db_path;

pub struct DrinkTypeRepository;

impl DrinkTypeRepository {
    pub fn new() -> rusqlite::Result<Self> {
        let repo = Self;
        repo.init_schema()?;
        repo.seed_defaults()?;
        Ok(repo)
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        let conn = open_db(db_path())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS drink_types (
                key TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                caffeine_mg INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    fn seed_defaults(&self) -> rusqlite::Result<()> {
        use crate::entity::ALL;
        let conn = open_db(db_path())?;
        for drink in ALL {
            conn.execute(
                "INSERT OR IGNORE INTO drink_types (key, name, caffeine_mg) VALUES (?1, ?2, ?3)",
                rusqlite::params![&drink.id, &drink.name, &drink.caffeine_mg],
            )?;
        }
        Ok(())
    }

    pub fn add_custom_drink(&self, name: &str, caffeine_mg: i32) -> rusqlite::Result<String> {
        let key = format!("custom_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let conn = open_db(db_path())?;
        conn.execute(
            "INSERT INTO drink_types (key, name, caffeine_mg) VALUES (?1, ?2, ?3)",
            rusqlite::params![&key, &name, &caffeine_mg],
        )?;
        Ok(key)
    }

    pub fn get_drink_types_sorted_by_consumption(&self) -> rusqlite::Result<Vec<(String, String, i32)>> {
        let conn = open_db(db_path())?;
        let mut stmt = conn.prepare(
            "SELECT dt.key, dt.name, dt.caffeine_mg, COUNT(d.id) as consumption_count
             FROM drink_types dt
             LEFT JOIN drinks d ON dt.name = d.drink_name
             GROUP BY dt.key, dt.name, dt.caffeine_mg
             ORDER BY consumption_count DESC, dt.name ASC"
        )?;
        
        let mut rows = stmt.query([])?;
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ));
        }
        Ok(results)
    }

    pub fn get_all_drink_types(&self) -> rusqlite::Result<Vec<(String, String, i32)>> {
        let conn = open_db(db_path())?;
        let mut stmt = conn.prepare("SELECT key, name, caffeine_mg FROM drink_types ORDER BY name")?;
        let mut rows = stmt.query([])?;
        let mut records = Vec::new();
        while let Some(row) = rows.next()? {
            records.push((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ));
        }
        Ok(records)
    }

    pub fn get_drink_type_by_key(&self, key: &str) -> rusqlite::Result<Option<(String, String, i32)>> {
        let conn = open_db(db_path())?;
        let mut stmt = conn.prepare("SELECT key, name, caffeine_mg FROM drink_types WHERE key = ?1")?;
        let result = stmt.query_row([key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ))
        });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn delete_drink_type(&self, key: &str) -> rusqlite::Result<usize> {
        let conn = open_db(db_path())?;
        let rows = conn.execute("DELETE FROM drink_types WHERE key = ?1", [key])?;
        Ok(rows)
    }
}

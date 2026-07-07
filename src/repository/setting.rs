use chrono::NaiveTime;

use crate::entity::setting::{Setting, SettingType, SETTING_BEDTIME, SETTING_CAFFEINE_MG_AT_BEDTIME, SETTING_SYNC_REMOTE_URL};
use crate::repository::connection::DbConnection;

pub struct SettingRepository {
    db: DbConnection,
}

impl SettingRepository {
    pub fn new(db: DbConnection) -> duckdb::Result<Self> {
        let repo = Self { db };
        repo.init_schema()?;
        repo.ensure_defaults()?;
        Ok(repo)
    }

    fn init_schema(&self) -> duckdb::Result<()> {
        self.db.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                id VARCHAR PRIMARY KEY,
                value VARCHAR NOT NULL,
                setting_type VARCHAR NOT NULL
            )",
            &[],
        )?;
        Ok(())
    }

    /// Insert default settings if they don't already exist
    fn ensure_defaults(&self) -> duckdb::Result<()> {
        let defaults = crate::entity::setting::default_settings();
        for setting in defaults {
            let exists: bool = self.db.query_row(
                "SELECT 1 FROM settings WHERE id = ?",
                &[&setting.id],
                |_| Ok(true),
            ).unwrap_or(false);
            
            if !exists {
                self.db.execute(
                    "INSERT INTO settings (id, value, setting_type) VALUES (?, ?, ?)",
                    &[
                        &setting.id as &dyn duckdb::ToSql,
                        &setting.value,
                        &setting.setting_type.as_str(),
                    ],
                )?;
            }
        }
        Ok(())
    }

    pub fn get_setting(&self, id: &str) -> duckdb::Result<Option<Setting>> {
        let mut stmt = self.db.prepare(
            "SELECT id, value, setting_type FROM settings WHERE id = ?"
        )?;
        let rows = stmt.query_map(&[&id as &dyn duckdb::ToSql], |row| {
            let type_str: String = row.get(2)?;
            let setting_type = SettingType::from_str(&type_str)
                .unwrap_or(SettingType::String);
            
            Ok(Setting {
                id: row.get(0)?,
                value: row.get(1)?,
                setting_type,
            })
        })?;

        for row in rows {
            return Ok(Some(row?));
        }
        Ok(None)
    }

    pub fn get_all_settings(&self) -> duckdb::Result<Vec<Setting>> {
        let mut stmt = self.db.prepare(
            "SELECT id, value, setting_type FROM settings ORDER BY id"
        )?;
        let rows = stmt.query_map([], |row| {
            let type_str: String = row.get(2)?;
            let setting_type = SettingType::from_str(&type_str)
                .unwrap_or(SettingType::String);
            
            Ok(Setting {
                id: row.get(0)?,
                value: row.get(1)?,
                setting_type,
            })
        })?;

        let mut settings = Vec::new();
        for row in rows {
            settings.push(row?);
        }
        Ok(settings)
    }

    pub fn set_setting(&self, setting: &Setting) -> duckdb::Result<()> {
        self.db.execute(
            "INSERT OR REPLACE INTO settings (id, value, setting_type) VALUES (?, ?, ?)",
            &[
                &setting.id as &dyn duckdb::ToSql,
                &setting.value,
                &setting.setting_type.as_str(),
            ],
        )?;
        Ok(())
    }

    pub fn get_bedtime(&self) -> duckdb::Result<Option<NaiveTime>> {
        match self.get_setting(SETTING_BEDTIME)? {
            Some(s) => Ok(s.as_time()),
            None => Ok(None),
        }
    }

    pub fn get_caffeine_mg_at_bedtime(&self) -> duckdb::Result<Option<i32>> {
        match self.get_setting(SETTING_CAFFEINE_MG_AT_BEDTIME)? {
            Some(s) => Ok(s.as_int()),
            None => Ok(None),
        }
    }

    pub fn set_bedtime(&self, time: NaiveTime) -> duckdb::Result<()> {
        let mut setting = Setting::new(SETTING_BEDTIME, "", SettingType::Time);
        setting.set_time(time);
        self.set_setting(&setting)
    }

    pub fn set_caffeine_mg_at_bedtime(&self, mg: i32) -> duckdb::Result<()> {
        let mut setting = Setting::new(SETTING_CAFFEINE_MG_AT_BEDTIME, "", SettingType::Int);
        setting.set_int(mg);
        self.set_setting(&setting)
    }

    pub fn get_sync_remote_url(&self) -> duckdb::Result<Option<String>> {
        match self.get_setting(SETTING_SYNC_REMOTE_URL)? {
            Some(s) => {
                let url = s.as_string().trim().to_string();
                if url.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(url))
                }
            }
            None => Ok(None),
        }
    }

    pub fn set_sync_remote_url(&self, url: &str) -> duckdb::Result<()> {
        let mut setting = Setting::new(SETTING_SYNC_REMOTE_URL, url.trim(), SettingType::String);
        setting.value = url.trim().to_string();
        self.set_setting(&setting)
    }
}

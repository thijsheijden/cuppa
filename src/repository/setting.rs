use chrono::NaiveTime;

use crate::entity::setting::{Setting, SettingType, SETTING_BEDTIME, SETTING_CAFFEINE_MG_AT_BEDTIME, SETTING_SYNC_REMOTE_URL, SETTING_SYNC_LAST_SEQ};
use crate::repository::connection::open_db;
use crate::paths::db_path;

pub struct SettingRepository;

impl SettingRepository {
    pub fn new() -> rusqlite::Result<Self> {
        let repo = Self;
        repo.init_schema()?;
        repo.ensure_defaults()?;
        Ok(repo)
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        let conn = open_db(db_path())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                id TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                setting_type TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    /// Insert default settings if they don't already exist
    fn ensure_defaults(&self) -> rusqlite::Result<()> {
        let defaults = crate::entity::setting::default_settings();
        let conn = open_db(db_path())?;
        for setting in defaults {
            let exists: bool = conn.query_row(
                "SELECT 1 FROM settings WHERE id = ?1",
                [&setting.id],
                |_| Ok(true),
            ).unwrap_or(false);
            
            if !exists {
                conn.execute(
                    "INSERT INTO settings (id, value, setting_type) VALUES (?1, ?2, ?3)",
                    rusqlite::params![&setting.id, &setting.value, &setting.setting_type.as_str()],
                )?;
            }
        }
        Ok(())
    }

    pub fn get_setting(&self, id: &str) -> rusqlite::Result<Option<Setting>> {
        let conn = open_db(db_path())?;
        let mut stmt = conn.prepare(
            "SELECT id, value, setting_type FROM settings WHERE id = ?1"
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            let type_str: String = row.get(2)?;
            let setting_type = SettingType::from_str(&type_str)
                .unwrap_or(SettingType::String);
            return Ok(Some(Setting {
                id: row.get(0)?,
                value: row.get(1)?,
                setting_type,
            }));
        }
        Ok(None)
    }

    pub fn get_all_settings(&self) -> rusqlite::Result<Vec<Setting>> {
        let conn = open_db(db_path())?;
        let mut stmt = conn.prepare(
            "SELECT id, value, setting_type FROM settings ORDER BY id"
        )?;
        let mut rows = stmt.query([])?;
        let mut settings = Vec::new();
        while let Some(row) = rows.next()? {
            let type_str: String = row.get(2)?;
            let setting_type = SettingType::from_str(&type_str)
                .unwrap_or(SettingType::String);
            settings.push(Setting {
                id: row.get(0)?,
                value: row.get(1)?,
                setting_type,
            });
        }
        Ok(settings)
    }

    pub fn set_setting(&self, setting: &Setting) -> rusqlite::Result<()> {
        let conn = open_db(db_path())?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (id, value, setting_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![&setting.id, &setting.value, &setting.setting_type.as_str()],
        )?;
        Ok(())
    }

    pub fn get_bedtime(&self) -> rusqlite::Result<Option<NaiveTime>> {
        match self.get_setting(SETTING_BEDTIME)? {
            Some(s) => Ok(s.as_time()),
            None => Ok(None),
        }
    }

    pub fn get_caffeine_mg_at_bedtime(&self) -> rusqlite::Result<Option<i32>> {
        match self.get_setting(SETTING_CAFFEINE_MG_AT_BEDTIME)? {
            Some(s) => Ok(s.as_int()),
            None => Ok(None),
        }
    }

    pub fn set_bedtime(&self, time: NaiveTime) -> rusqlite::Result<()> {
        let mut setting = Setting::new(SETTING_BEDTIME, "", SettingType::Time);
        setting.set_time(time);
        self.set_setting(&setting)
    }

    pub fn set_caffeine_mg_at_bedtime(&self, mg: i32) -> rusqlite::Result<()> {
        let mut setting = Setting::new(SETTING_CAFFEINE_MG_AT_BEDTIME, "", SettingType::Int);
        setting.set_int(mg);
        self.set_setting(&setting)
    }

    pub fn get_sync_remote_url(&self) -> rusqlite::Result<Option<String>> {
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

    pub fn set_sync_remote_url(&self, url: &str) -> rusqlite::Result<()> {
        let mut setting = Setting::new(SETTING_SYNC_REMOTE_URL, url.trim(), SettingType::String);
        setting.value = url.trim().to_string();
        self.set_setting(&setting)
    }

    pub fn get_sync_last_seq(&self) -> rusqlite::Result<u64> {
        match self.get_setting(SETTING_SYNC_LAST_SEQ)? {
            Some(s) => Ok(s.as_int().unwrap_or(0) as u64),
            None => Ok(0),
        }
    }

    pub fn set_sync_last_seq(&self, seq: u64) -> rusqlite::Result<()> {
        let mut setting = Setting::new(SETTING_SYNC_LAST_SEQ, "", SettingType::Int);
        setting.set_int(seq as i32);
        self.set_setting(&setting)
    }
}

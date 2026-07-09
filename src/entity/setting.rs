use chrono::NaiveTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingType {
    Time,
    Int,
    String,
}

impl SettingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SettingType::Time => "time",
            SettingType::Int => "int",
            SettingType::String => "string",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "time" => Some(SettingType::Time),
            "int" => Some(SettingType::Int),
            "string" => Some(SettingType::String),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Setting {
    pub id: String,
    pub value: String,
    pub setting_type: SettingType,
}

impl Setting {
    pub fn new(id: impl Into<String>, value: impl Into<String>, setting_type: SettingType) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            setting_type,
        }
    }

    /// Parse the string value as a NaiveTime (for bedtime)
    pub fn as_time(&self) -> Option<NaiveTime> {
        if self.setting_type != SettingType::Time {
            return None;
        }
        NaiveTime::parse_from_str(&self.value, "%H:%M").ok()
    }

    /// Parse the string value as an i32 (for caffeine mg)
    pub fn as_int(&self) -> Option<i32> {
        if self.setting_type != SettingType::Int {
            return None;
        }
        self.value.parse().ok()
    }

    /// Get the raw string value
    pub fn as_string(&self) -> &str {
        &self.value
    }

    /// Set value from a NaiveTime
    pub fn set_time(&mut self, time: NaiveTime) {
        self.value = time.format("%H:%M").to_string();
        self.setting_type = SettingType::Time;
    }

    /// Set value from an i32
    pub fn set_int(&mut self, value: i32) {
        self.value = value.to_string();
        self.setting_type = SettingType::Int;
    }
}

/// Pre-defined setting IDs
pub const SETTING_BEDTIME: &str = "bedtime";
pub const SETTING_CAFFEINE_MG_AT_BEDTIME: &str = "caffeine_mg_at_bedtime";
pub const SETTING_SYNC_REMOTE_URL: &str = "sync_remote_url";
pub const SETTING_SYNC_LAST_SEQ: &str = "sync_last_seq";

/// Default settings
pub fn default_settings() -> Vec<Setting> {
    vec![
        Setting::new(SETTING_BEDTIME, "23:00", SettingType::Time),
        Setting::new(SETTING_CAFFEINE_MG_AT_BEDTIME, "50", SettingType::Int),
        Setting::new(SETTING_SYNC_REMOTE_URL, "", SettingType::String),
        Setting::new(SETTING_SYNC_LAST_SEQ, "0", SettingType::Int),
    ]
}

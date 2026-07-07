use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum SyncOp {
    #[serde(rename = "add")]
    AddDrink {
        drink_name: String,
        caffeine_mg: i32,
        consumed_at: DateTime<Utc>,
    },
    #[serde(rename = "delete")]
    DeleteDrink {
        drink_name: String,
        consumed_at: DateTime<Utc>,
    },
}

impl SyncOp {
    pub fn add_drink(drink_name: String, caffeine_mg: i32, consumed_at: DateTime<Utc>) -> Self {
        Self::AddDrink {
            drink_name,
            caffeine_mg,
            consumed_at,
        }
    }

    pub fn delete_drink(drink_name: String, consumed_at: DateTime<Utc>) -> Self {
        Self::DeleteDrink {
            drink_name,
            consumed_at,
        }
    }
}

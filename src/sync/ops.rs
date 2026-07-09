use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An operation to be applied to the local database during sync.
/// Produced by reading missing log entries from the sync repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum PendingOp {
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

impl PendingOp {
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

/// Operations recorded in the sync log. Same shape as `PendingOp` but kept
/// as a separate type to clarify the direction (outgoing vs incoming).
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

impl From<SyncOp> for PendingOp {
    fn from(op: SyncOp) -> Self {
        match op {
            SyncOp::AddDrink { drink_name, caffeine_mg, consumed_at } => {
                PendingOp::AddDrink { drink_name, caffeine_mg, consumed_at }
            }
            SyncOp::DeleteDrink { drink_name, consumed_at } => {
                PendingOp::DeleteDrink { drink_name, consumed_at }
            }
        }
    }
}

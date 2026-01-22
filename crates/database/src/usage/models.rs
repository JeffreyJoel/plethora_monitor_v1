//! # Usage Models
//!
//! Data models for usage tracking.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Monthly usage record for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub period_start: NaiveDate,
    pub monitors_count: i64,
    pub notifications_sent: i64,
    pub overage_notifications: i64,
}

impl Usage {
    /// Total notifications including overage
    pub fn total_notifications(&self) -> i64 {
        self.notifications_sent
    }
}

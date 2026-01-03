//! # Database Connection
//!
//! Database connection pool management and initialization.
//! Provides a wrapper around SQLx connection pools with configuration
//! for connection limits, timeouts, and lifecycle management.

use std::time::Duration;

use anyhow;
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::{ChannelRepository, MonitorRepository, UserRepository};

#[derive(Clone)]
pub struct DbPool {
    pub pool: PgPool,
    pub users: UserRepository,
    pub monitors: MonitorRepository,
    pub channels: ChannelRepository,
}

impl DbPool {
    pub async fn new(connection_url: &str) -> Result<Self, anyhow::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(3))
            .connect(connection_url)
            .await?;

        Ok(Self {
            pool: pool.clone(),
            users: UserRepository::new(pool.clone()),
            monitors: MonitorRepository::new(pool.clone()),
            channels: ChannelRepository::new(pool.clone()),
        })
    }
}

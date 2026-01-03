//! # Database Migrations
//!
//! Database schema migration utilities.
//! Handles running SQL migrations to set up and update the database schema.

use sqlx::migrate::Migrator;
use sqlx::postgres::PgPool;

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    println!("Running Database Migrations...");
    MIGRATOR.run(pool).await?;
    println!("Migrations Complete.");
    Ok(())
}

//! # User Repository
//!
//! Data access layer for user records.
//! Handles user creation, retrieval, and updates. Integrates with Clerk
//! authentication by storing Clerk IDs and syncing user data.

use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // get existing or create new user
    pub async fn get_or_create(
        &self,
        clerk_id: &str,
        email: Option<&str>,
    ) -> Result<Uuid, sqlx::Error> {
        // check if the user exists
        if let Some(id) = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE clerk_id = $1")
            .bind(clerk_id)
            .fetch_optional(&self.pool)
            .await?
        {
            return Ok(id);
        }

        // register new user
        let new_user_id: Uuid =
            sqlx::query_scalar("INSERT INTO users (clerk_id, email) VALUES ($1, $2) RETURNING id")
                .bind(clerk_id)
                .bind(email)
                .fetch_one(&self.pool)
                .await?;

        Ok(new_user_id)
    }

    // Update user profile
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        full_name: Option<&str>,
        profile_picture: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        // we use COALESCE to only update fields where Some(value) is provided.
        // if None is passed, COALESCE keeps the existing database value unchanged.

        sqlx::query(
            r#"
            UPDATE users
            SET
                full_name = COALESCE($1, full_name),
                profile_picture = COALESCE($2, profile_picture)
            WHERE id = $3
            "#,
        )
        .bind(full_name)
        .bind(profile_picture)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<(String, Option<String>, Option<String>)>, sqlx::Error> {
        let rec = sqlx::query("SELECT clerk_id, email, full_name FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = rec {
            let clerk_id: String = row.try_get("clerk_id")?;
            let email: Option<String> = row.try_get("email")?;
            let full_name: Option<String> = row.try_get("full_name")?;
            return Ok(Some((clerk_id, email, full_name)));
        }

        Ok(None)
    }
}

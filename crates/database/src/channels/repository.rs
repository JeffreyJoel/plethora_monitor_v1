//! # Channel Repository
//!
//! Data access layer for notification channel configurations.
//! Provides CRUD operations for storing and retrieving notification channel
//! settings (email, webhook, etc.) associated with users.

use chrono::{DateTime, Duration, Utc};
use notifications::primitives::models::{NotificationChannel, NotificationChannelType};
use sqlx::{PgPool, Row};
use utils::crypto;
use uuid::Uuid;

#[derive(Clone)]
pub struct ChannelRepository {
    pool: PgPool,
}

impl ChannelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Add a new notification channel.
    /// For email channels, generates a verification token and returns it.
    /// Returns (channel_id, optional_verification_token)
    pub async fn add_channel(
        &self,
        user_id: Uuid,
        channel: &NotificationChannel,
    ) -> Result<(Uuid, Option<String>), sqlx::Error> {
        let type_str = channel.type_.to_string();

        // Generate verification token only for email channels
        let (verified, verification_token, token_expires_at): (
            bool,
            Option<String>,
            Option<DateTime<Utc>>,
        ) = if channel.type_ == NotificationChannelType::Email {
            let token = crypto::generate_verification_token();
            let expires_at = Utc::now() + Duration::hours(24);
            (false, Some(token), Some(expires_at))
        } else {
            (true, None, None) // Auto-verify non-email channels
        };

        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO notification_channels (user_id, type, label, value, verified, verification_token, verification_token_expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(type_str)
        .bind(&channel.label)
        .bind(&channel.value)
        .bind(verified)
        .bind(&verification_token)
        .bind(token_expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok((id, verification_token))
    }

    /// Verify an email channel using a verification token.
    /// Returns the channel ID if verification was successful.
    pub async fn verify_channel(&self, token: &str) -> Result<Option<Uuid>, sqlx::Error> {
        let result = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE notification_channels 
            SET verified = true, verification_token = NULL, verification_token_expires_at = NULL
            WHERE verification_token = $1 
              AND verification_token_expires_at > NOW()
              AND verified = false
            RETURNING id
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Regenerate a verification token for an unverified email channel.
    /// Returns the new token if successful.
    pub async fn regenerate_verification_token(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<(String, String)>, sqlx::Error> {
        let token = crypto::generate_verification_token();
        let expires_at = Utc::now() + Duration::hours(24);

        // First get the email address
        let email: Option<String> = sqlx::query_scalar(
            r#"
            SELECT value FROM notification_channels 
            WHERE id = $1 AND user_id = $2 AND type = 'email' AND verified = false
            "#,
        )
        .bind(channel_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(email) = email else {
            return Ok(None);
        };

        let rows = sqlx::query(
            r#"
            UPDATE notification_channels 
            SET verification_token = $1, verification_token_expires_at = $2
            WHERE id = $3 AND user_id = $4 AND type = 'email' AND verified = false
            "#,
        )
        .bind(&token)
        .bind(expires_at)
        .bind(channel_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(if rows > 0 { Some((token, email)) } else { None })
    }

    pub async fn get_user_channels(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<NotificationChannel>, sqlx::Error> {
        let recs = sqlx::query(
            "SELECT id, type, label, value, verified FROM notification_channels WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut channels = Vec::with_capacity(recs.len());
        for row in recs {
            let id: Uuid = row.try_get("id")?;
            let type_str: String = row.try_get("type")?;
            let label: String = row.try_get("label")?;
            let value: String = row.try_get("value")?;
            let verified: bool = row.try_get("verified")?;

            channels.push(NotificationChannel {
                id: Some(id),
                type_: NotificationChannelType::try_from(type_str)
    .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?,
                label,
                value,
                verified,
            });
        }

        Ok(channels)
    }

    pub async fn get_channel_by_id(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<NotificationChannel>, sqlx::Error> {
        let rec = sqlx::query(
            "SELECT id, type, label, value, verified FROM notification_channels WHERE id = $1 AND user_id = $2",
        )
        .bind(channel_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = rec {
            let id: Uuid = row.try_get("id")?;
            let type_str: String = row.try_get("type")?;
            let label: String = row.try_get("label")?;
            let value: String = row.try_get("value")?;
            let verified: bool = row.try_get("verified")?;

            Ok(Some(NotificationChannel {
                id: Some(id),
                type_: NotificationChannelType::try_from(type_str)
    .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?,
                label,
                value,
                verified,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_channel(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        label: Option<String>,
        value: Option<String>,
    ) -> Result<(), sqlx::Error> {
        // Build dynamic update query based on which fields are provided
        let mut query = String::from("UPDATE notification_channels SET ");
        let mut updates = Vec::new();
        let mut bind_count = 1;

        if label.is_some() {
            updates.push(format!("label = ${}", bind_count));
            bind_count += 1;
        }

        if value.is_some() {
            updates.push(format!("value = ${}", bind_count));
            bind_count += 1;
        }

        if updates.is_empty() {
            // Nothing to update
            return Ok(());
        }

        query.push_str(&updates.join(", "));
        query.push_str(&format!(
            " WHERE id = ${} AND user_id = ${}",
            bind_count,
            bind_count + 1
        ));

        let mut q = sqlx::query(&query);

        if let Some(l) = label {
            q = q.bind(l);
        }
        if let Some(v) = value {
            q = q.bind(v);
        }

        q = q.bind(channel_id).bind(user_id);

        let result = q.execute(&self.pool).await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }

    pub async fn delete_channel(&self, channel_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM notification_channels WHERE id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

//! # Channel Repository
//!
//! Data access layer for notification channel configurations.
//! Provides CRUD operations for storing and retrieving notification channel
//! settings (email, webhook, etc.) associated with users.

use notifications::primitives::models::{NotificationChannel, NotificationChannelType};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct ChannelRepository {
    pool: PgPool,
}

impl ChannelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn add_channel(
        &self,
        user_id: Uuid,
        channel: &NotificationChannel,
    ) -> Result<Uuid, sqlx::Error> {
        let type_str = channel.type_.to_string();

        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO notification_channels (user_id, type, label, value)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(type_str)
        .bind(&channel.label)
        .bind(&channel.value)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn get_user_channels(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<NotificationChannel>, sqlx::Error> {
        let recs = sqlx::query(
            "SELECT id, type, label, value FROM notification_channels WHERE user_id = $1",
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

            channels.push(NotificationChannel {
                id: Some(id),
                type_: NotificationChannelType::try_from(type_str)
    .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?,
                label,
                value,
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
            "SELECT id, type, label, value FROM notification_channels WHERE id = $1 AND user_id = $2",
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

            Ok(Some(NotificationChannel {
                id: Some(id),
                type_: NotificationChannelType::try_from(type_str)
    .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))))?,
                label,
                value,
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

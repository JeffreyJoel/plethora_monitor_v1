use notifications::primitives::models::{ChannelType, NotificationChannel};
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
                type_: ChannelType::from(type_str),
                label,
                value,
            });
        }

        Ok(channels)
    }
}

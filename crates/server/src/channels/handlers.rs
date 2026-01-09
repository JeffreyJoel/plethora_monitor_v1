//! # Channel Handlers
//!
//! HTTP request handlers for notification channel management endpoints.
//! Implements CRUD operations for notification channels (email, webhook, etc.)
//! that are used to deliver alerts from monitors.
//!
//! ## Usage
//!
//! Channels are referenced by ID when creating monitors. When a monitor
//! detects a matching transaction or event, it sends alerts to the
//! configured notification channel.

use crate::channels::models::{ChannelResponse, CreateChannelRequest, UpdateChannelRequest};
use crate::state::AppState;
use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use clerk_rs::validators::authorizer::ClerkJwt;
use database::UserRepository;
use notifications::primitives::models::{NotificationChannel, NotificationChannelType};
use std::sync::Arc;
use utils::crypto;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/channels",
    request_body = CreateChannelRequest,
    responses(
        (status = 200, description = "Channel created successfully", body = ChannelResponse),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "channels"
)]
pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Json(payload): Json<CreateChannelRequest>,
) -> Result<Json<ChannelResponse>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo.get_or_create(&jwt.sub, None).await.map_err(|e| {
        tracing::error!("Failed to get user: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let channel_type = NotificationChannelType::try_from(payload.type_.clone()).map_err(|e| {
        tracing::error!("Invalid channel type: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let value_to_store = if payload.type_ == "telegram" || payload.type_ == "discord" {
        crypto::encrypt(&payload.value)
    } else {
        payload.value
    };

    let channel = NotificationChannel {
        id: None,
        type_: channel_type,
        label: payload.label,
        value: value_to_store,
    };

    let channel_id = state
        .db
        .channels
        .add_channel(user_id, &channel)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create channel: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ChannelResponse {
        id: channel_id.to_string(),
        type_: channel.type_.to_string(),
        label: channel.label,
        value: channel.value,
    }))
}

#[utoipa::path(
    get,
    path = "/api/channels",
    responses(
        (status = 200, description = "List of channels", body = Vec<ChannelResponse>),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "channels"
)]
pub async fn get_channels(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
) -> Result<Json<Vec<ChannelResponse>>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let channels = state
        .db
        .channels
        .get_user_channels(user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch channels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response: Vec<ChannelResponse> = channels
        .into_iter()
        .map(|c| ChannelResponse {
            id: c.id.map(|id| id.to_string()).unwrap_or_default(),
            type_: c.type_.to_string(),
            label: c.label,
            value: c.value,
        })
        .collect();

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/channels/{id}",
    params(
        ("id" = Uuid, Path, description = "Channel ID")
    ),
    responses(
        (status = 200, description = "Channel details", body = ChannelResponse),
        (status = 404, description = "Channel not found"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "channels"
)]
pub async fn get_channel(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<ChannelResponse>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let channel = state
        .db
        .channels
        .get_channel_by_id(channel_id, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match channel {
        Some(c) => Ok(Json(ChannelResponse {
            id: c.id.map(|id| id.to_string()).unwrap_or_default(),
            type_: c.type_.to_string(),
            label: c.label,
            value: c.value,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
    put,
    path = "/api/channels/{id}",
    params(
        ("id" = Uuid, Path, description = "Channel ID")
    ),
    request_body = UpdateChannelRequest,
    responses(
        (status = 200, description = "Channel updated successfully", body = ChannelResponse),
        (status = 404, description = "Channel not found"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "channels"
)]
pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<UpdateChannelRequest>,
) -> Result<Json<ChannelResponse>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // First, verify the channel exists and belongs to the user
    let existing_channel = state
        .db
        .channels
        .get_channel_by_id(channel_id, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(existing_channel) = existing_channel else {
        return Err(StatusCode::NOT_FOUND);
    };

    let value_to_store = payload.value.clone().map(|v| match existing_channel.type_ {
        NotificationChannelType::Telegram | NotificationChannelType::Discord => crypto::encrypt(&v),
        _ => v,
    });

    // Update the channel
    state
        .db
        .channels
        .update_channel(channel_id, user_id, payload.label.clone(), value_to_store)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update channel: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Fetch the updated channel to return
    let updated_channel = state
        .db
        .channels
        .get_channel_by_id(channel_id, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ChannelResponse {
        id: updated_channel
            .id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        type_: updated_channel.type_.to_string(),
        label: updated_channel.label,
        value: updated_channel.value,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/channels/{id}",
    params(
        ("id" = Uuid, Path, description = "Channel ID")
    ),
    responses(
        (status = 204, description = "Channel deleted successfully"),
        (status = 404, description = "Channel not found"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "channels"
)]
pub async fn delete_channel(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Path(channel_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let channel = state
        .db
        .channels
        .get_channel_by_id(channel_id, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if channel.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    state
        .db
        .channels
        .delete_channel(channel_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete channel: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::NO_CONTENT)
}

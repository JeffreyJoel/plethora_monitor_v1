//! # Monitor Handlers
//!
//! HTTP request handlers for monitor management endpoints.
//! Implements CRUD operations for blockchain monitors, including creation,
//! retrieval, updates, deletion, and automatic restoration on server startup.
//!
//! ## Key Features
//!
//! - **Monitor Creation**: Validates configuration, fetches ABI, and starts monitoring
//! - **Monitor Restoration**: Automatically restores active monitors on server startup
//! - **Dynamic Updates**: Allows updating monitor rules without downtime
//! - **User Isolation**: Ensures users can only access their own monitors
//!
//! ## Workflow
//!
//! When creating a monitor:
//!
//! 1. Validates user authentication
//! 2. Saves monitor configuration to database
//! 3. Fetches contract ABI from block explorer
//! 4. Maps user rules to ABI functions/events
//! 5. Creates and starts the monitoring engine
//! 6. Stores the monitor task handle in application state

use crate::{
    monitors::models::{CreateMonitorResponse, MonitorResponse},
    state::AppState,
};
use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use clerk_rs::validators::authorizer::ClerkJwt;
use database::{MonitorRepository, ToDestination, UserRepository};
use monitor::events::map_event_rules_to_abi;
use monitor::primitives::utils::fetch_abi;
use monitor::tx::map_rules_to_abi;
use monitor::{PollingMonitor, primitives::models::MonitorConfig};
use std::sync::Arc;
use tracing::{error, warn, info};
use uuid::Uuid;

/// Restores all active monitors from the database on server startup
pub async fn restore_monitors(state: Arc<AppState>) {
    let monitor_repo = MonitorRepository::new(state.db.pool.clone());
    
    let monitors = match monitor_repo.get_all_active().await {
        Ok(m) => m,
        Err(e) => {
            error!("Failed to fetch active monitors for restoration: {}", e);
            return;
        }
    };

    if monitors.is_empty() {
        info!("No active monitors to restore");
        return;
    }

    info!("Restoring {} active monitor(s) from database...", monitors.len());

    let mut restored_count = 0;
    let mut failed_count = 0;

    for (monitor_id, user_id, config) in monitors {
        let monitor_id_str = monitor_id.to_string();

        let rpc_url = if config.rpc_url.is_empty() {
            state.default_rpc_url.clone()
        } else {
            config.rpc_url.clone()
        };

        // Fetch ABI - if this fails, skip the monitor
        let abi = match fetch_abi(&config.chain, config.address, &rpc_url).await {
            Ok(abi) => abi,
            Err(e) => {
                warn!(
                    "Failed to fetch ABI for monitor {} ({}): {}. Skipping restoration.",
                    monitor_id, config.name, e
                );
                failed_count += 1;
                continue;
            }
        };

        let function_rules = map_rules_to_abi(config.function_rules.clone().unwrap_or_default(), &abi);
        let event_rules = map_event_rules_to_abi(config.event_rules.clone().unwrap_or_default(), &abi);

        // Fetch notification channel if configured
        let notification_destination = if let Some(channel_id) = config.notification_channel_id {
            match state.db.channels.get_channel_by_id(channel_id, user_id).await {
                Ok(Some(channel)) => channel.to_destination(),
                Ok(None) => {
                    warn!(
                        "Notification channel {} not found for monitor {}. Continuing without notifications.",
                        channel_id, monitor_id
                    );
                    None
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch notification channel {} for monitor {}: {}. Continuing without notifications.",
                        channel_id, monitor_id, e
                    );
                    None
                }
            }
        } else {
            None
        };

        // Create and start the monitor
        match PollingMonitor::new(&rpc_url, config.address, abi) {
            Ok(monitor_engine) => {
                let handle = monitor_engine.start_background_monitoring(
                    config.name.clone(),
                    function_rules,
                    event_rules,
                    notification_destination,
                );

                state
                    .active_monitors
                    .write()
                    .await
                    .insert(monitor_id_str, handle);

                info!("✓ Restored monitor: {} ({})", config.name, monitor_id);
                restored_count += 1;
            }
            Err(e) => {
                error!(
                    "Failed to create monitor engine for {} ({}): {}. Skipping.",
                    config.name, monitor_id, e
                );
                failed_count += 1;
            }
        }
    }

    info!(
        "Monitor restoration complete: {} restored, {} failed",
        restored_count, failed_count
    );
}

// POST /api/monitors

#[utoipa::path(
    post,
    path = "/api/monitors",
    request_body = MonitorConfig,
    responses(
        (status = 200, description = "Monitor created successfully", body = CreateMonitorResponse),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "monitors"
)]
pub async fn create_monitor(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Json(payload): Json<MonitorConfig>,
) -> Result<Json<CreateMonitorResponse>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_uuid = user_repo.get_or_create(&jwt.sub, None).await.map_err(|e| {
        tracing::error!("DB User Error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let monitor_repo = MonitorRepository::new(state.db.pool.clone());
    let monitor_id = monitor_repo
        .create(user_uuid, &payload)
        .await
        .map_err(|e| {
            tracing::error!("DB Monitor Error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let monitor_id_str = monitor_id.to_string();

    let rpc_url = if payload.rpc_url.is_empty() {
        state.default_rpc_url.clone()
    } else {
        payload.rpc_url.clone()
    };

    // If fetching ABI fails, the monitor is saved but won't start running.
    // TODO: update endpoint to allow the user upload a custom ABI or Not?
    // will come back to this.
    let abi = fetch_abi(&payload.chain, payload.address, &rpc_url)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let function_rules = map_rules_to_abi(payload.function_rules.unwrap_or_default(), &abi);
    let event_rules = map_event_rules_to_abi(payload.event_rules.unwrap_or_default(), &abi);

    let notification_destination = if let Some(channel_id) = payload.notification_channel_id {
        match state
            .db
            .channels
            .get_channel_by_id(channel_id, user_uuid)
            .await
        {
            Ok(Some(channel)) => channel.to_destination(),
            Ok(None) => {
                tracing::warn!("Notification channel {} not found", channel_id);
                None
            }
            Err(e) => {
                tracing::error!("Failed to fetch notification channel: {}", e);
                None
            }
        }
    } else {
        None
    };

    let monitor_engine = PollingMonitor::new(&rpc_url, payload.address, abi)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let handle = monitor_engine.start_background_monitoring(
        payload.name,
        function_rules,
        event_rules,
        notification_destination,
    );

    state
        .active_monitors
        .write()
        .await
        .insert(monitor_id_str.clone(), handle);

    Ok(Json(CreateMonitorResponse {
        id: monitor_id_str,
        status: "Running".to_string(),
    }))
}

// GET /api/monitors
#[utoipa::path(
    get,
    path = "/api/monitors",
    responses(
        (status = 200, description = "List of monitors", body = Vec<MonitorResponse>),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "monitors"
)]
pub async fn get_monitors(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
) -> Result<Json<Vec<MonitorResponse>>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let monitor_repo = MonitorRepository::new(state.db.pool.clone());

    let monitors = monitor_repo
        .get_by_user(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = monitors
        .into_iter()
        .map(|(id, config)| MonitorResponse { id, config })
        .collect();

    Ok(Json(response))
}

// GET /api/monitors/:id
#[utoipa::path(
    get,
    path = "/api/monitors/{id}",
    params(
        ("id" = Uuid, Path, description = "Monitor ID")
    ),
    responses(
        (status = 200, description = "Monitor configuration", body = MonitorConfig),
        (status = 404, description = "Monitor not found"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "monitors"
)]
pub async fn get_monitor(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Path(monitor_id): Path<Uuid>,
) -> Result<Json<MonitorConfig>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let monitor_repo = MonitorRepository::new(state.db.pool.clone());

    match monitor_repo.get_by_id(monitor_id, user_id).await {
        Ok(Some(config)) => Ok(Json(config)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// PUT /api/monitors/:id
#[utoipa::path(
    put,
    path = "/api/monitors/{id}",
    params(
        ("id" = Uuid, Path, description = "Monitor ID")
    ),
    request_body = MonitorConfig,
    responses(
        (status = 200, description = "Monitor updated successfully", body = serde_json::Value),
        (status = 404, description = "Monitor not found"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "monitors"
)]
pub async fn update_monitor(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Path(monitor_id): Path<Uuid>,
    Json(payload): Json<MonitorConfig>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // update DB
    let monitor_repo = MonitorRepository::new(state.db.pool.clone());
    monitor_repo
        .update(monitor_id, user_id, &payload)
        .await
        .map_err(|e| {
            tracing::error!("Update failed: {}", e);
            if let sqlx::Error::RowNotFound = e {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    {
        let mut monitors = state.active_monitors.write().await;
        if let Some(handle) = monitors.remove(&monitor_id.to_string()) {
            handle.abort();
            println!("Stopped monitor task: {}", monitor_id);
        }
    }

    let rpc_url = if payload.rpc_url.is_empty() {
        state.default_rpc_url.clone()
    } else {
        payload.rpc_url.clone()
    };

    if let Ok(abi) = fetch_abi(&payload.chain, payload.address, &rpc_url).await {
        let function_rules = map_rules_to_abi(payload.function_rules.unwrap_or_default(), &abi);
        let event_rules = map_event_rules_to_abi(payload.event_rules.unwrap_or_default(), &abi);

        let notification_destination = if let Some(channel_id) = payload.notification_channel_id {
            match state
                .db
                .channels
                .get_channel_by_id(channel_id, user_id)
                .await
            {
                Ok(Some(channel)) => channel.to_destination(),
                Ok(None) => {
                    tracing::warn!("Notification channel {} not found", channel_id);
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to fetch notification channel: {}", e);
                    None
                }
            }
        } else {
            None
        };

        if let Ok(engine) = PollingMonitor::new(&rpc_url, payload.address, abi) {
            let new_handle = engine.start_background_monitoring(
                payload.name,
                function_rules,
                event_rules,
                notification_destination,
            );

            state
                .active_monitors
                .write()
                .await
                .insert(monitor_id.to_string(), new_handle);
            println!("Restarted monitor task: {}", monitor_id);
        }
    }

    Ok(Json(
        serde_json::json!({ "status": "updated", "id": monitor_id }),
    ))
}

// DELETE /api/monitors/:id
#[utoipa::path(
    delete,
    path = "/api/monitors/{id}",
    params(
        ("id" = Uuid, Path, description = "Monitor ID")
    ),
    responses(
        (status = 204, description = "Monitor deleted successfully"),
        (status = 404, description = "Monitor not found"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "monitors"
)]
pub async fn delete_monitor(
    State(state): State<Arc<AppState>>,
    Extension(jwt): Extension<ClerkJwt>,
    Path(monitor_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let user_repo = UserRepository::new(state.db.pool.clone());
    let user_id = user_repo
        .get_or_create(&jwt.sub, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Remove monitor from hashmap first
    {
        let mut monitors = state.active_monitors.write().await;
        if let Some(handle) = monitors.remove(&monitor_id.to_string()) {
            handle.abort();
            println!("Stopped monitor task: {}", monitor_id);
        }
    }

    // Then delete from database
    let monitor_repo = MonitorRepository::new(state.db.pool.clone());
    monitor_repo
        .delete(monitor_id, user_id)
        .await
        .map_err(|e| {
            if let sqlx::Error::RowNotFound = e {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

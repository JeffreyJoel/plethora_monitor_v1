use crate::{monitors::models::CreateMonitorResponse, state::AppState};
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
use uuid::Uuid;

// POST /api/monitors
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

// GET /api/monitors/:id
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

    {
        let mut monitors = state.active_monitors.write().await;
        if let Some(handle) = monitors.remove(&monitor_id.to_string()) {
            handle.abort();
            println!("Deleted monitor task: {}", monitor_id);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

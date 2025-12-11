use crate::state::AppState;
use axum::{Json, extract::State, http::StatusCode};
use config::MonitorConfig;
use monitor::PollingMonitor;
use monitor::primitives::utils::fetch_abi;
use monitor::tx::map_rules_to_abi;
use std::sync::Arc;
use uuid::Uuid;

#[derive(serde::Serialize)]
pub struct CreateMonitorResponse {
    pub id: String,
    pub status: String,
}

pub async fn create_monitor(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<MonitorConfig>,
) -> Result<Json<CreateMonitorResponse>, StatusCode> {
    let monitor_id = Uuid::new_v4().to_string();
    let rpc_url = if payload.rpc_url.is_empty() {
        state.default_rpc_url.clone()
    } else {
        payload.rpc_url.clone()
    };

    println!("Creating Monitor '{}' [{}]", payload.name, monitor_id);

    // fetch ABI
    let abi = fetch_abi(&payload.chain, &payload.address, &rpc_url)
        .await
        .map_err(|e| {
            eprintln!("❌ ABI Error: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    //  prepare Rules
    let tx_rules = map_rules_to_abi(payload.functions.unwrap_or_default(), &abi);
    let event_names = payload.events.unwrap_or_default();

    let contract_addr = payload
        .address
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let monitor = PollingMonitor::new(&rpc_url, contract_addr, abi)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let handle = monitor.start_background_monitoring(payload.name, tx_rules, event_names);

    state
        .active_monitors
        .write()
        .await
        .insert(monitor_id.clone(), handle);

    Ok(Json(CreateMonitorResponse {
        id: monitor_id,
        status: "Running".to_string(),
    }))
}

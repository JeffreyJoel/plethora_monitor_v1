use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct AppState {
    // maps a uuid to a task handler
    pub active_monitors: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    pub default_rpc_url: String,
}

impl AppState {
    pub fn new(default_rpc: String) -> Self {
        Self {
            active_monitors: Arc::new(RwLock::new(HashMap::new())),
            default_rpc_url: default_rpc,
        }
    }
}

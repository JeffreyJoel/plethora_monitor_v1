use clerk_rs::clerk::Clerk;
use database::DbPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct AppState {
    pub active_monitors: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    pub db: DbPool,
    pub clerk: Clerk,
    pub default_rpc_url: String,
}

impl AppState {
    pub fn new(default_rpc: String, db: DbPool, clerk: Clerk) -> Self {
        Self {
            active_monitors: Arc::new(RwLock::new(HashMap::new())),
            db,
            clerk,
            default_rpc_url: default_rpc,
        }
    }
}

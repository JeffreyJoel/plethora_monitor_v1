//! # Application State
//!
//! Global application state shared across all request handlers.
//! Maintains active monitor instances, database connections, and
//! authentication client.
//!
//! ## State Components
//!
//! - **`active_monitors`**: Thread-safe map of monitor ID to background task handles.
//!   Used to track running monitors and allow graceful shutdown/updates.
//! - **`db`**: Database connection pool for persistence operations.
//! - **`clerk`**: Clerk authentication client for JWT validation.
//! - **`block_watcher_registry`**: Shared registry for BlockWatcher instances per chain.
//!   Reduces RPC calls by consolidating block polling across multiple monitors.
//!
//! ## Thread Safety
//!
//! All state is designed to be safely shared across async tasks:
//! - `Arc` for reference counting across threads
//! - `RwLock` for concurrent read access to active monitors
//! - Database pool handles connection management internally
//! - BlockWatcherRegistry uses internal RwLock for thread-safe access

use clerk_rs::clerk::Clerk;
use database::DbPool;
use monitor::BlockWatcherRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct AppState {
    pub active_monitors: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    pub db: DbPool,
    pub clerk: Clerk,
    pub block_watcher_registry: Arc<BlockWatcherRegistry>,
}

impl AppState {
    pub fn new(db: DbPool, clerk: Clerk) -> Self {
        Self {
            active_monitors: Arc::new(RwLock::new(HashMap::new())),
            db,
            clerk,
            block_watcher_registry: Arc::new(BlockWatcherRegistry::new()),
        }
    }
}

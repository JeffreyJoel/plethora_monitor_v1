//! # Database Crate
//!
//! Provides database access layer for Plethora Monitor.
//! Handles all persistence operations using SQLx with PostgreSQL/CockroachDB.
//!
//! ## Responsibilities
//!
//! - **Connection Management**: Database connection pooling and lifecycle
//! - **Repository Pattern**: Data access objects for each entity type
//! - **Migrations**: Database schema versioning and migration execution
//! - **Type Safety**: Leverages SQLx compile-time query checking
//!
//! ## Modules
//!
//! - **`connection`** - Database pool initialization and connection management
//! - **`migrations`** - Schema migration utilities
//! - **`users`** - User repository for authentication and profile data
//! - **`monitors`** - Monitor configuration persistence
//! - **`channels`** - Notification channel storage
//!
//! ## Data Models
//!
//! The database stores:
//!
//! - **Users**: Clerk ID, email, and profile information
//! - **Monitors**: Monitor configurations (stored as JSONB for flexibility)
//! - **Channels**: Notification channel configurations (email, webhook, etc.)
//!
//! ## Usage
//!
//! ```no_run
//! use database::{DbPool, MonitorRepository};
//!
//! let pool = DbPool::new(&database_url).await?;
//! let repo = MonitorRepository::new(pool.pool.clone());
//! let monitors = repo.get_by_user(user_id).await?;
//! ```

pub mod channels;
pub mod connection;
pub mod migrations;
pub mod monitors;
pub mod users;

// re-export specific items for cleaner imports
pub use channels::repository::ChannelRepository;
pub use connection::DbPool;
pub use migrations::run_migrations;
pub use monitors::repository::MonitorRepository;
pub use users::repository::UserRepository;

// re-export notification traits
pub use notifications::ToDestination;

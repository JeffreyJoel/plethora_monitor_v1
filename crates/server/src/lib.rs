//! # Server Crate
//!
//! The server crate provides the REST API layer for Plethora Monitor.
//! It handles HTTP requests, authentication, routing, and coordinates
//! between the monitoring engine, database, and notification systems.
//!
//! ## Architecture
//!
//! This crate follows a modular structure:
//!
//! - **`router`** - HTTP routing and middleware configuration
//! - **`state`** - Global application state management
//! - **`middleware`** - Authentication and request processing middleware
//! - **`monitors`** - Monitor lifecycle management (create, update, delete, list)
//! - **`users`** - User registration and profile management
//! - **`channels`** - Notification channel management (email, webhook, etc.)
//! - **`docs`** - OpenAPI/Swagger documentation generation
//!
//! ## Key Responsibilities
//!
//! 1. **API Endpoints**: Exposes RESTful endpoints for monitor management
//! 2. **Authentication**: Integrates with Clerk for JWT-based authentication
//! 3. **State Management**: Maintains active monitor instances in memory
//! 4. **Request Validation**: Validates and processes incoming API requests
//! 5. **Error Handling**: Provides consistent error responses
//!
//! ## Usage
//!
//! The server is typically started via the `plethora_monitor` binary,
//! which initializes the application state and starts the HTTP server.

pub mod billing;
pub mod channels;
pub mod docs;
pub mod middleware;
pub mod monitors;
pub mod notification_service;
pub mod router;
pub mod state;
pub mod users;

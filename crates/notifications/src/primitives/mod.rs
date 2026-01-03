//! # Notification Primitives
//!
//! Core data structures for the notification system.
//!
//! ## Models
//!
//! - **`Alert`** - Structured alert data containing source, subject, and message
//! - **`NotificationDestination`** - Enum representing different delivery methods
//! - **`NotificationChannel`** - Database model for stored channel configurations
//! - **`NotificationChannelType`** - Enum of supported channel types
//!
//! These types provide type-safe abstractions for the notification system,
//! allowing different delivery mechanisms to be handled uniformly.

pub mod models;

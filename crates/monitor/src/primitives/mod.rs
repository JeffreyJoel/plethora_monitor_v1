//! # Monitor Primitives
//!
//! Core data structures and utilities for the monitor crate.
//! Contains shared types, models, and helper functions used throughout
//! the monitoring system.
//!
//! ## Modules
//!
//! - **`models`** - Core data structures:
//!   - `MonitorConfig` - Complete monitor configuration
//!   - `MonitorRule` - Transaction/event matching rules
//!   - `Condition` - Rule conditions (from/to addresses, function calls, arguments)
//!   - `Operator` - Comparison operators for argument matching
//!
//! - **`utils`** - Utility functions:
//!   - `fetch_abi` - Retrieves contract ABIs from block explorers
//!   - `format_value` - Formats decoded ABI values for display
//!   - `check_argument_condition` - Evaluates rule conditions against decoded values
//!
//! - **`chains`** - Chain configuration:
//!   - `get_default_rpc_url` - Returns default public RPC URL for a chain
//!   - `get_chain_id` - Returns chain ID for a chain name
//!   - `supported_chains` - Lists all supported chain names
//!
//! ## Key Concepts
//!
//! **Monitor Rules**: User-defined patterns that specify which transactions
//! or events should trigger alerts. Rules can match on:
//! - Function names
//! - Event signatures
//! - Transaction participants (from/to addresses)
//! - Function/event arguments with operators (equals, greater than, contains, etc.)

pub mod chains;
pub mod models;
pub mod utils;

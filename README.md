# Plethora Monitor

🚧 *Under active development.* 🚧

Plethora Monitor is a high-performance, dynamic EVM blockchain monitoring agent written in Rust. It is designed to track smart contract events and transactions in real-time, allowing users to spawn, manage, and supervise monitoring tasks dynamically via a REST API.

## Architecture Overview
   
   1.  **`crates/server` (The Brain)**:
   
         - Exposes the REST API using `axum`.
         - Manages Global State (`AppState`) via `RwLock` to track active monitor handles.
         - Handles **Rule Hydration**: Fetches ABIs from Etherscan/Block Explorers to convert human-readable config (e.g., `"transfer"`) into machine-executable logic (Selectors & Decoders).
   
   2.  **`crates/monitor` (The Muscle)**:
   
         - **`PollingMonitor`**: The core engine that maintains the RPC connection.
         - **`TransactionMonitor`**: Scans blocks for transactions matching specific rules.
         - **`EventMonitor`**: Scans logs for specific event signatures.
         - **`primitives/models.rs`**: Defines the shared DTOs (`MonitorRule`, `Condition`) used by both the config and the logic engine.
   
   3.  **`crates/config` (The Contract)**:
   
         - Defines the data structures for configuration, acting as the interface between user input (JSON/TOML) and system logic.



## To-Do List

### Completed
- [x] Workspace Setup: Modular crate structure (bin, crates/monitor, crates/server, crates/config).
- [x] Core Engine: Implemented PollingMonitor with resilient loop logic.
- [x] Rule Engine: Developed filter.rs for dynamic ABI decoding and argument comparison.


 ### Remaining
  - [ ] **Implement monitor endpoints**: Implement logic and expose endpoints to create, update and delete monitors.
  - [ ] **Implement user endpoints**: Implement user logic and map users to monitors
  - [ ] **Webhooks**: Implement HTTP POST callbacks to notify users when a rule matches (currently prints to console).
  - [ ] **Persistence**: Save `active_monitors` state to disk (DB) to survive server restarts.
  - [ ] **Metrics**: Add Prometheus metrics for blocks processed and latency.
  - [ ] **Tests**: Write unit tests for the Rule Engine decoding logic.

## Getting Started
   
   ## Getting Started
   
   ### Prerequisites
   
     - Rust (edition 2024)
     - Cargo
     - An Etherscan/Basescan API Key (for ABI fetching)
   
   ### Installation
   
   1.  Clone the repository:
       ```bash
       git clone <repository-url>
       cd plethora_monitor
       ```
   2.  Set up your environment:
       ```bash
       cp .env.example .env
       # Add your ETHERSCAN_API_KEY in .env
       ```
   3.  Run the project:
       ```bash
       cargo run --bin plethora_monitor
       ```

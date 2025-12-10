# Plethora Monitor

🚧 *Under active development.* 🚧

Plethora Monitor is a high-performance, dynamic EVM blockchain monitoring agent written in Rust. It is designed to track smart contract events and transactions in real-time, allowing users to spawn, manage, and supervise monitoring tasks dynamically via a REST API.

## Features

- **Modular Design**: The project is divided into `config`, `monitor`, and `server` crates for better maintainability and scalability.
- **Asynchronous Programming**: Built with `tokio` and `futures` for high-performance asynchronous operations.
- **Web Server**: Powered by `axum` for handling HTTP requests.
- **Configuration Management**: Utilizes `config` and `dotenvy` for flexible configuration handling.
- **Blockchain Integration**: Includes dependencies like `alloy` and `foundry-block-explorers` for blockchain-related functionalities.
- **Serialization**: Uses `serde` and `serde_json` for data serialization and deserialization.

## Crate Overview

1. **Config**: Handles application configuration.
2. **Monitor**: Core monitoring logic and utilities.
3. **Server**: Web server for exposing APIs and managing requests.



## To-Do List

### Completed
- [x] Workspace Setup: Modular crate structure (bin, crates/monitor, crates/server, crates/config).
- [x] Core Engine: Implemented PollingMonitor with resilient loop logic.
- [x] Rule Engine: Developed filter.rs for dynamic ABI decoding and argument comparison.
- [x] API layer `server` crate for web server functionalities.

 ### Remaining
     - [ ] **Implement endpoints**: Implement logic and expose endpoints to create, update and delete monitors.
     - [ ] **Webhooks**: Implement HTTP POST callbacks to notify users when a rule matches (currently prints to console).
     - [ ] **Persistence**: Save `active_monitors` state to disk (DB or File) to survive server restarts.
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

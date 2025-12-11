# Plethora Monitor

🚧 *Under active development.* 🚧

Plethora Monitor is a high-performance, dynamic EVM blockchain monitoring agent written in Rust. It is designed to track smart contract events and transactions in real-time, allowing users to spawn, manage, and supervise monitoring tasks dynamically via a REST API.

## Architecture Overview
   
   1.  **`crates/server` (The Brain)**:
   
         - Exposes the REST API using `axum`.
         - Manages Global State (`AppState`) via `RwLock` to track active monitor handles.
         - Handles **Rule Hydration**: Fetches ABIs from Etherscan/Block Explorers to convert human-readable config (e.g., `"transfer"`) into machine-executable logic (Selectors & Decoders).
         - Implements endpoints to create, update, and delete monitors dynamically.
         - Maps users to monitors for better organization and access control.
   
   2.  **`crates/monitor` (The Muscle)**:
   
         - **`PollingMonitor`**: The core engine that maintains the RPC connection.
         - **`TransactionMonitor`**: Scans blocks for transactions matching specific rules.
         - **`EventMonitor`**: Scans logs for specific event signatures.
         - **`primitives/models.rs`**: Defines the shared DTOs (`MonitorRule`, `Condition`) used by both the config and the logic engine.
         - Implements HTTP POST webhooks to notify users when a rule matches.

3.  **`crates/server` (The API layer)**:
   
         - Manages the REST API endpoints for creating, updating, and deleting monitors.
         - Handles user authentication and mapping users to their respective monitors.
         - Maintains the global application state (`AppState`) to track active monitors and their configurations.
         - Implements graceful shutdown and state cleanup to ensure reliability.

4.  **`crates/notifications` (The Messenger)**:
   
         - Handles all notification-related functionality, including sending email alerts and webhooks.
         - Supports HTTP POST callbacks to notify users when a rule matches.
         - Provides extensibility for adding new notification channels (e.g., SMS, Slack, etc.).
         - Ensures reliable delivery of notifications with retry mechanisms for transient failures.




## To-Do List

### Completed
- [x] Workspace Setup: Modular crate structure (bin, crates/monitor, crates/server, crates/config).
- [x] Core Engine: Implemented PollingMonitor with resilient loop logic.
- [x] Rule Engine: Developed filter.rs for dynamic ABI decoding and argument comparison.
- [x] Monitor Endpoints: Implemented logic and exposed endpoints to create monitors.
- [x] Notification: Implemented email notification to alert the user when conditions have been met

### Remaining
  - [ ] **Persistence**: Finalize saving `active_monitors` state to database.
  - [ ] **User Endpoints**: Implemented user logic and mapped users to monitors.
  - [ ] **Metrics**: Add Prometheus metrics for blocks processed and latency.
  - [ ] **Tests**: Write unit tests for the Rule Engine decoding logic.

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
       # Add your ETHERSCAN_API_KEY and BREVO_API_KEY in .env
       ```
   3.  Run the project:
       ```bash
       cargo run --bin plethora_monitor
       ```

## Example: Test Monitor Request

Send a POST request to /monitors to spawn a new monitor.

```json
{
  "name": "USDC Whale Watcher",
  "chain": "base-sepolia",
  "address": "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
  "rpc_url": "https://sepolia.base.org", 
  "email_recipient": "<YOUR_EMAIL>@gmail.com",
  "functions": [
    {
      "name": "Large Transfer Alert",
      "conditions": [
        {
          "Function": "transfer"
        },
        {
          "From": "<YOUR_ADDRESS>"
        }
      ]
    }
  ]
}

# Plethora Monitor

#RustAfricaHackathon

🚀 **[Try the Live Demo](https://plethora-monitor-ui.vercel.app/)** 🚀

🚧 _Under active development._ 🚧

Plethora Monitor is a high-performance, dynamic smart contract monitoring agent written in Rust. It is designed to track smart contract events and transactions in real-time, allowing users to spawn, manage, and supervise monitoring tasks dynamically via a REST API.

## Architecture Overview

1.  **`crates/server` (The Brain)**:
    - Exposes the REST API using `axum`.
    - Manages Global State (`AppState`).
    - Integrates with **Clerk** for authentication.
    - Handles **Rule Hydration**: Fetches ABIs from Etherscan/Block Explorers to convert human-readable config into machine-executable logic.
    - **Swagger UI**: Provides interactive API documentation at `/swagger-ui`.

2.  **`crates/monitor` (The Muscle)**:
    - **`PollingMonitor`**: The core engine that maintains RPC connections.
    - **`TransactionMonitor`**: Scans blocks for transactions matching specific rules.
    - **`EventMonitor`**: Scans logs for specific event signatures.
    - Defines shared DTOs (`MonitorConfig`, `MonitorRule`) used by the system.

3.  **`crates/database` (The Memory)**:
    - Handles persistence using **CockroachDB** and **SQLx**.
    - Stores Users, Monitors, and Notification Channels.
    - Manages migrations and connection pooling.

4.  **`crates/notifications` (The Messenger)**:
    - Handles notification delivery (Email, Webhooks, etc.).
    - Abstracted via `ToDestination` trait.
    - Manages different channel types (e.g., Email, Webhook).

## Current Functionality ✨

Plethora Monitor is **production-ready** with the following features:

### 🔗 Multi-Chain Support

Monitor events and transactions across multiple EVM-compatible blockchains:

- **Ethereum**: Mainnet, Sepolia
- **Base**: Mainnet, Sepolia
- **Other EVM-compatible blockchains**: In progress

### 🎯 Advanced Monitoring

- **Event Monitoring**: Track specific smart contract events with custom filters
- **Transaction Monitoring**: Detect function calls matching specific conditions
- **Dynamic Rule Engine**:
  - Automatic ABI fetching from block explorers
  - Support for proxy contract detection
  - Flexible argument filtering with operators: `eq`, `gt`, `lt`, `contains`
  - Multi-condition rules (AND logic)

### 🔔 Notification Channels

- **Email Notifications**: SMTP-based email alerts with verification
- **Webhook Support**: HTTP POST to custom endpoints _(coming soon)_
- **Discord Integration**: Discord webhook notifications _(coming soon)_
- **Slack Integration**: Slack webhook notifications _(coming soon)_

### 🔐 Authentication & Security

- **Clerk Integration**: Secure user authentication
- **API Key Management**: Protected REST endpoints
- **Email Verification**: Required for email notification channels

### 📊 API & Documentation

- **REST API**: Full CRUD operations for monitors and channels
- **OpenAPI/Swagger**: Interactive API documentation at `/swagger-ui`
- **Real-time Status**: Monitor health and activity tracking

### 🗄️ Data Persistence

- **CockroachDB**: Distributed SQL database for reliability
- **SQLx**: Type-safe database queries
- **Automatic Migrations**: Schema versioning and updates

## Roadmap

- [ ] **Metrics**: Prometheus metrics for monitoring performance
- [ ] **Tests**: Comprehensive unit and integration tests
- [ ] **WebSocket Support**: Real-time event streaming
- [ ] **Discord/Slack Channels**: Complete notification integrations
- [ ] **Additional Blockchains**: Solana, Stellar, and Polkadot support

## Getting Started

### Prerequisites

- Rust (edition 2024)
- PostgreSQL Database
- An Etherscan/Basescan API Key (for ABI fetching)
- Clerk API Keys (for Auth)

### Installation

1.  Clone the repository:

    ```bash
    git clone <repository-url>
    cd plethora_monitor
    ```

2.  Set up your environment:

    ```bash
    cp .env.example .env
    # Configure DB_URL, ETHERSCAN_API_KEY, CLERK_SECRET_KEY, etc.
    ```

3.  Run migrations:

    ```bash
    sqlx migrate run
    ```

4.  Run the server:
    ```bash
    cargo run --bin plethora_monitor
    ```

## API Documentation

Once the server is running, visit:

- **Swagger UI**: `http://localhost:4000/swagger-ui`
- **OpenAPI Spec**: `http://localhost:4000/api-docs/openapi.json`

## Example: Create Monitor

Send a `POST` request to `/api/monitors` (requires Auth Bearer Token).

```json
{
  "name": "USDC Whale Watcher",
  "chain": "base-sepolia",
  "address": "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
  "rpc_url": "https://sepolia.base.org",
  "notification_channel_id": "550e8400-e29b-41d4-a716-446655440000",
  "function_rules": [
    {
      "name": "Large Transfer Alert",
      "conditions": [
        { "Function": "transfer" },
        { "From": "0xYourWalletAddress" }
      ]
    }
  ]
}
```

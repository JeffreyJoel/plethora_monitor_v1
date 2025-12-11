use axum::{Router, routing::post};
use dotenvy::dotenv;
use server::handler::create_monitor;
use server::state::AppState;
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();

    let default_rpc =
        env::var("DEFAULT_RPC_URL").unwrap_or_else(|_| "https://sepolia.base.org".to_string());
    let shared_state = Arc::new(AppState::new(default_rpc));

    // routes
    let app = Router::new()
        .route("/monitors", post(create_monitor))
        .with_state(shared_state);

    // start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🌍 Server running on http://0.0.0.0:3000");

    axum::serve(listener, app).await?;

    Ok(())
}

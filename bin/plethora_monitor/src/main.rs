use clerk_rs::{ClerkConfiguration, clerk::Clerk};
use database::DbPool;
use dotenvy::dotenv;
use server::monitors::handlers::restore_monitors;
use server::router::create_router;
use server::state::AppState;
use std::env;
use std::sync::Arc;


#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    //database connection
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Connecting to Database...");
    let db_pool = DbPool::new(&db_url).await?;

    //clerk authentication
    let clerk_secret_key = env::var("CLERK_SECRET_KEY").expect("CLERK_SECRET_KEY not set");
    let clerk_config = ClerkConfiguration::new(None, None, Some(clerk_secret_key), None);
    let clerk_client = Clerk::new(clerk_config);

    let default_rpc =
        env::var("DEFAULT_RPC_URL").unwrap_or_else(|_| "https://sepolia.base.org".to_string());

    let shared_state = Arc::new(AppState::new(default_rpc, db_pool, clerk_client));

    // Restore active monitors from database
    restore_monitors(shared_state.clone()).await;

    // create router
    let app = create_router(shared_state);

    // start server
    let port = env::var("PORT").unwrap_or_else(|_| "4000".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    println!("Server running on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await?;

    Ok(())
}


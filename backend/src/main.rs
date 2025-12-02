mod error;
mod handler;
mod middleware;
mod models;
mod state;

use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use dashmap::DashMap;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load Environment Variables
    // This reads the .env file so we can access DATABASE_URL
    dotenvy::dotenv().ok();

    // 2. Initialize Structured Logging (Tracing)
    // We filter logs based on RUST_LOG env var. Default to "debug" for this app.
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "synapse_backend=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 3. Database Connection Pool Setup
    // Big O: Establishing connections is expensive (Handshake + Auth).
    // We create a pool of 50 persistent connections.
    // Handlers "borrow" these connections, making DB access O(1) overhead.
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    tracing::info!("🔌 Connecting to Database...");
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&database_url)
        .await?;

    tracing::info!("✅ Database connection established");

    let state = AppState {
        pool,
        jwt_secret,
        hub: Arc::new(DashMap::new()),
    };

    // 4. Router & State Injection
    // .with_state(state) passes the AppState to every route.
    // This is "Dependency Injection" via the Type System.
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/auth/register", post(handler::register))
        .route("/auth/login", post(handler::login))
        .route("/ws", get(handler::ws_handler))
        .route("/protected", get(handler::protected))
        .with_state(state);

    // 5. Start the Server
    // We bind to 0.0.0.0 to allow access from other devices (like your phone/emulator)
    // 127.0.0.1 would only work on the laptop itself.
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("🚀 Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Simple handler to verify the server is running
async fn health_check() -> &'static str {
    "OK"
}

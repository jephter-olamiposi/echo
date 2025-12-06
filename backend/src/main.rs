mod error;
mod handler;
mod middleware;
mod models;
mod state;
#[cfg(test)]
mod tests;

use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "echo_backend=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET required");

    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&database_url)
        .await?;
    tracing::info!("Database connected");

    let state = AppState::new(pool, jwt_secret);

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/register", post(handler::register))
        .route("/login", post(handler::login))
        .route("/ws", get(handler::ws_handler))
        .route("/protected", get(handler::protected))
        .route("/history", get(handler::get_history))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

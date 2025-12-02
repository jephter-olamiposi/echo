use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub hub: Arc<DashMap<Uuid, broadcast::Sender<String>>>,
}

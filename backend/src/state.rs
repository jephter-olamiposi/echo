use crate::models::ClipboardMessage;
use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use uuid::Uuid;

const MAX_MESSAGES_PER_WINDOW: u32 = 30;
const WINDOW_DURATION_SECS: u64 = 60;
const MIN_INTERVAL_MS: u128 = 100;
const MAX_HISTORY_SIZE: usize = 50;

#[derive(Clone, Default)]
pub struct RateLimitState {
    pub last_message: Option<Instant>,
    pub message_count: u32,
    pub window_start: Option<Instant>,
}

type Hub = Arc<DashMap<Uuid, broadcast::Sender<ClipboardMessage>>>;
type RateLimits = Arc<DashMap<String, RateLimitState>>;
type History = Arc<DashMap<Uuid, Vec<ClipboardMessage>>>;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    hub: Hub,
    rate_limits: RateLimits,
    history: History,
}

impl AppState {
    pub fn new(pool: PgPool, jwt_secret: String) -> Self {
        Self {
            pool,
            jwt_secret,
            hub: Arc::default(),
            rate_limits: Arc::default(),
            history: Arc::default(),
        }
    }

    pub fn check_rate_limit(&self, device_id: &str) -> bool {
        let now = Instant::now();
        let mut entry = self.rate_limits.entry(device_id.to_string()).or_default();
        let state = entry.value_mut();

        if let Some(last) = state.last_message {
            if now.duration_since(last).as_millis() < MIN_INTERVAL_MS {
                return false;
            }
        }

        let window_start = state.window_start.get_or_insert(now);
        if now.duration_since(*window_start).as_secs() >= WINDOW_DURATION_SECS {
            state.window_start = Some(now);
            state.message_count = 0;
        }

        if state.message_count >= MAX_MESSAGES_PER_WINDOW {
            return false;
        }

        state.last_message = Some(now);
        state.message_count += 1;
        true
    }

    pub fn add_to_history(&self, user_id: Uuid, msg: ClipboardMessage) {
        let mut entry = self.history.entry(user_id).or_default();
        let history = entry.value_mut();
        history.insert(0, msg);
        history.truncate(MAX_HISTORY_SIZE);
    }

    pub fn get_history(&self, user_id: &Uuid) -> Vec<ClipboardMessage> {
        self.history
            .get(user_id)
            .map(|h| h.value().clone())
            .unwrap_or_default()
    }

    pub fn get_or_create_channel(&self, user_id: Uuid) -> broadcast::Sender<ClipboardMessage> {
        self.hub
            .entry(user_id)
            .or_insert_with(|| broadcast::channel(100).0)
            .clone()
    }

    pub fn cleanup_channel_if_empty(
        &self,
        user_id: &Uuid,
        tx: &broadcast::Sender<ClipboardMessage>,
    ) {
        if tx.receiver_count() == 0 {
            self.hub.remove(user_id);
            tracing::info!(user = %user_id, "fully disconnected");
        } else {
            tracing::debug!(user = %user_id, devices = tx.receiver_count(), "devices connected");
        }
    }
}

#[cfg(test)]
pub use test_utils::*;

#[cfg(test)]
mod test_utils {
    use super::*;

    #[derive(Clone, Default)]
    pub struct SyncEngine {
        pub hub: Hub,
        pub rate_limits: RateLimits,
        pub history: History,
    }

    impl SyncEngine {
        pub fn check_rate_limit(&self, device_id: &str) -> bool {
            let now = Instant::now();
            let mut entry = self.rate_limits.entry(device_id.to_string()).or_default();
            let state = entry.value_mut();

            if let Some(last) = state.last_message {
                if now.duration_since(last).as_millis() < MIN_INTERVAL_MS {
                    return false;
                }
            }

            let window_start = state.window_start.get_or_insert(now);
            if now.duration_since(*window_start).as_secs() >= WINDOW_DURATION_SECS {
                state.window_start = Some(now);
                state.message_count = 0;
            }

            if state.message_count >= MAX_MESSAGES_PER_WINDOW {
                return false;
            }

            state.last_message = Some(now);
            state.message_count += 1;
            true
        }

        pub fn add_to_history(&self, user_id: Uuid, msg: ClipboardMessage) {
            let mut entry = self.history.entry(user_id).or_default();
            let history = entry.value_mut();
            history.insert(0, msg);
            history.truncate(MAX_HISTORY_SIZE);
        }

        pub fn get_history(&self, user_id: &Uuid) -> Vec<ClipboardMessage> {
            self.history
                .get(user_id)
                .map(|h| h.value().clone())
                .unwrap_or_default()
        }

        pub fn get_or_create_channel(&self, user_id: Uuid) -> broadcast::Sender<ClipboardMessage> {
            self.hub
                .entry(user_id)
                .or_insert_with(|| broadcast::channel(100).0)
                .clone()
        }

        pub fn cleanup_channel_if_empty(
            &self,
            user_id: &Uuid,
            tx: &broadcast::Sender<ClipboardMessage>,
        ) {
            if tx.receiver_count() == 0 {
                self.hub.remove(user_id);
            }
        }
    }
}

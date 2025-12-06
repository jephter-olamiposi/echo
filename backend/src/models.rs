use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardMessage {
    pub device_id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(default)]
    pub encrypted: bool,
    pub timestamp: u64,
}

impl ClipboardMessage {
    pub fn new(device_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            device_id: device_id.into(),
            content: content.into(),
            nonce: None,
            encrypted: false,
            timestamp: now_millis(),
        }
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

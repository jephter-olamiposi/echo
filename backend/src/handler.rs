use crate::{
    error::AppError,
    middleware::AuthUser,
    models::{AuthResponse, Claims, ClipboardMessage, LoginRequest, RegisterRequest, WsQuery},
    state::AppState,
};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use uuid::Uuid;

const JWT_EXPIRY_HOURS: u64 = 24;
const PING_INTERVAL_SECS: u64 = 30;

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::Auth("Invalid credentials".into()))?;

    let hash = user.password_hash.clone();
    let password = payload.password;

    let valid = tokio::task::spawn_blocking(move || {
        argon2::PasswordHash::new(&hash)
            .ok()
            .map(|h| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &h)
                    .is_ok()
            })
            .unwrap_or(false)
    })
    .await?;

    if !valid {
        return Err(AppError::Auth("Invalid credentials".into()));
    }

    let token = generate_jwt(user.id, &state.jwt_secret)?;
    Ok((StatusCode::OK, Json(AuthResponse { token })))
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let password = payload.password;

    let hash = tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await?
    .map_err(|e| AppError::Internal(format!("Hash failed: {e}")))?;

    let result = sqlx::query!(
        "INSERT INTO users (first_name, last_name, email, password_hash) VALUES ($1, $2, $3, $4) RETURNING id",
        payload.first_name,
        payload.last_name,
        payload.email,
        hash
    )
    .fetch_one(&state.pool)
    .await;

    let user_id = match result {
        Ok(r) => r.id,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(AppError::Conflict("Email already exists".into()));
        }
        Err(e) => return Err(e.into()),
    };

    let token = generate_jwt(user_id, &state.jwt_secret)?;
    Ok((StatusCode::CREATED, Json(AuthResponse { token })))
}

fn generate_jwt(user_id: Uuid, secret: &str) -> Result<String, AppError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        iat: now,
        exp: now + (JWT_EXPIRY_HOURS as usize * 3600),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn protected(AuthUser { user_id }: AuthUser) -> impl IntoResponse {
    format!("Welcome! Your ID is: {user_id}")
}

pub async fn get_history(
    AuthUser { user_id }: AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    Json(state.get_history(&user_id))
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let validation = jsonwebtoken::Validation::default();
    let key = jsonwebtoken::DecodingKey::from_secret(state.jwt_secret.as_bytes());

    let claims = match jsonwebtoken::decode::<Claims>(&params.token, &key, &validation) {
        Ok(data) => data.claims,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid user ID").into_response(),
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid) {
    let device_id = Uuid::new_v4().to_string();
    tracing::info!(user = %user_id, device = %device_id, "device connected");

    let (sender, receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    let tx = state.get_or_create_channel(user_id);
    let mut rx = tx.subscribe();

    let ping_sender = Arc::clone(&sender);
    let ping_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(PING_INTERVAL_SECS));
        loop {
            interval.tick().await;
            if ping_sender
                .lock()
                .await
                .send(Message::Ping(vec![].into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let my_device = device_id.clone();
    let broadcast_sender = Arc::clone(&sender);
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if msg.device_id == my_device {
                continue;
            }
            if let Ok(json) = serde_json::to_string(&msg) {
                if broadcast_sender
                    .lock()
                    .await
                    .send(Message::Text(json.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    let recv_task = tokio::spawn(handle_incoming(
        receiver,
        tx.clone(),
        device_id,
        state.clone(),
        user_id,
    ));

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
        _ = ping_task => {},
    }

    state.cleanup_channel_if_empty(&user_id, &tx);
}

async fn handle_incoming(
    mut receiver: futures::stream::SplitStream<WebSocket>,
    tx: tokio::sync::broadcast::Sender<ClipboardMessage>,
    device_id: String,
    state: AppState,
    user_id: Uuid,
) {
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if !state.check_rate_limit(&device_id) {
                    tracing::warn!(device = %device_id, "rate limited");
                    continue;
                }

                let mut clipboard_msg = serde_json::from_str::<ClipboardMessage>(&text)
                    .unwrap_or_else(|_| ClipboardMessage::new(&device_id, text.to_string()));

                clipboard_msg.device_id = device_id.clone();
                state.add_to_history(user_id, clipboard_msg.clone());
                let _ = tx.send(clipboard_msg);
            }
            Message::Pong(_) => tracing::debug!(device = %device_id, "pong received"),
            Message::Close(_) => break,
            _ => {}
        }
    }
}

use crate::{
    error::AppError,
    middleware::AuthUser,
    models::{Claims, LoginRequest, LoginResponse, RegisterRequest, WsQuery},
    state::AppState,
};
use argon2::password_hash::rand_core::OsRng;
use argon2::{Argon2, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json,
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use jsonwebtoken::{EncodingKey, Header, encode};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use uuid::Uuid;

pub async fn protected(AuthUser { user_id }: AuthUser) -> impl IntoResponse {
    format!("Welcome to the protected area! Your ID is: {}", user_id)
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Find User by Email
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::DatabaseError)?
    .ok_or_else(|| AppError::AuthError("Invalid credentials".to_string()))?;

    // 2. Verify Password (Argon2)
    // We run this blocking operation on a separate thread to avoid freezing the server.
    let password_hash = user.password_hash.clone();
    let password = payload.password.clone();

    let is_valid = tokio::task::spawn_blocking(move || {
        let parsed_hash = argon2::PasswordHash::new(&password_hash).ok()?;
        let valid = Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();
        Some(valid)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
    .unwrap_or(false);

    if !is_valid {
        return Err(AppError::AuthError("Invalid credentials".to_string()));
    }

    // 3. Generate JWT
    // Expiration: Current Time + 24 Hours
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + (24 * 60 * 60);

    let claims = Claims {
        sub: user.id.to_string(),
        exp: expiration,
        iat: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalServerError(format!("Token creation failed: {}", e)))?;

    Ok((StatusCode::OK, Json(LoginResponse { token })))
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Hash Password with Argon2
    let salt = SaltString::generate(&mut OsRng);
    let password = payload.password.clone();

    let password_hash = tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
    .map_err(|e| AppError::InternalServerError(format!("Password hashing failed: {}", e)))?;

    // 2. Insert User into Database
    let result = sqlx::query!(
        "INSERT INTO users (email, password_hash) VALUES ($1, $2) RETURNING id",
        payload.email,
        password_hash
    )
    .fetch_one(&state.pool)
    .await;

    let user_id = match result {
        Ok(record) => record.id,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(AppError::Conflict("Email already exists".to_string()));
        }
        Err(e) => return Err(AppError::DatabaseError(e)),
    };

    // 3. Generate JWT for new user
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + (24 * 60 * 60);

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
        iat: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::InternalServerError(format!("Token creation failed: {}", e)))?;

    Ok((StatusCode::CREATED, Json(LoginResponse { token })))
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 1. Verify Token
    let decoding_key = jsonwebtoken::DecodingKey::from_secret(state.jwt_secret.as_bytes());
    let validation = jsonwebtoken::Validation::default();

    let claims = match jsonwebtoken::decode::<crate::models::Claims>(
        &params.token,
        &decoding_key,
        &validation,
    ) {
        Ok(token_data) => token_data.claims,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid Token").into_response(),
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(uid) => uid,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid User ID").into_response(),
    };

    // 2. Upgrade connection
    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid) {
    // Split the socket into Sender and Receiver
    let (mut sender, mut receiver) = socket.split();

    // 1. Get or Create the Broadcast Channel for this User
    // If the user has no room, create one. If they do, grab a reference to it.
    let tx = state
        .hub
        .entry(user_id)
        .or_insert_with(|| {
            let (tx, _rx) = broadcast::channel(100);
            tx
        })
        .clone();

    // Subscribe to this channel (Create a unique Receiver for THIS device)
    let mut rx = tx.subscribe();

    // 2. Spawn a Task to Forward Broadcasts -> Device
    // "If another device sends a message, push it down my socket"
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Send the encrypted payload to the client
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // 3. Listen for Messages from Device -> Broadcast
    // "If I copy text, broadcast it to everyone else"
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Determine if this is a pong or real data?
            // For V1, we assume all text is encrypted clipboard data.
            let _ = tx.send(text.to_string());
        }
    }); // 4. Wait for either task to finish (Disconnection)
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}

use serde::{Deserialize, Serialize};

// 1. What the client sends to register
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

// 2. What the client sends to log in
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

// 3. What we send back on success
#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

// 4. The data inside the JWT (The "Claims")
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (User ID)
    pub exp: usize,  // Expiration Timestamp
    pub iat: usize,  // Issued At Timestamp
}

// 5. Query parameters for WebSocket connection
#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    Auth(String),
    Database(sqlx::Error),
    Internal(String),
    Conflict(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::Auth(msg) => (StatusCode::UNAUTHORIZED, msg),
            Self::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".into())
            }
            Self::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into())
            }
            Self::Conflict(msg) => (StatusCode::CONFLICT, msg),
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Internal(err.to_string())
    }
}

use axum::{
    RequestPartsExt,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{DecodingKey, Validation, decode};

use crate::{error::AppError, models::Claims, state::AppState};

// This struct will be extracted from the request
// If extraction fails, the request is rejected (401 Unauthorized)
pub struct AuthUser {
    pub user_id: uuid::Uuid,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        // 1. Extract the "Authorization: Bearer <token>" header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| {
                AppError::AuthError("Missing or invalid Authorization header".to_string())
            })?;

        // 2. Decode and Validate the JWT
        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::AuthError("Invalid or expired token".to_string()))?;

        // 3. Parse the User ID
        let user_id = uuid::Uuid::parse_str(&token_data.claims.sub)
            .map_err(|_| AppError::AuthError("Invalid user ID in token".to_string()))?;

        Ok(AuthUser { user_id })
    }
}

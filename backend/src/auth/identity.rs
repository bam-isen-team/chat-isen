//! Extract an authenticated identity from an access-token bearer header.

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

use crate::{auth::jwt::verify_access_token, error::AppError, state::AppState};

/// The identity and token version certified by a verified access token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AuthIdentity {
    pub(crate) user_id: uuid::Uuid,
    pub(crate) token_version: i64,
}

impl FromRequestParts<AppState> for AuthIdentity {
    type Rejection = AppError;

    /// Extract and verify the bearer access token before running a protected handler.
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(bearer_token)
            .ok_or(AppError::Unauthenticated)?;

        let claims = verify_access_token(&state.config.jwt_secret, token)
            .map_err(|_| AppError::Unauthenticated)?;
        let user_id =
            uuid::Uuid::parse_str(claims.subject()).map_err(|_| AppError::Unauthenticated)?;

        Ok(Self {
            user_id,
            token_version: claims.token_version(),
        })
    }
}

/// Return the token from a well-formed `Bearer <token>` authorization value.
fn bearer_token(value: &str) -> Option<&str> {
    let mut parts = value.split_ascii_whitespace();
    let scheme = parts.next()?;
    let token = parts.next()?;

    (scheme.eq_ignore_ascii_case("Bearer") && parts.next().is_none()).then_some(token)
}

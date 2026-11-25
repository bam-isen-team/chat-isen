//! Error types shared by request handlers and their HTTP representations.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

use crate::auth::jwt::JwtError;
use crate::auth::password::PasswordError;

/// Application-wide HTTP errors (cf. M3): each variant becomes a clean API
/// response via `IntoResponse` — never a raw panic, never a leaked detail.
#[derive(Debug, Error)]
pub enum AppError {
    /// Well-formed JSON, semantically invalid payload: 400, field-precise.
    #[error("{0}")]
    Validation(String),

    /// Username or email already taken: 409, deliberately generic —
    /// a register probe must not reveal WHICH of the two exists.
    #[error("username or email already taken")]
    Conflict,

    /// Login failed without revealing whether the account or password was
    /// incorrect.
    #[error("invalid credentials")]
    Unauthorized,

    /// An access token is missing, invalid, or no longer current.
    #[error("unauthorized")]
    Unauthenticated,

    /// Anything unexpected: 500 with a generic body — the real cause is
    /// logged server-side only (cf. ADR-001: no internal detail in answers).
    #[error("internal server error")]
    Internal,
}

/// The translator: each variant becomes its HTTP status with a uniform JSON
/// body `{"error": "..."}`. Messages come from the `thiserror` attributes —
/// one source of truth. `Internal` stays deliberately mute here: its true
/// cause has already been traced at the mapping site in the handler.
impl IntoResponse for AppError {
    /// Convert an application error into its stable JSON HTTP response.
    fn into_response(self) -> Response {
        let (status, message): (StatusCode, String) = match &self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict => (StatusCode::CONFLICT, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Unauthenticated => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

// ---- `?` bridges: domain errors → HTTP families ----
// Hashing (M1) and minting (M2) are server-side operations: their failures
// are never the client's fault, so they become Internal (500). The cause is
// traced HERE, at the conversion point itself, so that no mapping site can
// ever forget to diagnose it.

impl From<PasswordError> for AppError {
    /// Log internal password-processing details and return a generic server error.
    fn from(err: PasswordError) -> Self {
        tracing::error!(cause = %err, "password hashing or verification failed");
        AppError::Internal
    }
}

impl From<JwtError> for AppError {
    /// Log internal JWT-processing details and return a generic server error.
    fn from(err: JwtError) -> Self {
        tracing::error!(cause = %err, "jwt operation failed");
        AppError::Internal
    }
}

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

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
    fn into_response(self) -> Response {
        let (status, message): (StatusCode, String) = match &self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict => (StatusCode::CONFLICT, self.to_string()),
            AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

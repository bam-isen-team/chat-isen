use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct DomainError {
    status: StatusCode,
    message: String,
    #[source]
    source: Option<BoxError>,
}

impl DomainError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn internal(message: impl Into<String>, source: Option<BoxError>) -> Self {
        Self::with_source(StatusCode::INTERNAL_SERVER_ERROR, message, source)
    }

    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self::with_source(status, message, None)
    }

    fn with_source(
        status: StatusCode,
        message: impl Into<String>,
        source: Option<BoxError>,
    ) -> Self {
        Self {
            status,
            message: message.into(),
            source,
        }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
}

impl IntoResponse for DomainError {
    fn into_response(self) -> Response {
        match self.source {
            Some(source) => tracing::error!(%self.message, error = %source, "request failed"),
            None if self.status.is_server_error() => {
                tracing::error!(%self.message, "request failed")
            }
            _ => {}
        }
        (
            self.status,
            Json(ErrorBody {
                error: &self.message,
            }),
        )
            .into_response()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("failed to bind listener: {0}")]
    Listener(std::io::Error),
    #[error("server error: {0}")]
    Server(std::io::Error),
}

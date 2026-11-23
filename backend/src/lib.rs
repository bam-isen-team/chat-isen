pub mod config;
pub mod error;
pub mod routes;

use axum::Router;
use axum::response::IntoResponse;
use axum::routing::get;

pub use error::{DomainError, StartupError};

pub fn app() -> Router {
    Router::new()
        .route("/healthz", get(routes::health::healthz))
        .merge(routes::api())
        .fallback(not_found)
}

async fn not_found() -> impl IntoResponse {
    DomainError::not_found("resource not found")
}

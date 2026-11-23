pub mod health;

use axum::Router;
use axum::routing::get;

pub fn api() -> Router {
    Router::new().route("/api/v1/ping", get(health::ping))
}

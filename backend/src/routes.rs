use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};

use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auth/register", post(crate::auth::handlers::register))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> StatusCode {
    // Check a connection out of the pool: failure means the db is
    // unreachable (stopped, network issue, pool timed out).
    let Ok(client) = state.pool.get().await else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };

    // Actually talk to the database; anything but success means unhealthy.
    let Ok(_) = client.query_one("SELECT 1", &[]).await else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };

    StatusCode::OK
}

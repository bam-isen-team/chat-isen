use axum::{
    Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
};

use crate::auth::rate_limit::{AuthRateLimiter, limit_auth_requests};
use crate::state::AppState;

/// Build the HTTP router and apply shared state and authentication policies.
pub fn build_router(state: AppState) -> Router {
    // Only password/refresh operations consume these quotas; GET /auth/me is
    // authenticated but does not use the request-rate buckets.
    let limited_auth_routes = Router::new()
        .route("/auth/register", post(crate::auth::handlers::register))
        .route("/auth/login", post(crate::auth::handlers::login))
        .route("/auth/refresh", post(crate::auth::handlers::refresh))
        .layer(middleware::from_fn_with_state(
            AuthRateLimiter::default(),
            limit_auth_requests,
        ));

    let current_user_route = Router::new().route("/auth/me", get(crate::auth::handlers::me));
    let logout_route = Router::new().route("/auth/logout", post(crate::auth::handlers::logout));

    Router::new()
        .route("/health", get(health))
        .merge(limited_auth_routes)
        .merge(current_user_route)
        .merge(logout_route)
        .with_state(state)
}

/// Check that PostgreSQL is reachable and can execute a simple query.
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

#[cfg(test)]
mod tests {
    use super::build_router;
    use crate::{config::Config, state::AppState};
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode, header},
    };
    use deadpool_postgres::{Manager, Pool};
    use serde_json::json;
    use std::sync::Arc;
    use tower::ServiceExt;

    /// Build the router with a lazy, deliberately unreachable database pool.
    fn test_router() -> Router {
        let pg_config = "host=127.0.0.1 port=1 user=test dbname=chatisen_test"
            .parse()
            .expect("valid test database config");
        let pool = Pool::builder(Manager::new(pg_config, tokio_postgres::NoTls))
            .build()
            .expect("build lazy test database pool");
        let config = Config::new(
            "postgres://test:test@127.0.0.1:1/chatisen_test".to_owned(),
            "Y2hhdC1pc2VuLXRlc3Qtc2VjcmV0".to_owned(),
            "127.0.0.1:0".parse().expect("valid test bind address"),
        );

        build_router(AppState::new(pool, Arc::new(config)))
    }

    /// Ensure registration is rate limited while the idempotent logout route is not.
    #[tokio::test]
    async fn register_is_throttled_and_logout_is_not() {
        let app = test_router();

        for _ in 0..5 {
            let request = Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": "a",
                        "email": "a@example.test",
                        "password": "correct horse battery staple"
                    })
                    .to_string(),
                ))
                .expect("build invalid registration request");
            let response = app.clone().oneshot(request).await.expect("router response");
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        let limited = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "username": "a",
                            "email": "a@example.test",
                            "password": "correct horse battery staple"
                        })
                        .to_string(),
                    ))
                    .expect("build throttled registration request"),
            )
            .await
            .expect("router response");
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(limited.headers()[header::RETRY_AFTER], "60");

        for _ in 0..11 {
            let logout = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/auth/logout")
                        .body(Body::empty())
                        .expect("build logout request"),
                )
                .await
                .expect("router response");
            assert_eq!(logout.status(), StatusCode::NO_CONTENT);
        }
    }
}

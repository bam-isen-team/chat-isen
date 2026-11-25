//! Shared resources made available to Axum request handlers.

use crate::config::Config;

/// Cloneable application state containing the database pool and runtime configuration.
#[derive(Clone)]
pub struct AppState {
    /// Connection pool shared by database-backed handlers.
    pub(crate) pool: deadpool_postgres::Pool,
    /// Immutable settings shared across the application.
    pub(crate) config: std::sync::Arc<Config>,
}

impl AppState {
    /// Create request state from a connection pool and shared configuration.
    pub fn new(pool: deadpool_postgres::Pool, config: std::sync::Arc<Config>) -> Self {
        Self { pool, config }
    }
}

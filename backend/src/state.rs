use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: deadpool_postgres::Pool,
    pub(crate) config: std::sync::Arc<Config>
}

impl AppState {
    pub fn new(pool: deadpool_postgres::Pool, config: std::sync::Arc<Config>) -> Self {
        Self { pool, config }
    }
}

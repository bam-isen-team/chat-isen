use axum::Router;

mod config;
mod routes;
mod state;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    
    tracing_subscriber::fmt::init();

    // Init the config
    let config = config::Config::from_env().expect("invalid configuration: check .env / .env.example");

    // Init the PostgreSQL pool
    let pg_config: tokio_postgres::Config = config
        .database_url
        .parse()
        .expect("invalid DATABASE_URL: expected postgres://user:password@host:port/db");
    let manager = deadpool_postgres::Manager::new(pg_config, tokio_postgres::NoTls);
    let pool = deadpool_postgres::Pool::builder(manager)
        .max_size(16)
        .build()
        .expect("failed to build the database pool");

    let app_state = state::AppState::new(pool, std::sync::Arc::new(config));
}

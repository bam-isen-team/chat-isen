use backend::{config, routes, state};

#[tokio::main]
/// Load configuration, prepare application state, and serve HTTP requests.
async fn main() {
    dotenvy::dotenv().ok();

    // Log to stdout, level driven by RUST_LOG (cf. .env.example), defaulting to "info".
    let log_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(log_filter).init();

    // Read secrets and bind settings once, then share them with request handlers.
    let config =
        config::Config::from_env().expect("invalid configuration: check .env / .env.example");

    // Build the connection pool before binding so startup fails early on bad configuration.
    let pg_config = config
        .database_config()
        .expect("invalid DATABASE_URL: expected postgres://user:password@host:port/db");
    let manager = deadpool_postgres::Manager::new(pg_config, tokio_postgres::NoTls);
    let pool = deadpool_postgres::Pool::builder(manager)
        .max_size(16)
        .build()
        .expect("failed to build the database pool");

    let bind_addr = config.bind_addr();
    let app_state = state::AppState::new(pool, std::sync::Arc::new(config));

    let app = routes::build_router(app_state);
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("failed to bind the listening address: is the port already in use?");

    // The only thing this process says out loud: its address, never its secrets.
    tracing::info!("listening on {bind_addr}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("server error");
}

use chatisen_backend::StartupError;
use chatisen_backend::app;
use chatisen_backend::config::Config;

#[tokio::main]
async fn main() -> Result<(), StartupError> {
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    init_tracing(&config.log_level);

    let listener = tokio::net::TcpListener::bind((config.host.as_str(), config.port))
        .await
        .map_err(StartupError::Listener)?;
    tracing::info!("listening on http://{}:{}", config.host, config.port);

    axum::serve(listener, app())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(StartupError::Server)?;

    Ok(())
}

fn init_tracing(log_level: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{log_level},tower_http=info").into()),
        )
        .init();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    tracing::info!("shutdown signal received, stopping gracefully");
}

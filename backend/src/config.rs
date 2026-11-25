//! Environment-backed configuration for the backend process.

use std::net::SocketAddr;

/// Runtime settings required to connect to PostgreSQL, sign tokens, and bind HTTP.
pub struct Config {
    /// PostgreSQL connection URL. Keep credentials out of logs and responses.
    pub(crate) database_url: String,
    /// Base64-encoded secret used to sign and verify access tokens.
    pub(crate) jwt_secret: String,
    /// Socket address where the HTTP listener accepts connections.
    pub(crate) bind_addr: SocketAddr,
}

impl Config {
    /// Construct configuration from already parsed values.
    pub fn new(database_url: String, jwt_secret: String, bind_addr: SocketAddr) -> Self {
        Self {
            database_url,
            jwt_secret,
            bind_addr,
        }
    }

    /// Read the required database URL, JWT secret, and bind address from the environment.
    ///
    /// # Errors
    /// Returns an error if a required variable is absent or the bind address is invalid.
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new(
            std::env::var("DATABASE_URL")?,
            std::env::var("JWT_SECRET")?,
            std::env::var("BACKEND_BIND_ADDR")?.parse()?,
        ))
    }

    /// Parse the configured database URL into a PostgreSQL connection configuration.
    ///
    /// # Errors
    /// Returns the PostgreSQL parser error when the URL is malformed.
    pub fn database_config(&self) -> Result<tokio_postgres::Config, tokio_postgres::Error> {
        self.database_url.parse()
    }

    /// Return the address used by the HTTP listener.
    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }
}

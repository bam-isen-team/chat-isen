use std::net::SocketAddr;

pub struct Config {
    pub(crate) database_url: String,
    pub(crate) jwt_secret: String,
    pub(crate) bind_addr: SocketAddr,
}

impl Config {
    pub fn new(database_url: String, jwt_secret: String, bind_addr: SocketAddr) -> Self {
        Self {
            database_url,
            jwt_secret,
            bind_addr,
        }
    }

    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new(
            std::env::var("DATABASE_URL")?,
            std::env::var("JWT_SECRET")?,
            std::env::var("BACKEND_BIND_ADDR")?.parse()?,
        ))
    }

    pub fn database_config(&self) -> Result<tokio_postgres::Config, tokio_postgres::Error> {
        self.database_url.parse()
    }

    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }
}

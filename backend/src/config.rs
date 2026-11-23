use std::env;

use crate::error::StartupError;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self, StartupError> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: parse_var("PORT", 8080)?,
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        })
    }
}

fn parse_var<T: std::str::FromStr>(key: &str, default: T) -> Result<T, StartupError> {
    match env::var(key) {
        Ok(v) => v
            .parse()
            .map_err(|_| StartupError::Config(format!("invalid value for {key}: {v}"))),
        Err(_) => Ok(default),
    }
}

//! Backend library for the ChatISEN HTTP service.
//!
//! The backend binary assembles these modules into the Axum application and
//! starts the PostgreSQL-backed server.

/// Authentication endpoints, token handling, and password utilities.
pub mod auth;
/// Application configuration loaded from environment variables.
pub mod config;
/// Shared API error types and their HTTP response mapping.
pub mod error;
/// HTTP route construction and health checks.
pub mod routes;
/// State shared by request handlers.
pub mod state;

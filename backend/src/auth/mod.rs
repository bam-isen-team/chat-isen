//! Authentication handlers and supporting token/password modules.

/// HTTP handlers for account and session operations.
pub mod handlers;
/// Axum extractor for requests authenticated with an access token.
pub(crate) mod identity;
/// Access-token claims, signing, and verification.
pub mod jwt;
/// Argon2 password hashing and verification.
pub mod password;
/// Per-IP request limits for selected authentication endpoints.
pub(crate) mod rate_limit;
/// Opaque refresh-token creation and hashing.
pub(crate) mod tokens;

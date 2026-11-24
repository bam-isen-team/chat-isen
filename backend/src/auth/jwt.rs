//! Access tokens: mint & verify JWTs signed with HS256 (cf. ADR-001 §1).
//!
//! Internal contract: keys are derived with `from_base64_secret` on BOTH
//! sides (the .env secret is base64) — diverging on either one would make
//! our own tokens unverifiable.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

/// Access token lifetime, in seconds (ADR-001 §1: 10 minutes).
const ACCESS_TOKEN_TTL_SECS: i64 = 600;

/// Payload of an access token — minimal claims, no sensitive data
/// (the payload is readable base64, cf. ADR-001 §1).
#[derive(Serialize, Deserialize)]
pub struct AccessClaims {
    /// Authenticated user id (UUID as text; parsed at the edge, cf. M7).
    sub: String,
    /// Issued at (unix seconds).
    iat: i64,
    /// Expires at (unix seconds) = iat + ACCESS_TOKEN_TTL_SECS.
    exp: i64,
    /// Unique token id, generated at mint time.
    jti: String,
    /// `users.token_version` at mint time; checked on sensitive routes so a
    /// global sign-out also invalidates living tokens (cf. ADR-001 §3).
    ver: i64,
}

/// Failures raised while minting or verifying access tokens.
#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    /// Minting failed (bad secret encoding, signing error...)
    #[error("failed to mint the access token: {0}")]
    Mint(jsonwebtoken::errors::Error),

    /// The token did not verify: bad signature, expired, wrong algorithm...
    #[error("invalid access token: {0}")]
    Invalid(jsonwebtoken::errors::Error),
}

/// Mint an access token certifying `sub` at token version `ver`.
///
/// The claims are built here — this is the identity the token certifies:
/// `sub`/`ver` come from the caller, timestamps from the clock, `jti` is
/// unique per token (cf. ADR-001 §1) — then sealed with HS256.
pub fn mint_access_token(secret: &str, sub: &str, ver: i64) -> Result<String, JwtError> {
    // HS256 by intention, never by default (cf. ADR-001: algorithm pinned).
    let header = Header::new(Algorithm::HS256);
    let key = EncodingKey::from_base64_secret(secret).map_err(JwtError::Mint)?;

    let now = chrono::Utc::now().timestamp();
    let claims = AccessClaims {
        sub: sub.to_string(),
        iat: now,
        exp: now + ACCESS_TOKEN_TTL_SECS,
        jti: uuid::Uuid::new_v4().to_string(),
        ver,
    };

    encode(&header, &claims, &key)
        .map_err(JwtError::Mint)
}

/// Verify an access token and return its claims.
///
/// Signature, expiration AND algorithm are checked. The token's own header
/// is never trusted: only HS256 is accepted, anything else ("none",
/// "HS384", "RS256"...) is rejected as a matter of policy (cf. ADR-001).
pub fn verify_access_token(secret: &str, token: &str) -> Result<AccessClaims, JwtError> {
    // Same key derivation as mint — the module's internal contract.
    let key = DecodingKey::from_base64_secret(secret).map_err(JwtError::Invalid)?;

    let validation = Validation::new(Algorithm::HS256);

    decode::<AccessClaims>(token, &key, &validation)
        .map_err(JwtError::Invalid)
        .map(|data| data.claims)
}

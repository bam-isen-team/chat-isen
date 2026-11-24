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
#[derive(Debug, Serialize, Deserialize)]
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

    encode(&header, &claims, &key).map_err(JwtError::Mint)
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // Fixed test secrets — valid base64 (the module's key-derivation
    // contract). Test material only, zero relation to the production secret.
    const TEST_SECRET: &str = "Y2hhdC1pc2VuLXRlc3Qtc2VjcmV0";
    // A second, different secret, to prove the signature engages THIS secret.
    const OTHER_SECRET: &str = "Y29tcGxldGVseS1vdGhlci1rZXktZm9yLXZlcmlmeQ==";

    #[rstest]
    fn roundtrip_preserves_identity() {
        let token =
            mint_access_token(TEST_SECRET, "user-uuid-1234", 7).expect("minting should succeed");
        let claims =
            verify_access_token(TEST_SECRET, &token).expect("a freshly minted token must verify");

        assert_eq!(claims.sub, "user-uuid-1234");
        assert_eq!(claims.ver, 7);
        assert!(
            !claims.jti.is_empty(),
            "jti is mint-generated, it must exist"
        );
        // ADR-001 §1: access tokens live exactly 10 minutes. Literal on
        // purpose: changing the TTL must force revisiting the ADR.
        assert_eq!(claims.exp - claims.iat, 600);
    }

    // Hostile tokens are forged DIRECTLY with the library below: our mint
    // can never produce a bad token — the verify path must survive them all.
    // The private fields of AccessClaims are reachable from here because a
    // child module sees its parent's private items.

    // A perfectly signed token, simply expired one hour ago (well beyond the
    // validation leeway).
    #[rstest]
    fn expired_token_is_rejected() {
        let now = chrono::Utc::now().timestamp();
        let claims = AccessClaims {
            sub: "user-uuid".to_string(),
            iat: now - 7200,
            exp: now - 3600,
            jti: uuid::Uuid::new_v4().to_string(),
            ver: 3,
        };
        let key = EncodingKey::from_base64_secret(TEST_SECRET).expect("valid base64 secret");
        let token =
            encode(&Header::new(Algorithm::HS256), &claims, &key).expect("forging should succeed");

        let res = verify_access_token(TEST_SECRET, &token);
        assert!(matches!(res, Err(JwtError::Invalid(_))), "got: {res:?}");
    }

    // THE ADR-001 security case: a token whose signature is perfectly valid
    // (same secret) but which claims a different algorithm. The pinned
    // validation rejects it as a matter of policy, not of signature.
    #[rstest]
    fn wrong_algorithm_is_rejected() {
        let now = chrono::Utc::now().timestamp();
        let claims = AccessClaims {
            sub: "user-uuid".to_string(),
            iat: now,
            exp: now + 600,
            jti: uuid::Uuid::new_v4().to_string(),
            ver: 0,
        };
        let key = EncodingKey::from_base64_secret(TEST_SECRET).expect("valid base64 secret");
        let token =
            encode(&Header::new(Algorithm::HS512), &claims, &key).expect("forging should succeed");

        let res = verify_access_token(TEST_SECRET, &token);
        assert!(matches!(res, Err(JwtError::Invalid(_))), "got: {res:?}");
    }

    #[rstest]
    fn wrong_secret_is_rejected() {
        let token = mint_access_token(TEST_SECRET, "user-uuid", 1).expect("minting should succeed");

        let res = verify_access_token(OTHER_SECRET, &token);
        assert!(matches!(res, Err(JwtError::Invalid(_))), "got: {res:?}");
    }

    // Attacker-controlled input must degrade cleanly, never panic.
    #[rstest]
    #[case::empty("")]
    #[case::three_bad_segments("a.b.c")]
    #[case::plain_garbage("not-a-jwt")]
    fn garbage_token_is_rejected(#[case] garbage: &str) {
        let res = verify_access_token(TEST_SECRET, garbage);
        assert!(matches!(res, Err(JwtError::Invalid(_))), "got: {res:?}");
    }

    // The module's internal contract is base64 secrets (cf. module doc): an
    // unusable secret must fail loudly at mint time — this is where a future
    // corrupted .env will explode, with the right family of error.
    #[rstest]
    fn non_base64_secret_fails_loudly() {
        let res = mint_access_token("!!! definitely not base64 !!!", "user-uuid", 1);
        assert!(matches!(res, Err(JwtError::Mint(_))), "got: {res:?}");
    }
}

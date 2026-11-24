//! Password hashing with Argon2id (cf. ADR-001 §6).
//!
//! Hashes are stored as PHC strings in `users.password_hash`; the SQL CHECK
//! (`password_hash LIKE '$argon2%'`) and this module must stay aligned.

use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::Argon2;

// NOTE: alias the library error types to keep our own error enum readable.
use argon2::password_hash::Error as HashError;
use argon2::password_hash::phc::Error as PhcError;

/// Failures raised while hashing or checking passwords.
///
/// A wrong password is NOT an error (see [`verify_password`]): only broken
/// hashes or hashing failures end up here.
#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    /// Hashing itself failed (out of memory, params...).
    #[error("failed to hash the password: {0}")]
    Hash(HashError),

    /// The stored hash is not a valid PHC string at all.
    #[error("unparseable stored password hash: {0}")]
    Parse(PhcError),

    /// The stored hash parses but is unusable (unsupported algorithm...).
    #[error("invalid stored password hash: {0}")]
    InvalidHash(HashError),
}

/// Hash a plaintext password with Argon2id and a fresh random salt.
///
/// `Argon2::default()` in the pinned version is exactly the OWASP profile
/// required by ADR-001 (Argon2id, m = 19456 KiB, t = 2, p = 1). The returned
/// PHC string (`$argon2id$v=19$m=19456,...`) is meant to be stored as-is in
/// `users.password_hash`.
pub fn hash_password(plain: &str) -> Result<String, PasswordError> {
    Argon2::default()
        .hash_password(plain.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(PasswordError::Hash)
}

/// Check a plaintext password against a stored PHC hash.
///
/// Returns `Ok(false)` when the hash is well-formed but the password is
/// wrong (invalid credentials are NOT a system error, cf. ADR-001 §2:
/// generic 401), and `Err` when the stored hash itself is unusable
/// (unparseable, unsupported algorithm...).
pub fn verify_password(plain: &str, phc: &str) -> Result<bool, PasswordError> {
    let parsed = PasswordHash::new(phc).map_err(PasswordError::Parse)?;
    match Argon2::default().verify_password(plain.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(HashError::PasswordInvalid) => Ok(false),
        Err(e) => Err(PasswordError::InvalidHash(e)),
    }
}

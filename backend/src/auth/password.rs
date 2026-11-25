//! Password hashing with Argon2id (cf. ADR-001 §6).
//!
//! Hashes are stored as PHC strings in `users.password_hash`; the SQL CHECK
//! (`password_hash LIKE '$argon2%'`) and this module must stay aligned.

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};

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
///
/// # Errors
/// Returns an error if Argon2 cannot produce a password hash.
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
///
/// # Errors
/// Returns an error if the stored PHC hash is malformed or unusable.
pub fn verify_password(plain: &str, phc: &str) -> Result<bool, PasswordError> {
    let parsed = PasswordHash::new(phc).map_err(PasswordError::Parse)?;
    match Argon2::default().verify_password(plain.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(HashError::PasswordInvalid) => Ok(false),
        Err(e) => Err(PasswordError::InvalidHash(e)),
    }
}

#[cfg(test)]
/// Password hashing, verification, and malformed-hash tests.
mod tests {
    use super::{PasswordError, hash_password, verify_password};
    use rstest::{fixture, rstest};

    // Reference password for the "wrong password" matrix.
    const REFERENCE_PWD: &str = "correct horse battery staple";

    // The primary contract: a hash this module produces can be verified back.
    // It is the canary of the module — the first test to fail if an argon2
    // upgrade ever breaks the roundtrip, long before any endpoint does.
    /// Hash and verify the same password successfully.
    #[rstest]
    fn roundtrip_works() {
        let phc = hash_password("test").expect("hashing should succeed");
        let ok =
            verify_password("test", &phc).expect("a well-formed hash must verify without error");
        assert!(ok);
    }

    // One Argon2id hash of the reference password, amortized over the cases
    // below (hashing is slow ON PURPOSE; never pay it repeatedly for nothing).
    /// Compute one stored hash reused by wrong-password parameter cases.
    #[fixture]
    fn stored_hash() -> String {
        hash_password(REFERENCE_PWD).expect("hashing the reference password should succeed")
    }

    // Contract of the future 401 (cf. ADR-001 §2): a wrong password is a clean
    // `Ok(false)`, never a system error. If this test ever fails with an Err,
    // the login layer would start answering 500 to bad credentials.
    /// Treat an incorrect password as `Ok(false)` rather than a system error.
    #[rstest]
    #[case::empty("")]
    #[case::almost_identical("correct horse battery stapl")]
    #[case::case_swapped("Correct horse battery staple")]
    #[case::long_garbage("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")]
    fn wrong_password_is_false_not_an_error(stored_hash: String, #[case] candidate: &str) {
        let ok = verify_password(candidate, &stored_hash)
            .expect("a wrong password must be a clean false, never a system error");
        assert!(!ok);
    }

    // Fresh salt on every call: same password in, two different hashes out —
    // yet both valid. Without a fresh salt, two users sharing a password would
    // show IDENTICAL password_hash columns in a database dump.
    /// Use a fresh salt for identical passwords while keeping both hashes valid.
    #[rstest]
    fn same_password_yields_different_hashes() {
        let hash1 = hash_password("letmein123").expect("hashing should succeed");
        let hash2 = hash_password("letmein123").expect("hashing should succeed");
        assert_ne!(hash1, hash2);
        // Different outputs, but both must still open the same door.
        assert!(verify_password("letmein123", &hash1).expect("verify hash1"));
        assert!(verify_password("letmein123", &hash2).expect("verify hash2"));
    }

    // Cross-artifact coherence: what Rust produces must satisfy what the SQL
    // CHECK (`password_hash LIKE '$argon2%'`) will enforce at insert time.
    /// Keep the encoded hash format aligned with the database check and ADR.
    #[rstest]
    fn hash_format_matches_sql_check() {
        let phc = hash_password("formatting").expect("hashing should succeed");
        assert!(phc.starts_with("$argon2"), "must satisfy the users CHECK");
        assert!(phc.starts_with("$argon2id"), "ADR-001 mandates argon2id");
        assert!(phc.contains("v=19"), "PHC version marker");
    }

    // Hostile input: an unparseable stored hash is a Parse error, never a panic
    // (this string will one day come from database content we do not control).
    /// Return a parse error for malformed stored hash strings.
    #[rstest]
    #[case::empty("")]
    #[case::truncated("$argon2id$")]
    #[case::not_phc("0xdeadbeef")]
    fn malformed_hash_is_an_error_not_a_panic(#[case] bad: &str) {
        let res = verify_password("whatever", bad);
        assert!(
            matches!(res, Err(PasswordError::Parse(_))),
            "expected a Parse error, got: {res:?}"
        );
    }

    // Edge exploration: pin what the hashing LAYER does for extreme inputs.
    // The registration POLICY (non-empty, bounded length) is M3's validation
    // job — remember that layer split when this surprises you.
    /// Hash and verify extreme lengths without panicking at this low-level layer.
    #[rstest]
    #[case::empty_password(String::new())]
    #[case::very_long("x".repeat(1000))]
    fn hashing_survives_extreme_lengths(#[case] password: String) {
        let phc = hash_password(&password).expect("hashing should succeed");
        let ok = verify_password(&password, &phc).expect("verify should succeed");
        assert!(ok);
    }
}

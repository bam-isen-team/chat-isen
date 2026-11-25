//! Opaque refresh-token generation and hashing.

use sha2::{Digest, Sha256};

/// A new refresh token and the values persisted alongside it.
pub(crate) struct GeneratedRefreshToken {
    pub(crate) raw: String,
    pub(crate) hash: Vec<u8>,
    pub(crate) family: uuid::Uuid,
}

/// Generate an unpredictable refresh token, its database hash and a family id.
///
/// Three UUID v4 values provide more than the 256 bits required by ADR-001.
pub(crate) fn generate_refresh_token() -> GeneratedRefreshToken {
    let raw = format!(
        "{}{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );

    GeneratedRefreshToken {
        hash: hash_refresh_token(&raw),
        raw,
        family: uuid::Uuid::new_v4(),
    }
}

/// Hash a presented token before looking it up in the database.
pub(crate) fn hash_refresh_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::{generate_refresh_token, hash_refresh_token};
    use rstest::rstest;

    #[rstest]
    fn generated_tokens_are_opaque_and_have_a_sha256_hash() {
        let token = generate_refresh_token();

        assert_eq!(token.raw.len(), 96);
        assert!(token.raw.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_eq!(token.hash.len(), 32);
        assert_eq!(token.hash, hash_refresh_token(&token.raw));
        assert_ne!(token.raw.as_bytes(), token.hash.as_slice());
    }

    #[rstest]
    fn each_issued_refresh_token_has_a_new_value_and_family() {
        let first = generate_refresh_token();
        let second = generate_refresh_token();

        assert_ne!(first.raw, second.raw);
        assert_ne!(first.hash, second.hash);
        assert_ne!(first.family, second.family);
    }

    #[rstest]
    fn hashing_is_stable_for_database_lookup() {
        let presented = "opaque-refresh-token";

        assert_eq!(hash_refresh_token(presented), hash_refresh_token(presented));
    }
}

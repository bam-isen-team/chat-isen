use serde::Deserialize;

use crate::error::AppError;

// ---- Field rules ----
// The username rule MUST mirror the users table CHECK in
// db/migrations/001_schema.up.sql: a divergence surfaces as an unexpected
// 500 on a valid-looking payload, not as a clean 400.
const USERNAME_MIN: usize = 3;
const USERNAME_MAX: usize = 32;
// Pragmatic email check (no RFC 5322 regex: the database enforces the real
// constraint anyway — uniqueness).
const EMAIL_MAX: usize = 254;
// Password policy: a project decision (neither the schema nor ADR-001 fix
// one); OWASP floor is 8, the ceiling keeps payloads sane.
const PASSWORD_MIN: usize = 8;
const PASSWORD_MAX: usize = 128;

/// Registration payload. Accepted as JSON by the `Json` extractor in the
/// register handler.
#[derive(Debug, Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

impl RegisterRequest {
    /// Semantic validation, field-precise — register MAY say which field is
    /// wrong (the generic-error rule of ADR-001 applies to login, not here).
    /// Called BEFORE hashing: argon2 costs ~40 ms, never paid for a payload
    /// that would die at the database CHECK anyway.
    pub fn validate(&self) -> Result<(), AppError> {
        let username_len = self.username.chars().count();
        if !(USERNAME_MIN..=USERNAME_MAX).contains(&username_len)
            || !self
                .username
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        {
            return Err(AppError::Validation(format!(
                "username must be {USERNAME_MIN}..{USERNAME_MAX} chars from [A-Za-z0-9_.-]"
            )));
        }

        if !self.email.contains('@') || self.email.chars().count() > EMAIL_MAX {
            return Err(AppError::Validation(
                "email must contain '@' and be at most 254 chars".to_string(),
            ));
        }

        let password_len = self.password.chars().count();
        if !(PASSWORD_MIN..=PASSWORD_MAX).contains(&password_len) {
            return Err(AppError::Validation(format!(
                "password must be {PASSWORD_MIN}..{PASSWORD_MAX} chars"
            )));
        }

        Ok(())
    }
}

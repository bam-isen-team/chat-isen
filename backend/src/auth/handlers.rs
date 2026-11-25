use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    auth::{jwt::mint_access_token, password::hash_password},
    error::AppError,
    state::AppState,
};

// ---- Field rules ----
// The username rule MUST mirror the users table CHECK in
// db/migrations/001_schema.up.sql: a divergence surfaces as an unexpected
// 500 on a valid-looking payload, not as a clean 400.
const USERNAME_MIN: usize = 3;
const USERNAME_MAX: usize = 32;
// Pragmatic email check (full RFC 5322 parsing is deliberately out of scope).
const EMAIL_MAX: usize = 254;
// Password policy: a project decision (neither the schema nor ADR-001 fix
// one); OWASP floor is 8, the ceiling keeps payloads sane.
const PASSWORD_MIN: usize = 8;
const PASSWORD_MAX: usize = 128;
const REFRESH_TOKEN_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;

/// Registration payload. Accepted as JSON by the `Json` extractor in the
/// register handler.
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

#[derive(Serialize)]
struct RegisterResponse {
    access_token: String,
    user: RegisteredUser,
}

#[derive(Serialize)]
struct RegisteredUser {
    id: uuid::Uuid,
    username: String,
    email: String,
    display_name: String,
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

        let email_is_plausible = self.email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && !domain.is_empty()
                && !domain.contains('@')
                && !self.email.chars().any(char::is_whitespace)
        });
        if !email_is_plausible || self.email.chars().count() > EMAIL_MAX {
            return Err(AppError::Validation(
                "email must have a local part and domain, contain no spaces, and be at most 254 chars"
                    .to_string(),
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

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Response, AppError> {
    req.validate()?;

    let password_hash = hash_password(&req.password)?;
    let mut client = state.pool.get().await.map_err(|err| {
        tracing::error!(cause = %err, "db pool checkout failed");
        AppError::Internal
    })?;
    let transaction = client.transaction().await.map_err(|err| {
        tracing::error!(cause = %err, "starting registration transaction failed");
        AppError::Internal
    })?;

    // The unique constraints remain the source of truth under concurrent
    // registrations; the pre-validation above only handles payload shape.
    let user = match transaction
        .query_one(
            "INSERT INTO users (username, email, password_hash, display_name) \
             VALUES ($1, $2, $3, $1) \
             RETURNING id, token_version, username, email, display_name",
            &[&req.username, &req.email, &password_hash],
        )
        .await
    {
        Ok(user) => user,
        Err(err)
            if err.as_db_error().is_some_and(|db_err| {
                db_err.code() == &tokio_postgres::error::SqlState::UNIQUE_VIOLATION
            }) =>
        {
            return Err(AppError::Conflict);
        }
        Err(err) => {
            tracing::error!(cause = %err, "creating registered user failed");
            return Err(AppError::Internal);
        }
    };

    let user_id: uuid::Uuid = user.get("id");
    let token_version: i32 = user.get("token_version");
    let access_token = mint_access_token(
        &state.config.jwt_secret,
        &user_id.to_string(),
        i64::from(token_version),
    )?;

    // UUID v4 uses the operating system CSPRNG. Three UUIDs provide more
    // than the 256 bits required by the refresh-token decision in ADR-001.
    let refresh_token = format!(
        "{}{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let refresh_token_hash = Sha256::digest(refresh_token.as_bytes()).to_vec();
    let family = uuid::Uuid::new_v4();

    transaction
        .execute(
            "INSERT INTO refresh_tokens (user_id, token_hash, family, expires_at) \
             VALUES ($1, $2, $3, now() + interval '30 days')",
            &[&user_id, &refresh_token_hash, &family],
        )
        .await
        .map_err(|err| {
            tracing::error!(cause = %err, "creating initial refresh token failed");
            AppError::Internal
        })?;

    transaction.commit().await.map_err(|err| {
        tracing::error!(cause = %err, "committing registration transaction failed");
        AppError::Internal
    })?;

    let cookie = format!(
        "refresh_token={refresh_token}; HttpOnly; Secure; SameSite=Lax; Path=/auth/refresh; Max-Age={REFRESH_TOKEN_TTL_SECONDS}"
    );
    let cookie = HeaderValue::from_str(&cookie).map_err(|err| {
        tracing::error!(cause = %err, "building refresh-token cookie failed");
        AppError::Internal
    })?;

    let mut response = (
        StatusCode::CREATED,
        Json(RegisterResponse {
            access_token,
            user: RegisteredUser {
                id: user_id,
                username: user.get("username"),
                email: user.get("email"),
                display_name: user.get("display_name"),
            },
        }),
    )
        .into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::{PASSWORD_MAX, RegisterRequest};
    use rstest::{fixture, rstest};

    #[fixture]
    fn valid_request() -> RegisterRequest {
        RegisterRequest {
            username: "alex.dev-1".to_owned(),
            email: "alex@example.test".to_owned(),
            password: "correct horse battery staple".to_owned(),
        }
    }

    #[rstest]
    fn accepts_a_valid_registration_payload(valid_request: RegisterRequest) {
        assert!(valid_request.validate().is_ok());
    }

    #[rstest]
    #[case::too_short("ab")]
    #[case::contains_space("contains space")]
    #[case::contains_slash("a/b")]
    fn rejects_usernames_outside_the_schema_rules(
        mut valid_request: RegisterRequest,
        #[case] username: &str,
    ) {
        valid_request.username = username.to_owned();

        assert!(valid_request.validate().is_err());
    }

    #[rstest]
    #[case::missing_local_part("@example.test")]
    #[case::missing_domain("alex@")]
    #[case::multiple_separators("alex@@example.test")]
    #[case::contains_space("alex @example.test")]
    fn rejects_malformed_email_shapes(mut valid_request: RegisterRequest, #[case] email: &str) {
        valid_request.email = email.to_owned();

        assert!(valid_request.validate().is_err());
    }

    #[rstest]
    #[case::too_short("short".to_owned())]
    #[case::too_long("x".repeat(PASSWORD_MAX + 1))]
    fn rejects_passwords_outside_the_policy_bounds(
        mut valid_request: RegisterRequest,
        #[case] password: String,
    ) {
        valid_request.password = password;

        assert!(valid_request.validate().is_err());
    }
}

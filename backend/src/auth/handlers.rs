use crate::{
    auth::{
        jwt::mint_access_token,
        password::{hash_password, verify_password},
        tokens::{generate_refresh_token, hash_refresh_token},
    },
    error::AppError,
    state::AppState,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    access_token: String,
    user: AuthenticatedUser,
}

#[derive(Serialize)]
struct AuthenticatedUser {
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

        if !is_valid_email(&self.email) {
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

impl LoginRequest {
    fn validate(&self) -> Result<(), AppError> {
        if !is_valid_email(&self.email) {
            return Err(AppError::Validation(
                "email must have a local part and domain, contain no spaces, and be at most 254 chars"
                    .to_string(),
            ));
        }

        let password_len = self.password.chars().count();
        if !(1..=PASSWORD_MAX).contains(&password_len) {
            return Err(AppError::Validation(format!(
                "password must be 1..{PASSWORD_MAX} chars"
            )));
        }

        Ok(())
    }
}

fn is_valid_email(email: &str) -> bool {
    let plausible = email.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && !domain.is_empty()
            && !domain.contains('@')
            && !email.chars().any(char::is_whitespace)
    });

    plausible && email.chars().count() <= EMAIL_MAX
}

fn refresh_token_from_headers(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|cookies| cookies.split(';'))
        .find_map(|cookie| {
            let (name, value) = cookie.trim().split_once('=')?;
            (name == "refresh_token" && !value.is_empty()).then_some(value)
        })
}

fn invalid_refresh_response() -> Response {
    let mut response = (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "invalid refresh token" })),
    )
        .into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "refresh_token=; HttpOnly; Secure; SameSite=Lax; Path=/auth/refresh; Max-Age=0",
        ),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn auth_response(
    status: StatusCode,
    access_token: String,
    user: AuthenticatedUser,
    refresh_token: &str,
) -> Result<Response, AppError> {
    let cookie = format!(
        "refresh_token={refresh_token}; HttpOnly; Secure; SameSite=Lax; Path=/auth/refresh; Max-Age={REFRESH_TOKEN_TTL_SECONDS}"
    );
    let cookie = HeaderValue::from_str(&cookie).map_err(|err| {
        tracing::error!(cause = %err, "building refresh-token cookie failed");
        AppError::Internal
    })?;

    let mut response = (status, Json(AuthResponse { access_token, user })).into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));

    Ok(response)
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

    let refresh_token = generate_refresh_token();

    transaction
        .execute(
            "INSERT INTO refresh_tokens (user_id, token_hash, family, expires_at) \
             VALUES ($1, $2, $3, now() + interval '30 days')",
            &[&user_id, &refresh_token.hash, &refresh_token.family],
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

    auth_response(
        StatusCode::CREATED,
        access_token,
        AuthenticatedUser {
            id: user_id,
            username: user.get("username"),
            email: user.get("email"),
            display_name: user.get("display_name"),
        },
        &refresh_token.raw,
    )
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Response, AppError> {
    req.validate()?;

    let client = state.pool.get().await.map_err(|err| {
        tracing::error!(cause = %err, "db pool checkout failed during login");
        AppError::Internal
    })?;
    let user = client
        .query_opt(
            "SELECT id, token_version, username, email, display_name, password_hash \
             FROM users WHERE lower(email) = lower($1)",
            &[&req.email],
        )
        .await
        .map_err(|err| {
            tracing::error!(cause = %err, "looking up login account failed");
            AppError::Internal
        })?;

    let Some(user) = user else {
        // Pay the same Argon2 cost for an unknown email as for a known one,
        // reducing the timing signal that could reveal registered accounts.
        hash_password(&req.password)?;
        return Err(AppError::Unauthorized);
    };

    let password_hash: String = user.get("password_hash");
    if !verify_password(&req.password, &password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let user_id: uuid::Uuid = user.get("id");
    let token_version: i32 = user.get("token_version");
    let access_token = mint_access_token(
        &state.config.jwt_secret,
        &user_id.to_string(),
        i64::from(token_version),
    )?;
    let refresh_token = generate_refresh_token();

    client
        .execute(
            "INSERT INTO refresh_tokens (user_id, token_hash, family, expires_at) \
             VALUES ($1, $2, $3, now() + interval '30 days')",
            &[&user_id, &refresh_token.hash, &refresh_token.family],
        )
        .await
        .map_err(|err| {
            tracing::error!(cause = %err, "creating login refresh token failed");
            AppError::Internal
        })?;

    auth_response(
        StatusCode::OK,
        access_token,
        AuthenticatedUser {
            id: user_id,
            username: user.get("username"),
            email: user.get("email"),
            display_name: user.get("display_name"),
        },
        &refresh_token.raw,
    )
}

pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let Some(raw_token) = refresh_token_from_headers(&headers) else {
        return Ok(invalid_refresh_response());
    };
    let token_hash = hash_refresh_token(raw_token);

    let mut client = state.pool.get().await.map_err(|err| {
        tracing::error!(cause = %err, "db pool checkout failed during refresh");
        AppError::Internal
    })?;
    let transaction = client.transaction().await.map_err(|err| {
        tracing::error!(cause = %err, "starting refresh transaction failed");
        AppError::Internal
    })?;

    let token = transaction
        .query_opt(
            "SELECT rt.id, rt.user_id, rt.family, \
                    (rt.revoked_at IS NOT NULL OR rt.replaced_by IS NOT NULL) AS is_used, \
                    (rt.expires_at <= now()) AS is_expired, \
                    u.token_version, u.username, u.email, u.display_name \
             FROM refresh_tokens rt \
             JOIN users u ON u.id = rt.user_id \
             WHERE rt.token_hash = $1 \
             FOR UPDATE OF rt",
            &[&token_hash],
        )
        .await
        .map_err(|err| {
            tracing::error!(cause = %err, "looking up refresh token failed");
            AppError::Internal
        })?;

    let Some(token) = token else {
        return Ok(invalid_refresh_response());
    };

    let family: uuid::Uuid = token.get("family");
    let is_used: bool = token.get("is_used");
    if is_used {
        // A consumed token being presented again signals possible theft.
        // Revoke every still-active token in its family before rejecting it.
        transaction
            .execute(
                "UPDATE refresh_tokens SET revoked_at = now() \
                 WHERE family = $1 AND revoked_at IS NULL",
                &[&family],
            )
            .await
            .map_err(|err| {
                tracing::error!(cause = %err, "revoking replayed refresh-token family failed");
                AppError::Internal
            })?;
        transaction.commit().await.map_err(|err| {
            tracing::error!(cause = %err, "committing refresh-token replay revocation failed");
            AppError::Internal
        })?;
        return Ok(invalid_refresh_response());
    }

    let is_expired: bool = token.get("is_expired");
    if is_expired {
        return Ok(invalid_refresh_response());
    }

    let token_id: uuid::Uuid = token.get("id");
    let user_id: uuid::Uuid = token.get("user_id");
    let token_version: i32 = token.get("token_version");
    let access_token = mint_access_token(
        &state.config.jwt_secret,
        &user_id.to_string(),
        i64::from(token_version),
    )?;
    let successor = generate_refresh_token();
    let successor_id: uuid::Uuid = transaction
        .query_one(
            "INSERT INTO refresh_tokens (user_id, token_hash, family, expires_at) \
             VALUES ($1, $2, $3, now() + interval '30 days') \
             RETURNING id",
            &[&user_id, &successor.hash, &family],
        )
        .await
        .map_err(|err| {
            tracing::error!(cause = %err, "inserting rotated refresh token failed");
            AppError::Internal
        })?
        .get("id");

    let updated = transaction
        .execute(
            "UPDATE refresh_tokens \
             SET revoked_at = now(), replaced_by = $2 \
             WHERE id = $1 AND revoked_at IS NULL AND replaced_by IS NULL \
                   AND expires_at > now()",
            &[&token_id, &successor_id],
        )
        .await
        .map_err(|err| {
            tracing::error!(cause = %err, "marking refresh token as rotated failed");
            AppError::Internal
        })?;
    if updated == 0 {
        // The row lock makes a concurrent consume impossible; the remaining
        // expected reason is that the token expired during this request.
        return Ok(invalid_refresh_response());
    }
    if updated != 1 {
        tracing::error!(token_id = %token_id, "refresh token changed while locked");
        return Err(AppError::Internal);
    }

    transaction.commit().await.map_err(|err| {
        tracing::error!(cause = %err, "committing refresh token rotation failed");
        AppError::Internal
    })?;

    auth_response(
        StatusCode::OK,
        access_token,
        AuthenticatedUser {
            id: user_id,
            username: token.get("username"),
            email: token.get("email"),
            display_name: token.get("display_name"),
        },
        &successor.raw,
    )
}

#[cfg(test)]
mod tests {
    use super::{LoginRequest, PASSWORD_MAX, RegisterRequest, refresh_token_from_headers};
    use axum::http::{HeaderMap, HeaderValue, header};
    use rstest::{fixture, rstest};

    #[fixture]
    fn valid_request() -> RegisterRequest {
        RegisterRequest {
            username: "alex.dev-1".to_owned(),
            email: "alex@example.test".to_owned(),
            password: "correct horse battery staple".to_owned(),
        }
    }

    #[fixture]
    fn valid_login_request() -> LoginRequest {
        LoginRequest {
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

    #[rstest]
    #[case::missing_local_part("@example.test")]
    #[case::missing_domain("alex@")]
    #[case::multiple_separators("alex@@example.test")]
    fn login_rejects_malformed_emails(mut valid_login_request: LoginRequest, #[case] email: &str) {
        valid_login_request.email = email.to_owned();

        assert!(valid_login_request.validate().is_err());
    }

    #[rstest]
    #[case::empty("")]
    #[case::too_long("x".repeat(PASSWORD_MAX + 1))]
    fn login_rejects_empty_or_excessively_long_passwords(
        mut valid_login_request: LoginRequest,
        #[case] password: String,
    ) {
        valid_login_request.password = password;

        assert!(valid_login_request.validate().is_err());
    }

    #[rstest]
    fn finds_refresh_token_among_other_cookies() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("theme=dark; refresh_token=opaque-value; locale=fr"),
        );

        assert_eq!(refresh_token_from_headers(&headers), Some("opaque-value"));
    }

    #[rstest]
    #[case::missing_cookie(None)]
    #[case::empty_cookie(Some("refresh_token="))]
    #[case::different_cookie(Some("session=opaque-value"))]
    fn rejects_missing_refresh_cookie(#[case] cookie: Option<&str>) {
        let mut headers = HeaderMap::new();
        if let Some(cookie) = cookie {
            headers.insert(header::COOKIE, HeaderValue::from_str(cookie).unwrap());
        }

        assert_eq!(refresh_token_from_headers(&headers), None);
    }
}

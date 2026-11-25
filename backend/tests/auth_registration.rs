use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use backend::{
    auth::{jwt::verify_access_token, password::verify_password},
    config::Config,
    routes::build_router,
    state::AppState,
};
use deadpool_postgres::{Manager, Pool};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::{env, sync::Arc};
use tower::ServiceExt;

// Test-only key. Never reuse this value outside the test suite.
const TEST_JWT_SECRET: &str = "Y2hhdC1pc2VuLXRlc3Qtc2VjcmV0";

fn test_app() -> (Router, Pool) {
    dotenvy::dotenv().ok();
    let database_url = env::var("TEST_DATABASE_URL")
        .expect("set TEST_DATABASE_URL to a dedicated PostgreSQL test database");
    let pg_config: tokio_postgres::Config = database_url
        .parse()
        .expect("TEST_DATABASE_URL must be a valid PostgreSQL URL");
    let database_name = pg_config.get_dbname().unwrap_or_default();
    assert!(
        database_name.ends_with("_test"),
        "refusing to run registration integration tests unless the database name ends in _test"
    );

    let manager = Manager::new(pg_config, tokio_postgres::NoTls);
    let pool = Pool::builder(manager)
        .max_size(4)
        .build()
        .expect("build test database pool");
    let bind_addr = "127.0.0.1:0".parse().expect("valid test bind address");
    let config = Config::new(database_url, TEST_JWT_SECRET.to_owned(), bind_addr);
    let app = build_router(AppState::new(pool.clone(), Arc::new(config)));

    (app, pool)
}

fn unique_credentials() -> (String, String) {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let username = format!("t{}", &suffix[..31]);
    let email = format!("{username}@example.test");
    (username, email)
}

async fn post_register(
    app: Router,
    username: &str,
    email: &str,
    password: &str,
) -> axum::response::Response {
    let body = json!({
        "username": username,
        "email": email,
        "password": password,
    });
    let request = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build registration request");

    app.oneshot(request)
        .await
        .expect("registration router should respond")
}

async fn post_login(app: Router, email: &str, password: &str) -> axum::response::Response {
    let body = json!({
        "email": email,
        "password": password,
    });
    let request = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("build login request");

    app.oneshot(request)
        .await
        .expect("login router should respond")
}

async fn cleanup_user(pool: &Pool, username: &str) {
    let client = pool.get().await.expect("get cleanup database connection");
    client
        .execute("DELETE FROM users WHERE username = $1", &[&username])
        .await
        .expect("delete test user and cascading refresh tokens");
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = response
        .into_body()
        .collect()
        .await
        .expect("collect response body")
        .to_bytes();
    serde_json::from_slice(&body).expect("response should be valid JSON")
}

// These tests are opt-in because they write to PostgreSQL. Create a dedicated
// database named with the `_test` suffix, apply the SQL migrations, then run
// from backend/: TEST_DATABASE_URL=postgres://.../chatisen_test cargo test -- --ignored
#[tokio::test]
#[ignore = "requires a dedicated PostgreSQL test database; see test module comment"]
async fn register_persists_hash_and_sets_refresh_cookie() {
    let (app, pool) = test_app();
    let (username, email) = unique_credentials();
    let password = "correct horse battery staple";

    let response = post_register(app, &username, &email, password).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");

    let set_cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .expect("Set-Cookie should be valid text")
        .to_owned();
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("Secure"));
    assert!(set_cookie.contains("SameSite=Lax"));
    assert!(set_cookie.contains("Path=/auth/refresh"));

    let refresh_token = set_cookie
        .split(';')
        .next()
        .and_then(|pair| pair.strip_prefix("refresh_token="))
        .expect("cookie should contain the refresh token");
    let body = response_json(response).await;
    let access_token = body["access_token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .expect("response should contain an access token");
    assert!(verify_access_token(TEST_JWT_SECRET, access_token).is_ok());
    assert!(body.get("refresh_token").is_none());
    assert_eq!(body["user"]["username"], username);
    assert_eq!(body["user"]["email"], email);

    let client = pool.get().await.expect("get test database connection");
    let user = client
        .query_one(
            "SELECT id, password_hash FROM users WHERE username = $1",
            &[&username],
        )
        .await
        .expect("registered user should be persisted");
    let user_id: uuid::Uuid = user.get("id");
    let password_hash: String = user.get("password_hash");
    assert!(password_hash.starts_with("$argon2id$"));
    assert_ne!(password_hash, password);
    assert!(verify_password(password, &password_hash).expect("stored hash should parse"));

    let stored_refresh_hash: Vec<u8> = client
        .query_one(
            "SELECT token_hash FROM refresh_tokens WHERE user_id = $1",
            &[&user_id],
        )
        .await
        .expect("refresh token hash should be persisted")
        .get("token_hash");
    use sha2::Digest;
    assert_eq!(
        stored_refresh_hash,
        sha2::Sha256::digest(refresh_token.as_bytes()).to_vec()
    );
    drop(client);
    cleanup_user(&pool, &username).await;
}

#[tokio::test]
#[ignore = "requires a dedicated PostgreSQL test database; see test module comment"]
async fn register_returns_conflict_for_duplicate_email_or_username() {
    let (app, pool) = test_app();
    let (username, email) = unique_credentials();
    let password = "correct horse battery staple";

    let first = post_register(app.clone(), &username, &email, password).await;
    assert_eq!(first.status(), StatusCode::CREATED);

    let (other_username, _) = unique_credentials();
    let duplicate_email = post_register(app.clone(), &other_username, &email, password).await;
    assert_eq!(duplicate_email.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(duplicate_email).await["error"],
        "username or email already taken"
    );

    let (_, other_email) = unique_credentials();
    let duplicate_username = post_register(app, &username, &other_email, password).await;
    assert_eq!(duplicate_username.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(duplicate_username).await["error"],
        "username or email already taken"
    );

    cleanup_user(&pool, &username).await;
}

#[tokio::test]
#[ignore = "requires a dedicated PostgreSQL test database; see test module comment"]
async fn register_returns_bad_request_for_invalid_input() {
    let (app, _) = test_app();
    let (_, email) = unique_credentials();

    let response = post_register(
        app,
        "invalid username",
        &email,
        "correct horse battery staple",
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(response).await["error"].is_string());
}

#[tokio::test]
#[ignore = "requires a dedicated PostgreSQL test database; see test module comment"]
async fn login_accepts_valid_credentials_and_issues_a_new_token_pair() {
    let (app, pool) = test_app();
    let (username, email) = unique_credentials();
    let password = "correct horse battery staple";

    let registered = post_register(app.clone(), &username, &email, password).await;
    assert_eq!(registered.status(), StatusCode::CREATED);

    let response = post_login(app, &email.to_uppercase(), password).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let set_cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .expect("Set-Cookie should be valid text");
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("Secure"));
    assert!(set_cookie.contains("SameSite=Lax"));
    assert!(set_cookie.contains("Path=/auth/refresh"));

    let body = response_json(response).await;
    let access_token = body["access_token"]
        .as_str()
        .filter(|token| !token.is_empty())
        .expect("response should contain an access token");
    assert!(verify_access_token(TEST_JWT_SECRET, access_token).is_ok());
    assert_eq!(body["user"]["username"], username);

    let client = pool.get().await.expect("get test database connection");
    let refresh_count: i64 = client
        .query_one(
            "SELECT count(*) FROM refresh_tokens r \
             JOIN users u ON u.id = r.user_id WHERE u.username = $1",
            &[&username],
        )
        .await
        .expect("login should persist another refresh token")
        .get(0);
    assert_eq!(refresh_count, 2);
    drop(client);
    cleanup_user(&pool, &username).await;
}

#[tokio::test]
#[ignore = "requires a dedicated PostgreSQL test database; see test module comment"]
async fn login_uses_the_same_generic_error_for_unknown_email_and_wrong_password() {
    let (app, pool) = test_app();
    let (username, email) = unique_credentials();
    let password = "correct horse battery staple";

    let registered = post_register(app.clone(), &username, &email, password).await;
    assert_eq!(registered.status(), StatusCode::CREATED);

    let wrong_password = post_login(app.clone(), &email, "not the correct password").await;
    let wrong_password_status = wrong_password.status();
    let wrong_password_body = response_json(wrong_password).await;

    let (_, unknown_email) = unique_credentials();
    let unknown_account = post_login(app, &unknown_email, password).await;
    let unknown_account_status = unknown_account.status();
    let unknown_account_body = response_json(unknown_account).await;

    assert_eq!(wrong_password_status, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown_account_status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_password_body, unknown_account_body);
    assert_eq!(wrong_password_body["error"], "invalid credentials");

    cleanup_user(&pool, &username).await;
}

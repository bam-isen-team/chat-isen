use axum::Json;
use axum::http::StatusCode;
use serde_json::json;

pub async fn healthz() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

pub async fn ping() -> Json<serde_json::Value> {
    Json(json!({ "pong": true }))
}

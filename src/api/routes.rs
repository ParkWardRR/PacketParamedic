//! API route definitions.

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

pub fn api_routes() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/self-test/latest", get(self_test_latest))
        .route("/incidents", get(list_incidents))
        .route("/probes/status", get(probe_status))
        .route("/speed-test/latest", get(speed_test_latest))
        .route("/speed-test/history", get(speed_test_history))
        .route("/schedules", get(list_schedules))
        .route("/schedules/dry-run", get(schedule_dry_run))
        .route("/network/interfaces", get(network_interfaces))
}

async fn health() -> Json<Value> {
    Json(json!({
        "data": {
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION")
        },
        "meta": {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "version": env!("CARGO_PKG_VERSION")
        }
    }))
}

async fn self_test_latest() -> Json<Value> {
    Json(json!({ "data": null, "meta": { "message": "no self-test results yet" } }))
}

async fn list_incidents() -> Json<Value> {
    Json(json!({ "data": [], "meta": { "total": 0 } }))
}

async fn probe_status() -> Json<Value> {
    Json(json!({ "data": { "active_probes": 0 } }))
}

async fn speed_test_latest() -> Json<Value> {
    Json(json!({ "data": null, "meta": { "message": "no speed test results yet" } }))
}

async fn speed_test_history() -> Json<Value> {
    Json(json!({ "data": [], "meta": { "total": 0 } }))
}

async fn list_schedules() -> Json<Value> {
    Json(json!({ "data": [], "meta": { "total": 0 } }))
}

async fn schedule_dry_run() -> Json<Value> {
    Json(json!({ "data": { "upcoming": [] } }))
}

async fn network_interfaces() -> Json<Value> {
    Json(json!({ "data": { "interfaces": [] } }))
}

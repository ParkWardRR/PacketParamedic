//! API route definitions.

use axum::{
    routing::{delete, get},
    Json, Router,
};
use serde_json::{json, Value};

use crate::api::state::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/self-test/latest", get(self_test_latest))
        .route("/incidents", get(list_incidents))
        .route("/probes/status", get(probe_status))
        .route("/speed-test/latest", get(speed_test_latest))
        .route("/speed-test/history", get(speed_test_history))
        .route("/schedules", get(list_schedules).post(create_schedule))
        .route("/schedules/:name", delete(delete_schedule))
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

use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct DryRunParams {
    hours: u64,
}

#[derive(Deserialize)]
struct CreateSchedule {
    name: String,
    cron: String,
    test: String,
}

#[derive(Serialize)]
struct ScheduleDto {
    name: String,
    cron: String,
    test: String,
    enabled: bool,
}

async fn list_schedules(State(state): State<AppState>) -> Json<Value> {
    match state.scheduler.list().await {
        Ok(list) => {
            let dtos: Vec<ScheduleDto> = list
                .into_iter()
                .map(|(name, cron, test, enabled)| ScheduleDto {
                    name,
                    cron,
                    test,
                    enabled,
                })
                .collect();
            Json(json!({ "data": dtos, "meta": { "total": dtos.len() } }))
        }
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

use axum::http::StatusCode;

async fn create_schedule(
    State(state): State<AppState>,
    Json(payload): Json<CreateSchedule>,
) -> (StatusCode, Json<Value>) {
    match state
        .scheduler
        .add_schedule(&payload.name, &payload.cron, &payload.test)
        .await
    {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({ "data": { "message": "created" } })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

async fn delete_schedule(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> (StatusCode, Json<Value>) {
    match state.scheduler.remove(&name).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "data": { "message": "deleted" } })),
        ),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

async fn schedule_dry_run(
    State(state): State<AppState>,
    Query(params): Query<DryRunParams>,
) -> Json<Value> {
    match state.scheduler.preview_next_runs(params.hours).await {
        Ok(preview) => {
            let runs: Vec<Value> = preview
                .into_iter()
                .map(|(time, name, test)| json!({ "time": time, "name": name, "test": test }))
                .collect();
            Json(json!({ "data": { "upcoming": runs } }))
        }
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn network_interfaces() -> Json<Value> {
    Json(json!({ "data": { "interfaces": [] } }))
}

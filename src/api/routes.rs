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
        .route("/schedules/{name}", delete(delete_schedule))
        .route("/schedules/dry-run", get(schedule_dry_run))
        .route("/trace", get(list_traces).post(run_trace))
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

use crate::probes::trace::{self, MtrReport};
use rusqlite::params;

#[derive(Deserialize)]
struct TraceRequest {
    target: String,
}

async fn run_trace(
    State(state): State<AppState>,
    Json(payload): Json<TraceRequest>,
) -> (StatusCode, Json<Value>) {
    // 1. Run trace (blocking operation)
    let target = payload.target.clone();
    let result = tokio::task::spawn_blocking(move || trace::run_trace(&target)).await;

    match result {
        Ok(Ok(report)) => {
            // 2. Persist to DB
            let conn = match state.pool.get() {
                Ok(c) => c,
                Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "DB pool error" }))),
            };

            let json_str = serde_json::to_string(&report).unwrap_or_default();
            
            // Calculate summary stats
            let hubs = &report.report.mtr.hubs;
            let hop_count = hubs.len();
            let max_lat = hubs.iter().map(|h| h.worst).fold(0.0, f32::max);
            let avg_loss = if hop_count > 0 {
                hubs.iter().map(|h| h.loss_percent).sum::<f32>() / hop_count as f32
            } else { 0.0 };

            if let Err(e) = conn.execute(
                "INSERT INTO trace_results (target, hop_count, max_latency_ms, avg_loss_percent, result_json) 
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![report.report.mtr.dst, hop_count, max_lat, avg_loss, json_str],
            ) {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() })));
            }

            (StatusCode::OK, Json(json!({ "data": report })))
        }
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Task join error: {}", e) }))),
    }
}

async fn list_traces(State(state): State<AppState>) -> Json<Value> {
    let conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => return Json(json!({ "error": "DB pool error" })),
    };

    let mut stmt = match conn.prepare(
        "SELECT id, target, hop_count, max_latency_ms, avg_loss_percent, created_at FROM trace_results ORDER BY created_at DESC LIMIT 50"
    ) {
        Ok(s) => s,
        Err(e) => return Json(json!({ "error": e.to_string() })),
    };

    let rows = stmt.query_map([], |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "target": row.get::<_, String>(1)?,
            "hop_count": row.get::<_, i32>(2)?,
            "max_latency": row.get::<_, f64>(3)?,
            "avg_loss": row.get::<_, f64>(4)?,
            "created_at": row.get::<_, String>(5)?,
        }))
    });

    match rows {
        Ok(iter) => {
            let data: Vec<Value> = iter.filter_map(|r| r.ok()).collect();
            Json(json!({ "data": data, "meta": { "total": data.len() } }))
        }
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

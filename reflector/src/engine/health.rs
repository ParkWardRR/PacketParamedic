//! Health check endpoint for the PacketParamedic Reflector.
//!
//! Provides a simple HTTP `GET /health` endpoint that returns the reflector's
//! status, version, and current system load.

use axum::response::IntoResponse;
use axum::{routing::get, Json, Router};
use serde_json::json;
use sysinfo::System;

// ---------------------------------------------------------------------------
// Health handler
// ---------------------------------------------------------------------------

/// Axum handler for `GET /health`.
///
/// Returns a JSON object with:
/// - `status`: always `"ok"` if the server is running
/// - `version`: the crate version from `Cargo.toml`
/// - `load`: 1-minute system load average
pub async fn health_handler() -> impl IntoResponse {
    let load = System::load_average();

    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "load": load.one,
    }))
}

/// Build an Axum router with the health endpoint.
///
/// Mount this router on a separate HTTP listener (e.g. port 7301) so that
/// monitoring systems can probe the reflector without TLS/mTLS.
pub fn build_health_router() -> Router {
    Router::new().route("/health", get(health_handler))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = build_health_router();

        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 10_000)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "ok");
        assert!(json["version"].is_string());
        assert!(json["load"].is_number());
    }
}

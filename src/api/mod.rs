//! API layer -- axum routes, handlers, and middleware.

mod routes;

use axum::Router;

/// Build the application router with all API routes.
pub fn router() -> Router {
    Router::new()
        .nest("/api/v1", routes::api_routes())
        .fallback(fallback)
}

async fn fallback() -> (axum::http::StatusCode, &'static str) {
    (axum::http::StatusCode::NOT_FOUND, "not found")
}

//! API layer -- axum routes, handlers, and middleware.

mod routes;
pub mod state;

use axum::Router;
use self::state::AppState;

/// Build the application router with all API routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api/v1", routes::api_routes())
        .fallback(fallback)
        .with_state(state)
}

async fn fallback() -> (axum::http::StatusCode, &'static str) {
    (axum::http::StatusCode::NOT_FOUND, "not found")
}

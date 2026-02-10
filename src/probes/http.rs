//! HTTP probe implementation.

use anyhow::Result;

/// Run an HTTP reachability probe against the specified URL.
pub async fn probe(url: &str, timeout_ms: u64) -> Result<HttpResult> {
    // TODO: Implement HTTP GET with timing
    tracing::debug!(%url, %timeout_ms, "HTTP probe (stub)");
    Ok(HttpResult {
        url: url.to_string(),
        status: None,
        latency_ms: None,
        reachable: false,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct HttpResult {
    pub url: String,
    pub status: Option<u16>,
    pub latency_ms: Option<f64>,
    pub reachable: bool,
}

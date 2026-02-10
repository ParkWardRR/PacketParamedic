//! DNS probe implementation.

use anyhow::Result;

/// Run a DNS resolution probe for the specified hostname.
pub async fn probe(hostname: &str, timeout_ms: u64) -> Result<DnsResult> {
    // TODO: Implement DNS resolution with timing
    tracing::debug!(%hostname, %timeout_ms, "DNS probe (stub)");
    Ok(DnsResult {
        hostname: hostname.to_string(),
        resolver: None,
        latency_ms: None,
        resolved: false,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct DnsResult {
    pub hostname: String,
    pub resolver: Option<String>,
    pub latency_ms: Option<f64>,
    pub resolved: bool,
}

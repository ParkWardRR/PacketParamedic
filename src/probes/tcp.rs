//! TCP connection probe implementation.

use anyhow::Result;

/// Run a TCP connection probe to the specified host:port.
pub async fn probe(host: &str, port: u16, timeout_ms: u64) -> Result<TcpResult> {
    // TODO: Implement TCP connect with timing
    tracing::debug!(%host, %port, %timeout_ms, "TCP probe (stub)");
    Ok(TcpResult {
        host: host.to_string(),
        port,
        latency_ms: None,
        reachable: false,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct TcpResult {
    pub host: String,
    pub port: u16,
    pub latency_ms: Option<f64>,
    pub reachable: bool,
}

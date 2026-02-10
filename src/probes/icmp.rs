//! ICMP probe implementation.

use anyhow::Result;

/// Run an ICMP ping probe against the specified target.
pub async fn probe(target: &str, timeout_ms: u64) -> Result<PingResult> {
    // TODO: Implement raw ICMP socket ping (requires CAP_NET_RAW)
    tracing::debug!(%target, %timeout_ms, "ICMP probe (stub)");
    Ok(PingResult {
        target: target.to_string(),
        rtt_ms: None,
        reachable: false,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct PingResult {
    pub target: String,
    pub rtt_ms: Option<f64>,
    pub reachable: bool,
}

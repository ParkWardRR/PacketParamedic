//! Native Rust TCP/UDP throughput engine (fallback when iperf3 unavailable).
//!
//! Pure safe Rust using tokio::net primitives. No unsafe.

use anyhow::Result;

/// Run a native TCP throughput test to the specified peer.
pub async fn tcp_throughput(peer: &str, port: u16, duration_secs: u64) -> Result<NativeResult> {
    // TODO: Open TCP connection, send data for duration, measure throughput
    tracing::debug!(%peer, %port, %duration_secs, "Native TCP throughput (stub)");
    Ok(NativeResult {
        throughput_mbps: 0.0,
        duration_secs: duration_secs as f64,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct NativeResult {
    pub throughput_mbps: f64,
    pub duration_secs: f64,
}

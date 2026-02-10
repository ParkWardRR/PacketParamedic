//! Throughput testing engine: iperf3 wrapper + native Rust fallback.

pub mod iperf;
pub mod lan;
pub mod native;
pub mod report;
pub mod wan;

use anyhow::Result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ThroughputError {
    #[error("iperf3 not found at {path}")]
    Iperf3NotFound { path: String },

    #[error("iperf3 process exited with code {code}: {stderr}")]
    Iperf3Failed { code: i32, stderr: String },

    #[error("no 10GbE-capable interface detected")]
    No10GbeInterface,

    #[error("thermal limit exceeded ({temp_c}°C); test aborted")]
    ThermalAbort { temp_c: f64 },

    #[error("peer {peer} not reachable for LAN test")]
    PeerUnreachable { peer: String },
}

/// Throughput test result.
#[derive(Debug, serde::Serialize)]
pub struct ThroughputResult {
    pub mode: String,
    pub direction: String,
    pub throughput_mbps: f64,
    pub jitter_ms: Option<f64>,
    pub loss_percent: Option<f64>,
    pub streams: u32,
    pub duration_secs: f64,
    pub link_speed_mbps: Option<u64>,
    pub engine: String, // "iperf3" or "native"
}

/// Run a throughput test with the given parameters.
pub async fn run_test(
    mode: &str,
    peer: Option<&str>,
    duration: &str,
    streams: u32,
) -> Result<()> {
    tracing::info!(%mode, ?peer, %duration, %streams, "Throughput test (stub)");
    // TODO: Parse duration string
    // TODO: Select iperf3 or native engine
    // TODO: Run test with thermal monitoring
    // TODO: Store results in SQLite
    Ok(())
}

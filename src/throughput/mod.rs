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
pub async fn run_test(mode: &str, peer: Option<&str>, duration: &str, streams: u32) -> Result<()> {
    tracing::info!(%mode, ?peer, %duration, %streams, "Running throughput test");

    // Parse duration ("30s" -> 30)
    let dur_secs: u32 = duration.trim_end_matches('s').parse().unwrap_or(30);

    let target = if let Some(p) = peer { p } else { 
        if mode == "wan" { 
            // Try to find a working public server
            find_public_server()
        } else { 
            anyhow::bail!("Peer required for LAN test (use --peer <IP>)") 
        }
    };

    println!("Running {} throughput test against {} for {}s ({} streams)...", mode.to_uppercase(), target, dur_secs, streams);

    // Run Upload (Client -> Server)
    run_iperf_direction(target, dur_secs, streams, false)?;

    // Run Download (Server -> Client, -R)
    run_iperf_direction(target, dur_secs, streams, true)?;

    Ok(())
}

fn find_public_server() -> &'static str {
    // List of public iperf3 servers (best effort)
    // In a real pro app, we'd ping them for lowest latency first.
    let _servers = [
        "speedtest.wtnet.de",
        "ping.online.net",
        "iperf.biznetnetworks.com",
        "bouygues.iperf.fr",
    ];
    // For now, just return the first one as default, but ideally we'd loop.
    // Given execution constraints, let's pick a high-availability one.
    // wtnet is often busy. ping.online.net is robust.
    "ping.online.net"
}

fn validate_target(target: &str) -> Result<()> {
    if target.is_empty() {
        anyhow::bail!("Target cannot be empty");
    }
    if target.starts_with('-') {
        anyhow::bail!("Target cannot start with hyphen (security)");
    }
    // Allow alphanumeric, dot, hyphen, colon (IPv6)
    if target.chars().any(|c| !c.is_alphanumeric() && c != '.' && c != '-' && c != ':') {
        anyhow::bail!("Target contains invalid characters");
    }
    Ok(())
}

fn run_iperf_direction(target: &str, duration: u32, streams: u32, reverse: bool) -> Result<()> {
    validate_target(target)?;

    let dir_str = if reverse { "DOWNLOAD" } else { "UPLOAD" };
    println!("Starting {} test...", dir_str);

    // Build iperf3 arguments
    let mut iperf_args = vec![
        "-c".to_string(),
        target.to_string(),
        "-J".to_string(),
        "-t".to_string(),
        duration.to_string(),
        "-P".to_string(),
        streams.to_string(),
    ];
    if reverse {
        iperf_args.push("-R".to_string());
    }

    // Optimization: Pin to cores 2,3 on Pi 5 (leave 0,1 for OS/API)
    // Pi 5 guarantees 'taskset' availability (util-linux).
    // We strictly enforce this isolation strategy.
    let mut cmd = std::process::Command::new("taskset");
    cmd.arg("-c");
    cmd.arg("2,3");
    cmd.arg("iperf3");
    cmd.args(&iperf_args);

    let output = cmd.output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let json_str = String::from_utf8_lossy(&out.stdout);
                match crate::throughput::iperf::parse_output(&json_str) {
                    Ok(res) => {
                        let mbps = res.end.sum_received.bits_per_second / 1_000_000.0;
                         println!("  -> {}: {:.2} Mbps", dir_str, mbps);
                    },
                    Err(e) => println!("  -> Failed to parse JSON: {}", e),
                }
            } else {
                let err = String::from_utf8_lossy(&out.stderr);
                println!("  -> iperf3 failed: {}", err.trim());
            }
        },
        Err(e) => {
            println!("  -> Error executing iperf3: {}", e);
            println!("     (Is 'iperf3' installed? try 'sudo apt install iperf3')");
        }
    }
    Ok(())
}

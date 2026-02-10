use super::{Measurement, Probe, ProbeType};
use anyhow::{Result, Context};
use std::time::{Duration, Instant, SystemTime};
 // Temporary until we use ICMP raw socket crate
use tracing::{warn};

/// Simple ICMP probe wrapper (uses system ping for now)
/// Future: Use `pnet` or `socket2` for raw sockets to avoid fork/exec overhead.
pub struct IcmpProbe;

#[async_trait::async_trait]
impl Probe for IcmpProbe {
    async fn run(&self, target: &str, timeout: Duration) -> Result<Measurement> {
        let start = Instant::now();
        
        // Use system ping command
        // -c 1: count 1
        // -W N: timeout in seconds
        // -q: quiet
        
        // CAREFUL: target injection. In real app, validate target is IP or domain.
        // For now, simple String.
        
        let timeout_secs = timeout.as_secs_f64().max(1.0);
        
        let output = tokio::process::Command::new("ping")
            .arg("-c").arg("1")
            .arg("-W").arg(timeout_secs.to_string())
            .arg("-q")
            .arg(target)
            .output()
            .await
            .context("Failed to execute ping")?;

        let duration = start.elapsed();
        let timestamp = SystemTime::now();

        if output.status.success() {
            // Parse stdout for time=X ms
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Example: ... time=14.2 ms ...
            // Or rtt min/avg/max/mdev = 14.188/14.188/14.188/0.000 ms
            
            // Extract RTT from "time=X ms" or stats line
            let rtt_ms = extract_rtt(&stdout).unwrap_or_else(|| {
                warn!("Ping success but failed to parse RTT for {}", target);
                duration.as_secs_f64() * 1000.0 // Fallback to wall clock
            });

            Ok(Measurement {
                probe_type: ProbeType::Icmp,
                target: target.to_string(),
                value: rtt_ms,
                unit: "ms".to_string(),
                success: true,
                timestamp,
            })
        } else {
            // Timeout or unreachable
            Ok(Measurement {
                probe_type: ProbeType::Icmp,
                target: target.to_string(),
                value: -1.0, // Sentinel for failure
                unit: "ms".to_string(),
                success: false,
                timestamp,
            })
        }
    }
}

fn extract_rtt(output: &str) -> Option<f64> {
    // Try to find "time=12.3 ms"
    if let Some(pos) = output.find("time=") {
        let rest = &output[pos+5..];
        if let Some(end) = rest.find(" ") {
             return rest[..end].parse::<f64>().ok();
        }
    }
    
    // Try "min/avg/max" line: rtt min/avg/max/mdev = ...
    if let Some(pos) = output.find(" = ") {
        if output[..pos].contains("rtt") || output[..pos].contains("round-trip") {
            let rest = &output[pos+3..];
             let parts: Vec<&str> = rest.split('/').collect();
             if parts.len() >= 2 {
                 return parts[1].parse::<f64>().ok(); // avg
             }
        }
    }
    
    None
}

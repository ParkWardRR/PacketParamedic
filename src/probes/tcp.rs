use super::{Measurement, Probe, ProbeType};
use anyhow::Result;
use std::time::{Duration, Instant, SystemTime};
use tokio::net::TcpStream;

/// TCP Connect Probe
pub struct TcpProbe;

#[async_trait::async_trait]
impl Probe for TcpProbe {
    async fn run(&self, target: &str, timeout: Duration) -> Result<Measurement> {
        let start = Instant::now();

        // Target format: "host:port", e.g. "google.com:80"
        // If no port, default to 80? Or error?
        // Let's assume input includes port for now.
        let addr = if target.contains(':') {
            target.to_string()
        } else {
            format!("{}:80", target)
        };

        let connect_future = TcpStream::connect(&addr);
        let result = tokio::time::timeout(timeout, connect_future).await;

        let duration = start.elapsed();
        let timestamp = SystemTime::now();

        match result {
            Ok(Ok(_stream)) => Ok(Measurement {
                probe_type: ProbeType::Tcp,
                target: target.to_string(),
                value: duration.as_secs_f64() * 1000.0,
                unit: "ms".to_string(),
                success: true,
                timestamp,
            }),
            Ok(Err(_)) => {
                // Connection refused or other IO error
                Ok(Measurement {
                    probe_type: ProbeType::Tcp,
                    target: target.to_string(),
                    value: -1.0,
                    unit: "ms".to_string(),
                    success: false,
                    timestamp,
                })
            }
            Err(_) => {
                // Timeout
                Ok(Measurement {
                    probe_type: ProbeType::Tcp,
                    target: target.to_string(),
                    value: -1.0,
                    unit: "ms".to_string(),
                    success: false,
                    timestamp,
                })
            }
        }
    }
}

use anyhow::Result;
use std::time::Duration;

pub mod icmp;

#[derive(Debug, Clone, PartialEq)]
pub enum ProbeType {
    Icmp,
    Dns,
    Http,
    Tcp,
}

impl std::fmt::Display for ProbeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeType::Icmp => write!(f, "icmp"),
            ProbeType::Dns => write!(f, "dns"),
            ProbeType::Http => write!(f, "http"),
            ProbeType::Tcp => write!(f, "tcp"),
        }
    }
}

pub struct Measurement {
    pub probe_type: ProbeType,
    pub target: String,
    pub value: f64, // ms for latency, or custom value
    pub unit: String,
    pub success: bool,
    pub timestamp: std::time::SystemTime,
}

/// Trait for all active probes
#[async_trait::async_trait]
pub trait Probe: Send + Sync {
    /// Run the probe against a target
    /// Returns a Measurement result
    async fn run(&self, target: &str, timeout: Duration) -> Result<Measurement>;
}

/// Run a blame check sequence (stub)
pub async fn blame_check() -> Result<()> {
    // TODO: Implement full blame check logic
    tracing::info!("Blame check stub. Use 'packetparamedic self-test' for hardware checks.");
    Ok(())
}

//! Network probe implementations: ICMP, HTTP, DNS, TCP.

pub mod dns;
pub mod http;
pub mod icmp;
pub mod tcp;

use anyhow::Result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("DNS resolution failed for {host}: {source}")]
    DnsResolution {
        host: String,
        source: std::io::Error,
    },

    #[error("connection timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("unexpected response: status {status}")]
    UnexpectedStatus { status: u16 },
}

/// Run a full blame check: gateway, DNS, WAN reachability.
pub async fn blame_check() -> Result<()> {
    tracing::info!("Blame check: probing gateway...");
    // TODO: ICMP gateway probe
    // TODO: DNS resolver check
    // TODO: HTTP/TCP WAN reachability
    // TODO: Compare LAN vs WAN to attribute blame
    tracing::info!("Blame check complete (stub)");
    Ok(())
}

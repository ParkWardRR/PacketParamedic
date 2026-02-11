use anyhow::Result;
use std::time::Duration;

pub mod dns;
pub mod http;
pub mod icmp;
pub mod tcp;

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

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BlameReport {
    pub verdict: String,
    pub confidence: u8,
    pub details: Vec<String>,
}

/// Run an immediate blame check sequence
pub async fn run_blame_check() -> Result<BlameReport> {
    let timeout = Duration::from_secs(2);
    let mut details = Vec::new();

    // 1. Check Gateway (Local Network)
    // TODO: Use system::network::get_default_gateway(). For now, try detection or fallback.
    let gateway =
        crate::system::network::get_default_gateway().unwrap_or_else(|_| "192.168.1.1".to_string());
    let icmp = icmp::IcmpProbe;

    let gw_res = icmp.run(&gateway, timeout).await?;
    if !gw_res.success {
        details.push(format!("Gateway ({}) unreachable.", gateway));
        return Ok(BlameReport {
            verdict: "Local Network Issue".to_string(),
            confidence: 90,
            details,
        });
    }
    details.push(format!(
        "Gateway ({}) ping: {:.1} ms (OK)",
        gateway, gw_res.value
    ));

    // 2. Check WAN (ISP)
    let wan_target = "8.8.8.8";
    let wan_res = icmp.run(wan_target, timeout).await?;
    if !wan_res.success {
        details.push(format!("WAN target ({}) unreachable.", wan_target));
        return Ok(BlameReport {
            verdict: "ISP / Internet Connection Issue".to_string(),
            confidence: 80,
            details,
        });
    }
    details.push(format!(
        "WAN ({}) ping: {:.1} ms (OK)",
        wan_target, wan_res.value
    ));

    // 3. Check DNS
    let dns = dns::DnsProbe::default();
    let dns_target = "google.com";
    let dns_res = dns.run(dns_target, timeout).await?;
    if !dns_res.success {
        details.push("DNS Resolution failed.".to_string());
        return Ok(BlameReport {
            verdict: "DNS Configuration Issue".to_string(),
            confidence: 75,
            details,
        });
    }
    details.push(format!(
        "DNS check ({}) resolved in {:.1} ms (OK)",
        dns_target, dns_res.value
    ));

    // 4. Check HTTP (Service)
    let http = http::HttpProbe::default();
    let http_target = "http://google.com";
    let http_res = http.run(http_target, timeout).await?;
    if !http_res.success {
        details.push("HTTP Request failed.".to_string());
        return Ok(BlameReport {
            verdict: "Service / Application Layer Issue".to_string(),
            confidence: 60,
            details,
        });
    }
    details.push(format!(
        "HTTP check ({}) took {:.1} ms (OK)",
        http_target, http_res.value
    ));

    Ok(BlameReport {
        verdict: "Healthy".to_string(),
        confidence: 95,
        details,
    })
}

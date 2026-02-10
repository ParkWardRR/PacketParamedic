use super::{Measurement, Probe, ProbeType};
use anyhow::Result;
use std::time::{Duration, Instant, SystemTime};
use trust_dns_resolver::TokioAsyncResolver;

/// DNS Resolution Probe
pub struct DnsProbe {
    resolver: TokioAsyncResolver,
}

impl Default for DnsProbe {
    fn default() -> Self {
        // Use system config (from /etc/resolv.conf)
        let resolver = TokioAsyncResolver::tokio_from_system_conf()
            .expect("Failed to create DNS resolver");
        Self { resolver }
    }
}

#[async_trait::async_trait]
impl Probe for DnsProbe {
    async fn run(&self, target: &str, _timeout: Duration) -> Result<Measurement> {
        let start = Instant::now();
        
        let result = self.resolver.lookup_ip(target).await;

        let duration = start.elapsed();
        let timestamp = SystemTime::now();

        match result {
            Ok(lookup) => {
                // If we got IP addresses, success.
                // We don't necessarily care about the IPs themselves for availability, just that it resolved.
                let success = lookup.iter().count() > 0;
                
                Ok(Measurement {
                    probe_type: ProbeType::Dns,
                    target: target.to_string(),
                    value: duration.as_secs_f64() * 1000.0,
                    unit: "ms".to_string(),
                    success,
                    timestamp,
                })
            },
            Err(_) => {
                // Resolution failed (NXDOMAIN or Timeout)
                Ok(Measurement {
                    probe_type: ProbeType::Dns,
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

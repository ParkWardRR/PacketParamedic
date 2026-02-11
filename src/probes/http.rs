use super::{Measurement, Probe, ProbeType};
use anyhow::Result;
use reqwest::Client;
use std::time::{Duration, Instant, SystemTime};

/// HTTP Probe checking status code and TTFB
pub struct HttpProbe {
    client: Client,
}

impl Default for HttpProbe {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }
}

#[async_trait::async_trait]
impl Probe for HttpProbe {
    async fn run(&self, target: &str, _timeout: Duration) -> Result<Measurement> {
        let url = if target.starts_with("http") {
            target.to_string()
        } else {
            format!("http://{}", target)
        };

        let start = Instant::now();
        let result = self.client.get(&url).send().await;
        let duration = start.elapsed();
        let timestamp = SystemTime::now();

        match result {
            Ok(repl) => {
                let success = repl.status().is_success();
                let _status_code = repl.status().as_u16();

                // If success, value is latency. If 404/500, value is negative status code convention?
                // Or we stick to latency but mark success=false.
                // Let's stick to: value = latency, success = 200..299

                Ok(Measurement {
                    probe_type: ProbeType::Http,
                    target: target.to_string(),
                    value: duration.as_secs_f64() * 1000.0,
                    unit: "ms".to_string(),
                    success,
                    timestamp,
                })
            }
            Err(_) => Ok(Measurement {
                probe_type: ProbeType::Http,
                target: target.to_string(),
                value: -1.0,
                unit: "ms".to_string(),
                success: false,
                timestamp,
            }),
        }
    }
}

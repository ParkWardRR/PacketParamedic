use super::{SpeedTestProvider, ProviderMeta, SpeedTestRequest, SpeedTestResult, ProviderKind, Stability, MetricsSupported, Recommendation};
use anyhow::Result;
use std::process::Command;

pub struct FastProvider;

impl SpeedTestProvider for FastProvider {
    fn meta(&self) -> ProviderMeta {
        ProviderMeta {
            id: "fast-cli",
            display_name: "Fast.com (Netflix)",
            kind: ProviderKind::PublicWAN,
            recommendation: Recommendation::Optional, // Third choice
            description: "Measures download speed from Netflix servers. Good for streaming troubleshooting.",
            install_hint: "Install via npm: npm install --global fast-cli",
            licensing_note: None,
            stability: Stability::Beta, // Relies on undocumented API via cli tool
            metrics: MetricsSupported {
                download: true,
                upload: true, // fast-cli supports upload too
                latency: true,
                jitter: false,
                packet_loss: false,
                bufferbloat: true, // fast-cli reports bufferbloat!
            },
        }
    }

    fn is_available(&self) -> bool {
        Command::new("fast").arg("--version").output().is_ok()
    }

    fn run(&self, _req: SpeedTestRequest) -> Result<SpeedTestResult> {
        // Run: fast --json --upload
        let output = Command::new("fast")
            .arg("--json")
            .arg("--upload")
            .output()?;

        if !output.status.success() {
             return Err(anyhow::anyhow!("Fast CLI failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        let download = json.get("downloadSpeed").and_then(|v| v.as_f64());
        let upload = json.get("uploadSpeed").and_then(|v| v.as_f64());
        let latency = json.get("latency").and_then(|v| v.as_f64());
        let bufferbloat = json.get("bufferBloat").and_then(|v| v.as_f64());

        Ok(SpeedTestResult {
            provider_id: "fast-cli".to_string(),
            download_mbps: download,
            upload_mbps: upload, // Mbps
            latency_ms: latency,
            jitter_ms: None,
            packet_loss_pct: None,
            bufferbloat_ms: bufferbloat,
            raw_json: Some(json),
            timestamp: chrono::Utc::now(),
        })
    }
}

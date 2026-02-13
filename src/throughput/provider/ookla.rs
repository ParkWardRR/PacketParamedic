use super::{SpeedTestProvider, ProviderMeta, SpeedTestRequest, SpeedTestResult, ProviderKind, Stability, MetricsSupported, Recommendation};
use anyhow::Result;
use std::process::Command;

pub struct OoklaProvider;

#[async_trait::async_trait]
impl SpeedTestProvider for OoklaProvider {
    fn meta(&self) -> ProviderMeta {
        ProviderMeta {
            id: "ookla-cli",
            display_name: "Speedtest.net (Ookla)",
            kind: ProviderKind::PublicWAN,
            recommendation: Recommendation::Recommended,
            description: "The official industry-standard benchmark. Best for comparing against ISP marketing claims.",
            install_hint: "Install the official CLI: https://www.speedtest.net/apps/cli",
            licensing_note: Some("Personal Non-Commercial Use Only (EULA). Not for resale or router installation."),
            stability: Stability::Stable,
            metrics: MetricsSupported {
                download: true,
                upload: true,
                latency: true,
                jitter: true,
                packet_loss: true,
                bufferbloat: false, // Ookla CLI doesn't natively expose bufferbloat yet in JSON
            },
        }
    }

    fn is_available(&self) -> bool {
        // Check if `speedtest` is in PATH
        // Keep sync check for availability via std Command to avoid async complexity in is_available if possible, 
        // but trait is sync for is_available. std::process::Command is fine here.
        std::process::Command::new("speedtest").arg("--version").output().is_ok()
    }

    async fn run(&self, _req: SpeedTestRequest) -> Result<SpeedTestResult> {
        // Provide a real implementation stub that would fail gracefully but compiles
        // Run: speedtest --format=json --accept-license --accept-gdpr
        let output = tokio::process::Command::new("speedtest")
            .arg("--format=json")
            .arg("--accept-license")
            .arg("--accept-gdpr")
            .output()
            .await?;
            
        if !output.status.success() {
             return Err(anyhow::anyhow!("Ookla CLI failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        
        // Basic parsing (stub)
        let download = json.get("download").and_then(|v| v.get("bandwidth")).and_then(|v| v.as_f64()).map(|b| b * 8.0 / 1_000_000.0);
        let upload = json.get("upload").and_then(|v| v.get("bandwidth")).and_then(|v| v.as_f64()).map(|b| b * 8.0 / 1_000_000.0);
        let latency = json.get("ping").and_then(|v| v.get("latency")).and_then(|v| v.as_f64());
        let jitter = json.get("ping").and_then(|v| v.get("jitter")).and_then(|v| v.as_f64());
        let packet_loss = json.get("packetLoss").and_then(|v| v.as_f64());

        Ok(SpeedTestResult {
            provider_id: "ookla-cli".to_string(),
            download_mbps: download,
            upload_mbps: upload,
            latency_ms: latency,
            jitter_ms: jitter,
            packet_loss_pct: packet_loss,
            bufferbloat_ms: None,
            raw_json: Some(json),
            timestamp: chrono::Utc::now(),
        })
    }
}

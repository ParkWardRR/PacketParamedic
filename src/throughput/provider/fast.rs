use super::{SpeedTestProvider, ProviderMeta, SpeedTestRequest, SpeedTestResult, ProviderKind, Stability, MetricsSupported, Recommendation};
use anyhow::Result;
use std::process::Command;

pub struct FastProvider;

enum FastVariant {
    Node(String), // path to executable
    Go(String),   // path to executable
}

impl FastProvider {
    fn detect_variant() -> Option<FastVariant> {
        // 1. Check for Go version (Preferred if present as it's lighter/works headless)
        // Check PATH
        if let Ok(o) = Command::new("fast-cli").arg("--version").output() {
            if o.status.success() {
                 return Some(FastVariant::Go("fast-cli".to_string()));
            }
        }
        // Check local go/bin
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/alfa".to_string());
        let path = format!("{}/go/bin/fast-cli", home);
        if std::path::Path::new(&path).exists() {
             return Some(FastVariant::Go(path));
        }

        // 2. Check for Node version (Fallback)
        if let Ok(o) = Command::new("fast").arg("--version").output() {
            if o.status.success() {
                 // Double check it's not the go version aliased?
                 // Go version usually prints "github.com/gesquive/fast-cli"
                 let s = String::from_utf8_lossy(&o.stdout);
                 if s.contains("gesquive") {
                     return Some(FastVariant::Go("fast".to_string()));
                 }
                 return Some(FastVariant::Node("fast".to_string()));
            }
        }

        None
    }
}

#[async_trait::async_trait]
impl SpeedTestProvider for FastProvider {
    fn meta(&self) -> ProviderMeta {
        ProviderMeta {
            id: "fast",
            display_name: "Fast.com (Netflix)",
            kind: ProviderKind::PublicWAN,
            recommendation: Recommendation::Optional, 
            description: "Measures download speed from Netflix servers.",
            install_hint: "Install via npm: npm install --global fast-cli OR Go: go install github.com/gesquive/fast-cli@latest",
            licensing_note: None,
            stability: Stability::Beta, 
            metrics: MetricsSupported {
                download: true,
                upload: true, // Node version only
                latency: true, // Node version only
                jitter: false,
                packet_loss: false,
                bufferbloat: false, 
            },
        }
    }

    fn is_available(&self) -> bool {
        Self::detect_variant().is_some()
    }

    async fn run(&self, _req: SpeedTestRequest) -> Result<SpeedTestResult> {
        let variant = Self::detect_variant().ok_or_else(|| anyhow::anyhow!("Fast CLI not found"))?;

        // Note: detect_variant() uses synchronous Command, which is fine for small checks.
        // run() uses tokio::process::Command for the actual test.

        match variant {
            FastVariant::Node(exe) => {
                // Node Logic (JSON)
                let output = tokio::process::Command::new(exe)
                    .arg("--json")
                    .arg("--upload")
                    .output()
                    .await?;
        
                if !output.status.success() {
                     return Err(anyhow::anyhow!("Fast CLI (Node) failed: {}", String::from_utf8_lossy(&output.stderr)));
                }
        
                // Output from Node CLI might be noisy, try to parse JSON from stdout.
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Sometimes extra logs appear. Try to find the line that looks like JSON?
                // Or just parse_slice if clean.
                let json: serde_json::Value = serde_json::from_slice(&output.stdout)
                     .or_else(|_| serde_json::from_str(&stdout))?;

                let download = json.get("downloadSpeed").and_then(|v| v.as_f64());
                let upload = json.get("uploadSpeed").and_then(|v| v.as_f64());
                let latency = json.get("latency").and_then(|v| v.as_f64());
                let bufferbloat = json.get("bufferBloat").and_then(|v| v.as_f64());
        
                Ok(SpeedTestResult {
                    provider_id: "fast-node".to_string(),
                    download_mbps: download,
                    upload_mbps: upload, 
                    latency_ms: latency,
                    jitter_ms: None,
                    packet_loss_pct: None,
                    bufferbloat_ms: bufferbloat,
                    raw_json: Some(json),
                    timestamp: chrono::Utc::now(),
                })
            },
            FastVariant::Go(exe) => {
                // Go Logic (Text)
                // fast-cli --simple
                // Output: "85.2 Mbps\n"
                let output = tokio::process::Command::new(exe)
                    .arg("--simple")
                    .output()
                    .await?;
                
                if !output.status.success() {
                     return Err(anyhow::anyhow!("Fast CLI (Go) failed: {}", String::from_utf8_lossy(&output.stderr)));
                }
                
                let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let parts: Vec<&str> = s.split_whitespace().collect();
                let download = if !parts.is_empty() {
                    parts[0].parse::<f64>().ok()
                } else {
                    None
                };
                
                Ok(SpeedTestResult {
                    provider_id: "fast-go".to_string(),
                    download_mbps: download, 
                    upload_mbps: None,
                    latency_ms: None,
                    jitter_ms: None,
                    packet_loss_pct: None,
                    bufferbloat_ms: None,
                    raw_json: Some(serde_json::json!({ "raw_output": s })),
                    timestamp: chrono::Utc::now(),
                })
            }
        }
    }
}

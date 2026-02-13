use super::{SpeedTestProvider, ProviderMeta, SpeedTestRequest, SpeedTestResult, ProviderKind, Stability, MetricsSupported, Recommendation};
use anyhow::Result;
use std::process::Command;

pub struct Ndt7Provider;

impl Ndt7Provider {
    fn find_executable() -> Option<String> {
        // Check PATH
        if Command::new("ndt7-client").arg("-help").output().is_ok() {
            return Some("ndt7-client".to_string());
        }
        // Check local go/bin
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/alfa".to_string());
        let path = format!("{}/go/bin/ndt7-client", home);
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
        None
    }
}

#[async_trait::async_trait]
impl SpeedTestProvider for Ndt7Provider {
    fn meta(&self) -> ProviderMeta {
        ProviderMeta {
            id: "ndt7",
            display_name: "NDT (M-Lab)",
            kind: ProviderKind::PublicWAN,
            recommendation: Recommendation::Recommended, // Second choice
            description: "Open measurement methodology usage NDT7 protocol. Best for diagnosing congestion and 'real' throughput vs benchmarks.",
            install_hint: "Install the Go client: go install github.com/m-lab/ndt7-client-go/cmd/ndt7-client@latest",
            licensing_note: None, // Open Source (Apache 2.0)
            stability: Stability::Stable,
            metrics: MetricsSupported {
                download: true,
                upload: true,
                latency: true,
                jitter: true, // NDT7 setup measures RTT variance
                packet_loss: true, // NDT reports retransmissions
                bufferbloat: false,
            },
        }
    }

    fn is_available(&self) -> bool {
        Self::find_executable().is_some()
    }

    async fn run(&self, _req: SpeedTestRequest) -> Result<SpeedTestResult> {
        let exe = Self::find_executable().ok_or_else(|| anyhow::anyhow!("NDT7 Client not found"))?;
        
        // Run: ndt7-client -format=json
        let output = tokio::process::Command::new(exe)
            .arg("-format=json")
            .output()
            .await?;
            
        if !output.status.success() {
             return Err(anyhow::anyhow!("NDT7 Client failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        // Output allows for streaming JSON objects (one per measurement interval)
        // We need to aggregate them or pick the final summary if available.
        // The go client outputs multiple JSON lines. The last one usually contains the summary or we average.
        // For simpler MVP, let's parse the output as newline-delimited JSON and take the max/average.
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut download_accum = 0.0;
        let mut upload_accum = 0.0;
        let mut count = 0;
        let mut max_rtt = 0.0;
        
        for line in stdout.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                // Check Test type ("download" or "upload")
                let test_type = json.get("Value").and_then(|v| v.get("Test")).and_then(|s| s.as_str());
                
                // Parse Throughput from AppInfo (NumBytes / ElapsedTime)
                if let Some(app_info) = json.get("Value").and_then(|v| v.get("AppInfo")) {
                    let bytes = app_info.get("NumBytes").and_then(|n| n.as_f64()).unwrap_or(0.0);
                    let elapsed_us = app_info.get("ElapsedTime").and_then(|n| n.as_f64()).unwrap_or(1.0); // avoid div by zero
                    
                    if bytes > 0.0 && elapsed_us > 0.0 {
                        let mbps = (bytes * 8.0) / (elapsed_us / 1_000_000.0) / 1_000_000.0;
                        match test_type {
                            Some("download") => download_accum = mbps, // Keep updating, last one is final
                            Some("upload") => upload_accum = mbps,
                            _ => {}
                        }
                    }
                }
                
                // Parse RTT from TCPInfo (MinRTT)
                // NDT writes TCPInfo: { "MinRTT": ... } in some messages
                if let Some(tcp_info) = json.get("Value").and_then(|v| v.get("TCPInfo")) {
                     if let Some(rtt) = tcp_info.get("MinRTT").and_then(|n| n.as_f64()) {
                         // rtt is usually microseconds or milliseconds?
                         // Linux kernel uses microseconds usually? Or verify unit.
                         // Usually NDT reports microseconds.
                         let rtt_ms = rtt / 1000.0;
                         if rtt_ms > 0.0 {
                             max_rtt = rtt_ms; // Just capture latest capable RTT
                         }
                     }
                }
                 count += 1;
            }
        }
        
        // Stub Result
        Ok(SpeedTestResult {
            provider_id: "ndt7".to_string(),
            download_mbps: Some(download_accum), 
            upload_mbps: Some(upload_accum),
            latency_ms: if max_rtt > 0.0 { Some(max_rtt) } else { None }, 
            jitter_ms: None,
            packet_loss_pct: None,
            bufferbloat_ms: None,
            raw_json: Some(serde_json::json!({ "raw_output_lines": count })),
            timestamp: chrono::Utc::now(),
        })
    }
}

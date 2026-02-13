
use anyhow::{anyhow, Context, Result};
use std::net::SocketAddr;
use crate::throughput::provider::{SpeedTestProvider, SpeedTestRequest, SpeedTestResult, ProviderMeta, ProviderKind, Stability, MetricsSupported, Recommendation};
use crate::reflector_proto::{client::ReflectorClient, identity::Identity};

pub struct ReflectorProvider;

#[async_trait::async_trait]
impl SpeedTestProvider for ReflectorProvider {
    fn meta(&self) -> ProviderMeta {
        ProviderMeta {
            id: "reflector",
            display_name: "PacketParamedic Reflector",
            kind: ProviderKind::SelfHostedWAN,
            recommendation: Recommendation::Recommended,
            description: "Dedicated high-performance endpoint using Paramedic Link protocol.",
            install_hint: "Deploy a Reflector instance using Docker or the binary.",
            licensing_note: None,
            stability: Stability::Stable,
            metrics: MetricsSupported {
                download: true,
                upload: true,
                latency: false, // Could be derived from iperf3
                jitter: true,   // iperf3 returns jitter
                packet_loss: true,
                bufferbloat: false,
            },
        }
    }

    fn is_available(&self) -> bool {
         std::process::Command::new("iperf3").arg("-v").output().is_ok()
    }

    async fn run(&self, req: SpeedTestRequest) -> Result<SpeedTestResult> {
         // 1. Get Control Plane Address
         let host_str = req.server_hint.ok_or_else(|| anyhow!("Reflector provider requires a host (use --peer)"))?;
         let control_addr: SocketAddr = host_str.parse().context("Invalid reflector address (e.g. 1.2.3.4:4000)")?;
         
         // 2. Load Identity
         let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
         let path = std::path::Path::new(&home).join(".packetparamedic/identity.key");
         if !path.exists() {
             return Err(anyhow!("Identity key not found at {}. Run 'packetparamedic pair-reflector' first.", path.display()));
         }
         let identity = Identity::load(&path).context("Failed to load identity")?;
         
         // 3. Connect Control Plane
         let mut client = ReflectorClient::connect(control_addr, &identity).await?;
         
         // Hardcoded streams/test parameters for now since SpeedTestRequest is limited
         let streams = 4; // Parallel streams used by Reflector usually
         let duration = 10.max(req.timeout.as_secs()); // Ensure at least 10s
         
         let data_plane_ip = control_addr.ip().to_string();

         // 4. Run Upload (Client -> Server)
         // Note: Reflector protocol 'reverse' param in SessionRequest means "Server sends to Client" (Download).
         // So for Upload: reverse = false.
         let up_grant = client.request_throughput_session(duration, streams, false).await?;
         tracing::info!(?up_grant, "Received throughput session grant (Upload)");
         // Note: up_grant.port is the data plane port on the server.
         
         // Add a small delay to allow the server's iperf3 process to initialize and bind the port.
         tokio::time::sleep(std::time::Duration::from_millis(500)).await;

         let up_mbps = run_iperf3_async(&data_plane_ip, up_grant.port, duration, streams, false).await?;

         // 5. Run Download (Client <- Server)
         // reverse = true.
         let down_grant = client.request_throughput_session(duration, streams, true).await?;
         tracing::info!(?down_grant, "Received throughput session grant (Download)");
         // Add a small delay to allow the server's iperf3 process to initialize and bind the port.
         tokio::time::sleep(std::time::Duration::from_millis(500)).await;

         let down_mbps = run_iperf3_async(&data_plane_ip, down_grant.port, duration, streams, true).await?;

         Ok(SpeedTestResult {
             provider_id: "reflector".to_string(),
             download_mbps: Some(down_mbps),
             upload_mbps: Some(up_mbps),
             latency_ms: None, // todo: extract from iperf json
             jitter_ms: None,
             packet_loss_pct: None,
             bufferbloat_ms: None,
             raw_json: None,
             timestamp: chrono::Utc::now(),
         })
    }
}

async fn run_iperf3_async(host: &str, port: u16, duration: u64, streams: u32, reverse: bool) -> Result<f64> {
    // Construct arguments for iperf3 itself
    let mut iperf_args = vec![
        "-c".to_string(),
        host.to_string(),
        "-p".to_string(),
        port.to_string(),
        "-t".to_string(),
        duration.to_string(),
        "-P".to_string(),
        streams.to_string(),
        "-J".to_string(),
    ];
    
    if reverse {
        iperf_args.push("-R".to_string());
    }

    // Determine executable and final args based on OS/taskset availability
    let (exe, final_args) = if cfg!(target_os = "linux") {
        // Use taskset on Linux (Pi 5 optimization)
        let mut args = vec!["-c".to_string(), "2,3".to_string(), "iperf3".to_string()];
        args.extend(iperf_args);
        ("taskset", args)
    } else {
        // Direct iperf3 on macOS/other
        ("iperf3", iperf_args)
    };

    tracing::debug!("Running: {} {:?}", exe, final_args);
    tracing::info!(?final_args, "Executing iperf3 command");

    let output = tokio::process::Command::new(exe)
        .args(&final_args)
        .output()
        .await?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "iperf3 failed: stdout: {}, stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    // Parse JSON
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    
    // Extract throughput (sum_received preferred for accuracy)
    let end = json.get("end").ok_or_else(|| anyhow::anyhow!("No 'end' field in iperf3 JSON"))?;
    
    let sum_received = end.get("sum_received").and_then(|v| v.get("bits_per_second")).and_then(|v| v.as_f64());
    let sum_sent = end.get("sum_sent").and_then(|v| v.get("bits_per_second")).and_then(|v| v.as_f64());
    
    // Prefer received (goodput), fallback to sent if missing (e.g. UDP sender report only)
    let bits_per_second = sum_received.or(sum_sent).ok_or_else(|| anyhow::anyhow!("Could not find throughput data in JSON"))?;
        
    Ok(bits_per_second / 1_000_000.0)
}

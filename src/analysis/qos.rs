use crate::throughput;
use crate::probes::{Probe, icmp::IcmpProbe};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

#[derive(Debug, serde::Serialize)]
pub struct QosResult {
    pub target: String,
    pub baseline_rtt_ms: f64,
    pub loaded_rtt_ms: f64,
    pub bufferbloat_ms: f64,
    pub grade: char,
    pub download_mbps: Option<f64>, // If we capture it
}

pub async fn run_qos_test(target: &str) -> anyhow::Result<QosResult> {
    info!("Phase 13: Starting QoS / Bufferbloat analysis against {}", target);

    // 1. Measure Baseline (Idle)
    info!("Measuring idle baseline latency...");
    let baseline_rtt = measure_rtt_batch(target, 10, Duration::from_millis(200)).await?;
    info!("Baseline RTT: {:.2} ms", baseline_rtt);

    // 2. Setup Background Pinger for Load Phase
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let target_clone = target.to_string();
    
    let pinger_handle = tokio::spawn(async move {
        let probe = IcmpProbe;
        loop {
            // Check if receiver dropped (test done)
            if tx.is_closed() { break; }
            
            let start = std::time::Instant::now();
            match probe.run(&target_clone, Duration::from_secs(1)).await {
                Ok(m) => {
                    if m.success {
                        if tx.send(m.value).await.is_err() { break; }
                    }
                }
                Err(_) => {}
            }
            
            // Aim for 5Hz (200ms interval) accounting for execution time
            let elapsed = start.elapsed();
            if elapsed < Duration::from_millis(200) {
                sleep(Duration::from_millis(200) - elapsed).await;
            }
        }
    });

    // 3. Saturate Link (Download)
    info!("Saturating downstream bandwidth (10s)...");
    // We use iperf3 "wan" mode, 4 streams for max load
    // Note: run_test returns Result<ThroughputResult> (if I updated it) or just Result<()> (logging only).
    // Reviewing throughput/mod.rs implies it returns Result<()>.
    // So we don't get Mbps programmatically easily unless we parse logs or update run_test.
    // For QoS, we care about the LATENCY impact.
    let load_start = std::time::Instant::now();
    let load_result = throughput::run_test("wan", None, "10s", 4).await;
    
    // 4. Stop Pinger
    // We abort the task to stop it immediately
    pinger_handle.abort();
    
    if let Err(e) = load_result {
        warn!("Throughput test failed: {}. Bufferbloat metric may be invalid.", e);
    }

    // 5. Collect Loaded Metrics
    let mut loaded_samples = Vec::new();
    while let Ok(rtt) = rx.try_recv() {
        loaded_samples.push(rtt);
    }
    
    // Filter samples that occurred *during* the load? 
    // The pinger ran concurrently. 
    // We assume samples collected are "loaded".
    
    let loaded_rtt = if loaded_samples.is_empty() {
        warn!("No latency samples collected during load!");
        baseline_rtt // Fallback
    } else {
        // Use Median or Mean? Mean is fine.
        let sum: f64 = loaded_samples.iter().sum();
        sum / loaded_samples.len() as f64
    };
    
    info!("Loaded RTT: {:.2} ms (samples: {})", loaded_rtt, loaded_samples.len());

    // 6. Calculate Grade
    let bloat = (loaded_rtt - baseline_rtt).max(0.0);
    let grade = match bloat {
        b if b < 5.0 => 'A', // Excellent
        b if b < 30.0 => 'B', // Good
        b if b < 60.0 => 'C', // Fair
        b if b < 150.0 => 'D', // Poor
        _ => 'F', // Bad
    };

    Ok(QosResult {
        target: target.to_string(),
        baseline_rtt_ms: baseline_rtt,
        loaded_rtt_ms: loaded_rtt,
        bufferbloat_ms: bloat,
        grade,
        download_mbps: None, // Not captured yet
    })
}

async fn measure_rtt_batch(target: &str, count: usize, interval: Duration) -> anyhow::Result<f64> {
    let mut total = 0.0;
    let mut valid = 0;
    let probe = IcmpProbe;
    
    for _ in 0..count {
        if let Ok(m) = probe.run(target, Duration::from_secs(1)).await {
            if m.success {
                total += m.value;
                valid += 1;
            }
        }
        sleep(interval).await;
    }
    
    if valid == 0 {
        anyhow::bail!("Baseline measurement failed (all pings lost)");
    }
    
    Ok(total / valid as f64)
}

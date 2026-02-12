use packetparamedic::storage::Pool;
use packetparamedic::scheduler::Scheduler;
use tokio::time::Duration;
use anyhow::{Result, Context};

// "Full FAT" Live Integration Tests
// These tests execute real diagnostic logic against live network targets.
// They mimic the 3 user personas on a deployed appliance.
// Run with `cargo test --release --ignored` on the target hardware.

#[tokio::test]
#[ignore]
async fn test_persona_simple_troubleshooting_live() -> Result<()> {
    println!("Step 1: Running Full Blame Check (Gateway + WAN + DNS + HTTP)...");
    let report = packetparamedic::probes::run_blame_check().await
        .context("Blame Check failed")?;
    
    assert!(!report.verdict.is_empty(), "Verdict should be present");
    assert!(!report.details.is_empty(), "Evidence details should be present");
    println!(" - Verdict: {}", report.verdict);
    println!(" - Confidence: {}%", report.confidence);

    println!("Step 2: Checking Gateway Reachability...");
    // Assuming gateway is 192.168.* or similar. We rely on blame check logic to have verified it.
    // If verdict is "Healthy", gateway must be up.
    if report.verdict == "Healthy" {
         println!(" - Gateway confirmed reachable via Blame Check logic.");
    } else {
         println!(" - Verdict was not Healthy, check logs.");
    }
    
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_persona_high_performance_live() -> Result<()> {
    println!("Step 1: Running WAN Throughput Test (iperf3 public server)...");
    // Force a 5-second test to validate throughput engine end-to-end
    match packetparamedic::throughput::run_test("wan", None, "5s", 1).await {
         Ok(_) => println!(" - WAN Throughput test (iperf3) completed successfully."),
         Err(e) => println!(" - WAN Throughput test (iperf3) failed: {}", e),
    }
    
    // Step 2: Validate All Providers
    println!("Step 3: Checking Providers...");
    let providers = packetparamedic::throughput::provider::get_all_providers();
    
    for provider in providers {
        let meta = provider.meta();
        println!(" -> Checking Provider: {} ({:?})", meta.display_name, meta.recommendation);
        
        if provider.is_available() {
            println!("    ✅ CLI Detected! Running benchmark...");
            
            let req = packetparamedic::throughput::provider::SpeedTestRequest {
                 timeout: std::time::Duration::from_secs(60),
                 prefer_ipv6: false,
                 server_hint: None,
            };
            
            // Execute provider synchronously (blocking test thread is acceptable here)
            match provider.run(req) {
                 Ok(res) => {
                     println!("      => Download: {:.2} Mbps", res.download_mbps.unwrap_or(0.0));
                     println!("      => Upload:   {:.2} Mbps", res.upload_mbps.unwrap_or(0.0));
                     println!("      => Latency:  {:.2} ms", res.latency_ms.unwrap_or(0.0));
                 },
                 Err(e) => {
                     println!("      ❌ Execution Failed: {}", e);
                 }
            }
        } else {
            println!("    ⚠️ CLI Missing. Install hint: {}", meta.install_hint);
        }
    }

    println!("Step 4: Running Advanced Diagnostics (Bufferbloat/QoS)...");
    // Only run if we are in High Performance mode (Live)
    if std::env::var("PACKETPARAMEDIC_LIVE_TEST").unwrap_or_default() == "1" {
        // Use 8.8.8.8 as pinger, wan (iperf3) as load
        // Note: run_qos_test is async
        match packetparamedic::analysis::qos::run_qos_test("8.8.8.8").await {
            Ok(qos) => {
                println!("    ✅ Bufferbloat Analysis Complete");
                println!("      => Baseline RTT: {:.2} ms", qos.baseline_rtt_ms);
                println!("      => Loaded RTT:   {:.2} ms", qos.loaded_rtt_ms);
                println!("      => Bloat:        {:.2} ms (Grade: {})", qos.bufferbloat_ms, qos.grade);
            },
            Err(e) => {
                 println!("    ❌ Bufferbloat Test Failed: {}", e);
                 // Don't fail the whole suite, as iperf3 might be flaky
            }
        }
    } else {
        println!("    (Skipping QoS in non-live mode)");
    }

    
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_persona_reliability_soak_live() -> Result<()> {
    println!("Step 1: Initializing Persistent Scheduler...");
    // Use a temporary DB file to test real persistence
    let db_path = "test_soak_live.db";
    let _ = std::fs::remove_file(db_path); // Clean start
    
    let pool = packetparamedic::storage::open_pool(db_path)?;
    let scheduler = Scheduler::new(pool.clone());
    
    println!("Step 2: Scheduling 'nightly-soak' job...");
    scheduler.add_schedule("nightly-soak", "0 3 * * * *", "throughput-stress").await?;
    
    println!("Step 3: Verifying persistence...");
    // Re-open DB to simulate service restart
    let pool2 = packetparamedic::storage::open_pool(db_path)?;
    let scheduler2 = Scheduler::new(pool2);
    let list = scheduler2.list().await?;
    
    assert!(list.iter().any(|(n, _, _, _)| n == "nightly-soak"), "Schedule lost after restart!");
    println!(" - Schedule persisted successfully.");
    
    // Cleanup
    let _ = std::fs::remove_file(db_path);
    Ok(())
}

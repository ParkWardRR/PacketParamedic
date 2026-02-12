//! Hardware and environment self-test for the PacketParamedic Reflector.
//!
//! Validates that the host running the reflector has the CPU, memory, network,
//! and tooling needed to saturate a 1 Gbps link during throughput tests.
//! Produces a structured report with pass/warn/fail verdicts and remediation
//! guidance.

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::Serialize;
use tracing::info;

use crate::config::ReflectorConfig;

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

/// Overall self-test report.
#[derive(Debug, Serialize, Clone)]
pub struct SelfTestReport {
    /// Individual component check results.
    pub results: Vec<ComponentResult>,
    /// Capability verdicts derived from the component results.
    pub capabilities: HashMap<String, bool>,
    /// One-line summary: "READY", "DEGRADED", or "NOT READY".
    pub verdict: String,
    /// Estimated maximum sustainable throughput (Mbps) based on checks.
    pub estimated_max_mbps: u32,
}

/// Result of a single component check.
#[derive(Debug, Serialize, Clone)]
pub struct ComponentResult {
    pub component: String,
    pub status: TestStatus,
    pub details: String,
    pub remediation: Option<String>,
    /// Measured value if applicable (e.g. "940 Mbps", "3.2 GB").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measured: Option<String>,
}

/// Status of a single check.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub enum TestStatus {
    Pass,
    Fail,
    Warning,
    Skipped,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Pass => write!(f, "PASS"),
            TestStatus::Fail => write!(f, "FAIL"),
            TestStatus::Warning => write!(f, "WARN"),
            TestStatus::Skipped => write!(f, "SKIP"),
        }
    }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the full self-test suite and return a structured report.
pub async fn run(config: &ReflectorConfig) -> SelfTestReport {
    info!("self-test: checking host readiness for 1 Gbps reflector operation");

    let mut results = Vec::new();

    // 1. CPU
    results.push(check_cpu());

    // 2. CPU features (AVX2 on x86_64, NEON on aarch64)
    results.push(check_cpu_features());

    // 3. Memory
    results.push(check_memory());

    // 4. Network interfaces
    results.extend(check_network_interfaces());

    // 5. iperf3 availability
    results.push(check_iperf3(&config.iperf3.path));

    // 6. Loopback throughput (iperf3 self-test)
    results.push(check_loopback_throughput(&config.iperf3.path).await);

    // 7. Disk I/O (audit log write speed)
    results.push(check_disk_io().await);

    // 8. Crypto performance (Ed25519 sign/verify throughput)
    results.push(check_crypto_performance());

    // 9. System clock / NTP sync
    results.push(check_time_sync());

    // 10. Open file limits
    results.push(check_ulimits());

    info!("self-test complete: {} checks run", results.len());

    // Derive capabilities and verdict.
    let capabilities = derive_capabilities(&results);
    let estimated_max_mbps = estimate_max_throughput(&results);
    let verdict = derive_verdict(&results, estimated_max_mbps);

    SelfTestReport {
        results,
        capabilities,
        verdict,
        estimated_max_mbps,
    }
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

/// Check CPU core count and clock speed adequacy for 1 Gbps.
fn check_cpu() -> ComponentResult {
    let sys = sysinfo::System::new_all();
    let cpu_count = sys.cpus().len();
    let cpu_brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "unknown".into());
    let freq_mhz = sys
        .cpus()
        .first()
        .map(|c| c.frequency())
        .unwrap_or(0);

    // For 1 Gbps iperf3: need at least 2 cores. 4+ is comfortable.
    // N100 has 4 cores at ~3.4 GHz — easily sufficient.
    let (status, remediation) = if cpu_count >= 4 {
        (TestStatus::Pass, None)
    } else if cpu_count >= 2 {
        (
            TestStatus::Warning,
            Some("2-core host may struggle at sustained 1 Gbps with concurrent mTLS. 4+ cores recommended.".into()),
        )
    } else {
        (
            TestStatus::Fail,
            Some("Single-core host cannot sustain 1 Gbps throughput testing. Upgrade to 2+ core CPU.".into()),
        )
    };

    ComponentResult {
        component: "CPU".into(),
        status,
        details: format!("{} ({} cores, {} MHz)", cpu_brand, cpu_count, freq_mhz),
        remediation,
        measured: Some(format!("{} cores @ {} MHz", cpu_count, freq_mhz)),
    }
}

/// Check for SIMD instruction support (AVX2 on x86_64, NEON on aarch64).
fn check_cpu_features() -> ComponentResult {
    #[cfg(target_arch = "x86_64")]
    {
        let has_avx2 = is_x86_feature_detected!("avx2");
        let has_aes = is_x86_feature_detected!("aes");
        let has_sse42 = is_x86_feature_detected!("sse4.2");

        let mut features = Vec::new();
        if has_avx2 {
            features.push("AVX2");
        }
        if has_aes {
            features.push("AES-NI");
        }
        if has_sse42 {
            features.push("SSE4.2");
        }

        let (status, remediation) = if has_avx2 && has_aes {
            (TestStatus::Pass, None)
        } else if has_sse42 {
            (
                TestStatus::Warning,
                Some("AVX2 not detected. Crypto operations will be slower. Consider a newer CPU.".into()),
            )
        } else {
            (
                TestStatus::Warning,
                Some("Limited SIMD support. TLS throughput may be constrained.".into()),
            )
        };

        ComponentResult {
            component: "CPU Features".into(),
            status,
            details: format!("Detected: {}", features.join(", ")),
            remediation,
            measured: None,
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // NEON is mandatory on aarch64
        ComponentResult {
            component: "CPU Features".into(),
            status: TestStatus::Pass,
            details: "ARM NEON/ASIMD detected (mandatory on aarch64)".into(),
            remediation: None,
            measured: None,
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        ComponentResult {
            component: "CPU Features".into(),
            status: TestStatus::Skipped,
            details: format!("Unsupported architecture: {}", std::env::consts::ARCH),
            remediation: None,
            measured: None,
        }
    }
}

/// Check available memory. 1 Gbps iperf3 with 4 streams needs ~128 MB buffers.
fn check_memory() -> ComponentResult {
    let sys = sysinfo::System::new_all();
    let total_mb = sys.total_memory() / (1024 * 1024);
    let available_mb = sys.available_memory() / (1024 * 1024);

    // iperf3 with 4 streams at 1 Gbps needs ~128 MB of socket buffers.
    // Reflector itself uses ~20 MB. Total comfortable minimum: 512 MB available.
    let (status, remediation) = if available_mb >= 512 {
        (TestStatus::Pass, None)
    } else if available_mb >= 256 {
        (
            TestStatus::Warning,
            Some("Low available memory. 1 Gbps tests with multiple streams may be constrained.".into()),
        )
    } else {
        (
            TestStatus::Fail,
            Some("Insufficient memory for 1 Gbps throughput testing. Need 512 MB+ available.".into()),
        )
    };

    ComponentResult {
        component: "Memory".into(),
        status,
        details: format!("{} MB total, {} MB available", total_mb, available_mb),
        remediation,
        measured: Some(format!("{} MB available", available_mb)),
    }
}

/// Scan network interfaces for link speed and 1 Gbps capability.
fn check_network_interfaces() -> Vec<ComponentResult> {
    let mut results = Vec::new();

    // Try reading from /sys/class/net (Linux)
    let net_dir = Path::new("/sys/class/net");
    if !net_dir.exists() {
        // macOS or non-Linux: try ifconfig fallback
        results.push(check_network_fallback());
        return results;
    }

    let entries = match std::fs::read_dir(net_dir) {
        Ok(e) => e,
        Err(e) => {
            results.push(ComponentResult {
                component: "Network".into(),
                status: TestStatus::Warning,
                details: format!("Cannot enumerate interfaces: {}", e),
                remediation: Some("Ensure /sys/class/net is readable.".into()),
                measured: None,
            });
            return results;
        }
    };

    let mut found_1g = false;
    let mut max_speed = 0i32;

    for entry in entries.flatten() {
        let iface = entry.file_name().to_string_lossy().to_string();
        if iface == "lo" {
            continue;
        }

        // Read link speed
        let speed_path = entry.path().join("speed");
        let speed_mbps = std::fs::read_to_string(&speed_path)
            .unwrap_or_default()
            .trim()
            .parse::<i32>()
            .unwrap_or(-1);

        // Read operstate
        let state_path = entry.path().join("operstate");
        let state = std::fs::read_to_string(&state_path)
            .unwrap_or_else(|_| "unknown".into())
            .trim()
            .to_string();

        if speed_mbps <= 0 && state != "up" {
            continue; // Skip interfaces that are down with no speed
        }

        if speed_mbps > max_speed {
            max_speed = speed_mbps;
        }

        let (status, remediation) = if speed_mbps >= 10000 {
            found_1g = true;
            (TestStatus::Pass, None)
        } else if speed_mbps >= 2500 {
            found_1g = true;
            (TestStatus::Pass, None)
        } else if speed_mbps >= 1000 {
            found_1g = true;
            (TestStatus::Pass, None)
        } else if speed_mbps >= 100 {
            (
                TestStatus::Warning,
                Some("100 Mbps link. Cannot reach 1 Gbps. Upgrade NIC or check cable.".into()),
            )
        } else {
            (
                TestStatus::Warning,
                Some(format!("Low link speed ({} Mbps). Check cable and switch.", speed_mbps)),
            )
        };

        let details = format!(
            "{}: {} Mbps (state: {})",
            iface,
            if speed_mbps > 0 {
                speed_mbps.to_string()
            } else {
                "unknown".into()
            },
            state
        );

        results.push(ComponentResult {
            component: format!("Network: {}", iface),
            status,
            details,
            remediation,
            measured: if speed_mbps > 0 {
                Some(format!("{} Mbps", speed_mbps))
            } else {
                None
            },
        });
    }

    if results.is_empty() {
        results.push(ComponentResult {
            component: "Network".into(),
            status: TestStatus::Warning,
            details: "No active network interfaces detected".into(),
            remediation: Some("Ensure at least one NIC is connected and up.".into()),
            measured: None,
        });
    }

    if !found_1g && !results.is_empty() {
        results.push(ComponentResult {
            component: "Network: 1G Capability".into(),
            status: TestStatus::Fail,
            details: format!("No interface with 1000+ Mbps link detected (max: {} Mbps)", max_speed),
            remediation: Some("Connect a 1 Gbps or faster NIC for full throughput testing.".into()),
            measured: None,
        });
    }

    results
}

/// Fallback network check for macOS / non-Linux.
fn check_network_fallback() -> ComponentResult {
    // Try `ifconfig` or just report that we can't check
    let output = std::process::Command::new("ifconfig")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let iface_count = stdout.matches("flags=").count();
            ComponentResult {
                component: "Network".into(),
                status: TestStatus::Warning,
                details: format!(
                    "{} interfaces detected (link speed check not available on this platform)",
                    iface_count
                ),
                remediation: Some("Run self-test on Linux for accurate NIC speed detection.".into()),
                measured: None,
            }
        }
        Err(_) => ComponentResult {
            component: "Network".into(),
            status: TestStatus::Skipped,
            details: "Cannot detect network interfaces on this platform".into(),
            remediation: None,
            measured: None,
        },
    }
}

/// Check that iperf3 is installed and functional.
fn check_iperf3(iperf3_path: &str) -> ComponentResult {
    let output = std::process::Command::new(iperf3_path)
        .arg("--version")
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("unknown")
                .to_string();
            ComponentResult {
                component: "iperf3".into(),
                status: TestStatus::Pass,
                details: version,
                remediation: None,
                measured: None,
            }
        }
        Ok(out) => ComponentResult {
            component: "iperf3".into(),
            status: TestStatus::Fail,
            details: format!(
                "iperf3 found but returned error: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            remediation: Some(format!("Reinstall iperf3: apt install iperf3 / dnf install iperf3")),
            measured: None,
        },
        Err(_) => ComponentResult {
            component: "iperf3".into(),
            status: TestStatus::Fail,
            details: format!("iperf3 not found at '{}'", iperf3_path),
            remediation: Some("Install iperf3: apt install iperf3 / dnf install iperf3".into()),
            measured: None,
        },
    }
}

/// Run a 5-second loopback iperf3 test to measure raw TCP throughput capacity.
///
/// This is the key "can we push 1 Gbps?" check. Loopback removes NIC as a
/// variable and measures CPU + kernel networking stack.
async fn check_loopback_throughput(iperf3_path: &str) -> ComponentResult {
    // Check iperf3 exists first
    if std::process::Command::new(iperf3_path)
        .arg("--version")
        .output()
        .is_err()
    {
        return ComponentResult {
            component: "Loopback Throughput".into(),
            status: TestStatus::Skipped,
            details: "iperf3 not available, skipping loopback test".into(),
            remediation: None,
            measured: None,
        };
    }

    info!("self-test: running 5-second loopback iperf3 throughput test");

    // Start iperf3 server on loopback
    let server = tokio::process::Command::new(iperf3_path)
        .args(["-s", "-p", "5199", "--one-off"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    let mut server_child = match server {
        Ok(child) => child,
        Err(e) => {
            return ComponentResult {
                component: "Loopback Throughput".into(),
                status: TestStatus::Warning,
                details: format!("Failed to start iperf3 server: {}", e),
                remediation: Some("Ensure iperf3 can bind to port 5199.".into()),
                measured: None,
            };
        }
    };

    // Give server a moment to bind
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Run client against loopback
    let client = tokio::process::Command::new(iperf3_path)
        .args(["-c", "127.0.0.1", "-p", "5199", "-t", "5", "-J", "-P", "4"])
        .output()
        .await;

    // Clean up server
    let _ = server_child.kill().await;

    match client {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Parse iperf3 JSON for sum_received.bits_per_second
            let throughput_mbps = parse_iperf3_throughput(&stdout);

            let (status, remediation) = if throughput_mbps >= 10000.0 {
                (TestStatus::Pass, None)
            } else if throughput_mbps >= 1000.0 {
                (TestStatus::Pass, None)
            } else if throughput_mbps >= 500.0 {
                (
                    TestStatus::Warning,
                    Some("Loopback throughput below 1 Gbps. CPU may bottleneck real tests.".into()),
                )
            } else {
                (
                    TestStatus::Fail,
                    Some("Loopback throughput too low for 1 Gbps testing. Check CPU load and system resources.".into()),
                )
            };

            ComponentResult {
                component: "Loopback Throughput".into(),
                status,
                details: format!(
                    "Loopback iperf3 (4 streams, 5s): {:.0} Mbps",
                    throughput_mbps
                ),
                remediation,
                measured: Some(format!("{:.0} Mbps", throughput_mbps)),
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            ComponentResult {
                component: "Loopback Throughput".into(),
                status: TestStatus::Warning,
                details: format!("iperf3 client failed: {}", stderr),
                remediation: Some("Port 5199 may be in use. Try again after a moment.".into()),
                measured: None,
            }
        }
        Err(e) => ComponentResult {
            component: "Loopback Throughput".into(),
            status: TestStatus::Warning,
            details: format!("Failed to run iperf3 client: {}", e),
            remediation: None,
            measured: None,
        },
    }
}

/// Parse iperf3 JSON output for total throughput in Mbps.
fn parse_iperf3_throughput(json_str: &str) -> f64 {
    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return 0.0,
    };

    // Try end.sum_received.bits_per_second first
    if let Some(bps) = parsed
        .get("end")
        .and_then(|e| e.get("sum_received"))
        .and_then(|s| s.get("bits_per_second"))
        .and_then(|v| v.as_f64())
    {
        return bps / 1_000_000.0;
    }

    // Fallback: try sum.bits_per_second in end.streams
    if let Some(bps) = parsed
        .get("end")
        .and_then(|e| e.get("sum"))
        .and_then(|s| s.get("bits_per_second"))
        .and_then(|v| v.as_f64())
    {
        return bps / 1_000_000.0;
    }

    0.0
}

/// Check disk I/O by writing and fsyncing 1 MB of data.
async fn check_disk_io() -> ComponentResult {
    let test_path = std::env::temp_dir().join("reflector_selftest_io.tmp");

    let data = vec![0x42u8; 1_048_576]; // 1 MB
    let start = Instant::now();
    let iterations = 10;

    for _ in 0..iterations {
        match tokio::fs::write(&test_path, &data).await {
            Ok(_) => {}
            Err(e) => {
                return ComponentResult {
                    component: "Disk I/O".into(),
                    status: TestStatus::Warning,
                    details: format!("Write test failed: {}", e),
                    remediation: None,
                    measured: None,
                };
            }
        }
    }

    let elapsed = start.elapsed();
    let _ = tokio::fs::remove_file(&test_path).await;

    let mb_per_sec = (iterations as f64) / elapsed.as_secs_f64();

    // Audit log writes are tiny (< 1 KB per entry). Even slow disks are fine.
    // But we flag if writes are extremely slow (< 1 MB/s), which could
    // indicate a failing disk or saturated I/O.
    let (status, remediation) = if mb_per_sec >= 10.0 {
        (TestStatus::Pass, None)
    } else if mb_per_sec >= 1.0 {
        (
            TestStatus::Warning,
            Some("Disk write speed is low. Audit logging may lag under heavy test load.".into()),
        )
    } else {
        (
            TestStatus::Fail,
            Some("Disk I/O critically slow. Check disk health and available space.".into()),
        )
    };

    ComponentResult {
        component: "Disk I/O".into(),
        status,
        details: format!("{:.1} MB/s sequential write (1 MB x {})", mb_per_sec, iterations),
        remediation,
        measured: Some(format!("{:.1} MB/s", mb_per_sec)),
    }
}

/// Benchmark Ed25519 sign+verify throughput. TLS handshakes need this fast.
fn check_crypto_performance() -> ComponentResult {
    use ed25519_dalek::{Signer, Verifier};

    let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
    let verifying_key = signing_key.verifying_key();
    let message = b"PacketParamedic Reflector self-test benchmark payload";

    let iterations = 5000;
    let start = Instant::now();

    for _ in 0..iterations {
        let sig = signing_key.sign(message);
        let _ = verifying_key.verify(message, &sig);
    }

    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

    // Each mTLS handshake does ~2 sign + 2 verify ops.
    // At 1 Gbps with 30s tests, we're doing maybe 1-2 handshakes/min.
    // Even 100 ops/sec would be fine, but faster is better for burst pairing.
    let (status, remediation) = if ops_per_sec >= 1000.0 {
        (TestStatus::Pass, None)
    } else if ops_per_sec >= 100.0 {
        (
            TestStatus::Warning,
            Some("Crypto performance is adequate but slower than expected. Check CPU governor.".into()),
        )
    } else {
        (
            TestStatus::Fail,
            Some("Crypto performance too low. TLS handshakes will be slow. Check CPU.".into()),
        )
    };

    ComponentResult {
        component: "Crypto (Ed25519)".into(),
        status,
        details: format!(
            "{:.0} sign+verify ops/sec ({} iterations in {:.1}ms)",
            ops_per_sec,
            iterations,
            elapsed.as_secs_f64() * 1000.0
        ),
        remediation,
        measured: Some(format!("{:.0} ops/sec", ops_per_sec)),
    }
}

/// Check NTP synchronization status.
fn check_time_sync() -> ComponentResult {
    // Try timedatectl (systemd)
    if let Ok(output) = std::process::Command::new("timedatectl")
        .arg("show")
        .arg("--property=NTPSynchronized")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("NTPSynchronized=yes") {
            return ComponentResult {
                component: "Time Sync".into(),
                status: TestStatus::Pass,
                details: "NTP synchronized (timedatectl)".into(),
                remediation: None,
                measured: None,
            };
        } else if stdout.contains("NTPSynchronized=no") {
            return ComponentResult {
                component: "Time Sync".into(),
                status: TestStatus::Warning,
                details: "NTP not synchronized. Timestamps in audit log may drift.".into(),
                remediation: Some("Enable NTP: timedatectl set-ntp true".into()),
                measured: None,
            };
        }
    }

    // Try chronyc
    if let Ok(output) = std::process::Command::new("chronyc")
        .arg("tracking")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Leap status") && !stdout.contains("Not synchronised") {
            return ComponentResult {
                component: "Time Sync".into(),
                status: TestStatus::Pass,
                details: "NTP synchronized (chrony)".into(),
                remediation: None,
                measured: None,
            };
        }
    }

    ComponentResult {
        component: "Time Sync".into(),
        status: TestStatus::Skipped,
        details: "Cannot determine NTP status on this platform".into(),
        remediation: None,
        measured: None,
    }
}

/// Check open file descriptor limits. iperf3 with many streams needs headroom.
fn check_ulimits() -> ComponentResult {
    // Try reading from /proc/self/limits (Linux)
    if let Ok(content) = std::fs::read_to_string("/proc/self/limits") {
        for line in content.lines() {
            if line.starts_with("Max open files") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(soft) = parts.get(3).and_then(|s| s.parse::<u64>().ok()) {
                    let (status, remediation) = if soft >= 4096 {
                        (TestStatus::Pass, None)
                    } else if soft >= 1024 {
                        (
                            TestStatus::Warning,
                            Some("Open file limit is low. Increase with: ulimit -n 4096".into()),
                        )
                    } else {
                        (
                            TestStatus::Fail,
                            Some("Open file limit critically low. Set: ulimit -n 4096".into()),
                        )
                    };

                    return ComponentResult {
                        component: "File Descriptors".into(),
                        status,
                        details: format!("Soft limit: {}", soft),
                        remediation,
                        measured: Some(format!("{}", soft)),
                    };
                }
            }
        }
    }

    // macOS / fallback
    ComponentResult {
        component: "File Descriptors".into(),
        status: TestStatus::Skipped,
        details: "Cannot read file descriptor limits on this platform".into(),
        remediation: None,
        measured: None,
    }
}

// ---------------------------------------------------------------------------
// Verdict derivation
// ---------------------------------------------------------------------------

/// Derive high-level capability flags from component results.
fn derive_capabilities(results: &[ComponentResult]) -> HashMap<String, bool> {
    let mut caps = HashMap::new();

    let get_status = |name: &str| -> TestStatus {
        results
            .iter()
            .find(|r| r.component.contains(name))
            .map(|r| r.status.clone())
            .unwrap_or(TestStatus::Skipped)
    };

    // Can push 1 Gbps?
    let cpu_ok = get_status("CPU") != TestStatus::Fail;
    let mem_ok = get_status("Memory") != TestStatus::Fail;
    let iperf3_ok = get_status("iperf3") == TestStatus::Pass;
    let loopback_ok = get_status("Loopback") == TestStatus::Pass;

    caps.insert(
        "1 Gbps Throughput Testing".into(),
        cpu_ok && mem_ok && iperf3_ok && loopback_ok,
    );

    // Has NIC >= 1 Gbps?
    let has_1g_nic = results
        .iter()
        .any(|r| r.component.starts_with("Network:") && r.status == TestStatus::Pass);
    caps.insert("1 Gbps NIC Detected".into(), has_1g_nic);

    // Crypto fast enough for burst mTLS?
    let crypto_ok = get_status("Crypto") != TestStatus::Fail;
    caps.insert("mTLS Performance".into(), crypto_ok);

    // Audit logging won't lag?
    let disk_ok = get_status("Disk") != TestStatus::Fail;
    caps.insert("Audit Log Performance".into(), disk_ok);

    // Time sync for accurate timestamps?
    let time_ok = get_status("Time") == TestStatus::Pass;
    caps.insert("Accurate Timestamps".into(), time_ok);

    caps
}

/// Estimate maximum sustainable throughput from test results.
fn estimate_max_throughput(results: &[ComponentResult]) -> u32 {
    // Base estimate from loopback test
    let loopback_mbps = results
        .iter()
        .find(|r| r.component == "Loopback Throughput")
        .and_then(|r| r.measured.as_ref())
        .and_then(|m| m.split_whitespace().next())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    // Cap by NIC speed
    let max_nic_mbps = results
        .iter()
        .filter(|r| r.component.starts_with("Network:"))
        .filter_map(|r| r.measured.as_ref())
        .filter_map(|m| m.split_whitespace().next())
        .filter_map(|s| s.parse::<f64>().ok())
        .fold(0.0_f64, f64::max);

    // The bottleneck is the minimum of CPU capability and NIC speed.
    // Loopback throughput represents CPU capability.
    let estimated = if max_nic_mbps > 0.0 && loopback_mbps > 0.0 {
        loopback_mbps.min(max_nic_mbps)
    } else if loopback_mbps > 0.0 {
        loopback_mbps
    } else if max_nic_mbps > 0.0 {
        max_nic_mbps
    } else {
        0.0
    };

    // Apply 90% efficiency factor (TCP overhead, kernel overhead)
    (estimated * 0.9) as u32
}

/// Derive overall verdict string.
fn derive_verdict(results: &[ComponentResult], estimated_max_mbps: u32) -> String {
    let fail_count = results.iter().filter(|r| r.status == TestStatus::Fail).count();
    let warn_count = results
        .iter()
        .filter(|r| r.status == TestStatus::Warning)
        .count();

    if fail_count > 0 {
        format!(
            "NOT READY - {} critical issue(s). Estimated max: {} Mbps",
            fail_count, estimated_max_mbps
        )
    } else if warn_count > 0 {
        format!(
            "DEGRADED - {} warning(s). Estimated max: {} Mbps",
            warn_count, estimated_max_mbps
        )
    } else if estimated_max_mbps >= 940 {
        format!("READY - Host can sustain 1 Gbps. Estimated max: {} Mbps", estimated_max_mbps)
    } else {
        format!(
            "READY (limited) - Estimated max: {} Mbps",
            estimated_max_mbps
        )
    }
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

/// Print a human-readable self-test report to stdout.
pub fn print_report(report: &SelfTestReport) {
    println!();
    println!("  PacketParamedic Reflector Self-Test");
    println!("  ===================================");
    println!();

    // Component results table
    println!(
        "  {:<25} {:<6} {}",
        "Component", "Status", "Details"
    );
    println!("  {}", "-".repeat(75));

    for result in &report.results {
        let status_str = match result.status {
            TestStatus::Pass => "\x1b[32mPASS\x1b[0m",
            TestStatus::Fail => "\x1b[31mFAIL\x1b[0m",
            TestStatus::Warning => "\x1b[33mWARN\x1b[0m",
            TestStatus::Skipped => "\x1b[90mSKIP\x1b[0m",
        };

        println!(
            "  {:<25} {}   {}",
            result.component, status_str, result.details
        );

        if let Some(ref rem) = result.remediation {
            println!("  {:<25}        -> {}", "", rem);
        }
    }

    // Capabilities
    println!();
    println!("  Capabilities:");
    for (cap, ready) in &report.capabilities {
        let icon = if *ready { "\x1b[32m+\x1b[0m" } else { "\x1b[31m-\x1b[0m" };
        println!("    {} {}", icon, cap);
    }

    // Verdict
    println!();
    let verdict_color = if report.verdict.starts_with("READY -") {
        "\x1b[32m"
    } else if report.verdict.starts_with("DEGRADED") {
        "\x1b[33m"
    } else {
        "\x1b[31m"
    };
    println!(
        "  Verdict: {}{}\x1b[0m",
        verdict_color, report.verdict
    );
    println!();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iperf3_throughput_valid() {
        let json = r#"{
            "end": {
                "sum_received": {
                    "bits_per_second": 9400000000.0
                }
            }
        }"#;
        let mbps = parse_iperf3_throughput(json);
        assert!((mbps - 9400.0).abs() < 1.0);
    }

    #[test]
    fn test_parse_iperf3_throughput_invalid() {
        assert_eq!(parse_iperf3_throughput("not json"), 0.0);
        assert_eq!(parse_iperf3_throughput("{}"), 0.0);
    }

    #[test]
    fn test_derive_verdict_ready() {
        let results = vec![
            ComponentResult {
                component: "CPU".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: None,
            },
        ];
        let verdict = derive_verdict(&results, 940);
        assert!(verdict.starts_with("READY"));
    }

    #[test]
    fn test_derive_verdict_not_ready() {
        let results = vec![
            ComponentResult {
                component: "CPU".into(),
                status: TestStatus::Fail,
                details: "bad".into(),
                remediation: None,
                measured: None,
            },
        ];
        let verdict = derive_verdict(&results, 100);
        assert!(verdict.starts_with("NOT READY"));
    }

    #[test]
    fn test_derive_verdict_degraded() {
        let results = vec![
            ComponentResult {
                component: "CPU".into(),
                status: TestStatus::Warning,
                details: "slow".into(),
                remediation: None,
                measured: None,
            },
        ];
        let verdict = derive_verdict(&results, 800);
        assert!(verdict.starts_with("DEGRADED"));
    }

    #[test]
    fn test_estimate_max_throughput() {
        let results = vec![
            ComponentResult {
                component: "Loopback Throughput".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: Some("12000 Mbps".into()),
            },
            ComponentResult {
                component: "Network: eth0".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: Some("1000 Mbps".into()),
            },
        ];
        let max = estimate_max_throughput(&results);
        // Should be capped by NIC: 1000 * 0.9 = 900
        assert_eq!(max, 900);
    }

    #[test]
    fn test_check_cpu_returns_result() {
        let result = check_cpu();
        assert!(!result.component.is_empty());
        assert!(!result.details.is_empty());
    }

    #[test]
    fn test_check_memory_returns_result() {
        let result = check_memory();
        assert_eq!(result.component, "Memory");
        assert!(result.measured.is_some());
    }

    #[test]
    fn test_check_cpu_features_returns_result() {
        let result = check_cpu_features();
        assert_eq!(result.component, "CPU Features");
        assert!(result.status != TestStatus::Fail); // Should at least be pass or skip
    }

    #[test]
    fn test_check_crypto_performance() {
        let result = check_crypto_performance();
        assert_eq!(result.component, "Crypto (Ed25519)");
        assert!(result.measured.is_some());
        // Should be fast enough on any modern CPU
        assert!(result.status == TestStatus::Pass || result.status == TestStatus::Warning);
    }

    #[test]
    fn test_test_status_display() {
        assert_eq!(format!("{}", TestStatus::Pass), "PASS");
        assert_eq!(format!("{}", TestStatus::Fail), "FAIL");
        assert_eq!(format!("{}", TestStatus::Warning), "WARN");
        assert_eq!(format!("{}", TestStatus::Skipped), "SKIP");
    }

    #[test]
    fn test_derive_capabilities() {
        let results = vec![
            ComponentResult {
                component: "CPU".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: None,
            },
            ComponentResult {
                component: "Memory".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: None,
            },
            ComponentResult {
                component: "iperf3".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: None,
            },
            ComponentResult {
                component: "Loopback Throughput".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: None,
            },
            ComponentResult {
                component: "Network: eth0".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: None,
            },
            ComponentResult {
                component: "Crypto (Ed25519)".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: None,
            },
            ComponentResult {
                component: "Disk I/O".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: None,
            },
            ComponentResult {
                component: "Time Sync".into(),
                status: TestStatus::Pass,
                details: "ok".into(),
                remediation: None,
                measured: None,
            },
        ];
        let caps = derive_capabilities(&results);
        assert_eq!(caps.get("1 Gbps Throughput Testing"), Some(&true));
        assert_eq!(caps.get("1 Gbps NIC Detected"), Some(&true));
        assert_eq!(caps.get("mTLS Performance"), Some(&true));
        assert_eq!(caps.get("Audit Log Performance"), Some(&true));
        assert_eq!(caps.get("Accurate Timestamps"), Some(&true));
    }
}

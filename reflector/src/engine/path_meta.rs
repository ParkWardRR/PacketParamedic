//! System metadata reporter for the PacketParamedic Reflector.
//!
//! Collects system metrics (CPU, memory, load averages, MTU, NTP sync status)
//! and packages them into a [`PathMeta`] for transmission to the requesting
//! peer.

use sysinfo::System;
use tracing::debug;

use crate::rpc::PathMeta;

// ---------------------------------------------------------------------------
// MTU detection
// ---------------------------------------------------------------------------

/// Attempt to read the MTU of the primary network interface.
///
/// On Linux, reads from `/sys/class/net/{iface}/mtu`.  On other platforms
/// (or if the file is not accessible), returns `None`.
fn detect_mtu() -> Option<u32> {
    // Try common interface names.
    let interfaces = ["eth0", "ens0", "en0", "eno1", "enp0s3"];

    for iface in &interfaces {
        let path = format!("/sys/class/net/{}/mtu", iface);
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(mtu) = contents.trim().parse::<u32>() {
                debug!(interface = *iface, mtu = mtu, "detected MTU");
                return Some(mtu);
            }
        }
    }

    // Fallback: try running `networksetup` on macOS or `ip link` on Linux.
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("ifconfig")
            .arg("en0")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("mtu ") {
                    if let Ok(mtu) = rest.split_whitespace().next().unwrap_or("").parse::<u32>() {
                        debug!(interface = "en0", mtu = mtu, "detected MTU via ifconfig");
                        return Some(mtu);
                    }
                }
            }
        }
    }

    None
}

// ---------------------------------------------------------------------------
// NTP sync detection
// ---------------------------------------------------------------------------

/// Heuristically check whether the system clock is synchronized.
///
/// Tries `timedatectl show` on systemd-based systems.  Falls back to
/// assuming synchronized if the current time looks reasonable (year >= 2024).
fn check_ntp_sync() -> bool {
    // Try timedatectl (systemd-based Linux).
    if let Ok(output) = std::process::Command::new("timedatectl")
        .arg("show")
        .arg("--property=NTPSynchronized")
        .arg("--value")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed == "yes" {
            debug!("NTP synchronized (timedatectl)");
            return true;
        } else if trimmed == "no" {
            debug!("NTP NOT synchronized (timedatectl)");
            return false;
        }
    }

    // Try chronyc (chrony-based systems).
    if let Ok(output) = std::process::Command::new("chronyc")
        .arg("tracking")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Leap status     : Normal") {
            debug!("NTP synchronized (chrony)");
            return true;
        }
    }

    // Fallback heuristic: if the system year is >= 2024, assume synced.
    let now = chrono::Utc::now();
    let synced = now.format("%Y").to_string().parse::<u32>().unwrap_or(0) >= 2024;
    debug!(heuristic = synced, "NTP sync status via heuristic");
    synced
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Collect system and path metadata.
///
/// Gathers CPU load, memory usage, load averages, MTU, NTP sync status,
/// and build information into a [`PathMeta`] struct.
pub fn collect_path_meta() -> PathMeta {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_usage();

    // CPU load as a fraction (average across all cores).
    // sysinfo returns per-CPU usage as percentages; we take the global average.
    let cpu_load = {
        let load = System::load_average();
        // Normalize 1-minute load by CPU count to get a 0.0-1.0+ fraction.
        let cpu_count = sys.cpus().len().max(1) as f64;
        load.one / cpu_count
    };

    // Memory.
    let memory_total_mb = sys.total_memory() / (1024 * 1024);
    let memory_used_mb = sys.used_memory() / (1024 * 1024);

    // Load averages.
    let load = System::load_average();
    let load_avg = [load.one, load.five, load.fifteen];

    // MTU.
    let mtu = detect_mtu();

    // NTP sync.
    let time_synced = check_ntp_sync();

    // Build info.
    let build_version = env!("CARGO_PKG_VERSION").to_string();
    let build_hash = option_env!("GIT_HASH").unwrap_or("unknown").to_string();

    PathMeta {
        cpu_load,
        memory_used_mb,
        memory_total_mb,
        load_avg,
        mtu,
        time_synced,
        build_version,
        build_hash,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_path_meta_returns_valid_data() {
        let meta = collect_path_meta();

        // CPU load should be non-negative.
        assert!(meta.cpu_load >= 0.0, "cpu_load should be non-negative");

        // Memory total should be positive (we're running on a real machine).
        assert!(meta.memory_total_mb > 0, "memory_total_mb should be > 0");

        // Memory used should be <= total.
        assert!(
            meta.memory_used_mb <= meta.memory_total_mb,
            "used memory should be <= total"
        );

        // Load averages should be non-negative.
        for (i, &val) in meta.load_avg.iter().enumerate() {
            assert!(val >= 0.0, "load_avg[{}] should be non-negative", i);
        }

        // Build version should not be empty.
        assert!(!meta.build_version.is_empty());

        // Build hash should have a value (even if "unknown").
        assert!(!meta.build_hash.is_empty());
    }

    #[test]
    fn test_detect_mtu_does_not_panic() {
        // Just verify it doesn't panic; result depends on the platform.
        let _mtu = detect_mtu();
    }

    #[test]
    fn test_check_ntp_sync_does_not_panic() {
        // Just verify it doesn't panic; result depends on the platform.
        let _synced = check_ntp_sync();
    }
}

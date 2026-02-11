use anyhow::{Context, Result};
use std::process::Command;

/// Check available disk space/inodes on /var/lib/packetparamedic
pub fn check_disk_space(path: &str) -> Result<u64> {
    // df -B1 --output=avail /var/lib/packetparamedic
    // A simplified cross-platform (linux) way: assume 'df' is available.
    // Or use statvfs via libc (but that's hard to verify in cross-compile).
    // Let's stick with 'df', it's standard on the Pi.
    let output = Command::new("df")
        .arg("--block-size=1")
        .arg("--output=avail")
        .arg(path)
        .output()
        .context("Failed to check disk space")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Usually header + one line of number
    // "Avail\n 123456\n"
    let lines: Vec<&str> = stdout.trim().split('\n').collect();
    if lines.len() < 2 {
        anyhow::bail!("Unexpected df output format");
    }

    let avail_str = lines.last().unwrap().trim();
    let avail = avail_str
        .parse::<u64>()
        .context("Invalid disk space value")?;
    Ok(avail)
}

/// Should we stop writing? (True if severe pressure, e.g. < 500MB)
/// This is a "guardrail".
pub fn is_disk_critical(path: &str) -> bool {
    // 500MB safeguard
    const CRITICAL_THRESHOLD_BYTES: u64 = 500 * 1024 * 1024;
    match check_disk_space(path) {
        Ok(avail) => avail < CRITICAL_THRESHOLD_BYTES,
        Err(_) => true, // Fail safe: assume critical on error
    }
}

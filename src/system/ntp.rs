use anyhow::{Context, Result};
use std::process::Command;

/// Check if NTP is synchronized using timedatectl
pub fn is_ntp_synchronized() -> Result<bool> {
    let output = Command::new("timedatectl")
        .arg("show")
        .arg("--property=NTPSynchronized")
        .output()
        .context("Failed to run timedatectl")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Output should be "NTPSynchronized=yes"
    Ok(stdout.trim() == "NTPSynchronized=yes")
}

/// Check clock skew by comparing against a well-known time server (optional, simple check)
/// This is a fallback if timedatectl is trusted but we want a hard check.
/// For now, just rely on systemd-timesyncd status.
pub fn check_clock_status() -> Result<String> {
    if is_ntp_synchronized()? {
        Ok("synced".to_string())
    } else {
        Ok("unsynced".to_string())
    }
}

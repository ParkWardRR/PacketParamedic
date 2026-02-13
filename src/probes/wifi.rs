use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WifiStatus {
    pub interface: String,
    pub connected: bool,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub freq_mhz: Option<u32>,
    pub signal_dbm: Option<i32>,
    pub tx_bitrate_mbps: Option<f32>,
}

pub fn get_wifi_status() -> Result<Vec<WifiStatus>> {
    let interfaces = get_wireless_interfaces()?;
    let mut statuses = Vec::new();

    for iface in interfaces {
        let status = get_link_status(&iface)?;
        statuses.push(status);
    }

    Ok(statuses)
}

fn get_wireless_interfaces() -> Result<Vec<String>> {
    // Run 'iw dev' to list interfaces
    // Use absolute path as iw is in /usr/sbin/ typically
    let output = Command::new("/usr/sbin/iw")
        .arg("dev")
        .output()
        .context("Failed to execute '/usr/sbin/iw dev'")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("iw dev failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ifaces = Vec::new();

    for line in stdout.lines() {
        // Line format: "	Interface wlan0"
        let line = line.trim();
        if let Some(iface) = line.strip_prefix("Interface ") {
            ifaces.push(iface.to_string());
        }
    }

    Ok(ifaces)
}

fn get_link_status(iface: &str) -> Result<WifiStatus> {
    let output = Command::new("/usr/sbin/iw")
        .arg("dev")
        .arg(iface)
        .arg("link")
        .output()
        .context(format!("Failed to execute '/usr/sbin/iw dev {} link'", iface))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Default: Not connected
    let mut status = WifiStatus {
        interface: iface.to_string(),
        connected: false,
        ssid: None,
        bssid: None,
        freq_mhz: None,
        signal_dbm: None,
        tx_bitrate_mbps: None,
    };

    if stdout.contains("Not connected.") {
        return Ok(status);
    }

    // Parse Connected Output
    // Connected to 00:11:22:33:44:55 (on wlan0)
    // 	SSID: MyNet
    // 	freq: 5180
    // 	signal: -50 dBm
    // 	tx bitrate: 866.7 MBit/s
    
    status.connected = true;

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("Connected to") {
            // "Connected to 00:11:22:33:44:55 (on wlan0)"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                status.bssid = Some(parts[2].to_string());
            }
        } else if let Some(ssid) = line.strip_prefix("SSID: ") {
            status.ssid = Some(ssid.to_string());
        } else if let Some(freq) = line.strip_prefix("freq: ") {
            if let Ok(f) = freq.parse::<u32>() {
                status.freq_mhz = Some(f);
            }
        } else if let Some(signal) = line.strip_prefix("signal: ") {
            // "-50 dBm"
            let val = signal.replace(" dBm", "");
            if let Ok(s) = val.parse::<i32>() {
                status.signal_dbm = Some(s);
            }
        } else if let Some(tx) = line.strip_prefix("tx bitrate: ") {
            // "866.7 MBit/s VHT-MCS 9 80MHz short GI VHT-NSS 2"
            let parts: Vec<&str> = tx.split_whitespace().collect();
            if !parts.is_empty() {
                 if let Ok(rate) = parts[0].parse::<f32>() {
                     status.tx_bitrate_mbps = Some(rate);
                 }
            }
        }
    }

    Ok(status)
}

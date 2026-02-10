use anyhow::{Result, Context};
use std::process::Command;
use crate::selftest::{ComponentResult, TestStatus};

/// Check Wi-Fi interfaces and capabilities (Monitor Mode, Injection)
pub fn check_wifi() -> Result<Vec<ComponentResult>> {
    let mut results = Vec::new();

    // 1. Enumerate PHYs using `iw phy` or `iw list`
    // We want to see if they support monitor mode.
    let output = Command::new("iw")
        .arg("list")
        .output()
        .context("Failed to run 'iw list'. Is iw installed?")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Simple parsing logic: 'iw list' output is grouped by "Wiphy <name>"
    // We look for "Supported interface modes:" block and "monitor" line inside it.
    
    let mut sections: Vec<&str> = stdout.split("Wiphy ").collect();
    if sections.len() > 1 {
        // Skip text before first "Wiphy"
        sections.remove(0);
    } else if !stdout.contains("Wiphy") {
        // No PHYs found
         results.push(ComponentResult {
            component: "Wi-Fi".to_string(),
            status: TestStatus::Warning,
            details: "No Wi-Fi PHYs found via 'iw list'.".to_string(),
            remediation: Some("Check if Wi-Fi is enabled in config.txt or RF-kill state.".to_string()),
        });
        return Ok(results);
    }

    for section in sections {
        // First line is phy name, e.g. "phy0"
        let phy_name = section.lines().next().unwrap_or("unknown").trim();
        
        // check for "monitor" in "Supported interface modes:"
        let has_monitor = section.contains("monitor");
        let has_managed = section.contains("managed");
        let has_ap = section.contains("AP");
        
        // Driver check? Hard to get from 'iw list' directly, but we can guess or use /sys
        
        let mut caps = Vec::new();
        if has_managed { caps.push("Client"); }
        if has_ap { caps.push("AP"); }
        if has_monitor { caps.push("Monitor"); }
        
        let status = if has_monitor {
            TestStatus::Pass
        } else {
            TestStatus::Warning // Monitor mode is key for advanced diagnostics
        };
        
        let details = format!("PHY: {}, Capabilities: [{}]", phy_name, caps.join(", "));
        
        let remediation = if !has_monitor {
            Some("Hardware does not support Monitor mode. External USB adapter (e.g. MT7921au) recommended for packet capture.".to_string())
        } else {
            None
        };
        
        results.push(ComponentResult {
            component: format!("Wi-Fi Radio ({})", phy_name),
            status,
            details,
            remediation,
        });
    }

    Ok(results)
}

/// Check current Wi-Fi interfaces (wlan0, etc.)
pub fn check_interfaces() -> Result<Vec<ComponentResult>> {
    // iw dev
    let output = Command::new("iw")
        .arg("dev")
        .output()
        .context("Failed to run 'iw dev'")?;
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();
    
    // Look for "Interface <name>"
    for line in stdout.lines() {
        if let Some(iface) = line.trim().strip_prefix("Interface ") {
             results.push(ComponentResult {
                component: format!("Wi-Fi Interface ({})", iface),
                status: TestStatus::Pass,
                details: "Interface present".to_string(),
                remediation: None,
            });
        }
    }
    
    if results.is_empty() {
         results.push(ComponentResult {
            component: "Wi-Fi Interface".to_string(),
            status: TestStatus::Warning,
            details: "No active Wi-Fi interfaces found (wlanX).".to_string(),
            remediation: Some("Use 'rfkill' or 'nmtui' to enable Wi-Fi.".to_string()),
        });
    }
    
    Ok(results)
}

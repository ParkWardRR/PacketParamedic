use anyhow::{Result, Context};
use std::fs;
use crate::selftest::{ComponentResult, TestStatus};

/// Scan network interfaces for 10GbE capability
pub fn check_interfaces() -> Result<Vec<ComponentResult>> {
    let mut results = Vec::new();
    
    // 1. Scan /sys/class/net
    let entries = fs::read_dir("/sys/class/net").context("Failed to read network interfaces")?;
    
    for entry in entries {
        let entry = entry?;
        let iface = entry.file_name().into_string().unwrap();
        
        if iface == "lo" { continue; }
        
        // Check speed
        let speed_path = entry.path().join("speed");
        let speed_mbps = if speed_path.exists() {
             fs::read_to_string(speed_path).unwrap_or_default().trim().parse::<i32>().unwrap_or(-1)
        } else {
            -1
        };
        
        // PCIe check? 
        // /sys/class/net/eth0/device -> symlink to ../../../../../0000:01:00.0 etc
        let is_pcie = entry.path().join("device").read_link().map(|p| p.to_string_lossy().contains("pci")).unwrap_or(false);
        
        // Classify
        if speed_mbps >= 10000 {
            results.push(ComponentResult {
                component: format!("Interface: {}", iface),
                status: TestStatus::Pass,
                details: format!("10GbE detected (Link: {} Mbps, PCIe: {})", speed_mbps, is_pcie),
                remediation: None,
            });
        } else if speed_mbps >= 2500 {
             results.push(ComponentResult {
                component: format!("Interface: {}", iface),
                status: TestStatus::Pass,
                details: format!("Multi-Gig detected (Link: {} Mbps)", speed_mbps),
                remediation: None,
            });
        } else if speed_mbps >= 1000 {
             results.push(ComponentResult {
                component: format!("Interface: {}", iface),
                status: TestStatus::Warning,
                details: format!("1GbE detected (Link: {} Mbps). Sufficient for WAN/LAN, insufficient for 10G testing.", speed_mbps),
                remediation: Some("Install 10GbE PCIe NIC for full throughput tests.".to_string()),
            });
        }
        
        // If it's a PCIe device but link is slow, warn explicitly
        if is_pcie && speed_mbps < 5000 && speed_mbps > 0 {
             results.push(ComponentResult {
                component: format!("PCIe Link Speed: {}", iface),
                status: TestStatus::Warning,
                details: format!("PCIe NIC link negotiated at {} Mbps. Verify cable/switch.", speed_mbps),
                remediation: Some("Check cabling (Cat6a+) and switch port configuration.".to_string()),
            });
        }
    }
    
    if results.is_empty() {
        results.push(ComponentResult {
            component: "Network".to_string(),
            status: TestStatus::Fail,
            details: "No active network interfaces found.".to_string(),
            remediation: Some("Check cables and drivers.".to_string()),
        });
    }

    Ok(results)
}

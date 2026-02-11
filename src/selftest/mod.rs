//! Hardware self-test subsystem for Pi 5.

use anyhow::Result;
use serde::Serialize;
use tracing::info;

pub mod hardware;
pub mod network;
pub mod thermal;
pub mod wifi;

use std::collections::HashMap;

/// Run the full hardware self-test suite.
/// Returns a list of component results and use case compatibility.
pub async fn run() -> Result<SelfTestReport> {
    info!("Self-test: checking Pi 5 hardware...");

    let mut results = Vec::new();

    // 1. Board & RAM
    match hardware::check_board() {
        Ok(res) => results.push(res),
        Err(e) => results.push(ComponentResult {
            component: "Board".to_string(),
            status: TestStatus::Fail,
            details: format!("Failed to inspect board: {}", e),
            remediation: None,
        }),
    }

    // 2. CPU Features (NEON)
    match hardware::check_cpu_features() {
        Ok(res) => results.push(res),
        Err(e) => results.push(ComponentResult {
            component: "CPU Features".to_string(),
            status: TestStatus::Fail,
            details: format!("Failed to check CPU features: {}", e),
            remediation: None,
        }),
    }

    // 3. GPU (VideoCore VII)
    match hardware::check_gpu() {
        Ok(res) => results.push(res),
        Err(e) => results.push(ComponentResult {
            component: "GPU".to_string(),
            status: TestStatus::Warning,
            details: format!("Failed to check GPU: {}", e),
            remediation: None,
        }),
    }

    // 4. Storage Type
    match hardware::check_storage() {
        Ok(res) => results.push(res),
        Err(e) => results.push(ComponentResult {
            component: "Storage".to_string(),
            status: TestStatus::Warning,
            details: format!("Failed to check storage: {}", e),
            remediation: None,
        }),
    }

    // 5. Thermal & Power (vcgencmd)
    match thermal::check_throttling() {
        Ok(res) => results.push(res),
        Err(e) => results.push(ComponentResult {
            component: "Thermal".to_string(),
            status: TestStatus::Warning,
            details: format!("Failed to check thermal throttling: {}", e),
            remediation: Some("Ensure 'vcgencmd' is available.".to_string()),
        }),
    }

    // 6. Network Interfaces (10GbE)
    match network::check_interfaces() {
        Ok(net_results) => results.extend(net_results),
        Err(e) => results.push(ComponentResult {
            component: "Network".to_string(),
            status: TestStatus::Warning,
            details: format!("Failed to enumerate interfaces: {}", e),
            remediation: None,
        }),
    }

    // 7. Wi-Fi (Phase 2.2)
    match wifi::check_wifi() {
        Ok(wifi_results) => results.extend(wifi_results),
        Err(e) => results.push(ComponentResult {
            component: "Wi-Fi".to_string(),
            status: TestStatus::Warning,
            details: format!("Failed to check Wi-Fi: {}", e),
            remediation: Some("Ensure 'iw' is installed.".to_string()),
        }),
    }

    info!("Self-test complete. {} check(s) run.", results.len());

    // Calculate Use Case Compatibility
    let compatibility = calculate_use_case_compatibility(&results);

    Ok(SelfTestReport {
        results,
        compatibility,
    })
}

#[derive(Debug, Serialize)]
pub struct SelfTestReport {
    pub results: Vec<ComponentResult>,
    pub compatibility: HashMap<String, bool>, // Use Case Name -> Is Compatible
}

fn calculate_use_case_compatibility(results: &[ComponentResult]) -> HashMap<String, bool> {
    let mut map = HashMap::new();

    // Helper to find status of a component
    let get_status = |name_part: &str| -> TestStatus {
        results
            .iter()
            .find(|r| r.component.contains(name_part))
            .map(|r| r.status.clone())
            .unwrap_or(TestStatus::Fail)
    };

    let get_details = |name_part: &str| -> String {
        results
            .iter()
            .find(|r| r.component.contains(name_part))
            .map(|r| r.details.clone())
            .unwrap_or_default()
    };

    // Simple Troubleshooting: Needs Pi 5 (Board Pass) + minimal network
    // Board must PASS. Network must not be FAIL.
    let board_pass = get_status("Board") == TestStatus::Pass;
    let net_ok =
        get_status("Network") != TestStatus::Fail && get_status("Interface") != TestStatus::Fail;
    map.insert("Simple Troubleshooting".to_string(), board_pass && net_ok);

    // Reliability & Uptime: Needs reliability -> NVMe storage preferred (or at least storage pass) + Thermal Pass
    // We check if Storage details contain "NVMe"
    let storage_nvme = get_details("Storage").contains("NVMe");
    let thermal_pass = get_status("Thermal") == TestStatus::Pass;
    map.insert(
        "Reliability & Uptime".to_string(),
        board_pass && net_ok && storage_nvme && thermal_pass,
    );

    // High Performance: Needs performance -> 2.5GbE+ (Multi-Gig)
    let multigig = results
        .iter()
        .any(|r| r.details.contains("Multi-Gig") || r.details.contains("10GbE"));
    map.insert("High Performance".to_string(), board_pass && multigig);

    map
}

/// Self-test result for a single hardware component.
#[derive(Debug, Serialize, Clone)]
pub struct ComponentResult {
    pub component: String,
    pub status: TestStatus,
    pub details: String,
    pub remediation: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub enum TestStatus {
    Pass,
    Fail,
    Warning,
    Skipped,
}

//! Hardware self-test subsystem for Pi 5.

use anyhow::Result;
use serde::Serialize;
use tracing::info;

pub mod hardware;
pub mod thermal;
pub mod network;

/// Run the full hardware self-test suite.
/// Returns a list of component results.
pub async fn run() -> Result<Vec<ComponentResult>> {
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

    info!("Self-test complete. {} check(s) run.", results.len());
    Ok(results)
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

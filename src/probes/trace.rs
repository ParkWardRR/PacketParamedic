use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MtrReport {
    pub report: ReportData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportData {
    pub mtr: MtrDetails,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MtrDetails {
    pub src: String,
    pub dst: String,
    pub tests: u32,
    pub hubs: Vec<Hop>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hop {
    pub count: u32,
    pub host: String,
    #[serde(rename = "Loss%")]
    pub loss_percent: f32,
    #[serde(rename = "Snt")]
    pub sent: u32,
    #[serde(rename = "Last")]
    pub last: f32,
    #[serde(rename = "Avg")]
    pub avg: f32,
    #[serde(rename = "Best")]
    pub best: f32,
    #[serde(rename = "Wrst")]
    pub worst: f32,
    #[serde(rename = "StDev")]
    pub stdev: f32,
}

/// Executes a trace to the target using `mtr` in JSON mode.
/// Requires `mtr` (or `mtr-tiny`) to be installed and properly capacitied (CAP_NET_RAW).
pub fn run_trace(target: &str) -> Result<MtrReport> {
    // Validate target lightly to avoid injection (though Command protects mostly)
    if target.chars().any(|c| !c.is_alphanumeric() && c != '.' && c != ':' && c != '-') {
        anyhow::bail!("Invalid target format");
    }

    info!(target, "Starting MTR trace...");

    // Build command: mtr --json --report -c 10 <target>
    // --report is implied by --json in modern versions but explicit is safer.
    // -c 10 sends 10 packets per hop.
    // -z reports ASN (optional, might break JSON schema if unexpected fields? No, extra fields ignored by default serde)
    
    let output = Command::new("mtr")
        .arg("--json")
        .arg("--report")
        .arg("-c")
        .arg("10")
        .arg(target)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let json_str = String::from_utf8_lossy(&out.stdout);
                // Attempt parsing
                match serde_json::from_str::<MtrReport>(&json_str) {
                    Ok(report) => {
                        info!(target, hops=report.report.mtr.hubs.len(), "MTR trace complete");
                        Ok(report)
                    },
                    Err(e) => {
                        warn!(target, error=%e, "Failed to parse MTR JSON output");
                        // Log raw output for debugging
                        warn!(raw_output=%json_str, "Raw MTR JSON");
                        Err(anyhow::anyhow!("MTR JSON parse error: {}", e))
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                warn!(target, error=%stderr, "MTR execution failed");
                Err(anyhow::anyhow!("MTR execution failed: {}", stderr))
            }
        },
        Err(e) => {
            warn!(target, error=%e, "Failed to launch mtr command");
            Err(anyhow::anyhow!("Failed to launch 'mtr': {}. Is it installed?", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mtr_json() {
        let json = r#"{
            "report": {
                "mtr": {
                    "src": "192.168.1.10",
                    "dst": "8.8.8.8",
                    "tests": 10,
                    "hubs": [
                        {
                            "count": 1,
                            "host": "192.168.1.1",
                            "Loss%": 0.0,
                            "Snt": 10,
                            "Last": 0.5,
                            "Avg": 0.4,
                            "Best": 0.3,
                            "Wrst": 0.6,
                            "StDev": 0.1
                        }
                    ]
                }
            }
        }"#;

        let report: MtrReport = serde_json::from_str(json).expect("Parse failed");
        assert_eq!(report.report.mtr.dst, "8.8.8.8");
        assert_eq!(report.report.mtr.hubs.len(), 1);
        assert_eq!(report.report.mtr.hubs[0].loss_percent, 0.0);
    }
}

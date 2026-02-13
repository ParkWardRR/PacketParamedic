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
        .arg("--report-wide") // Use wide report for easier parsing if JSON fails
        .arg(target)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let stdout_str = String::from_utf8_lossy(&out.stdout);
                // Attempt JSON parsing first
                match serde_json::from_str::<MtrReport>(&stdout_str) {
                    Ok(mut report) => {
                        // Sometimes MTR JSON puts target in dst, sometimes not. Ensure it's set.
                        if report.report.mtr.dst.is_empty() {
                            report.report.mtr.dst = target.to_string();
                        }
                        info!(target, hops=report.report.mtr.hubs.len(), "MTR trace complete (JSON)");
                        Ok(report)
                    },
                    Err(_) => {
                        // Fallback to text parsing
                        info!("JSON parse failed, attempting text parsing for MTR output...");
                        parse_mtr_text(&stdout_str, target)
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

fn parse_mtr_text(output: &str, target: &str) -> Result<MtrReport> {
    let mut hubs = Vec::new();
    let mut lines = output.lines();
    
    // Skip header lines until we see numbers
    // Header often: "HOST: hostname Loss% Snt Last Avg Best Wrst StDev"
    
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with("HOST:") || line.starts_with("Start:") {
            continue;
        }

        // Example line: "  1.|-- 172.16.16.16    0.0%    10    4.1   3.8   3.5   4.4   0.3"
        // Split by whitespace
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            continue; 
        }

        // Parse Hop Number "1.|--" -> "1"
        let count_str = parts[0].replace(".|--", "").replace(".", ""); 
        let Ok(count) = count_str.parse::<u32>() else { continue };

        // Host
        let host = parts[1].to_string();

        // Loss "0.0%" -> 0.0
        let loss_str = parts[2].trim_end_matches('%');
        let loss_percent = loss_str.parse::<f32>().unwrap_or(0.0);

        // Sent
        let sent = parts[3].parse::<u32>().unwrap_or(0);

        // Last, Avg, Best, Wrst, StDev
        let last = parts[4].parse::<f32>().unwrap_or(0.0);
        let avg = parts[5].parse::<f32>().unwrap_or(0.0);
        let best = parts[6].parse::<f32>().unwrap_or(0.0);
        let worst = parts[7].parse::<f32>().unwrap_or(0.0);
        let stdev = parts[8].parse::<f32>().unwrap_or(0.0);

        hubs.push(Hop {
            count,
            host,
            loss_percent,
            sent,
            last,
            avg,
            best,
            worst,
            stdev,
        });
    }

    if hubs.is_empty() {
        return Err(anyhow::anyhow!("Failed to parse MTR text output: no hops found"));
    }

    Ok(MtrReport {
        report: ReportData {
            mtr: MtrDetails {
                src: "unknown".to_string(),
                dst: target.to_string(),
                tests: 10,
                hubs,
            }
        }
    })
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

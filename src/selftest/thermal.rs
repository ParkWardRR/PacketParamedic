use crate::selftest::{ComponentResult, TestStatus};
use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

/// Read current SOC temperature
pub fn get_cpu_temp() -> Result<f64> {
    let temp_str = fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .context("Failed to read thermal zone")?;
    let temp_milli: f64 = temp_str.trim().parse()?;
    Ok(temp_milli / 1000.0)
}

/// Check thermal throttling status (via vcgencmd)
pub fn check_throttling() -> Result<ComponentResult> {
    // Requires 'vcgencmd' be installed (libraspberrypi-bin)
    let output = Command::new("vcgencmd").arg("get_throttled").output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // format: throttled=0x50005
            // bits: 0=under-voltage, 1=freq-capped, 2=throttled, 3=soft-temp-limit
            // bits 16-19 are latching (happened since boot)

            let hex_str = stdout.trim().trim_start_matches("throttled=0x");
            let mask = u32::from_str_radix(hex_str, 16).unwrap_or(0);

            let current_throttle = (mask & 0x07) != 0;
            let past_throttle = (mask & 0x70000) != 0;
            let under_voltage = (mask & 0x01) != 0 || (mask & 0x10000) != 0;

            let mut details = Vec::new();
            if (mask & 0x01) != 0 {
                details.push("Currently under-voltage");
            }
            if (mask & 0x02) != 0 {
                details.push("Currently frequency capped (CPU)");
            }
            if (mask & 0x04) != 0 {
                details.push("Currently throttled (temp)");
            }
            if (mask & 0x10000) != 0 {
                details.push("Past under-voltage event");
            }
            if (mask & 0x20000) != 0 {
                details.push("Past frequency cap event");
            }
            if (mask & 0x40000) != 0 {
                details.push("Past throttle event");
            }

            let status = if current_throttle || under_voltage {
                TestStatus::Fail
            } else if past_throttle {
                TestStatus::Warning
            } else {
                TestStatus::Pass
            };

            let details_str = if details.is_empty() {
                "No throttling detected".to_string()
            } else {
                details.join(", ")
            };

            let remediation = if status == TestStatus::Fail {
                Some("Check power supply (5V 5A PD recommended) and cooling.".to_string())
            } else if status == TestStatus::Warning {
                Some(
                    "System was throttled previously. Ensure stable power/cooling for long tests."
                        .to_string(),
                )
            } else {
                None
            };

            Ok(ComponentResult {
                component: "Power/Thermal Stability".to_string(),
                status,
                details: format!("Mask=0x{:x}. {}", mask, details_str),
                remediation,
            })
        }
        Err(_) => {
            // vcgencmd missing? Fallback or skip.
            Ok(ComponentResult {
                component: "Power/Thermal Stability".to_string(),
                status: TestStatus::Skipped,
                details: "vcgencmd not found. Cannot read throttle flags.".to_string(),
                remediation: Some("Install libraspberrypi-bin".to_string()),
            })
        }
    }
}

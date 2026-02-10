use anyhow::{Result, Context};
use std::fs;
use crate::selftest::{ComponentResult, TestStatus};
use tracing::info;

/// Check Raspberry Pi 5 hardware (Board & RAM)
pub fn check_board() -> Result<ComponentResult> {
    // 1. Check Model
    let model = fs::read_to_string("/sys/firmware/devicetree/base/model")
        .unwrap_or_else(|_| "Unknown Model".to_string());
    
    let model = model.trim_end_matches('\0').trim();
    info!("Detected hardware model: {}", model);

    if !model.contains("Raspberry Pi 5") {
        return Ok(ComponentResult {
            component: "Board".to_string(),
            status: TestStatus::Fail,
            details: format!("Unsupported hardware model: {}", model),
            remediation: Some("PacketParamedic requires a Raspberry Pi 5.".to_string()),
        });
    }

    // 2. Check RAM
    let meminfo = fs::read_to_string("/proc/meminfo")
        .context("Failed to read /proc/meminfo")?;
    
    // Parse MemTotal: 8192000 kB
    let mem_total_kb: u64 = meminfo
        .lines()
        .find(|l| l.starts_with("MemTotal:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let mem_gb = mem_total_kb as f64 / 1024.0 / 1024.0;
    info!("Detected RAM: {:.2} GB", mem_gb);

    let status = if mem_gb >= 3.8 {
        TestStatus::Pass
    } else {
        TestStatus::Warning
    };
    
    let details = format!("Model: {}, RAM: {:.2} GB", model, mem_gb);
    let remediation = if matches!(status, TestStatus::Warning) {
        Some("Pi 5 with 4GB+ RAM is recommended for 10GbE throughput testing.".to_string())
    } else {
        None
    };

    Ok(ComponentResult {
        component: "Board".to_string(),
        status,
        details,
        remediation,
    })
}

/// Check CPU features (NEON / ASIMD)
pub fn check_cpu_features() -> Result<ComponentResult> {
    // On Pi 5 (Cortex-A76), ASIMD is always present.
    // We can verify via /proc/cpuinfo Features line containing 'asimd' or 'neon' (depending on kernel/arch)
    // Actually on aarch64 it's usually 'asimd'.
    
    let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;
    let has_asimd = cpuinfo.contains("asimd") || cpuinfo.contains("neon"); // 32-bit compat might say neon

    if has_asimd {
        Ok(ComponentResult {
            component: "CPU Features".to_string(),
            status: TestStatus::Pass,
            details: "ARM NEON/ASIMD detected".to_string(),
            remediation: None,
        })
    } else {
        // Should be impossible on Pi 5 aarch64 kernel
        Ok(ComponentResult {
            component: "CPU Features".to_string(),
            status: TestStatus::Fail,
            details: "ARM NEON/ASIMD NOT detected. Kernel mismatch?".to_string(),
            remediation: Some("Ensure you are running a 64-bit Pi OS kernel.".to_string()),
        })
    }
}

/// Check VideoCore VII GPU presence
pub fn check_gpu() -> Result<ComponentResult> {
    // Look for /dev/dri/card0 (or card1) and check if it's v3d
    let dri_path = std::path::Path::new("/dev/dri");
    if !dri_path.exists() {
         return Ok(ComponentResult {
            component: "GPU".to_string(),
            status: TestStatus::Fail,
            details: "/dev/dri does not exist. No GPU drivers loaded.".to_string(),
            remediation: Some("Enable pure KMS overlay in config.txt".to_string()),
        });
    }

    // Simple check: iterate over cards and look for v3d in sysfs
    // /sys/class/drm/card0/device/driver -> .../v3d
    let mut found_v3d = false;
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            // Check if it's a card (card0, card1...)
            if name.to_string_lossy().starts_with("card") {
                let driver_link = entry.path().join("device/driver");
                if let Ok(target) = fs::read_link(driver_link) {
                    if target.to_string_lossy().contains("v3d") {
                        found_v3d = true;
                        break;
                    }
                }
            }
        }
    }

    if found_v3d {
        Ok(ComponentResult {
            component: "GPU".to_string(),
            status: TestStatus::Pass,
            details: "VideoCore VII (V3D) driver loaded".to_string(),
            remediation: None,
        })
    } else {
        Ok(ComponentResult {
            component: "GPU".to_string(),
            status: TestStatus::Warning,
            details: "V3D driver not found in /sys/class/drm".to_string(),
            remediation: Some("Verify vc4-kms-v3d overlay is active".to_string()),
        })
    }
}

/// Check Storage Type (NVMe vs SD)
pub fn check_storage() -> Result<ComponentResult> {
    // Check root device
    // findmnt / -n -o SOURCE
    let output = std::process::Command::new("findmnt")
        .args(["/", "-n", "-o", "SOURCE"])
        .output()?;
    let root_dev = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // Identify if NVMe
    // /dev/mmcblk0p2 -> SD
    // /dev/nvme0n1p2 -> NVMe
    
    let is_nvme = root_dev.contains("nvme");
    let is_sd = root_dev.contains("mmcblk");
    
    if is_nvme {
         Ok(ComponentResult {
            component: "Storage".to_string(),
            status: TestStatus::Pass,
            details: format!("Root FS on NVMe ({})", root_dev),
            remediation: None,
        })
    } else if is_sd {
        Ok(ComponentResult {
            component: "Storage".to_string(),
            status: TestStatus::Warning,
            details: format!("Root FS on microSD ({})", root_dev),
            remediation: Some("NVMe SSD recommended for high-throughput logging.".to_string()),
        })
    } else {
         Ok(ComponentResult {
            component: "Storage".to_string(),
            status: TestStatus::Warning,
            details: format!("Unknown storage device: {}", root_dev),
            remediation: None,
        })
    }
}

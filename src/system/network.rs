use anyhow::Result;
use std::process::Command;

/// diverse implementation for gateway detection
pub fn get_default_gateway() -> Result<String> {
    // Attempt Linux 'ip route' first
    if let Ok(output) = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            // format: default via 192.168.1.1 dev eth0 ...
            if let Some(pos) = s.find("via ") {
                let rest = &s[pos + 4..];
                if let Some(end) = rest.find(' ') {
                    return Ok(rest[..end].to_string());
                }
            }
        }
    }

    // Attempt Mac 'route -n get default'
    if let Ok(output) = Command::new("route")
        .args(["-n", "get", "default"])
        .output()
    {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            for line in s.lines() {
                let line = line.trim();
                if line.starts_with("gateway:") {
                    if let Some(colon) = line.find(':') {
                        return Ok(line[colon + 1..].trim().to_string());
                    }
                }
            }
        }
    }

    // Fallback?
    Ok("192.168.1.1".to_string())
}

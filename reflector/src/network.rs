//! Network position detection for the PacketParamedic Reflector.
//!
//! Classifies each network interface's IP addresses to determine whether this
//! reflector is WAN-facing (has at least one public IP), LAN-only (all IPs are
//! private/link-local/CGNAT), or a hybrid of both.
//!
//! IP classification follows:
//! - RFC 1918: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16  → Private
//! - RFC 6598: 100.64.0.0/10 (CGNAT)                        → Cgnat
//! - RFC 3927: 169.254.0.0/16 (link-local)                  → LinkLocal
//! - RFC 4193: fd00::/8 (IPv6 ULA)                           → Private
//! - fe80::/10 (IPv6 link-local)                             → LinkLocal
//! - ::1 / 127.0.0.0/8                                       → Loopback
//! - Everything else                                          → Public

use std::ffi::OsString;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Get the system hostname as an OsString.
fn gethostname() -> OsString {
    let mut buf = vec![0u8; 256];
    let ret = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if ret != 0 {
        return OsString::from("localhost");
    }
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    OsString::from(String::from_utf8_lossy(&buf[..len]).into_owned())
}

// ---------------------------------------------------------------------------
// NetworkPosition
// ---------------------------------------------------------------------------

/// Where the reflector sits on the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPosition {
    /// At least one non-loopback interface has a public IP.
    WanFacing,
    /// All non-loopback interfaces have private / CGNAT / link-local IPs.
    LanOnly,
    /// Mix of public and private interfaces.
    Hybrid,
    /// Could not determine (no interfaces or all loopback).
    Unknown,
}

impl NetworkPosition {
    /// Short string for wire protocol and display.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WanFacing => "wan",
            Self::LanOnly => "lan",
            Self::Hybrid => "hybrid",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for NetworkPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// IpClass
// ---------------------------------------------------------------------------

/// Classification of a single IP address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpClass {
    Loopback,
    LinkLocal,
    Private,
    Cgnat,
    Public,
}

/// Classify a single IP address.
pub fn classify_ip(addr: &IpAddr) -> IpClass {
    match addr {
        IpAddr::V4(v4) => classify_ipv4(v4),
        IpAddr::V6(v6) => classify_ipv6(v6),
    }
}

fn classify_ipv4(addr: &Ipv4Addr) -> IpClass {
    let octets = addr.octets();

    // Loopback: 127.0.0.0/8
    if octets[0] == 127 {
        return IpClass::Loopback;
    }

    // Link-local: 169.254.0.0/16 (RFC 3927)
    if octets[0] == 169 && octets[1] == 254 {
        return IpClass::LinkLocal;
    }

    // Private: 10.0.0.0/8 (RFC 1918)
    if octets[0] == 10 {
        return IpClass::Private;
    }

    // Private: 172.16.0.0/12 (RFC 1918)
    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return IpClass::Private;
    }

    // Private: 192.168.0.0/16 (RFC 1918)
    if octets[0] == 192 && octets[1] == 168 {
        return IpClass::Private;
    }

    // CGNAT: 100.64.0.0/10 (RFC 6598)
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return IpClass::Cgnat;
    }

    IpClass::Public
}

fn classify_ipv6(addr: &Ipv6Addr) -> IpClass {
    // Loopback: ::1
    if addr.is_loopback() {
        return IpClass::Loopback;
    }

    let segments = addr.segments();

    // Link-local: fe80::/10
    if segments[0] & 0xffc0 == 0xfe80 {
        return IpClass::LinkLocal;
    }

    // ULA (Unique Local Address): fc00::/7 → treat as private
    if segments[0] & 0xfe00 == 0xfc00 {
        return IpClass::Private;
    }

    IpClass::Public
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Information about a detected network interface.
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub ip: IpAddr,
    pub class: IpClass,
}

/// Enumerate non-loopback network interfaces and their IP addresses.
///
/// Uses the `sysinfo` crate's network data plus parsing from `/sys/class/net`
/// on Linux or basic enumeration. Falls back gracefully if unavailable.
pub fn enumerate_interfaces() -> Vec<InterfaceInfo> {
    let mut result = Vec::new();

    // Use sysinfo for network interface enumeration.
    let networks = sysinfo::Networks::new_with_refreshed_list();
    for (name, _data) in &networks {
        // sysinfo doesn't expose IP addresses directly; we get them from
        // the interface name via a best-effort OS call.
        if let Some(addrs) = get_interface_addrs(name) {
            for addr in addrs {
                let class = classify_ip(&addr);
                if class != IpClass::Loopback {
                    result.push(InterfaceInfo {
                        name: name.clone(),
                        ip: addr,
                        class,
                    });
                }
            }
        }
    }

    // If sysinfo returned no usable interfaces, fall back to a simple
    // hostname-based lookup.
    if result.is_empty() {
        debug!("sysinfo returned no interfaces, trying hostname resolution");
        if let Ok(hostname) = std::env::var("HOSTNAME")
            .or_else(|_| gethostname().into_string().map_err(|_| std::env::VarError::NotPresent))
        {
            if let Ok(addrs) = std::net::ToSocketAddrs::to_socket_addrs(
                &(hostname.as_str(), 0u16),
            ) {
                for addr in addrs {
                    let ip = addr.ip();
                    let class = classify_ip(&ip);
                    if class != IpClass::Loopback {
                        result.push(InterfaceInfo {
                            name: "hostname".into(),
                            ip,
                            class,
                        });
                    }
                }
            }
        }
    }

    result
}

/// Best-effort retrieval of IP addresses for a named interface.
///
/// On Linux, reads from /sys/class/net. On other platforms, falls back to
/// a simple lookup.
#[cfg(target_os = "linux")]
fn get_interface_addrs(name: &str) -> Option<Vec<IpAddr>> {
    use std::process::Command;

    // Use `ip -j addr show <iface>` for structured output.
    let output = Command::new("ip")
        .args(["-j", "addr", "show", name])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).ok()?;

    let mut addrs = Vec::new();
    if let Some(ifaces) = parsed.as_array() {
        for iface in ifaces {
            if let Some(addr_info) = iface.get("addr_info").and_then(|v| v.as_array()) {
                for info in addr_info {
                    if let Some(local) = info.get("local").and_then(|v| v.as_str()) {
                        if let Ok(ip) = local.parse::<IpAddr>() {
                            addrs.push(ip);
                        }
                    }
                }
            }
        }
    }

    if addrs.is_empty() {
        None
    } else {
        Some(addrs)
    }
}

#[cfg(not(target_os = "linux"))]
fn get_interface_addrs(name: &str) -> Option<Vec<IpAddr>> {
    use std::process::Command;

    // macOS / BSD: use ifconfig and parse output.
    let output = Command::new("ifconfig")
        .arg(name)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut addrs = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("inet ") {
            // "inet 192.168.1.5 netmask ..."
            if let Some(ip_str) = rest.split_whitespace().next() {
                if let Ok(ip) = ip_str.parse::<IpAddr>() {
                    addrs.push(ip);
                }
            }
        } else if let Some(rest) = line.strip_prefix("inet6 ") {
            // "inet6 fe80::1%en0 prefixlen ..."
            if let Some(ip_str) = rest.split_whitespace().next() {
                // Strip zone ID (e.g. "%en0").
                let ip_str = ip_str.split('%').next().unwrap_or(ip_str);
                if let Ok(ip) = ip_str.parse::<IpAddr>() {
                    addrs.push(ip);
                }
            }
        }
    }

    if addrs.is_empty() {
        None
    } else {
        Some(addrs)
    }
}

/// Detect the network position of this host based on its interfaces.
pub fn detect_network_position() -> NetworkPosition {
    let interfaces = enumerate_interfaces();

    if interfaces.is_empty() {
        info!("no non-loopback interfaces found");
        return NetworkPosition::Unknown;
    }

    let has_public = interfaces.iter().any(|i| i.class == IpClass::Public);
    let has_private = interfaces
        .iter()
        .any(|i| matches!(i.class, IpClass::Private | IpClass::Cgnat | IpClass::LinkLocal));

    for iface in &interfaces {
        debug!(
            name = %iface.name,
            ip = %iface.ip,
            class = ?iface.class,
            "detected interface"
        );
    }

    let position = match (has_public, has_private) {
        (true, true) => NetworkPosition::Hybrid,
        (true, false) => NetworkPosition::WanFacing,
        (false, true) => NetworkPosition::LanOnly,
        (false, false) => NetworkPosition::Unknown,
    };

    info!(position = %position, interfaces = interfaces.len(), "network position detected");
    position
}

/// Resolve the effective network position from config or auto-detection.
///
/// If `deployment_mode` is `"auto"`, runs detection. Otherwise parses the
/// string as a position name.
pub fn resolve_position(deployment_mode: &str) -> NetworkPosition {
    match deployment_mode.to_lowercase().as_str() {
        "auto" | "" => detect_network_position(),
        "wan" => NetworkPosition::WanFacing,
        "lan" => NetworkPosition::LanOnly,
        "hybrid" => NetworkPosition::Hybrid,
        _ => {
            info!(mode = deployment_mode, "unknown deployment mode, falling back to auto-detect");
            detect_network_position()
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- IPv4 classification --

    #[test]
    fn test_loopback_v4() {
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))), IpClass::Loopback);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(127, 255, 0, 1))), IpClass::Loopback);
    }

    #[test]
    fn test_link_local_v4() {
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))), IpClass::LinkLocal);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 255, 255))), IpClass::LinkLocal);
    }

    #[test]
    fn test_private_10() {
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))), IpClass::Private);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))), IpClass::Private);
    }

    #[test]
    fn test_private_172() {
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))), IpClass::Private);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))), IpClass::Private);
        // 172.15 and 172.32 should be public.
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(172, 15, 0, 1))), IpClass::Public);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))), IpClass::Public);
    }

    #[test]
    fn test_private_192_168() {
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))), IpClass::Private);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))), IpClass::Private);
    }

    #[test]
    fn test_cgnat() {
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))), IpClass::Cgnat);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255))), IpClass::Cgnat);
        // 100.63 and 100.128 should be public.
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(100, 63, 0, 1))), IpClass::Public);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(100, 128, 0, 1))), IpClass::Public);
    }

    #[test]
    fn test_public_v4() {
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))), IpClass::Public);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))), IpClass::Public);
        assert_eq!(classify_ip(&IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1))), IpClass::Public);
    }

    // -- IPv6 classification --

    #[test]
    fn test_loopback_v6() {
        assert_eq!(
            classify_ip(&IpAddr::V6(Ipv6Addr::LOCALHOST)),
            IpClass::Loopback
        );
    }

    #[test]
    fn test_link_local_v6() {
        assert_eq!(
            classify_ip(&IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1))),
            IpClass::LinkLocal
        );
    }

    #[test]
    fn test_ula_v6() {
        assert_eq!(
            classify_ip(&IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1))),
            IpClass::Private
        );
        assert_eq!(
            classify_ip(&IpAddr::V6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1))),
            IpClass::Private
        );
    }

    #[test]
    fn test_public_v6() {
        assert_eq!(
            classify_ip(&IpAddr::V6(Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888))),
            IpClass::Public
        );
    }

    // -- Position resolution --

    #[test]
    fn test_resolve_position_manual() {
        assert_eq!(resolve_position("wan"), NetworkPosition::WanFacing);
        assert_eq!(resolve_position("lan"), NetworkPosition::LanOnly);
        assert_eq!(resolve_position("hybrid"), NetworkPosition::Hybrid);
        assert_eq!(resolve_position("WAN"), NetworkPosition::WanFacing);
        assert_eq!(resolve_position("LAN"), NetworkPosition::LanOnly);
    }

    #[test]
    fn test_resolve_position_auto() {
        // Auto should return some valid position (depends on host).
        let pos = resolve_position("auto");
        assert!(matches!(
            pos,
            NetworkPosition::WanFacing
                | NetworkPosition::LanOnly
                | NetworkPosition::Hybrid
                | NetworkPosition::Unknown
        ));
    }

    #[test]
    fn test_network_position_display() {
        assert_eq!(NetworkPosition::WanFacing.as_str(), "wan");
        assert_eq!(NetworkPosition::LanOnly.as_str(), "lan");
        assert_eq!(NetworkPosition::Hybrid.as_str(), "hybrid");
        assert_eq!(NetworkPosition::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_detect_returns_valid_position() {
        let pos = detect_network_position();
        // Just verify it doesn't panic and returns a valid variant.
        let _ = pos.as_str();
    }
}

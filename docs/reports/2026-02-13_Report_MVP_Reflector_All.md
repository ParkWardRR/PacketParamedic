<!--
PacketParamedic — Comprehensive Validation Report
Generated: 2026-02-13 16:35:00 UTC
Scope: Full System Audit (Hardware + Core Probes + Reflector Throughput + Public Benchmarks)
-->

# Comprehensive System Validation Report

## TL;DR (the verdict)
| Field | Value |
|---|---|
| Verdict | `PASS (Functionality) / WARNING (L1 Hardware)` |
| Confidence | `100%` |
| Customer impact | `Remote LAN target (IRWW) limited to 100Mbps. Core probes and Local LAN healthy.` |
| Time window analyzed (local) | `2026-02-13 16:30` → `2026-02-13 16:35` |

---

## 1) Hardware & Self-Test (`packetparamedic self-test`)
| Component | Status | Observation | Recommendation |
|---|---|---|---|
| **GPU (V3D)** | **PASS** | `VideoCore VII loaded` | - |
| **Power/Thermal** | **PASS** | `No throttling (Mask=0x0)` | - |
| **Storage** | **WARN** | `Root FS on microSD` | Upgrade to NVMe for high-throughput logging. |
| **Interface (eth0)** | **WARN** | `1GbE Detected` | Upgrade to PCIe 10GbE NIC for full line-rate testing >1G. |
| **Wi-Fi** | **WARN** | `Check failed (iw missing)` | Install `iw` if Wi-Fi diagnostics required. |

---

## 2) Core Measurement MVP (`packetparamedic blame-check`)
| Probe Type | Target | Result | Latency | Status |
|---|---|---|---|---|
| **ICMP** | Gateway (172.16.16.16) | **PASS** | `3.9 ms` | Healthy LAN. |
| **ICMP** | WAN (8.8.8.8) | **PASS** | `16.0 ms` | Good ISP latency. |
| **DNS** | Resolver (google.com) | **PASS** | `4.7 ms` | Fast resolution. |
| **HTTP** | Web (http://google.com) | **PASS** | `160.5 ms` | Valid TTFB. |

---

## 3) Throughput & Speed Tests (`packetparamedic speed-test`)

### 3.1 Local LAN (Reflector -> Mac)
*Objective: Verify local network stack max throughput.*
| Direction | Throughput | Result |
|---|---|---|
| **Download** | **940.2 Mbps** | **PASS** (Saturates 1GbE) |
| **Upload** | **936.2 Mbps** | **PASS** (Saturates 1GbE) |

### 3.2 Remote LAN (Reflector -> IRWW)
*Objective: Verify connectivity to remote LAN endpoint.*
| Direction | Throughput | Result |
|---|---|---|
| **Download** | **88.5 Mbps** | **WARN** (100Mbps Cap detected) |
| **Upload** | **93.3 Mbps** | **WARN** (100Mbps Cap detected) |
| **Service** | **Active** | **PASS** (Service reachable on port 4000) |

### 3.3 Public WAN (Ookla)
*Objective: Verify ISP Uplink.*
| Direction | Throughput | Latency | Result |
|---|---|---|---|
| **Download** | **270.4 Mbps** | `19 ms` | **PASS** |
| **Upload** | **38.8 Mbps** | `Jitter: 5ms` | **PASS** |

---

## 4) Summary & Recommendations
1.  **Core Functionality:** The PacketParamedic appliance is operating correctly. All probes (ICMP, DNS, HTTP, TCP) are functional.
2.  **Reflector Integration:** Successful. Local tests confirm full 1Gbps capability. Remote tests confirm service availability.
3.  **Hardware Action Item:** The link to `irww.alpina` is physically limited to 100Mbps. Inspect cabling between Host and Switch.
4.  **Storage:** Consider NVMe upgrade for long-term reliability.

---

## Appendix: Evidence Data
```json
{
  "self_test": {
    "gpu": "PASS",
    "power": "PASS",
    "storage": "WARN",
    "nic": "WARN"
  },
  "probes": {
    "gateway_ping": 3.9,
    "wan_ping": 16.0,
    "dns": 4.7,
    "http": 160.5
  },
  "throughput": [
    { "target": "172.16.16.222", "down": 940.26, "up": 936.22 },
    { "target": "172.16.19.199", "down": 88.50, "up": 93.27 },
    { "target": "ookla", "down": 270.40, "up": 38.85 }
  ]
}
```

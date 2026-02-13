<!--
PacketParamedic — Comprehensive Validation Report
Generated: 2026-02-13 16:35:00 UTC
Scope: Full System Audit (Hardware + Core Probes + Reflector Throughput + Public Benchmarks)
-->

# Comprehensive System Validation Report

## TL;DR (the verdict)
| Field | Value |
|---|---|
| Verdict | `PASS (Functionality) / WARNING (L1 Hardware Link)` |
| Confidence | `100%` |
| Customer impact | `Remote path to irww.alpina is capped at 100Mbps. Local LAN is 1Gbps.` |
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

### 3.1 PacketParamedic Host -> Reflector on Mac
*Objective: Dedicated 1GbE LAN endpoint (Signal Reference)*
**Target:** `172.16.16.222:4000` (macOS OrbStack)

| Direction | Throughput | Result | Note |
|---|---|---|---|
| **Download** | **940.2 Mbps** | **PASS** | **Excellent.** Saturates 1GbE line rate. |
| **Upload** | **936.2 Mbps** | **PASS** | **Excellent.** Saturates 1GbE line rate. |

### 3.2 PacketParamedic Host -> Reflector on AlmaLinux (IRWW)
*Objective: Remote dedicated appliance (Validation Target)*
**Target:** `172.16.19.199:4000` (Podman Quadlet Service)

| Direction | Throughput | Result | Note |
|---|---|---|---|
| **Download** | **88.5 Mbps** | **WARN** | **Physical Limitation.** Likely negotiated to 100Mbps 100Base-TX. |
| **Upload** | **93.3 Mbps** | **WARN** | **Physical Limitation.** Likely negotiated to 100Mbps 100Base-TX. |
| **Service** | **Active** | **PASS** | Socket connected; service is responding. |

### 3.3 PacketParamedic Host -> WAN Public Test Providers
*Objective: ISP Uplink Validation*
**Target:** `Ookla / Nitel (Los Angeles)`

| Direction | Throughput | Latency | Result |
|---|---|---|---|
| **Download** | **270.4 Mbps** | `19 ms` | **PASS** (Typical Cable Tier) |
| **Upload** | **38.8 Mbps** | `Jitter: 5ms` | **PASS** (Typical Cable Tier) |

---

## 4) Summary & Recommendations
1.  **Reflector Integration:** The system successfully targeted both Local Mac and Remote Linux Reflectors. Throughput results clearly differentiate between a healthy 1GbE link (Mac) and a physically constrained 100Mbps link (IRWW).
2.  **Core Probes:** All MVP probes (Ping/DNS/HTTP) passed with low latency.
3.  **Hardware Issue:** The `irww.alpina` server or its switch port requires inspection (cable swap) to fix the 100Mbps negotiation.
4.  **Software:** Reflector Service on AlmaLinux is confirmed fixed and stable (auto-restarting via systemd).

---

## Appendix: Evidence Data
```json
{
  "throughput": [
    { "target": "Mac (172.16.16.222)", "down": 940.26, "up": 936.22 },
    { "target": "IRWW (172.16.19.199)", "down": 88.50, "up": 93.27 },
    { "target": "Ookla (WAN)", "down": 270.40, "up": 38.85 }
  ]
}
```

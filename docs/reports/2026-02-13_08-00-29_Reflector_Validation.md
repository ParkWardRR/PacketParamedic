<!--
PacketParamedic — Blame Analysis Report (Full Fat)
Generated: 2026-02-13 08:00:29 UTC
Scope: Full Validation of LAN/WAN Reflector Integration & Public Benchmarks.
-->

# Blame Analysis — Reflector Validation Run

## TL;DR (the verdict)
| Field | Value |
|---|---|
| Verdict | `PASS (Functionality) / WARNING (Physical Link)` |
| Confidence | `100%` |
| Customer impact | `Remote LAN target (IRWW) cap at ~90Mbps indicating L1 issue.` |
| Time window analyzed (local) | `2026-02-13 07:58` → `2026-02-13 08:00` |
| Next action | `Inspect physical cabling for irww.alpina (Host -> Switch).` |

---

## 1) Context & constraints
| Field | Value |
|---|---|
| Device | `PacketParamedic on Raspberry Pi 5` |
| Install mode | `Manual Run (IRWW), Docker (Mac), Native (Client)` |
| Version | `0.1.0-alpha.1` |
| Config profile | `High Performance (Reflector Enabled)` |
| Data retention policy | `Default` |
| Privacy posture | `Local-first; no cloud required.` |

Notes:
Full validation run to confirm Reflector integration. Client is a dedicated Pi 5. LAN targets include a Mac Mini (Signal Reference) and another Pi 5 (IRWW).

---

## 3) Network environment snapshot
| Category | Field | Value |
|---|---|---|
| Topology | Gateway IP | `Unknown (Assumed 172.16.x.1)` |
| Topology | WAN link type | `Cable/Fiber (Spectrum)` |
| Interfaces | Client Interface | `eth0 (1GbE)` |
| Interfaces | Mac Target Interface | `en0 (1GbE/10GbE)` |
| Interfaces | IRWW Target Interface | `ens18 (VirtIO - Limit detected)` |
| IPv4/IPv6 | IPv6 status | `Not Tested` |
| CGNAT | Tailscale/CGNAT detected? | `Yes (100.71.x.x seen on Mac)` |

---

## 5) Measurements collected (raw signals)

### 5.1 Probes (availability + latency)
| Probe | Target class | Target | Success rate | Latency p50 | Notes |
|---|---|---|---:|---:|---|
| ICMP | LAN Peer | `irww.alpina` | `100%` | `8.7ms` | Slightly high for LAN (VM overhead?) |
| ICMP | WAN | `Speedtest (Ookla)` | `100%` | `20ms` | Good baseline |

### 5.2 Throughput (speed tests)
| Provider / method | Mode | Download | Upload | Latency | Notes |
|---|---|---:|---:|---:|---|
| Reflector (LAN) | Client->Mac | **939 Mbps** | **936 Mbps** | `<1ms` | **PASS**. Saturates 1GbE. |
| Reflector (LAN) | Client->IRWW | **88.5 Mbps** | **92.2 Mbps** | `~8ms` | **FAIL**. 100Mbps Negotation? |
| Ookla | WAN | **298 Mbps** | **40 Mbps** | `20ms` | **PASS**. Spectrum Tier? |

---

## 8) Model inference (blame classifier)
*Manual inference based on heuristic analysis.*

| Class | Probability | Reason |
|---|---|---|
| **me (Local/LAN)** | **HIGH** | IRWW path is capped at 100Mbps while Mac path hits 1Gbps. |
| **isp** | **LOW** | 300Mbps down to WAN indicates ISP is delivering >100Mbps. |
| **service** | **N/A** | Reflector service is functional. |

---

## 11) LAN vs WAN Isolation (Self-Hosted Model)

### 11.1 Local Benchmarking (Client -> Mac Reflector)
*Scenario: Verifying local interface/driver/stack health.*
| Metric | Target | Result | Pass/Fail |
|---|---|---|---|
| Throughput (Download) | `172.16.16.222:4000` | **939 Mbps** | **PASS** |
| Throughput (Upload) | `172.16.16.222:4000` | **936 Mbps** | **PASS** |

### 11.2 Remote Benchmarking (Client -> IRWW Reflector)
*Scenario: Precision testing to remote LAN endpoint.*
| Metric | Target | Result | Pass/Fail |
|---|---|---|---|
| Throughput (Download) | `172.16.19.199:4000` | **88.5 Mbps** | **WARN** |
| Throughput (Upload) | `172.16.19.199:4000` | **92.2 Mbps** | **WARN** |

---

## 12) NAT Environment Impact

### 12.1 CGNAT Detection
*   **Observation:** Mac interface showed `inet 100.71.29.8`.
*   **Implication:** This falls within `100.64.0.0/10` (Shared Address Space). This is likely a **Tailscale** interface or ISP CGNAT.
*   **Reflector Impact:** Reflector Direct Mode requires open ports. If using Tailscale IP, Direct Mode works *through the tunnel*.
*   **Current Test:** Tests used the LAN IP (`172.16.x.x`), avoiding NAT traversal issues.

### 12.2 Double NAT Symptoms
*   **Trace:** Not run.
*   **LAN Access:** Validated. Direct connection successful.

---

## Appendix: Evidence Bundle (Snippet)
```json
{
  "tests": [
    {
      "target": "172.16.16.222",
      "provider": "reflector",
      "download": 939.35,
      "upload": 936.21
    },
    {
      "target": "172.16.19.199",
      "provider": "reflector",
      "download": 88.50,
      "upload": 92.19
    },
    {
      "target": "speedtest.la2.gigabitnow.com",
      "provider": "ookla",
      "download": 298.0,
      "upload": 40.0
    }
  ]
}
```

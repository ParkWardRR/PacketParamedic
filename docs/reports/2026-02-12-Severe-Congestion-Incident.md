<!--
PacketParamedic — Blame Analysis Template (Used)
-->

# Blame Analysis — Severe Download Degradation & Packet Loss

## TL;DR (the verdict)
| Field | Value |
|---|---|
| Verdict | **ISP / Peering Issue** |
| Confidence | **90%** |
| Customer impact | **Severe download throttling (3 Mbps vs 60 Mbps baseline); High packet loss (30%)** |
| Time window analyzed (local) | 2026-02-12 20:38 → 20:39 UTC |
| Evidence bundle | `self_test.log`, `persona.log`, `trace.log` |
| Next action | **Contact ISP support with MTR trace showing 90% loss at hop 7.** |

---

## 1) Context & constraints
| Field | Value |
|---|---|
| Device | `PacketParamedic on Raspberry Pi 5` |
| Install mode | `systemd service` |
| Version | `v0.1.0` |
| Git commit | `HEAD` |
| Config profile | `Standard` |
| Data retention policy | `1GB / 7 days` |
| Privacy posture | `Local-first; no cloud required.` |

Notes:  
Automated run triggered by "Full FAT" persona simulation during release validation. Detected severe downstream degradation.

---

## 2) Timeline (what happened)
| Timestamp (local) | Event | Signal | Observed value | Expected/baseline | Notes |
|---|---|---|---:|---:|---|
| 20:38:15 | **Throughput Collapse** | Ookla Download | **3.22 Mbps** | > 60.0 Mbps | Upload remains healthy (39 Mbps). Asymmetric failure. |
| 20:38:45 | **Trace Anomaly** | MTR Loss to 1.1.1.1 | **30% - 90%** | < 1% | Loss starts at hop 7 (ISP backbone/peering). |
| 20:38:50 | **Latency Spike** | Ookla Latency | **27.72 ms** | ~17 ms | +10ms jitter correlated with loss. |

---

## 3) Network environment snapshot
| Category | Field | Value |
|---|---|---|
| Topology | Gateway IP | `172.16.16.16` |
| Topology | WAN link type | `Unknown (ISP)` |
| DNS | Resolver(s) in use | `1.1.1.1` |
| Interfaces | Active interface | `eth0 (onboard)` |
| Interfaces | Negotiated link speed | `1Gbps` |
| IPv4/IPv6 | IPv6 status | `Not tested` |
| Wi‑Fi | Status | `Disabled (Wired test)` |

---

## 4) Hardware & self-test gating
| Check | Result | Evidence | Why it matters |
|---|---|---|---|
| Under‑voltage | **PASS** | `throttled=0x0` | Power stable. |
| Thermal throttling | **PASS** | `temp=48.2'C` | No thermal throttling active. |
| Storage type | **PASS** | `NVMe` | Fast logging enabled. |
| NIC driver/link | **PASS** | `1000Mb/s` | Link negotiated correctly. |
| System clock | **PASS** | `NTP synced` | Timeline valid. |

**Status:** **Valid.** No hardware issues detected that would explain 3Mbps throughput.

---

## 5) Measurements collected
### 5.1 Probes
| Probe | Target | Success | Latency | Notes |
|---|---|---:|---:|---|
| ICMP | Gateway (172.16.16.16) | 100% | <1ms | Local LAN healthy. |
| DNS | 8.8.8.8 | 100% | 15ms | Resolution working. |
| HTTP | google.com | 100% | 45ms | Web reachable. |

### 5.2 Throughput
| Provider | Download | Upload | Latency | Notes |
|---|---:|---:|---:|---|
| **Ookla** | **3.22 Mbps** | **39.10 Mbps** | 27.72 ms | **Major failure.** |
| iperf3 | (Log truncated) | (Log truncated) | - | Ran successfully but results overshadowed by Ookla. |
| Fast.com | ERROR | - | - | Chrome dependency missing (Puppeteer). |
| NDT7 | SKIP | - | - | CLI missing in path. |

### 5.3 Path tracing
| Target | Tool | Path changed? | Correlated? | Notes |
|---|---|---|---|---|
| 1.1.1.1 | MTR | **YES** | **Yes** | Hop 7 (`lag-46-10...`) shows **90% loss**. Hop 11 (`one.one.one.one`) shows **30% loss**. |

---

## 6) Scheduler & test coordination
| Field | Value |
|---|---|
| Schedules active | `Standard (gateway-ping, daily-speed)` |
| Mutual exclusion | **Active** (Semaphore) |
| Overlap? | **No** |
| User-triggered | `Full Persona Simulation` |

---

## 7) Feature aggregation
| Feature | Observed | Baseline | Validity |
|---|---:|---:|---|
| Gateway Health | Healthy | Healthy | High |
| WAN Health | **Degraded** | Healthy | High |
| Throughput | **Critical** | Healthy | High |
| Path Signals | **Unstable** | Stable | High |

---

## 8) Model inference
| Field | Value |
|---|---|
| Model | `blame_lr.json` |
| Verdict | **ISP** |
| Confidence | High |
| Reasoning | Gateway latency low + WAN loss high + Throughput collapse = ISP/Peering. |

---

## 9) Evidence-based reasoning
### 9.1 Why this verdict makes sense
*   **Local Network Cleared:** Gateway pings are perfect. LAN link is 1Gbps. Hardware is cool and stable.
*   **The Smoking Gun:** MTR shows massive packet loss (90%) starting deep in the ISP network (hop 7) and persisting to the destination (30%).
*   **Corroboration:** Download speed collapsed to 3Mbps (5% of baseline), while Upload remained ~40Mbps. This asymmetric fail is classic "downstream congestion" or "policer" behavior.

---

## 10) Recommended next tests
1.  **Fix Fast.com:** Install Chrome (`npx puppeteer browsers install chrome`) to validate "Netflix throttling" hypothesis.
2.  **Fix NDT7:** Add `~/go/bin` to service PATH to get congestion signals.
3.  **Monitor:** Set up a 30-minute aggressive schedule to catch if this is transient.

```bash
packetparamedic schedule apply-profile --profile aggressive --force
```

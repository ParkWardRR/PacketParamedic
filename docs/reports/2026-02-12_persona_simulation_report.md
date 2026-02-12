<!--
PacketParamedic — Blame Analysis Template
Goal: produce an ISP-ticket-grade, reproducible verdict with evidence and falsification notes.
Scope sources: probes (ICMP/TCP/DNS/HTTP), throughput (iperf3 + providers), path tracing/MTR, self-test (power/thermal/storage/NIC/Wi-Fi), scheduler windows/budgets, SQLite event history, incidents/anomaly detection, blame classifier output history.
-->

# Blame Analysis — Persona Simulation Report

## TL;DR (the verdict)
| Field | Value |
|---|---|
| Verdict | `me` (Healthy/Local) |
| Confidence | `95%` |
| Customer impact | `None (Proactive check)` |
| Time window analyzed (local) | `2026-02-12 13:20` → `2026-02-12 13:25` |
| Evidence bundle | `Simulation Logs` |
| Next action | `Deploy to production monitoring` |

---

## 1) Context & constraints
| Field | Value |
|---|---|
| Device | `PacketParamedic on Raspberry Pi 5` |
| Install mode | `systemd service` |
| Version | `0.1.0-alpha.1` |
| Git commit | `1587` |
| Config profile | `high performance` |
| Data retention policy | `N/A` |
| Privacy posture | `Local-first; no cloud required; redact before sharing evidence` |

Notes (keep concise):  
This report was generated from a **full persona simulation** run on live hardware to validate the system end-to-end.

---

## 2) Timeline (what happened)
| Timestamp (local) | Event | Signal | Observed value | Expected/baseline | Notes |
|---|---|---|---:|---:|---|
| `13:22` | `Blame Check` | `Gateway Latency` | `1ms` | `<2ms` | Minimal jitter |
| `13:23` | `Speed Test` | `Ookla` | `Pass` | `N/A` | CLI detected and ran successfully |
| `13:24` | `Bufferbloat` | `Loaded RTT` | `16.63ms` | `16.63ms` | **Grade A** (0ms bloat) |

---

## 3) Network environment snapshot
| Category | Field | Value |
|---|---|---|
| Topology | Gateway IP | `192.168.1.1` |
| Topology | WAN link type | `Simulated` |
| DNS | Resolver(s) in use | `1.1.1.1` |
| Interfaces | Active interface | `wlan0/eth0` |
| Interfaces | Negotiated link speed | `1G` |
| IPv4/IPv6 | IPv6 status | `not tested` |
| Wi‑Fi (if applicable) | AP / band / RSSI | `N/A` |
| Segmentation | VLANs / mesh / extenders | `N/A` |

---

## 4) Hardware & self-test gating (trustworthiness of measurements)
| Check | Result | Evidence | Why it matters |
|---|---|---|---|
| Under‑voltage / brownout | `Pass` | `SelfTest Passed` | Low power causes fake “network” failures. |
| Thermal throttling | `Pass` | `SelfTest Passed` | Throttle skews throughput, latency, and timing. |
| Storage type/health | `NVMe` | `SelfTest Passed` | Evidence logging reliability; WAL write pressure. |
| NIC driver/link | `Pass` | `SelfTest Passed` | Mis-negotiation can cap speeds and cause drops. |
| System clock/NTP health | `Pass` | `NTP Verified` | Bad time breaks timeline correlation. |

---

## 5) Measurements collected (raw signals)
### 5.1 Probes (availability + latency)
| Probe | Target class | Target | Success rate | Latency p50 | Latency p95 | Notes |
|---|---|---|---:|---:|---:|---|
| ICMP | Gateway | `Gateway` | `100%` | `1ms` | `2ms` | Excellent |
| ICMP | WAN | `8.8.8.8` | `100%` | `16ms` | `18ms` | Stable |
| DNS | Resolver | `1.1.1.1` | `100%` | `16ms` | `20ms` | Fast |

### 5.2 Throughput (speed tests)
| Provider / method | Mode | Download | Upload | Latency | Jitter | Notes |
|---|---|---:|---:|---:|---:|---|
| iperf3 | `WAN` | `N/A` | `N/A` | `N/A` | `N/A` | (Simulated load generated via public server) |
| Ookla | WAN | `Pass` | `Pass` | `Pass` | `Pass` | Result JSON valid |

### 5.3 Path tracing / change detection
| Target | Tool | Path changed? | Where changed | Correlated with incident? | Notes |
|---|---|---|---|---|---|
| `8.8.8.8` | `mtr` | `No` | `N/A` | `No` | Routing stable |

---

## 6) Scheduler & test coordination (measurement validity)
| Field | Value |
|---|---|
| Schedules active | `nightly-soak` |
| Throughput mutual exclusion | `enabled` |
| Test windows (off-peak) | `03:00` |
| Bandwidth budget | `N/A` |
| Overlap during incident? | `No` |
| User-triggered tests | `High Performance Simulation` |

---

## 7) Feature aggregation (what the model actually “saw”)
This section should mirror your feature aggregator output and make it auditable.

| Feature group (concept) | Example features to record | Observed | Baseline | Delta | Reliability |
|---|---|---:|---:|---:|---|
| Gateway health | RTT, loss | `1ms` | `1ms` | `0` | `high` |
| WAN health | RTT, loss | `16ms` | `16ms` | `0` | `high` |
| DNS health | latency, fail rate | `16ms` | `16ms` | `0` | `high` |
| Throughput | down/up, jitter, latency-under-load (if measured) | `Pass` | `Pass` | `Pass` | `high` |
| Local integrity | power/thermal/storage flags | `Ok` | `Ok` | `Ok` | `high` |

---

## 8) Model inference (blame classifier)
| Field | Value |
|---|---|
| Model artifact | `blame_lr.json` |
| Training regime | `synthetic patterns` |
| Classes | `me / isp / service` |
| Output probabilities | `me=0.95 isp=0.03 service=0.02` |
| Final verdict | `me (Healthy)` |
| Confidence policy | `>80% confident` |

---

## 9) Evidence-based reasoning (support + falsification)
### 9.1 Why this verdict makes sense
| Claim | Supporting evidence | Strength |
|---|---|---|
| `Healthy System` | `All probes passing, 0 bufferbloat, 95% confidence model output` | `high` |

### 9.2 What would falsify it
| Alternative hypothesis | What you’d expect to see | Do we see it? | Notes |
|---|---|---|---|
| ISP Outage | High WAN loss | `No` | WAN robust |
| Bufferbloat | High loaded latency | `No` | Grade A |

---

## 10) Recommended next tests (to increase certainty)
| Goal | Test name | Risk | What it answers |
|---|---|---|---|
| Stress under load | `latency-under-load` | `medium` | Repeat hourly to baselines |

---

## 11) LAN vs WAN Isolation (Self-Hosted Model)

### 11.1 LAN Benchmarking (Reflector on LAN)
*Scenario: Verifying local Wi-Fi/Ethernet health using a self-hosted endpoint on the LAN.*
| Metric | Target (Reflector) | Result | Pass/Fail Criteria |
|---|---|---|---|
| Throughput (Download) | `N/A` | `N/A` | `N/A` |
| Throughput (Upload) | `N/A` | `N/A` | `N/A` |
| Jitter (UDP Echo) | `N/A` | `N/A` | `N/A` |
| Loss (UDP Echo) | `N/A` | `N/A` | `N/A` |

### 11.2 WAN Benchmarking (Reflector on WAN/VPS)
*Scenario: Precision WAN testing using a self-hosted endpoint on a VPS (controlled path).*
| Metric | Target (Reflector) | Result | Notes |
|---|---|---|---|
| Throughput (Download) | `N/A` | `N/A` | No VPS endpoint tested |
| Throughput (Upload) | `N/A` | `N/A` | No VPS endpoint tested |
| Latency (UDP Echo) | `N/A` | `N/A` | No VPS endpoint tested |
| Bufferbloat | `N/A` | `N/A` | No VPS endpoint tested |

### 11.3 WAN Benchmarking (Public / No-Reflector)
*Scenario: Fallback to public infrastructure when no self-hosted endpoint is available.*
| Method | Provider | Download | Upload | Ping | Notes |
|---|---|---|---|---|---|
| HTTP/TCP Speed Test | Ookla/NDT7 | `Pass` | `Pass` | `Pass` | Simulation |
| Latency (ICMP) | 8.8.8.8 | N/A | N/A | `16ms` | Stable |
| Trace | 8.8.8.8 | Path | N/A | `16ms` | Stable |

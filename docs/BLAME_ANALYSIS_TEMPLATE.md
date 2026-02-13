<!--
PacketParamedic — Blame Analysis Template
Goal: produce an ISP-ticket-grade, reproducible verdict with evidence and falsification notes.
Scope sources: probes (ICMP/TCP/DNS/HTTP), throughput (iperf3 + providers), path tracing/MTR, self-test (power/thermal/storage/NIC/Wi-Fi), scheduler windows/budgets, SQLite event history, incidents/anomaly detection, blame classifier output history.
-->

# Blame Analysis — <INCIDENT OR REPORT NAME>

## TL;DR (the verdict)
| Field | Value |
|---|---|
| Verdict | `<me | isp | service | insufficient-data>` |
| Confidence | `<0–100%>` |
| Customer impact | `<brief: buffering / drops / slow / intermittent / latency spikes>` |
| Time window analyzed (local) | `<YYYY-MM-DD HH:MM>` → `<YYYY-MM-DD HH:MM>` |
| Evidence bundle | `<path or link to exported bundle>` |
| Next action | `<what to do next, in one sentence>` |

---

## 1) Context & constraints
| Field | Value |
|---|---|
| Device | `PacketParamedic on Raspberry Pi 5` |
| Install mode | `<systemd service | manual run | container>` |
| Version | `<semver>` |
| Git commit | `<sha>` |
| Config profile | `<simple troubleshooting | reliability & uptime | high performance>` |
| Data retention policy | `<rows/time cap if configured>` |
| Privacy posture | `Local-first; no cloud required; redact before sharing evidence` |

Notes (keep concise):  
`<1–3 sentences about the user scenario and why this run was triggered>`

---

## 2) Timeline (what happened)
| Timestamp (local) | Event | Signal | Observed value | Expected/baseline | Notes |
|---|---|---|---:|---:|---|
| `<t0>` | `<symptom>` | `<loss/latency/dns/http/throughput>` | `<x>` | `<y>` | `<...>` |
| `<t1>` | `<change>` | `<route/dns/power/thermal>` | `<x>` | `<y>` | `<...>` |

---

## 3) Network environment snapshot
| Category | Field | Value |
|---|---|---|
| Topology | Gateway IP | `<e.g., 192.168.1.1>` |
| Topology | WAN link type | `<cable / fiber / DSL / cellular / other>` |
| DNS | Resolver(s) in use | `<IP(s) or provider>` |
| Interfaces | Active interface | `<eth0 / pcie-nic / wlan0>` |
| Interfaces | Negotiated link speed | `<100M/1G/2.5G/10G>` |
| IPv4/IPv6 | IPv6 status | `<working / broken / not used>` |
| Wi‑Fi (if applicable) | AP / band / RSSI | `<...>` |
| Segmentation | VLANs / mesh / extenders | `<...>` |
| 
| **System Fundamentals** | | |
| NTP Status | Synchronized? | `<yes/no (service active?)>` |
| DNS Configuration | Resolvers | `<nameserver IPs>` |
| Kernel Version | `uname -r` | `<version>` |
| Uptime | `uptime` | `<duration>` |

---

## 4) Hardware & self-test gating (trustworthiness of measurements)
| Check | Result | Evidence | Why it matters |
|---|---|---|---|
| Under‑voltage / brownout | `<pass/fail/unknown>` | `<self-test snippet>` | Low power causes fake “network” failures. |
| Thermal throttling | `<pass/fail/unknown>` | `<self-test snippet>` | Throttle skews throughput, latency, and timing. |
| Storage type/health | `<NVMe / microSD>` | `<self-test snippet>` | Evidence logging reliability; WAL write pressure. |
| NIC driver/link | `<pass/fail/unknown>` | `<self-test snippet>` | Mis-negotiation can cap speeds and cause drops. |
| PCIe link (if used) | `<width/speed>` | `<self-test snippet>` | Real throughput ceiling and stability. |
| System clock/NTP health | `<pass/fail/unknown>` | `<ntp check snippet>` | Bad time breaks timeline correlation. |

If any “fail”:  
Define whether this report is **invalid** (measurement integrity compromised) or **valid with caveats** (and list the caveats).

---

## 5) Measurements collected (raw signals)
### 5.1 Probes (availability + latency)
| Probe | Target class | Target | Success rate | Latency p50 | Latency p95 | Notes |
|---|---|---|---:|---:|---:|---|
| ICMP | Gateway | `<router>` | `<%>` | `<ms>` | `<ms>` | `<...>` |
| ICMP | WAN | `<ISP hop / public>` | `<%>` | `<ms>` | `<ms>` | `<...>` |
| DNS | Resolver | `<resolver>` | `<%>` | `<ms>` | `<ms>` | `<...>` |
| TCP | Service | `<host:port>` | `<%>` | `<ms>` | `<ms>` | `<...>` |
| HTTP | Service | `<url>` | `<%>` | `<ms>` | `<ms>` | `<status codes>` |

### 5.2 Throughput (speed tests)
| Provider / method | Mode | Download | Upload | Latency | Jitter | Notes |
|---|---|---:|---:|---:|---:|---|
| iperf3 | `<LAN/WAN>` | `<Mbps>` | `<Mbps>` | `<ms>` | `<ms>` | `<streams, duration>` |
| Ookla | WAN | `<Mbps>` | `<Mbps>` | `<ms>` | `<ms>` | `<server, id>` |
| NDT7 | WAN | `<Mbps>` | `<Mbps>` | `<ms>` | `<ms>` | `<M-Lab details>` |
| Fast.com | WAN | `<Mbps>` | `<Mbps>` | `<ms>` | `<ms>` | `<“Netflix experience”>` |

### 5.3 Path tracing / change detection
| Target | Tool | Path changed? | Where changed | Correlated with incident? | Notes |
|---|---|---|---|---|---|
| `<target>` | `<traceroute/mtr>` | `<yes/no/unknown>` | `<hop # / ASN / hostname>` | `<yes/no>` | `<...>` |

---

## 6) Scheduler & test coordination (measurement validity)
| Field | Value |
|---|---|
| Schedules active | `<names / count>` |
| Throughput mutual exclusion | `<enabled/disabled/unknown>` |
| Test windows (off-peak) | `<cron/time window>` |
| Bandwidth budget | `<daily cap or policy>` |
| Overlap during incident? | `<yes/no/unknown>` |
| User-triggered tests | `<what was run manually>` |

If a heavy throughput test overlapped with user traffic, state whether results are **representative** or **contaminated by load**.

---

## 7) Feature aggregation (what the model actually “saw”)
This section should mirror your feature aggregator output and make it auditable.

| Feature group (concept) | Example features to record | Observed | Baseline | Delta | Reliability |
|---|---|---:|---:|---:|---|
| Gateway health | RTT, loss | `<...>` | `<...>` | `<...>` | `<high/med/low>` |
| WAN health | RTT, loss | `<...>` | `<...>` | `<...>` | `<high/med/low>` |
| DNS health | latency, fail rate | `<...>` | `<...>` | `<...>` | `<high/med/low>` |
| TCP/HTTP health | connect fail rate, status anomalies, TTFB | `<...>` | `<...>` | `<...>` | `<high/med/low>` |
| Throughput | down/up, jitter, latency-under-load (if measured) | `<...>` | `<...>` | `<...>` | `<high/med/low>` |
| Path signals | route diff, hop instability | `<...>` | `<...>` | `<...>` | `<high/med/low>` |
| Local integrity | power/thermal/storage flags | `<...>` | `<...>` | `<...>` | `<high/med/low>` |

If the repo claims “N features” but you’re not collecting them yet, list the missing ones explicitly and mark this verdict as “provisional.”

---

## 8) Model inference (blame classifier)
| Field | Value |
|---|---|
| Model artifact | `<e.g., blame_lr.json>` |
| Training regime | `<synthetic patterns | real labeled data | mixed>` |
| Classes | `me / isp / service` |
| Output probabilities | `me=<p> isp=<p> service=<p>` |
| Final verdict | `<class>` |
| Confidence policy | `<thresholds / tie-break>` |
| Insufficient-data policy | `<what triggers it>` |

Correctness checks (required):
| Check | Result | Notes |
|---|---|---|
| Scalar vs accelerated parity | `<pass/fail/not tested>` | Acceleration must not change correctness. |
| Empty-history behavior | `<pass/fail>` | Must not NaN/panic; should return insufficient-data. |
| “Bad ISP” scenario resemblance | `<yes/no>` | Compare to known loss/jitter patterns. |
| “Flaky Wi‑Fi / local” scenario resemblance | `<yes/no>` | Compare to gateway vs WAN divergence. |

---

## 9) Evidence-based reasoning (support + falsification)
### 9.1 Why this verdict makes sense
| Claim | Supporting evidence | Strength |
|---|---|---|
| `<e.g., ISP issue>` | `<WAN loss high while gateway stable; throughput collapse across providers; path change>` | `<high/med/low>` |

### 9.2 What would falsify it
| Alternative hypothesis | What you’d expect to see | Do we see it? | Notes |
|---|---|---|---|
| Local Wi‑Fi / LAN issue | Gateway jitter/loss, RF degradation, local interface resets | `<yes/no/unknown>` | `<...>` |
| Remote service issue | Only one service failing; DNS/TCP/HTTP anomalies scoped | `<yes/no/unknown>` | `<...>` |
| Device measurement bias | Under‑voltage/throttle, clock skew, storage stalls | `<yes/no/unknown>` | `<...>` |

---

## 10) Recommended next tests (to increase certainty)
| Goal | Test name | Risk | What it answers |
|---|---|---|---|
| Reduce ambiguity | `<blame-check>` | `<low>` | Recompute verdict with fresh window. |
| Isolate LAN vs WAN | `<LAN iperf3>` | `<medium>` | Does local network sustain expected rates? |
| Isolate ISP path | `<mtr/traceroute>` | `<low>` | Which hop introduces jitter/loss? |
| Stress under load | `<latency-under-load>` | `<medium>` | Bufferbloat / congestion signature. |

If you need CLI snippets, put them here (do not paste secrets):
```bash
# <example placeholders>
# packetparamedic blame-check
# packetparamedic trace --target 8.8.8.8
# packetparamedic speed-test --provider ndt7
```

---

---
## 11) LAN vs WAN Isolation (Self-Hosted Model)

### 11.1 Local Benchmarking (Reflector on LAN/Loopback)
*Scenario: Verifying local interface/driver/stack health (elimination of WAN variability).*
| Metric | Target (Reflector) | Result | Pass/Fail Criteria |
|---|---|---|---|
| Throughput (Download) | `localhost:4000` | `<Mbps>` | Should match interface speed (e.g., >20Gbps on Loopback, >900Mbps on 1G LAN) |
| Throughput (Upload) | `localhost:4000` | `<Mbps>` | Should match interface speed |
| Jitter (UDP Echo) | `localhost:4000` | `<ms>` | Should be near zero (< 0.5ms) |
| Loss (UDP Echo) | `localhost:4000` | `<%>` | MUST be 0% |
| CPU Load (Client) | Self | `<%>` | Verify CPU isn't bottlenecking (core 2/3 usage) |

### 11.2 WAN Benchmarking (Reflector on Dedicated VM)
*Scenario: Precision WAN testing using a self-hosted endpoint on a VPS (controlled path).*
| Metric | Target (Reflector) | Result | Notes |
|---|---|---|---|
| Throughput (Download) | `irww.alpina` | `<Mbps>` | ISP limit check (single stream vs multi-stream) |
| Throughput (Upload) | `irww.alpina` | `<Mbps>` | ISP limit check (single stream vs multi-stream) |
| Latency (UDP Echo) | `irww.alpina` | `<ms>` | True application RTT (no ICMP de-prioritization) |
| Bufferbloat | `irww.alpina` | `<Grade>` | Latency delta under load |

### 11.3 WAN Benchmarking (Public Infrastructure)
*Scenario: Fallback when self-hosted endpoint is unreachable or for comparison.*
| Method | Provider | Download | Upload | Ping | Notes |
|---|---|---|---|---|---|
| HTTP/TCP Speed Test | Ookla/NDT7 | `<Mbps>` | `<Mbps>` | `<ms>` | Variable server distance/congestion |
| Latency (ICMP) | 8.8.8.8 | N/A | N/A | `<ms>` | ICMP may be rate-limited |
| Trace | Google/Cloudflare | Path | N/A | `<ms>` | Identifying bad hops via MTR |

---

## 12) NAT Environment Impact (CGNAT / Double NAT)

### 12.1 CGNAT Detection
*Indicator: WAN IP on router interface is in 100.64.0.0/10 range, but public IP check shows different address.*
| Check | Observation | Value | Implication |
|---|---|---|---|
| WAN IP (Router Interface) | `<IP Address>` | `<100.x.y.z?>` | If private/CGNAT range, inbound connections (reflector pairing) may fail without relay. |
| Public IP (STUN/HTTP) | `<IP Address>` | `<Public IP>` | Mismatch confirms NAT. |
| Traceroute Hops | Hoops 1-3 | `<IPs>` | Multiple private hops indicate ISP NAT layers. |

### 12.2 Double NAT Symptoms
*Scenario: User router behind ISP modem/router (bridge mode disabled).*
| Symptom | Test | Result |
|---|---|---|
| Traceroute | Hop 1 & 2 | `<IPs>` | If both are private (e.g., 192.168.1.1 then 192.168.0.1), Double NAT exists. |
| UPnP / PCP | Discovery | `<Success/Fail>` | Port mapping likely fails. |
| Peer-to-Peer | Reflector Pairing | `<Success/Fail>` | Direct inbound connection blocked. |

---

## 12) Evidence Bundle (Appendix)
> Eventually this will be a link, but for now we append the full evidence JSON bundle here.

```json
<INSERT_FULL_JSON_EVIDENCE_BUNDLE_HERE>
```

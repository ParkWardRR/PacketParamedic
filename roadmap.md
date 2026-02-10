# PacketParamedic -- Roadmap

> **License:** [Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0) (SPDX: `BlueOak-1.0.0`)
>
> **Target hardware: Raspberry Pi 5 only.** No backward compatibility with Pi 4 or earlier. Forward-looking, no legacy.

---

## Tech Stack

| Layer | Pick | Why |
|---|---|---|
| Host OS | Raspberry Pi OS Lite (Bookworm, 64-bit, Pi 5 only) | Native Pi 5 support; Cortex-A76 + VideoCore VII + PCIe |
| Init/Services | systemd units + tmpfiles.d + journald | Appliance-grade supervision + structured logs |
| API | axum + tokio + tower | Lightweight modern async stack; ergonomic middleware |
| Storage | SQLite (local-first) | Lowest ops burden for an appliance event store |
| Observability | tracing + tracing-journald | Structured logs straight into journald |
| Web UI | Server-rendered HTML + htmx (+ tiny JS) | Lightest "fast UI" approach; no SPA build pipeline |
| UI (local HDMI) | Wayland + labwc (optional) | Lightweight compositor for on-device desktop UI |
| Throughput | iperf3 (wrapped) + native Rust fallback | Industry-standard for 10GbE line-rate testing; Rust fallback when iperf3 unavailable |
| 10GbE | PCIe M.2 HAT NIC | Pi 5 native PCIe -- true 10GbE without USB bottlenecks |
| Scheduling | Tokio in-process cron engine | Unified scheduler for all probes and tests; bandwidth-aware coordination |
| BLE (server) | BlueZ + bluer | Pi 5 has built-in Bluetooth 5.0 / BLE; bluer is the official Rust interface |
| BLE (iOS client) | Swift + Core Bluetooth | Native iOS companion app; Web Bluetooth unavailable on iOS |
| BLE (Android/Desktop client) | Web Bluetooth API | Chrome/Edge; no native app needed for provisioning |
| Remote admin | Tailscale (optional) | No inbound ports; zero-trust appliance management |

---

## Phase 0: Project Definition (1--3 days)

### Goals
- Appliance-grade: unattended operation, safe updates, observable, supportable.
- Answers "Wi-Fi vs router vs ISP?" with evidence and a timeline.
- Manageable: local UI + BLE nearby admin + secure remote access (Tailscale) + optional OOB (cellular).

### Non-Goals
- No "always-on monitor/injection" unless the user explicitly enables it.
- No cloud dependency required for core diagnostics.
- **No support for Raspberry Pi 4 or earlier. Pi 5 only -- forward-looking, no legacy.**

### Checklist
- [ ] Create repo scaffolding (workspace layout, Cargo.toml, CI config)
- [ ] Define versioning scheme and release channels (stable / beta / nightly)
- [ ] Write security posture doc (defaults, ports, auth model)
- [ ] Document Pi 5-only hardware requirement in all user-facing docs
- [ ] Verify: one command builds a runnable dev version + produces a versioned artifact

---

## Phase 1: Backend Foundation (Week 1--2)

### 1.1 Base OS Image & Services
- [ ] Set up reproducible image build pipeline (Pi 5 target only)
- [ ] Design systemd unit layout (measurement, API, updater, OOB daemons as separate services)
- [ ] Configure log retention caps + disk space guardrails
- [ ] Implement NTP health check + "clock skew detected" alarms

### 1.2 Storage Reliability
- [ ] Prefer NVMe SSD via Pi 5 PCIe; document microSD mitigation plan (read-only root optional)
- [ ] Implement crash-safe spool/queue for measurements to avoid data loss

### Acceptance
- [ ] Survives 50+ power cuts; reboots cleanly; doesn't fill disk in 7-day soak

---

## Phase 2: Hardware Self-Test (Week 2--4)

### 2.1 Hardware Inventory & Capability Probing
- [ ] Verify Pi 5 board (Cortex-A76 quad-core, 4/8 GB RAM)
- [ ] Confirm CPU SIMD: Arm NEON / ASIMD (guaranteed on Cortex-A76)
- [ ] Detect GPU: Pi 5 VideoCore VII (Vulkan 1.2 via V3DV, OpenGL ES 3.1 via Mesa V3D)
- [ ] Detect storage type and health (NVMe via PCIe preferred, microSD fallback)
- [ ] Output results as machine-readable JSON for support bundles
- [ ] Expose hardware inventory via API

### 2.2 Wi-Fi Hardware Self-Test
- [ ] Enumerate Wi-Fi interfaces and driver stack (mac80211 vs vendor/out-of-tree)
- [ ] Check monitor mode support + radiotap quality via short capture sanity test
- [ ] Check injection capability only under explicit "RF test mode"
- [ ] Recommend hardware profile if capabilities are missing (Profile A: monitor/capture dongle; Profile B: dual-radio)

### 2.3 Thermal & Power Integrity
- [ ] Detect CPU/GPU throttling under load (frequency drops)
- [ ] Confirm PSU stability (brownout flags) and USB bus stability
- [ ] Validate Pi 5 active cooler presence and fan operation

### 2.4 Network Interface & 10GbE Detection
- [ ] Enumerate all network interfaces (onboard 1GbE, PCIe NICs via M.2 HAT)
- [ ] Detect 10GbE-capable PCIe NIC and validate driver status
- [ ] Report PCIe lane width and negotiated link speed
- [ ] Validate negotiated vs advertised link speed (ethtool)
- [ ] Warn if thermal limits may constrain sustained 10GbE throughput

### Acceptance
- [ ] One-call "Run Self-Test" (API/CLI) produces pass/fail report + remediation steps
- [ ] API/CLI indicates "single-radio constraints" vs "dual-radio available"
- [ ] Self-test flags invalid results due to throttling or power instability
- [ ] Self-test identifies 10GbE PCIe NIC and reports maximum achievable throughput

---

## Phase 3: Acceleration Plumbing (Week 3--5)

### 3.1 Acceleration Policy Layer
- [ ] Create internal "Acceleration Manager" abstraction
- [ ] Implement NEON-optimized codepaths (Cortex-A76 ASIMD guaranteed)
- [ ] Record which acceleration path was used (for supportability)

### 3.2 GPU Support Baseline
- [ ] Target OpenGL ES 3.1 via Pi 5 VideoCore VII (Mesa V3D)
- [ ] Target Vulkan 1.2 via V3DV for compute workloads where beneficial
- [ ] CPU reference implementation for all GPU-accelerated paths

### Acceptance
- [ ] Benchmark harness shows acceleration used when available, clean fallback when not

---

## Phase 4: Data Layer & Evidence Artifacts (Week 4--6)

### 4.1 Unified Event Schema & TSDB
- [ ] Design canonical schema (probe results, incidents, path changes, Wi-Fi telemetry, throughput results, self-test outputs)
- [ ] Implement retention policy + "export support bundle" (logs + config + redacted network facts)

### 4.2 Evidence-First Artifacts
- [ ] Per incident: attach "why we think this is ISP/router/Wi-Fi" with raw metrics

### Acceptance
- [ ] "Export ISP ticket bundle" (API/CLI) produces a shareable report with timeline + key metrics

---

## Phase 5: Core Measurement MVP (Week 6--9)

### 5.1 Availability & Latency Probes
- [ ] Implement ICMP probe with scheduling and target sets
- [ ] Implement HTTP probe with evidence outputs
- [ ] Implement DNS probe (resolution timing, resolver identification)
- [ ] Implement TCP probe (connection timing, port reachability)

### 5.2 Blame Check (LAN vs WAN)
- [ ] Gateway health checks (LAN RTT/loss, DNS resolution path)
- [ ] WAN reachability + IPv4/IPv6 parity checks
- [ ] DNS resolver visibility ("which resolver am I using?" + change detection)

### Acceptance
- [ ] One-call "Is it me or my ISP?" (API/CLI) yields a reproducible verdict with confidence and evidence

---

## Phase 6: Performance, Throughput & Quality (Week 9--12)

### 6.1 Speed Testing Framework
- [ ] Multi-provider speed testing (scheduled + on-demand)
- [ ] iperf3 wrapper for high-throughput testing (LAN and WAN, `--json` output parsing)
- [ ] Native Rust throughput engine as fallback (no iperf3 dependency required)
- [ ] Auto-detect maximum link speed and scale test parameters accordingly
- [ ] Support 1GbE (onboard) and 10GbE (PCIe) link speeds

### 6.2 10GbE LAN Stress Testing
- [ ] LAN peer discovery for iperf3 server/client pairing
- [ ] Sustained throughput test (TCP: 30s, 60s, 300s configurable windows)
- [ ] UDP flood test with configurable bandwidth targets and loss thresholds
- [ ] Bidirectional (full-duplex) throughput measurement
- [ ] Multi-stream testing (1, 4, 8, 16 parallel TCP streams)
- [ ] MTU / jumbo frame validation (1500 vs 9000 byte)
- [ ] CPU utilization tracking during throughput tests (detect Pi bottleneck vs network bottleneck)
- [ ] Thermal monitoring during sustained 10GbE tests (auto-abort on throttle)

### 6.3 10GbE WAN Bandwidth Testing
- [ ] WAN throughput measurement to configurable remote iperf3 endpoints
- [ ] Integration with public speed test infrastructure (Ookla, Cloudflare, M-Lab)
- [ ] 10Gbps-capable test server targeting (filter servers by capacity)
- [ ] Multi-connection WAN tests to saturate high-bandwidth links
- [ ] Upload + download + bidirectional WAN throughput
- [ ] WAN baseline tracking (daily/weekly trend with 10GbE resolution)
- [ ] ISP speed tier validation ("Am I getting the 10Gbps I pay for?")

### 6.4 Quality Metrics
- [ ] Jitter + loss tracking
- [ ] Bufferbloat / latency-under-load grading (including at 10GbE rates)
- [ ] "Consistent testing" mode (daily/weekly baselines)

### Acceptance
- [ ] Can distinguish: "bandwidth OK, latency/jitter bad" vs "true throughput issue"
- [ ] 10GbE LAN stress test saturates PCIe link and reports CPU vs network bottleneck
- [ ] WAN bandwidth test correctly measures throughput up to 10Gbps when hardware supports it

---

## Phase 6.5: Scheduling Engine (Week 11--14)

### 6.5.1 Core Scheduler
- [ ] Cron-like recurring schedule engine (second/minute/hour/day/week granularity)
- [ ] One-shot (ad-hoc) test triggers via API and CLI
- [ ] Named schedule profiles (e.g., "daily-baseline", "hourly-quick-check", "weekly-stress-test")
- [ ] Schedule persistence in SQLite (survives reboots)
- [ ] Timezone-aware scheduling with UTC normalization
- [ ] Jitter/randomization option to avoid thundering herd on shared infrastructure

### 6.5.2 Bandwidth-Aware Coordination
- [ ] Mutual exclusion: never run two throughput tests simultaneously
- [ ] Priority queue: blame-check > scheduled probes > speed tests > stress tests
- [ ] Test windows: restrict bandwidth-heavy tests to defined time ranges (e.g., 02:00--05:00)
- [ ] Preemption: user-triggered tests preempt scheduled background tests
- [ ] Resource budget: limit total daily/weekly bandwidth consumed by WAN testing

### 6.5.3 Schedule Management API
- [ ] CRUD operations for schedules (create, read, update, delete, enable/disable)
- [ ] Schedule dry-run: "show me what would run in the next 24h"
- [ ] Execution history: log every scheduled run with result summary
- [ ] Missed-run detection: if the device was off during a scheduled window, log it and optionally catch up
- [ ] Webhook/notification on schedule completion (optional)

### 6.5.4 Default Schedules (Out-of-Box)
- [ ] ICMP gateway probe: every 60 seconds
- [ ] DNS resolver check: every 5 minutes
- [ ] HTTP reachability: every 5 minutes
- [ ] Speed test (light): daily at a randomized off-peak hour
- [ ] Full blame check: weekly
- [ ] All defaults user-overridable and disable-able

### Acceptance
- [ ] Scheduler runs unattended for 14+ days without drift, missed runs, or resource conflicts
- [ ] Two throughput-heavy tests never overlap
- [ ] User can define, modify, and disable schedules via API, CLI, and UI

---

## Phase 7: Path Tracing & Change Detection (Week 14--16)

- [ ] Traceroute/MTR sampling with safe rate limits
- [ ] Path diffing and correlation to incidents
- [ ] Local hop tracing (AP/mesh/router/VLAN traversal where visible)

### Acceptance
- [ ] Timeline data shows "path changed at X" and correlates with user impact

---

## Phase 8: Incidents & Anomaly Detection (Week 16--19)

- [ ] Statistical anomaly detection (latency/loss/jitter/throughput deviations from baseline)
- [ ] Incident grouping, severity, and "what changed" diffs (DNS/route/Wi-Fi/CPU-throttle/power/throughput flags)
- [ ] Human-readable diagnostic verdict (rule-based first; AI optional later)

### Acceptance
- [ ] Incidents are emitted with recommended next tests + clear rationale

---

## Phase 9: Test Phase -- Gates Before UX/UI (Week 19--21)

- [ ] Unit tests: schema validation, probe scheduling, verdict rules, redaction routines, throughput parsing
- [ ] Integration tests: real network scenarios (LAN-only, DNS failure, captive portal, IPv6 broken, bufferbloat, packet loss, 10GbE throughput)
- [ ] Soak tests: 7--14 day continuous run, disk fill tests, power cycle tests, upgrade/rollback drills
- [ ] Security tests: authN/authZ, least-privilege services, port exposure audit, supply-chain scanning

### Acceptance
- [ ] "No known critical bugs" gate passed
- [ ] Reproducible results on Pi 5 hardware
- [ ] Upgrade/rollback proven

---

## Phase 10: UX/UI (Week 22--25)

- [ ] Onboarding flow + health status dashboard
- [ ] "Run Self-Test" workflow and results viewer
- [ ] "Is it me or my ISP?" guided run and evidence view
- [ ] Speed test / throughput results dashboard with historical trending
- [ ] Schedule management UI (create, edit, enable/disable, view history)
- [ ] Incident timeline + export bundle UI
- [ ] Mobile-first responsive layout
- [ ] Global search (Ctrl+K) UI feature

### Acceptance
- [ ] Non-technical user can run blame check, view incidents, manage schedules, and export a report without reading docs

---

## Phase 11: Secure Remote Access -- Tailscale (Week 24--27)

- [ ] Optional Tailscale install/enable during onboarding or later
- [ ] Device identity naming + tags (for ACLs)
- [ ] Expose only necessary services over tailnet (principle of least privilege)

### Acceptance
- [ ] Admin appliance remotely without opening inbound WAN ports; access revocable instantly

---

## Phase 12: BLE Nearby Admin, Client Apps & OOB Cellular (Week 27--36)

### 12.1 BLE GATT Service (Pi 5 built-in Bluetooth 5.0)
- [ ] Secure pairing + provisioning (Wi-Fi creds, admin token, enable Tailscale)
- [ ] Recovery actions: reboot, factory reset trigger, export support bundle via BLE
- [ ] BLE GATT service for status queries (health, last incident, uptime) without opening the web UI
- [ ] Auto-discoverable via BLE advertisement when in provisioning mode
- [ ] Define stable GATT service/characteristic UUIDs and document the BLE protocol

### 12.2 iOS Companion App (Core Bluetooth + Swift)
> Web Bluetooth is not available in iOS Safari or any iOS browser (all use WebKit). A native companion app is the only option for iPhone/iPad users.

- [ ] Xcode project scaffolding (`ios/` directory in repo, SwiftUI + Core Bluetooth)
- [ ] BLE scanning and secure pairing with PacketParamedic GATT service
- [ ] Wi-Fi provisioning flow (enter SSID/passphrase, send over BLE)
- [ ] Admin token exchange and storage in iOS Keychain
- [ ] Status dashboard: health, uptime, last incident, last speed-test result (read via GATT characteristics)
- [ ] Recovery actions: reboot, factory reset trigger via BLE
- [ ] Support bundle export trigger via BLE (download over local HTTP once Wi-Fi is up)
- [ ] iOS 16+ deployment target (minimum for modern Core Bluetooth APIs)
- [ ] App Store distribution (or TestFlight for beta)

### 12.3 Android & Desktop BLE via Web Bluetooth
> Android Chrome and desktop Chrome/Edge support the Web Bluetooth API. No native app required.

- [ ] Web Bluetooth integration in the htmx Web UI (progressive enhancement)
- [ ] BLE device scanning and pairing from the browser
- [ ] Wi-Fi provisioning flow via Web Bluetooth (same GATT protocol as iOS companion)
- [ ] Status read and recovery actions via Web Bluetooth
- [ ] HTTPS / secure context requirement documented (Web Bluetooth requires it)
- [ ] Tested on: Android Chrome, macOS Chrome, Windows Chrome/Edge, Linux Chrome
- [ ] Graceful fallback: if Web Bluetooth is unavailable (e.g., Firefox, older browsers), show instructions to use a supported browser or the iOS companion app

### 12.4 Cellular Management Plane (optional)
- [ ] Outbound-only management tunnel policy
- [ ] Data budget controls + emergency-only mode
- [ ] Strong separation: management traffic vs measurement traffic

### BLE Client Platform Matrix

| Platform | App required? | Path | Notes |
|---|---|---|---|
| iOS | Yes | Native companion (Core Bluetooth + Swift) | Web Bluetooth not available in iOS Safari/PWAs |
| Android | No | Web UI + Web Bluetooth in Chrome | Permissions UX can be finicky; workable for provisioning |
| Desktop (macOS/Windows/Linux) | No | Web UI + Web Bluetooth in Chrome/Edge | Requires HTTPS/secure context; BLE stacks vary by OS |

### Acceptance
- [ ] BLE GATT provisioning works out of box on Pi 5 with no additional hardware
- [ ] iOS companion app pairs, provisions Wi-Fi, reads status, and triggers recovery via BLE
- [ ] Android Chrome and desktop Chrome can provision and query status via Web Bluetooth
- [ ] If Web Bluetooth is unavailable, the UI guides the user to a supported path
- [ ] If primary WAN dies, unit is reachable via BLE (always) or cellular (if enabled) without blowing data caps

---

## Phase 13: Advanced Diagnostics (Week 34+)

- [ ] RF/monitor mode capture workflows (explicit opt-in only)
- [ ] QoS detection heuristics
- [ ] Stress test orchestrator: multi-protocol sustained load tests with strict safety limits
- [ ] 10GbE endurance testing: 1-hour and 24-hour sustained throughput runs
- [ ] Combined stress: simultaneous throughput + latency + jitter measurement under load
- [ ] Stress test safety: auto-abort on thermal throttle, power instability, or disk pressure
- [ ] Stress test reports: pass/fail with detailed timeline of throughput, CPU, thermal, and error metrics
- [ ] AI diagnostics assistant (local-first, privacy-preserving, optional module)
- [ ] Device identification / IP-to-device hints (hostname, MAC vendor, OS guess)
- [ ] Anomalous protocol detection (IPX, unusual L2 frames) as a specific detector

### Acceptance
- [ ] Advanced tools never run by default; always display risks + required permissions

---

## Release Gates (Apply to Every Phase)

- [ ] **Security:** No default passwords; minimal open ports; auditable actions
- [ ] **Reliability:** Soak tests, disk fill tests, power cycle tests pass
- [ ] **Supportability:** Support bundle always works; self-test always runnable
- [ ] **Performance:** Acceleration policy never breaks correctness; clean fallback paths
- [ ] **Scheduling:** Default schedules produce correct results; no test collisions in 7-day soak
- [ ] **Pi 5 only:** No codepaths for Pi 4 or earlier; all testing on Pi 5 hardware

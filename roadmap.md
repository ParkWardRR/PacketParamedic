# PacketParamedic -- Roadmap

> **License:** [Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0) (SPDX: `BlueOak-1.0.0`)
>
> **Target hardware: Raspberry Pi 5 only.** No backward compatibility with Pi 4 or earlier. Forward-looking, no legacy.

> **Build order mantra:** Thoroughly build EVERY function backend first. Then build the front end. Then do front end optimization. Then do back end optimization. No skipping ahead.

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
- [x] Create repo scaffolding (workspace layout, Cargo.toml, CI config)
- [x] Define versioning scheme and release channels (stable / beta / nightly)
- [x] Write security posture doc (defaults, ports, auth model)
- [x] Document Pi 5-only hardware requirement in all user-facing docs
- [ ] Verify: one command builds a runnable dev version + produces a versioned artifact

---

## Phase 1: Backend Foundation (Week 1--2)

### 1.1 Base OS Image & Services
- [x] Set up reproducible image build pipeline (Pi 5 target only)
- [x] Design systemd unit layout (measurement, API, updater, OOB daemons as separate services)
- [x] Configure log retention caps + disk space guardrails (journald + `src/system/disk.rs`)
- [x] Implement NTP health check + "clock skew detected" alarms (`src/system/ntp.rs`)

### 1.2 Storage Reliability
- [x] Configure SQLite for WAL mode + foreign keys (checked `src/storage/mod.rs`)
- [x] Implement `measurements` and `spool` tables with acceleration metadata columns
- [x] Implement crash-safe spool: write results to `spool` immediately, aggregate to `measurements` later
- [x] Store execution metadata: `backend_used` (vk/gles/neon/scalar) and `duration_us` for every op

### Acceptance
- [ ] Survives 50+ power cuts; reboots cleanly; doesn't fill disk in 7-day soak

---

## Phase 2: Hardware Self-Test (Week 2--4)

### 2.1 Hardware Inventory & Capability Probing
- [x] Verify Pi 5 board (Cortex-A76 quad-core, 4/8 GB RAM)
- [x] Confirm CPU SIMD: Arm NEON / ASIMD (guaranteed on Cortex-A76)
- [x] Detect GPU: Pi 5 VideoCore VII (Vulkan 1.2 via V3DV, OpenGL ES 3.1 via Mesa V3D)
- [x] Detect storage type and health (NVMe via PCIe preferred, microSD fallback)
- [x] Output results as machine-readable JSON for support bundles
- [x] Expose hardware inventory via API

### 2.2 Wi-Fi Hardware Self-Test
- [ ] Enumerate Wi-Fi interfaces and driver stack (mac80211 vs vendor/out-of-tree)
- [ ] Check monitor mode support + radiotap quality via short capture sanity test
- [ ] Check injection capability only under explicit "RF test mode"
- [ ] Recommend hardware profile if capabilities are missing (Profile A: monitor/capture dongle; Profile B: dual-radio)

### 2.3 Thermal & Power Integrity
- [x] Detect CPU/GPU throttling under load (frequency drops)
- [x] Confirm PSU stability (brownout flags) and USB bus stability
- [x] Validate Pi 5 active cooler presence and fan operation

### 2.4 Network Interface & Multi-Gig Detection
- [x] Enumerate all network interfaces (onboard 1GbE, PCIe NICs via M.2 HAT)
- [x] Detect 2.5GbE/Multi-Gig PCIe NIC and validate driver status
- [x] Report PCIe lane width and negotiated link speed
- [x] Validate negotiated vs advertised link speed (ethtool)
- [x] Warn if legacy cabling limits speeds (e.g. 100Mbps on 1GbE link)

### Acceptance
- [ ] One-call "Run Self-Test" (API/CLI) produces pass/fail report + remediation steps
- [ ] API/CLI indicates "single-radio constraints" vs "dual-radio available"
- [ ] Self-test flags invalid results due to throttling or power instability
- [ ] Self-test identifies 10GbE PCIe NIC and reports maximum achievable throughput

---

## Phase 3: MANDATORY Acceleration Implementation (Week 3--6)

> **CRITICAL:** "Overuse is Non-Negotiable". Every backend task must be implemented for Vulkan, GLES, and NEON. This is not optional.

### 3.1 Acceleration Policy Layer
- [x] Create internal `AccelerationManager` (Runtime detection, dispatch logic) - `src/accel/manager.rs`
- [x] Define trait `AcceleratedOp` (inputs -> outputs) with `vk`, `gles`, `neon`, `scalar` methods - `src/accel/ops.rs`
- [x] Implement backend selection heuristic (payload size vs. transfer overhead)
- [ ] Verification harness: random 0.1% sampling of accelerated results against scalar reference

### 3.2 NEON Backend (latency-sensitive)
- [x] Implement `neon_cpu` path for all statistical operations (mean, variance, percentiles)
- [x] Optimize critical hot loops using `std::arch::aarch64` intrinsics
- [ ] Benchmark NEON vs scalar (ensure >2x speedup on small batches)

### 3.3 OpenGL ES 3 Backend (render-pass compute)
- [x] Integrate `glow` and `glutin` for EGL context management (staged in `src/accel/gles.rs`)
- [ ] Initialize headless EGL context (Mesa V3D) - requires runtime verification
- [ ] Implement `gles3_computeish` path using fragment shaders + FBOs
- [ ] Map 2D grid tasks (heatmaps, pattern scanning) to render passes
- [ ] Buffer readback optimization (PBOs)

### 3.4 Vulkan Backend (heavy compute)
- [x] Integrate `ash` for Vulkan 1.2 bindings (staged in `src/accel/vulkan.rs`)
- [ ] Initialize Vulkan instance/device (V3DV) - requires runtime verification
- [ ] Implement `vk_compute` path using compute shaders (SPIR-V)
- [ ] Manage descriptor sets, pipeline barriers, and command buffers
- [ ] Benchmark large-batch throughput (target >10x vs NEON on huge datasets)

### Acceptance
- [ ] Benchmark harness shows distinct performance tiers: Vulkan > NEON > Scalar for large batches
- [ ] Graceful fallback: if GPU hangs, manager switches to NEON instantly
- [ ] All accelerated results match scalar reference exactly

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
- [ ] iperf3 wrapper for high-throughput testing
- [ ] Native Rust throughput engine as fallback
- [ ] Support specific speed limits: 250Mbps, 500Mbps, 750Mbps, 1Gbps, 2.5Gbps
- [ ] Auto-scale TCP window sizes for 2.5Gbps targets

### 6.2 2.5GbE LAN Stress Testing
- [ ] LAN peer discovery for iperf3 server/client pairing
- [ ] Sustained throughput test (TCP: 30s, 60s, 300s)
- [ ] UDP flood test with configurable bandwidth targets
- [ ] Verify modest hardware (Pi 5) can saturate 2.5Gbps line rate

### 6.3 WAN Bandwidth Testing (up to 2.5Gbps)
- [ ] WAN throughput measurement to configurable remote endpoints
- [ ] Tier validation: "Am I getting my 250/500/1000/2500 Mbps?"
- [ ] Link saturation tests for 1Gbps+ connections

### 6.4 Quality Metrics
- [ ] Jitter + loss tracking
- [ ] Bufferbloat / latency-under-load grading
- [ ] "Consistent testing" mode (daily/weekly baselines)

### Acceptance
- [ ] Can distinguish: "bandwidth OK, latency/jitter bad" vs "true throughput issue"
- [ ] LAN stress test consistently hits ~2.35Gbps on 2.5GbE hardware
- [ ] WAN bandwidth test validates ISP tiers 250Mbps through 2.5Gbps

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

---

## Phase 14: Future High-Performance (5GbE / 10GbE)
> Deferred to focus on mass-market 1Gbps--2.5Gbps tiers first.

- [ ] 10GbE PCIe NIC support (Aquantia/Intel driver validation on Pi 5)
- [ ] 5GbE / 10GbE throughput tuning (IRQ affinity, jumbo frames)
- [ ] Thermal management for sustained 10Gbps flows on Pi 5

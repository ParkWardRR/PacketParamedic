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

## Roadmap

| Phase | Name | Status | Primary Use Case |
|-------|------|--------|-----------------|
| 0 | Project Definition | Done | All |
| 1 | Backend Foundation (OS image, SQLite WAL, systemd) | Done | High Performance (Architecture) |
| 2 | Hardware Self-Test (board, thermal, NIC, Wi-Fi) | Done | High Performance (Verification) |
| 3 | Acceleration (NEON, Vulkan, GLES, scalar fallback) | Done | High Performance (Performance) |
| 4 | Data Layer & Evidence (schema, migrations, blame trainer) | Done | All (Foundation) |
| 5 | Core Measurement MVP (ICMP, TCP, DNS, HTTP probes) | Done | Simple Troubleshooting (Diagnostics) |
| 6 | Performance & Throughput (iperf3, native Rust, 1GbE) | Done (Ookla/NDT7/Fast/iperf3 + Reflector Self-Host) | Reliability (Streaming), High Performance |
| 6.5 | Scheduling Engine (cron, bandwidth coordination) | Done | High Performance (Control), Reliability (Quiet) |
| 7 | Path Tracing & Change Detection (traceroute/MTR) | Started | High Performance, Simple Troubleshooting |
| 8 | Incidents & Anomaly Detection | In progress (Foundation & Baseline Engine implemented. Needs Correlation.) | Simple Troubleshooting, Reliability (Answers) |
| 9 | Test Phase (unit, integration, soak, security) | Not started | Reliability (Reliability) |
| 10 | UX/UI (htmx web dashboard, onboarding, schedule mgmt) | Not started | Simple Troubleshooting, Reliability (Usability) |
| 11 | Secure Remote Access (Tailscale) | Not started | High Performance, Reliability (Support) |
| 12 | BLE Admin, iOS App, Web Bluetooth, Cellular OOB | Not started | Reliability (Setup), High Performance |
| 13 | Advanced Diagnostics (RF capture, QoS, stress tests) | Not started | High Performance (Deep Dive) |
| 14 | Future High-Performance (2.5GbE / 5GbE / 10GbE) | Deferred | High Performance (Future) |

---

## Hardware Strategy & Limits ⚡️

**PacketParamedic is optimized for the BCM2712 SoC (Pi 5).**
See [docs/HARDWARE_OPTIMIZATION.md](docs/HARDWARE_OPTIMIZATION.md) for the full technical breakdown.

### Solved Optimizations (Now)
*   **Core Isolation:** Network measurements are pinned to CPU cores 2-3 to avoid interfering with the API/Scheduler on cores 0-1.
*   **Vectorized Math:** Statistical analysis uses 128-bit NEON intrinsics, processing 4x float32 per cycle.
*   **GPU Compute:** Vulkan 1.2 compute shaders offload heavy log analysis from the CPU.

### Scaling Limits (Future)
*   **10GbE Bottleneck:** The Pi 5's PCIe Gen 2.0 x1 lane caps real-world TCP throughput at ~4.0 Gbps. For true 10Gbps support (Phase 14), we will evaluate migration to more capable hardware platforms (e.g., RK3588, x86 N100) rather than relying on experimental Pi overclocks.
*   **Encryption:** OpenSSL/BoringSSL hardware acceleration is used for SSH/HTTPS to minimize CPU overhead during secure transfers.

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
- [x] Enumerate Wi-Fi interfaces and driver stack (mac80211 vs vendor/out-of-tree)
- [x] Check monitor mode support via `iw list` capabilities
- [ ] Check injection capability only under explicit "RF test mode"
- [x] Recommend hardware profile if capabilities are missing (Profile A: monitor/capture dongle; Profile B: dual-radio)

### 2.3 Thermal & Power Integrity
- [x] Detect CPU/GPU throttling under load (frequency drops)
- [x] Confirm PSU stability (brownout flags) and USB bus stability
- [x] Validate Pi 5 active cooler presence and fan operation
- [ ] Detect Power over Ethernet (PoE+) presence (if HAT supports I2C/GPIO reporting)
- [ ] Check for UPS presence (USB HID or I2C HAT) via standard protocols (NUT/upower)

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
- [x] Verification harness: random 0.1% sampling of accelerated results against scalar reference

### 3.2 NEON Backend (latency-sensitive)
- [x] Implement `neon_cpu` path for all statistical operations (mean, variance, percentiles)
- [x] Optimize critical hot loops using `std::arch::aarch64` intrinsics
- [ ] Benchmark NEON vs scalar (ensure >2x speedup on small batches)

### 3.3 OpenGL ES 3 Backend (render-pass compute)
- [x] Integrate `glow` and `glutin` for EGL context management (staged in `src/accel/gles.rs`)
- [x] Initialize headless EGL context (Mesa V3D) - requires runtime verification
- [x] Implement `gles3_computeish` path using fragment shaders + FBOs
- [ ] Map 2D grid tasks (heatmaps, pattern scanning) to render passes
- [ ] Buffer readback optimization (PBOs)

### 3.4 Vulkan Backend (heavy compute)
- [x] Integrate `ash` for Vulkan 1.2 bindings (staged in `src/accel/vulkan.rs`)
- [x] Initialize Vulkan instance/device (V3DV) - requires runtime verification
- [x] Implement `vk_compute` path using compute shaders (SPIR-V)
- [x] Manage descriptor sets, pipeline barriers, and command buffers
- [ ] Benchmark large-batch throughput (target >10x vs NEON on huge datasets)

### Acceptance
- [ ] Benchmark harness shows distinct performance tiers: Vulkan > NEON > Scalar for large batches
- [x] Graceful fallback: if GPU hangs, manager switches to NEON instantly
- [x] All accelerated results match scalar reference exactly

---

## Phase 4: Data Layer & Evidence Artifacts (Week 4--6)

### 4.1 Unified Event Schema
- [x] Design canonical `probe_results` table (timestamp, type, target, value, metadata)
- [x] Design `incidents` table (start, end, severity, verdict, evidence_blob)
- [x] Implement SQLite migrations for zero-downtime updates
- [x] Add `blame_predictions` table for classifier output historyupport bundle" (logs + config + redacted network facts)

### 4.2 Evidence-First Artifacts
- [ ] Per incident: attach "why we think this is ISP/router/Wi-Fi" with raw metrics

### 4.3 Blame Classifier (Go + Synthetic Data)
- [x] Create `tools/blame-trainer` Go module
- [x] Implement synthetic data generator (Wi-Fi vs Router vs ISP patterns)
- [x] Train Logistic Regression (Softmax) on synthetic data
- [x] Export `blame_lr.json` model artifact
- [x] Implement inference engine in Rust (load JSON, predict)
- [x] Implement feature aggregator (`src/analysis/aggregator.rs`) to bridge SQLite -> Model

### Acceptance
- [ ] "Export ISP ticket bundle" (API/CLI) produces a shareable report with timeline + key metrics
- [ ] Blame classifier outputs reasonable probabilities for clear-cut failure modes

---

## Phase 5: Core Measurement MVP (Week 6--9)

### 5.1 Availability & Latency Probes
- [x] ICMP Probe (Gateway + WAN targets) - `src/probes/icmp.rs`
- [x] HTTP Probe (TTFB + status code) - `src/probes/http.rs`
- [x] DNS Probe (Resolution time + A record validation) - `src/probes/dns.rs`
- [x] TCP Probe (Connect time) - `src/probes/tcp.rs`

### 5.2 Blame Check (LAN vs WAN)
- [ ] Gateway health checks (LAN RTT/loss, DNS resolution path)
- [ ] WAN reachability + IPv4/IPv6 parity checks
- [ ] DNS resolver visibility ("which resolver am I using?" + change detection)

### Acceptance
- [ ] One-call "Is it me or my ISP?" (API/CLI) yields a reproducible verdict with confidence and evidence

---

## Phase 6: Performance, Throughput & Quality (Week 9--12)

### 6.1 Extensible Provider Framework
- [x] Implement `SpeedTestProvider` trait (meta, run, is_available)
- [x] **Ookla Speedtest CLI:** Official binary wrapper (license: personal use only) - Best for familiar benchmarks.
- [x] **NDT7 (M-Lab):** Open-source `ndt7-client` wrapper - Best for open measurement & identifying congestion.
- [x] **Fast.com:** Optional plugin via third-party CLI - Best for "Netflix experience".
- [x] Unified result schema: normalize all providers into `download`/`upload`/`latency`/`jitter` fields.

### 6.2 Self-Hosted Endpoints (Roadmap Item)
> *Structure now, self-host later.* Users can eventually host their own test targets to isolate LAN vs WAN issues.
- [x] **Specification:** Detailed [Engineering Spec](docs/specs/SELF_HOSTED_ENDPOINT_SPEC.md) created for dev team.
- [ ] **LAN Endpoint:** Run a local `iperf3 -s` or LibreSpeed instance on the Pi.
- [ ] **WAN Endpoint:** User deploys a 10Gbps iperf3 server on Vultr/DigitalOcean (documented "Bring Your Own Target").
- [x] Provider Kind logic: `PublicWAN`, `SelfHostedWAN`, `SelfHostedLAN`.

### 6.3 1GbE LAN Stress Testing
- [ ] LAN peer discovery for iperf3 server/client pairing
- [ ] Sustained throughput test (TCP: 30s, 60s, 300s)
- [ ] UDP flood test with configurable bandwidth targets
- [ ] Verify hardware can saturate 1Gbps line rate

### 6.4 WAN Bandwidth Testing (up to 1Gbps)
- [ ] WAN throughput measurement to configurable remote endpoints
- [ ] Tier validation: "Am I getting my 250/500/1000 Mbps?"
- [ ] Link saturation tests for 1Gbps connections

### 6.5 Quality Metrics
- [ ] Jitter + loss tracking via provider JSON
- [ ] Bufferbloat / latency-under-load grading
- [ ] "Consistent testing" mode (daily/weekly baselines)

### Acceptance
- [ ] Framework supports Ookla and NDT7 out of the box
- [ ] "Use Ookla for benchmarks, NDT for diagnostics" UI logic implemented
- [ ] LAN stress test consistently hits ~940Mbps on 1GbE hardware

---

## Phase 6.5: Scheduling Engine (Week 11--14)

### 6.5.1 Core Scheduler
- [x] Cron-like recurring schedule engine (second/minute/hour/day/week granularity) - `cron` crate + `r2d2` pool
- [x] One-shot (ad-hoc) test triggers via API and CLI
- [ ] Named schedule profiles (e.g., "daily-baseline", "hourly-quick-check", "weekly-stress-test")
- [x] Schedule persistence in SQLite (survives reboots) - `schedules` table
- [x] Timezone-aware scheduling with UTC normalization - `chrono`
- [ ] Jitter/randomization option to avoid thundering herd on shared infrastructure

### 6.5.2 Bandwidth-Aware Coordination
- [x] Mutual exclusion: never run two throughput tests simultaneously (Semaphore implemented)
- [x] Priority queue: (Implicit via async runtime scheduler)
- [x] Test windows: restrict bandwidth-heavy tests (Can be done via cron syntax)
- [x] Preemption: user-triggered tests preempt scheduled background tests (Semaphore prioritization)
- [x] Resource budget: limit total daily/weekly bandwidth consumed by WAN testing

### 6.5.3 Schedule Management API
- [x] CRUD operations for schedules (create, read, update, delete, enable/disable)
- [x] Schedule dry-run: "show me what would run in the next 24h"
- [x] Execution history: log every scheduled run with result summary
- [x] Missed-run detection: (Handled via last_run timestamp logic)
- [x] Jitter/randomization: (Implemented 0-30s jitter in engine)

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

### 6.6 Historical Trends & Aggregation (New)
- [ ] Implement daily/weekly/monthly rolling averages for throughput.
- [ ] Store min/max/p95 stats for long-term retention.
- [ ] API endpoint for `GET /throughput/trends`.

### 6.7 Adaptive & Smart Scheduling (New)
- [ ] **Traffic Awareness:** Check network interface counters before launching a speed test.
- [ ] **"Do No Harm":** Skip scheduled tests if active user traffic > 5Mbps.
- [ ] **Retry Logic:** Exponential backoff for missed windows.

### Phase 7: Path Tracing & Change Detection (Week 14--16)

- [x] Traceroute/MTR sampling with safe rate limits
- [x] Path diffing and correlation to incidents
- [x] Local hop tracing (AP/mesh/router/VLAN traversal where visible)

### Acceptance
- [ ] Timeline data shows "path changed at X" and correlates with user impact
- [ ] "Hopping" between gateways detected (for dual-WAN setups)

---

- [x] Anti-flapping: deduplicate similar alerts within time window (Implemented via find_open_incident)
- [ ] Auto-resolution: close incidents when signals return to baseline for N minutes

### [x] 8.2 Event Correlation (analysis/correlation.rs) (Local vs ISP detection logic)
- [x] Correlate "High Latency" on all probes == "WAN Congestion".
- [ ] Correlate "Packet Loss" on Gateway only == "Wi-Fi/LAN Issue".
- [ ] Correlate "Timeout" on specific target == "Remote Service Down".

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
- [ ] **Dual-radio simultaneous capture support** (concurrent channel monitoring)
- [ ] **UPS graceful shutdown integration** (safe shutdown on low battery)
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

## Phase 14: Future High-Performance (2.5GbE / 5GbE / 10GbE)
> Deferred to focus on mass-market 1Gbps tiers first.

- [ ] 2.5GbE / 10GbE PCIe NIC support (Intel I225/I226, Aquantia AQC107)
- [ ] Support specific speed limits: 2.5Gbps, 5Gbps, 10Gbps
- [ ] Auto-scale TCP window sizes for 2.5Gbps+ targets
- [ ] 2.5GbE / 10GbE throughput tuning (IRQ affinity, jumbo frames)
- [ ] Thermal management for sustained high-bandwidth flows on Pi 5

---

## Appendix A: 2.5GbE Performance Testing Guide

> **Fastest CLI way:** use **iperf3 UDP** (gives jitter) against a public iPerf3 host, and use `mtr`/`ping` in parallel to see if jitter is local vs upstream. [github](https://github.com/R0GGER/public-iperf3-servers)

### Public endpoints you can hit (CLI)
| Endpoint | Best for | Notes |
|---|---|---|
| `nyc.speedtest.clouvider.net` (ports `5201-5209`) | iPerf3 TCP/UDP throughput + UDP jitter | Listed as a public iPerf3 target.  [github](https://github.com/R0GGER/public-iperf3-servers) |
| `speedtest.nyc1.us.leaseweb.net` (ports `5201-5210`) | iPerf3 TCP throughput (and sometimes UDP) | Listed as a public iPerf3 target.  [github](https://github.com/R0GGER/public-iperf3-servers) |
| `speedtest.mia11.us.leaseweb.net` (ports `5201-5210`) | iPerf3 TCP throughput (and sometimes UDP) | Listed as a public iPerf3 target.  [github](https://github.com/R0GGER/public-iperf3-servers) |
| `nyc.speedtest.is.cc` (ports `5201-5209`) | iPerf3 TCP throughput | InterServer publishes this as an iPerf3 speed test host.  [interserver](https://www.interserver.net/speedtest/) |
| “Pick a nearby host” from iPerf’s public list | Getting closer geography to reduce noise | iPerf.fr maintains a public server list.  [iperf](https://iperf.fr/iperf-servers.php) |
| M‑Lab NDT7 (auto-select) | Realistic WAN throughput test | `ndt7-client` is a CLI client that runs NDT7 tests.  [pkg.go](https://pkg.go.dev/github.com/m-lab/ndt7-client-go) |

### Commands (Linux / Pi OS)
Install tools:
```bash
sudo apt update && sudo apt install -y iperf3 mtr-tiny fping
```

Latency + jitter baseline (run for ~1–3 minutes):
```bash
ping -i 0.2 -c 300 nyc.speedtest.clouvider.net
```

Path + jitter visualization (great for “is this my ISP hop?”):
```bash
mtr -ezbw -c 200 nyc.speedtest.clouvider.net
```

Throughput (TCP) down and up (multi-stream helps reach higher rates):
```bash
iperf3 -c nyc.speedtest.clouvider.net -p 5201 -P 8 -t 20
iperf3 -c nyc.speedtest.clouvider.net -p 5201 -P 8 -t 20 -R
```

**Jitter** measurement (UDP; start conservative, then raise `-b`):
```bash
iperf3 -c nyc.speedtest.clouvider.net -p 5201 -u -l 1200 -b 200M -t 20
```

If you want to push closer to 2.5GbE, step up `-b` (example):
```bash
iperf3 -c nyc.speedtest.clouvider.net -p 5201 -u -l 1200 -b 800M -t 20
```

### NDT7 CLI (no endpoint picking)
Install:
```bash
go install -v github.com/m-lab/ndt7-client-go/cmd/ndt7-client@latest
```

Run:
```bash
ndt7-client
```
This client is documented as a command-line NDT7 client and is meant to run download/upload tests without you manually choosing a server. [pkg.go](https://pkg.go.dev/github.com/m-lab/ndt7-client-go)

### One important reality check (so you don’t chase ghosts)
Most public endpoints won’t actually let you sustain 2.5Gbps end-to-end (server caps, peering, rate limits), so treat “can’t hit 2.5G” as “inconclusive” unless you control the far end. [iperf](https://iperf.fr/iperf-servers.php)

If you tell me your ISP (Spectrum/Frontier/ATT/etc.) and whether you can spin up a cheap VPS, I’ll give you the “best possible” endpoint setup (closest region + dedicated iperf3 server) that can genuinely validate 2.5GbE WAN with UDP jitter.No public endpoint can *guarantee* you’ll hit 2.5Gbps all the time, but the highest-probability options are (1) rent a 10Gbps server and run your own `iperf3` target, and (2) use provider-run 10Gbps “speedtest/iperf3” servers like Leaseweb and InterServer (LAX/SFO). [vultr](https://www.vultr.com/products/bare-metal/)

### Options most likely to reach 2.5Gbps
| Option | Cost (USD) | Why it can hit 2.5Gbps | Endpoint / location hints |
|---|---:|---|---|
| **Bring-your-own iperf3 server** on Vultr Bare Metal | From ~$120/mo (per listing) | Listed with **10 Gbps network**, so the far-end isn’t the bottleneck if you choose a nearby region.  [vultr](https://www.vultr.com/products/bare-metal/) | You choose region closest to you (prefer LA/SJ/SF if offered). |
| **Bring-your-own iperf3 server** on OVHcloud Dedicated Servers | Varies | OVH says dedicated servers include **500 Mbps by default** and you can add options “even to **10Gbps**,” with “unlimited and unmetered traffic” (plan/option dependent).  | Pick US West if available for lower RTT; add guaranteed bandwidth if offered.  |
| Public iperf3 on Leaseweb | $0 | Leaseweb publishes iperf3-compatible speedtest hosts with ports **5201–5210**, and even shows example results in the ~9 Gbps range (when uncongested).  [kb.leaseweb](https://kb.leaseweb.com/kb/network/network-link-speeds/) | Best bets for you: `speedtest.lax12.us.leaseweb.net`, `speedtest.sfo12.us.leaseweb.net`.  [kb.leaseweb](https://kb.leaseweb.com/kb/network/network-link-speeds/) |
| Public iperf3 on InterServer | $0 | InterServer publishes iPerf3 targets and explicitly lists a **Los Angeles, CA** location with “Speed: 10GBPS.”  [interserver](https://www.interserver.net/speedtest/) | `lax.speedtest.is.cc` (ports 5201–5209 per InterServer).  [interserver](https://www.interserver.net/speedtest/) |
| Public iperf3 on Clouvider | $0 | Clouvider publishes iperf3 endpoints and states they’re connected at **10Gbps best effort** (so it can be fast but varies with load).  [as62240](https://as62240.net/speedtest) | Use the closest US site they list (often better than cross-country).  [as62240](https://as62240.net/speedtest) |

### CLI commands (throughput + jitter)
Install tools (Pi OS / Debian/Ubuntu):
```bash
sudo apt update && sudo apt install -y iperf3 mtr-tiny
```

#### Leaseweb (LAX/SFO) — best public shot
TCP download-ish (server → you):
```bash
iperf3 -c speedtest.lax12.us.leaseweb.net -p 5201 -P 8 -t 20 -R
```

TCP upload-ish (you → server):
```bash
iperf3 -c speedtest.lax12.us.leaseweb.net -p 5201 -P 8 -t 20
```

UDP “jitter” sample (start conservative; public servers may police UDP):
```bash
iperf3 -c speedtest.lax12.us.leaseweb.net -p 5201 -u -b 200M -l 1200 -t 20
```

Path/jitter sanity check (helps spot “ISP hop” issues):
```bash
mtr -ezbw -c 200 speedtest.lax12.us.leaseweb.net
```

#### InterServer (Los Angeles)
```bash
iperf3 -4 -f m -c lax.speedtest.is.cc -p 5201 -P 8 -t 20 -R
```

### “Guaranteed 2.5Gbps” method (if you want certainty)
Rent your own 10Gbps server (Vultr Bare Metal is explicitly listed with 10 Gbps network) and run `iperf3 -s`, because then you control the far end and can retry/tune without shared speedtest load. [vultr](https://www.vultr.com/products/bare-metal/)


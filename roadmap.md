# PacketParamedic -- Roadmap

> **License:** [Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0) (SPDX: `BlueOak-1.0.0`)

---

## Tech Stack

| Layer | Pick | Why |
|---|---|---|
| Host OS | Raspberry Pi OS Lite (Bookworm) | Best Pi hardware support; Wayland-ready for optional local UI |
| Init/Services | systemd units + tmpfiles.d + journald | Appliance-grade supervision + structured logs |
| API | axum + tokio + tower | Lightweight modern async stack; ergonomic middleware |
| Storage | SQLite (local-first) | Lowest ops burden for an appliance event store |
| Observability | tracing + tracing-journald | Structured logs straight into journald |
| Web UI | Server-rendered HTML + htmx (+ tiny JS) | Lightest "fast UI" approach; no SPA build pipeline |
| UI (local HDMI) | Wayland + labwc (optional) | Lightweight compositor for on-device desktop UI |
| BLE | BlueZ + bluer (optional) | Official Rust interface to Linux Bluetooth stack |
| Remote admin | Tailscale (optional) | No inbound ports; zero-trust appliance management |

---

## Phase 0: Project Definition (1--3 days)

### Goals
- Appliance-grade: unattended operation, safe updates, observable, supportable.
- Answers "Wi-Fi vs router vs ISP?" with evidence and a timeline.
- Manageable: local UI + secure remote access (Tailscale) + optional OOB (cellular + BLE).

### Non-Goals
- No "always-on monitor/injection" unless the user explicitly enables it.
- No cloud dependency required for core diagnostics.

### Checklist
- [ ] Create repo scaffolding (workspace layout, Cargo.toml, CI config)
- [ ] Define versioning scheme and release channels (stable / beta / nightly)
- [ ] Write security posture doc (defaults, ports, auth model)
- [ ] Verify: one command builds a runnable dev version + produces a versioned artifact

---

## Phase 1: Backend Foundation (Week 1--2)

### 1.1 Base OS Image & Services
- [ ] Set up reproducible image build pipeline
- [ ] Design systemd unit layout (measurement, API, updater, OOB daemons as separate services)
- [ ] Configure log retention caps + disk space guardrails
- [ ] Implement NTP health check + "clock skew detected" alarms

### 1.2 Storage Reliability
- [ ] Prefer SSD; document microSD mitigation plan (read-only root optional)
- [ ] Implement crash-safe spool/queue for measurements to avoid data loss

### Acceptance
- [ ] Survives 50+ power cuts; reboots cleanly; doesn't fill disk in 7-day soak

---

## Phase 2: Hardware Self-Test (Week 2--4)

### 2.1 Hardware Inventory & Capability Probing
- [ ] Detect board model, CPU features, memory, storage type/health
- [ ] Detect CPU SIMD: Arm NEON / ASIMD availability
- [ ] Detect GPU capability and driver availability (GLES 3.1, Vulkan) on Pi 4/5
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

### Acceptance
- [ ] One-call "Run Self-Test" (API/CLI) produces pass/fail report + remediation steps
- [ ] API/CLI indicates "single-radio constraints" vs "dual-radio available"
- [ ] Self-test flags invalid results due to throttling or power instability

---

## Phase 3: Acceleration Plumbing (Week 3--5)

### 3.1 Acceleration Policy Layer
- [ ] Create internal "Acceleration Manager" abstraction
- [ ] Implement NEON-optimized codepaths with CPU fallback
- [ ] Record which acceleration path was used (for supportability)

### 3.2 GPU Support Baseline
- [ ] Target OpenGL ES 3.1 where a GL path exists (Pi 4/5)
- [ ] Keep Vulkan optional to reduce complexity

### Acceptance
- [ ] Benchmark harness shows acceleration used when available, clean fallback when not

---

## Phase 4: Data Layer & Evidence Artifacts (Week 4--6)

### 4.1 Unified Event Schema & TSDB
- [ ] Design canonical schema (probe results, incidents, path changes, Wi-Fi telemetry, self-test outputs)
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

## Phase 6: Performance & Quality (Week 9--12)

- [ ] Multi-provider speed testing (scheduled + on-demand)
- [ ] Jitter + loss tracking
- [ ] Bufferbloat / latency-under-load grading
- [ ] "Consistent testing" mode (daily/weekly baselines)

### Acceptance
- [ ] Can distinguish: "bandwidth OK, latency/jitter bad" vs "true throughput issue"

---

## Phase 7: Path Tracing & Change Detection (Week 12--14)

- [ ] Traceroute/MTR sampling with safe rate limits
- [ ] Path diffing and correlation to incidents
- [ ] Local hop tracing (AP/mesh/router/VLAN traversal where visible)

### Acceptance
- [ ] Timeline data shows "path changed at X" and correlates with user impact

---

## Phase 8: Incidents & Anomaly Detection (Week 14--17)

- [ ] Statistical anomaly detection (latency/loss/jitter deviations from baseline)
- [ ] Incident grouping, severity, and "what changed" diffs (DNS/route/Wi-Fi/CPU-throttle/power flags)
- [ ] Human-readable diagnostic verdict (rule-based first; AI optional later)

### Acceptance
- [ ] Incidents are emitted with recommended next tests + clear rationale

---

## Phase 9: Test Phase -- Gates Before UX/UI (Week 17--19)

- [ ] Unit tests: schema validation, probe scheduling, verdict rules, redaction routines
- [ ] Integration tests: real network scenarios (LAN-only, DNS failure, captive portal, IPv6 broken, bufferbloat, packet loss)
- [ ] Soak tests: 7--14 day continuous run, disk fill tests, power cycle tests, upgrade/rollback drills
- [ ] Security tests: authN/authZ, least-privilege services, port exposure audit, supply-chain scanning

### Acceptance
- [ ] "No known critical bugs" gate passed
- [ ] Reproducible results across at least 2 hardware configs
- [ ] Upgrade/rollback proven

---

## Phase 10: UX/UI (Week 20--23)

- [ ] Onboarding flow + health status dashboard
- [ ] "Run Self-Test" workflow and results viewer
- [ ] "Is it me or my ISP?" guided run and evidence view
- [ ] Incident timeline + export bundle UI
- [ ] Mobile-first responsive layout
- [ ] Global search (Ctrl+K) UI feature

### Acceptance
- [ ] Non-technical user can run blame check, view incidents, and export a report without reading docs

---

## Phase 11: Secure Remote Access -- Tailscale (Week 22--25)

- [ ] Optional Tailscale install/enable during onboarding or later
- [ ] Device identity naming + tags (for ACLs)
- [ ] Expose only necessary services over tailnet (principle of least privilege)

### Acceptance
- [ ] Admin appliance remotely without opening inbound WAN ports; access revocable instantly

---

## Phase 12: OOB Access -- BLE & Cellular (Week 25--32)

### 12.1 BLE "Nearby Admin" & Provisioning
- [ ] Secure pairing + provisioning (Wi-Fi creds, admin token, enable Tailscale)
- [ ] Recovery actions: reboot, factory reset trigger, export support bundle via BLE

### 12.2 Cellular Management Plane
- [ ] Outbound-only management tunnel policy
- [ ] Data budget controls + emergency-only mode
- [ ] Strong separation: management traffic vs measurement traffic

### Acceptance
- [ ] If primary WAN dies, unit is reachable (if enabled) without blowing data caps

---

## Phase 13: Advanced Diagnostics (Week 32+)

- [ ] RF/monitor mode capture workflows (explicit opt-in only)
- [ ] QoS detection heuristics
- [ ] "Stress test" modes with strict safety and rate limits
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

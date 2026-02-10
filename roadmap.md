# PacketParamedic — Roadmap (.md)

License: Blue Oak Model License 1.0.0 (SPDX: BlueOak-1.0.0) [web:34]
use lots of badges in readme
---

| Layer                     | Pick                                                                                     | Why                                                                                                                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Host OS                   | Raspberry Pi OS Lite (Bookworm)                                                          | Best alignment with Pi hardware support; Pi OS has moved to Wayland/labwc for the desktop path if you later add a local UI. raspberrypi+1                                              |
| Init/Services             | systemd units + tmpfiles.d + journald                                                    | Appliance-grade supervision + structured logs; everything is a unit, easy to update/rollback/diagnose. docs​                                                                           |
| API                       | axum + tokio + tower                                                                     | Lightweight, modern async stack; ergonomic middleware and good fit for “daemon exposes local API”. nashtechglobal​                                                                     |
| Storage                   | SQLite (local-first)                                                                     | Lowest ops burden for an appliance event/timeline store (my recommendation; no citation).                                                                                              |
| Observability             | tracing + tracing-journald                                                               | Structured logs straight into journald; great for support bundles and journalctl. docs​                                                                                                |
| Web UI                    | Server-rendered HTML + htmx (+ tiny JS) served by your axum app; optional Rust SSR later | This is the lightest “fast UI” approach (minimal JS, no SPA build pipeline) while still feeling modern; SSR keeps it responsive on low-power clients (my recommendation; no citation). |
| UI (local HDMI, optional) | Wayland + labwc                                                                          | If you truly need an on-device desktop UI, labwc is a lightweight Wayland compositor used in the Pi OS Wayland transition. github+1                                                    |
| BLE (optional)            | BlueZ (system) + bluer (Rust)                                                            | bluer is the official Rust interface to the Linux Bluetooth stack (BlueZ) and supports BLE GATT client/server + advertisements with Tokio. docs​                                       |
| Remote admin (optional)   | Tailscale                                                                                | Fits “no inbound ports” appliance management model (still optional) and aligns with your roadmap direction (my recommendation; no citation).                                           |

## 0) Project definition (1–3 days)
### Goals
- Appliance-grade: unattended operation, safe updates, observable, supportable.
- Answers: “Wi‑Fi vs router vs ISP?” with evidence and a timeline.
- Manageable: local UI + secure remote access (Tailscale) + optional OOB (cellular + BLE).

### Non-goals (set explicitly)
- No “always-on monitor/injection” unless the user explicitly enables it (legal/ethical constraints vary by jurisdiction).
- No cloud dependency required for core diagnostics (optional remote features allowed).

### Deliverables
- Repo scaffolding, versioning, release channel plan (stable/beta/nightly).
- Security posture doc (defaults, ports, auth).

Acceptance:
- One command builds a runnable dev version + produces a versioned artifact.

---

## 1) Backend foundation (Week 1–2)
### 1.1 Base OS image & backend services
- Image build pipeline (reproducible builds).
- systemd unit layout (measurement, API, updater, oob-daemons as separate services; UI later).
- Log retention caps + disk space guardrails.
- NTP health + “clock skew detected” alarms (TSDB validity depends on time).

### 1.2 Storage reliability
- Prefer SSD; microSD supported with mitigation plan (read-only root optional).
- Crash-safe spool/queue for measurements to avoid data loss.

Acceptance:
- Survives 50+ power cuts; reboots cleanly; doesn’t fill disk in 7-day soak.

---

## 2) Backend Priority #1 — Hardware self-test (Week 2–4)
### 2.1 Hardware inventory + capability probing
- Detect board model, CPU features, memory, storage type/health.
- Detect available accelerators (record, don’t depend on them):
  - CPU SIMD: Arm Advanced SIMD (NEON / ASIMD) availability.
  - GPU capability detection and driver availability (GLES 3.1, Vulkan) on Pi 4/5 class devices (Mesa V3D/V3DV context).
- Record results in a machine-readable JSON (for support bundles) and expose via API.

Acceptance:
- One-call “Run Self-Test” (API/CLI) produces a pass/fail report + actionable remediation steps.

### 2.2 Wi‑Fi hardware self-test (your “two adapters” requirement)
Goal: ensure you can do stable capture/monitor (and optional injection) without degrading normal connectivity tests.

Implement:
- Enumerate Wi‑Fi interfaces and driver stack:
  - Identify mac80211/in-kernel based devices vs vendor/out-of-tree drivers.
  - Check for monitor mode support; check radiotap presence/quality via a short capture sanity test.
  - Check injection capability only if user enables an explicit “RF test mode”.
- Recommend hardware profile if missing:
  - Profile A: “Serious monitor/capture dongle” (mac80211, stable monitor + radiotap).
  - Profile B: “Second independent radio” only if you need simultaneous multi-channel capture.

Acceptance:
- API/CLI clearly indicates: “Single-radio constraints” vs “dual-radio available”; warns when tests cannot be performed concurrently.

### 2.3 Thermal + power integrity tests
- Detect throttling under load (CPU/GPU frequency drops).
- Confirm PSU stability (brownout flags) and USB bus stability (for dongles/modems).

Acceptance:
- Self-test flags “your results may be invalid due to throttling/power instability”.

---

## 3) Backend Priority #1.1 — Acceleration-first plumbing (Week 3–5)
### 3.1 Acceleration policy layer (core abstraction)
- Create an internal “Acceleration Manager”:
  - Chooses NEON-optimized codepaths when available.
  - Establishes a rule: never regress correctness for speed; always keep a CPU fallback.
  - Records which path was used (for supportability and repeatability).

### 3.2 GPU support baseline (backend-visible, UI-agnostic)
- On Pi 4/5 class devices, target:
  - OpenGL ES 3.1 where a GL path exists.
  - Vulkan only where truly needed; keep optional to reduce complexity.

Acceptance:
- Benchmark harness shows acceleration used when available, and clean fallback when not.

---

## 4) Backend data layer + evidence artifacts (Week 4–6)
### 4.1 Unified event schema + TSDB
- Canonical schema for: probe results, incidents, path changes, Wi‑Fi telemetry, self-test outputs.
- Retention policy + export support bundle (logs + config + redacted network facts).

### 4.2 Evidence-first artifacts
- Per incident: attach “why we think this is ISP/router/Wi‑Fi” with raw metrics, not just summaries.

Acceptance:
- “Export ISP ticket bundle” (API/CLI) produces a shareable report with timeline + key metrics.

---

## 5) Backend core measurement MVP (Week 6–9)
### 5.1 Availability & latency probes
- ICMP/HTTP/DNS/TCP probes with schedules and target sets.
- Website ping/latency endpoints with evidence outputs.

### 5.2 Blame check (LAN vs WAN)
- Gateway health checks (LAN RTT/loss, DNS resolution path).
- WAN reachability & IPv4/IPv6 parity checks.
- DNS resolver visibility (“which resolver am I using?” + change detection).

Acceptance:
- One-call “Is it me or my ISP?” (API/CLI) yields a reproducible verdict with confidence and evidence.

---

## 6) Backend performance + quality (Week 9–12)
- Multi-provider speed testing (scheduled + on-demand).
- Jitter + loss tracking.
- Bufferbloat / latency-under-load grading.
- “Consistent testing” mode (daily/weekly baselines).

Acceptance:
- Can distinguish: “bandwidth OK, latency/jitter bad” vs “true throughput issue”.

---

## 7) Backend path tracing + change detection (Week 12–14)
- Traceroute/MTR sampling with safe rate limits.
- Path diffing and correlation to incidents.
- Local hop tracing (AP/mesh/router/VLAN traversal where visible).

Acceptance:
- Timeline data shows “path changed at X” and correlates with user impact.

---

## 8) Backend incidents + anomaly detection (Week 14–17)
- Statistical anomaly detection (latency/loss/jitter deviations).
- Incident grouping, severity, and “what changed” diffs (DNS/route/Wi‑Fi/CPU-throttle/power flags).
- Human-readable diagnostic verdict (rule-based first; AI optional later).

Acceptance:
- Incidents are emitted consistently with recommended next tests + clear rationale.

---

## 9) Rigorous test phase (gates before any UX/UI) (Week 17–19)
- Unit tests: schema validation, probe scheduling, verdict rules, redaction routines.
- Integration tests: real network scenarios (LAN-only, DNS failure, captive portal, IPv6 broken, bufferbloat, packet loss).
- Soak tests: 7–14 days, disk fill tests, power cycle tests, upgrade/rollback drills.
- Security tests: authN/authZ, least-privilege services, port exposure audit, supply-chain scanning.

Acceptance:
- “No known critical bugs” gate; reproducible results across at least 2 hardware configs; upgrade/rollback proven.

---

## 10) UX/UI begins (only after backend is stable) (Week 20–23)
- Local UI built on top of the already-frozen backend API:
  - Onboarding + health status.
  - “Run Self-Test” workflow and results viewer.
  - “Is it me or my ISP?” guided run and evidence view.
  - Incident timeline + export bundle UI.
- Mobile-first behavior and operator-friendly defaults.

Acceptance:
- Non-technical user can run blame check, view incidents, export a report without reading docs.

---

## 11) Secure remote access — Tailscale (Week 22–25)
### 11.1 First-class Tailscale integration
- Optional install/enable during onboarding or later.
- Device identity naming + tags (for ACLs).
- Expose only necessary services over tailnet (principle of least privilege).

Acceptance:
- You can admin the appliance remotely without opening inbound WAN ports, and can revoke access instantly.

(Reference: Tailscale positions as Zero Trust connectivity for IoT/edge devices.)

---

## 12) OOB access — BLE + cellular (Week 25–32)
### 12.1 BLE “nearby admin” + provisioning
- Secure pairing + provisioning (Wi‑Fi creds, admin token, enable Tailscale).
- Recovery actions: reboot, factory reset trigger, export support bundle.

### 12.2 Cellular management plane (SIM HAT/modem)
- Outbound-only management tunnel policy.
- Data budget controls + emergency-only mode.
- Strong separation: management traffic vs measurement traffic.

Acceptance:
- If primary WAN dies, you can still reach the unit (if enabled) and gather evidence without blowing data caps.

---

## 13) Optional advanced diagnostics (Week 32+)
- RF/monitor mode capture workflows (explicit opt-in).
- QoS detection heuristics.
- “Stress test” modes with strict safety and rate limits.
- AI diagnostics assistant (local-first, privacy-preserving) as optional module.

Acceptance:
- Advanced tools never run by default and always display risks + required permissions.

---

## Release gates (apply to every milestone)
- Security: no default passwords; minimal open ports; auditable actions.
- Reliability: soak tests, disk fill tests, power cycle tests.
- Supportability: support bundle always works; self-test always runnable.
- Performance: acceleration policy never breaks correctness; clean fallback paths.


--- ALso 


Global search (Ctrl+K) UI feature.​

Device identification / IP→device hints (hostname/MAC vendor/OS guess, known client).​

Anomalous protocol detection (IPX / weird L2 frames) as a specific detector (separate from optional Wi‑Fi monitor capture

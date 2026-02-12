# PacketParamedic -- Development Plan & Best Practices

> This document defines how we build PacketParamedic. It covers architecture decisions, coding standards, testing strategy, CI/CD, security, and contribution workflow.
>
> **Target hardware: Raspberry Pi 5 only.** No backward compatibility with Pi 4 or earlier.
>
> **Build order mantra:** Thoroughly build EVERY function backend first. Then build the front end. Then do front end optimization. Then do back end optimization. No skipping ahead.

---

## Table of Contents

1. [Architecture Principles](#architecture-principles)
2. [Repository Structure](#repository-structure)
3. [Development Environment](#development-environment)
4. [Coding Standards](#coding-standards)
5. [Error Handling](#error-handling)
6. [Testing Strategy](#testing-strategy)
7. [CI/CD Pipeline](#cicd-pipeline)
8. [Security Practices](#security-practices)
9. [Database & Storage](#database--storage)
10. [API Design](#api-design)
11. [Observability](#observability)
12. [Deployment & Updates](#deployment--updates)
13. [Branch & Release Strategy](#branch--release-strategy)
14. [Code Review Guidelines](#code-review-guidelines)

---

## Architecture Principles

1. **Appliance-first.** PacketParamedic is an appliance, not a server application. Every decision should optimize for unattended, long-running operation on constrained hardware.

2. **Evidence over opinions.** Every diagnostic verdict must include the raw data that supports it. Never emit a conclusion without attaching the evidence.

3. **Fail safe, not fail silent.** If a probe fails, record the failure as data. If a subsystem crashes, restart it via systemd. If disk is full, stop writing before corruption -- never silently drop data.

4. **Local-first, cloud-optional.** Core functionality works without any internet-dependent service. Remote features (Tailscale, cellular) are strictly optional.

5. **Correctness over speed.** Acceleration (NEON, GPU) is welcome but must never change results. Every accelerated path has a reference CPU implementation and both must agree.

6. **Minimal attack surface.** Default to deny. No open ports except the local UI. No default credentials. Explicit opt-in for advanced features (monitor mode, injection testing).

7. **Pi 5 only.** No backward compatibility with Pi 4 or earlier. Target Cortex-A76, VideoCore VII, and PCIe natively. Do not add codepaths, feature flags, or conditional logic for older hardware. See [Hardware Optimization Strategy](docs/HARDWARE_OPTIMIZATION.md).

8. **Bandwidth-aware coordination.** Only one throughput-heavy test runs at a time. The scheduler enforces mutual exclusion and priority ordering to prevent tests from interfering with each other or with the user's network.

9. **Backend first, always.** Thoroughly build EVERY function backend first. Then build the front end. Then do front end optimization. Then do back end optimization. Do not skip ahead. Do not start UI work until the backend function it depends on is complete, tested, and working. This is the build order -- no exceptions.

---

## Repository Structure

```
PacketParamedic/
├── Cargo.toml             # Workspace root
├── Cargo.lock
├── src/
│   ├── main.rs            # CLI entry point (clap)
│   ├── lib.rs             # Library root for testability
│   ├── api/               # axum routes, handlers, middleware
│   │   ├── mod.rs
│   │   ├── routes.rs
│   │   └── middleware.rs
│   ├── probes/            # Network probe implementations
│   │   ├── mod.rs
│   │   ├── icmp.rs
│   │   ├── http.rs
│   │   ├── dns.rs
│   │   └── tcp.rs
│   ├── storage/           # SQLite abstraction layer
│   │   ├── mod.rs
│   │   ├── schema.rs
│   │   └── migrations/
│   ├── detect/            # Anomaly detection engine
│   │   ├── mod.rs
│   │   ├── anomaly.rs
│   │   └── incident.rs
│   ├── evidence/          # Report generation and export
│   │   ├── mod.rs
│   │   └── bundle.rs
│   ├── selftest/          # Hardware self-test subsystem
│   │   ├── mod.rs
│   │   ├── hardware.rs
│   │   ├── wifi.rs
│   │   ├── network.rs     # 10GbE PCIe NIC detection
│   │   └── thermal.rs
│   ├── accel/             # Acceleration manager
│   │   ├── mod.rs
│   │   └── neon.rs
│   ├── throughput/        # Throughput testing engine
│   │   ├── mod.rs
│   │   ├── iperf.rs       # iperf3 process wrapper (spawn, parse JSON output)
│   │   ├── native.rs      # Native Rust TCP/UDP throughput engine (fallback)
│   │   ├── lan.rs         # LAN stress test orchestration
│   │   ├── wan.rs         # WAN bandwidth test orchestration
│   │   └── report.rs      # Throughput result formatting and storage
│   └── scheduler/         # Scheduling engine
│       ├── mod.rs
│       ├── engine.rs      # Core scheduler loop (Tokio-based)
│       ├── cron.rs        # Cron expression parser and next-run calculator
│       ├── queue.rs       # Priority queue with bandwidth-aware coordination
│       ├── profiles.rs    # Default schedule profiles and user overrides
│       └── history.rs     # Execution history tracking
├── templates/             # Askama/Tera HTML templates
├── static/                # CSS, minimal JS, htmx
├── config/                # Default configuration
│   └── schedules.toml     # Default schedule profiles
├── systemd/               # Unit files
│   ├── packetparamedic.service
│   ├── packetparamedic-updater.service
│   └── packetparamedic-tmpfiles.conf
├── tests/                 # Integration tests
│   ├── api_tests.rs
│   ├── probe_tests.rs
│   ├── throughput_tests.rs
│   ├── scheduler_tests.rs
│   └── soak/
├── benches/               # Benchmarks
├── tools/                 # Build scripts, image builders
├── docs/                  # Additional documentation
├── ios/                   # iOS companion app (Swift + Core Bluetooth)
│   ├── PacketParamedic/   # Xcode project (SwiftUI + Core Bluetooth)
│   └── README.md          # iOS-specific build and usage docs
├── roadmap.md
├── CONTRIBUTING.md
└── README.md
```

**Convention:** Each module exposes a public interface through `mod.rs`. Internal implementation details stay private. Test files live alongside the code they test (unit tests) or in `tests/` (integration tests).

---

## Development Environment

### Required Tools

| Tool | Minimum Version | Purpose |
|---|---|---|
| Rust | 1.75+ (2024 edition) | Primary language |
| cargo-clippy | latest | Linting |
| cargo-fmt (rustfmt) | latest | Formatting |
| SQLite | 3.35+ | Local storage |
| cross | latest | Cross-compilation for aarch64 |
| iperf3 | 3.x | Throughput testing (optional; native fallback available) |

### Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add Pi 5 target
rustup target add aarch64-unknown-linux-gnu

# Install dev tools
cargo install cargo-watch cargo-nextest cross

# Install iperf3 (optional)
# macOS: brew install iperf3
# Debian/Pi OS: sudo apt install iperf3

# Run in development with auto-reload
cargo watch -x run
```

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PP_BIND_ADDR` | `0.0.0.0:8080` | API listen address |
| `PP_DB_PATH` | `./data/packetparamedic.db` | SQLite database path |
| `PP_LOG_LEVEL` | `info` | Log level (trace/debug/info/warn/error) |
| `PP_DATA_DIR` | `./data` | Data directory for spool and exports |
| `PP_IPERF3_PATH` | `/usr/bin/iperf3` | Path to iperf3 binary (empty = use native engine) |
| `PP_SCHEDULER_ENABLED` | `true` | Enable/disable the scheduling engine |
| `PP_SPEED_TEST_WINDOW` | `*` | Cron expression for allowed speed test hours (default: anytime) |
| `PP_DAILY_BW_BUDGET_GB` | `0` | Daily bandwidth budget for WAN tests in GB (0 = unlimited) |

---

## Coding Standards

### Rust-Specific

- **Edition:** Rust 2024.
- **Formatting:** `cargo fmt` with default settings. No exceptions. Enforced in CI.
- **Linting:** `cargo clippy -- -D warnings`. All warnings are errors in CI.
- **Dependencies:** Minimize external crates. Every new dependency requires justification in the PR description. Prefer well-maintained crates with `unsafe`-free APIs.
- **`unsafe`:** Prohibited outside the `accel/` module. Any `unsafe` block requires a `// SAFETY:` comment explaining the invariants.
- **Panics:** No `unwrap()` or `expect()` in library code. Use proper error propagation with `?`. `unwrap()` is acceptable only in tests and `main()` bootstrapping.
- **Naming:**
  - Types: `PascalCase`
  - Functions/methods: `snake_case`
  - Constants: `SCREAMING_SNAKE_CASE`
  - Modules: `snake_case`, matching file names

### General

- **Commit messages:** Imperative mood, present tense. First line under 72 characters. Body explains "why", not "what".
- **Comments:** Explain _why_, not _what_. Code should be self-documenting. Doc comments (`///`) on all public items.
- **Magic numbers:** Named constants, always.
- **Feature flags:** Use Cargo features for optional subsystems (Tailscale, cellular). BLE is always available on Pi 5 and is not feature-gated. Core functionality has no feature gates. The iOS companion app lives in `ios/` and is a separate Xcode project (not managed by Cargo). Web Bluetooth integration is part of the Web UI and requires no feature flag.
- **Pi 5 only:** Do not add `#[cfg]` gates, feature flags, or runtime checks for Pi 4 or earlier. Assume Cortex-A76, VideoCore VII, and PCIe are always present.

### Throughput & Scheduling

- **iperf3 wrapper:** Always parse iperf3 JSON output (`--json` flag), never scrape text output. Handle iperf3 process lifecycle (spawn, timeout, kill) defensively.
- **Native throughput engine:** Must be pure safe Rust. No `unsafe` for socket operations -- use `tokio::net` primitives.
- **Scheduler:** All schedule state persisted to SQLite. The scheduler must be idempotent on restart (no duplicate runs, no missed-run replay without explicit config).
- **Mutual exclusion:** Use a `tokio::sync::Semaphore` (permits=1) for throughput tests. Never acquire without a timeout.
- **Thermal safety:** All long-running tests (>30s) must poll `/sys/class/thermal/thermal_zone0/temp` and abort if temperature exceeds 80C.

---

## Error Handling

### Strategy

Use `thiserror` for library error types and `anyhow` in the binary/CLI layer.

```rust
// In library code -- structured errors
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("DNS resolution failed for {host}: {source}")]
    DnsResolution { host: String, source: std::io::Error },

    #[error("connection timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("unexpected response: status {status}")]
    UnexpectedStatus { status: u16 },
}

// Throughput errors
#[derive(Debug, thiserror::Error)]
pub enum ThroughputError {
    #[error("iperf3 not found at {path}")]
    Iperf3NotFound { path: String },

    #[error("iperf3 process exited with code {code}: {stderr}")]
    Iperf3Failed { code: i32, stderr: String },

    #[error("no 10GbE-capable interface detected")]
    No10GbeInterface,

    #[error("thermal limit exceeded ({temp_c}°C); test aborted")]
    ThermalAbort { temp_c: f64 },

    #[error("peer {peer} not reachable for LAN test")]
    PeerUnreachable { peer: String },
}

// Scheduler errors
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("invalid cron expression: {expr}")]
    InvalidCron { expr: String },

    #[error("schedule '{name}' already exists")]
    DuplicateSchedule { name: String },

    #[error("resource conflict: {resource} is in use by '{holder}'")]
    ResourceConflict { resource: String, holder: String },
}
```

### Rules

1. **Never swallow errors.** If you handle an error, log it. If you can't handle it, propagate it.
2. **Structured errors for probes.** Probe failures are data, not crashes. Record them as measurement results with error context.
3. **Graceful degradation.** If a subsystem fails (e.g., iperf3 not installed, Tailscale not configured), log a warning and continue without it or fall back to the native engine.
4. **Timeouts everywhere.** Every network operation has an explicit timeout. No unbounded waits.

---

## Testing Strategy

### Test Pyramid

```
        ┌───────────┐
        │   Soak    │  7-14 day continuous runs (pre-release)
        ├───────────┤
        │Integration│  Real network scenarios, API contract tests
        ├───────────┤
        │   Unit    │  Logic, parsing, schema, verdict rules
        └───────────┘
```

### Unit Tests

- Live alongside the code in `#[cfg(test)]` modules.
- Cover: schema validation, probe result parsing, verdict rules, redaction routines, acceleration parity, cron expression parsing, iperf3 JSON parsing.
- Run with: `cargo nextest run`

### Integration Tests

- Live in `tests/`.
- Scenarios: LAN-only, DNS failure, captive portal, IPv6 broken, bufferbloat, packet loss simulation.
- Run with: `cargo nextest run --profile integration`

### Throughput Test Scenarios

- LAN throughput with mock iperf3 (inject canned JSON output)
- WAN throughput with simulated bandwidth limits (tc/netem in test environment)
- Thermal abort simulation (mock thermal zone readings)
- iperf3 missing/crashed gracefully falls back to native engine
- Multi-stream tests produce correct aggregate throughput
- 10GbE PCIe NIC detection on Pi 5 hardware

### Scheduler Test Scenarios

- Cron expression parsing: standard expressions, edge cases (@reboot, */5, ranges)
- Mutual exclusion: two speed tests submitted simultaneously; only one runs
- Priority preemption: user-triggered test preempts scheduled background test
- Missed-run detection: simulate device downtime across a scheduled window
- Schedule persistence: create schedules, restart daemon, verify schedules survive
- Dry-run accuracy: predicted schedule matches actual execution over 24h

### Soak Tests

- Live in `tests/soak/`.
- 7--14 day continuous run before any release.
- Validate: no disk fill, no memory leaks, clean reboot after power cuts, upgrade/rollback cycles, no scheduler drift, no test collisions.

### Coverage

- Target: 80%+ line coverage for core modules (`probes/`, `detect/`, `storage/`, `evidence/`, `throughput/`, `scheduler/`).
- Tool: `cargo llvm-cov`

### Pre-Commit Checks

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo nextest run
```

---

## CI/CD Pipeline

### On Every PR

1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo nextest run`
4. `cargo audit` (dependency vulnerability scan)
5. Build for `aarch64` (Pi 5 target)
6. License compliance check

### On Merge to `main`

1. All PR checks
2. Integration test suite (on Pi 5 hardware where available)
3. Build release artifacts
4. Tag as `nightly`

### On Release Tag

1. All checks + soak test results reviewed
2. Build signed release image
3. Publish to release channel (stable or beta)

### Self-Hosted Runner (Pi 5)
PacketParamedic uses a dedicated Pi 5 runner for native ARM64 testing.
See [CI Setup Guide](docs/CI_SETUP.md) for installation instructions.

---

## Security Practices

### Authentication & Authorization

- All API endpoints require authentication (token-based).
- Admin token generated on first boot; displayed once, stored hashed.
- BLE provisioning uses secure pairing with user confirmation (iOS companion app via Core Bluetooth, Android/Desktop via Web Bluetooth).

### Network Security

- Default: only local web UI port open (8080).
- No inbound WAN ports ever opened by PacketParamedic.
- Tailscale integration uses WireGuard (zero-trust, encrypted).
- All outbound measurement traffic uses standard protocols; no tunneling without user consent.
- iperf3 spawned as a child process with strict timeouts; never left running as a daemon.

### Data Handling

- Support bundles redact: MAC addresses (except OUI), internal IPs, SSIDs (configurable).
- No telemetry sent anywhere without explicit user opt-in.
- SQLite database encrypted at rest is optional but documented.

### Supply Chain

- `cargo audit` on every build.
- `cargo deny` for license compliance.
- Pin dependency versions in `Cargo.lock` (always committed).
- Minimal dependency tree; audit each new crate addition.

### Principle of Least Privilege

- Each systemd service runs under its own user with minimal capabilities.
- No service runs as root unless hardware access requires it (e.g., raw sockets for ICMP).
- Capabilities are granted explicitly (`CAP_NET_RAW`, `CAP_NET_ADMIN`) rather than running as root.

---

## Database & Storage

### SQLite Best Practices

- **WAL mode** enabled for concurrent readers + single writer.
- **Foreign keys** enforced (`PRAGMA foreign_keys = ON`).
- **Migrations** managed via versioned SQL files in `src/storage/migrations/`.
- **Prepared statements** for all queries (no string interpolation).
- **Retention policy:** Auto-prune data older than configurable threshold (default: 90 days). Run on a schedule, not on every write.
- **Disk guard:** Monitor available disk space. Stop writing at 90% full; alert at 80%.

### Schema Conventions

- All tables have `id INTEGER PRIMARY KEY`, `created_at TEXT` (ISO 8601), `updated_at TEXT`.
- Timestamps stored in UTC. Always.
- Enums stored as TEXT (human-readable), not integers.
- Indexes on columns used in WHERE clauses and JOINs.
- Schedule state stored in a `schedules` table with columns for name, cron expression, test type, enabled flag, and last-run timestamp.
- Throughput results stored with link speed, stream count, direction, and per-second samples for trending.

---

## API Design

### Conventions

- **Versioned:** All routes under `/api/v1/`.
- **RESTful:** Resources are nouns. Actions on resources use standard HTTP verbs.
- **JSON responses** with consistent envelope:

```json
{
  "data": { ... },
  "meta": {
    "timestamp": "2025-01-15T10:30:00Z",
    "version": "0.1.0"
  }
}
```

- **Error responses:**

```json
{
  "error": {
    "code": "PROBE_TIMEOUT",
    "message": "DNS probe timed out after 5000ms",
    "details": { ... }
  }
}
```

- **Pagination:** Cursor-based for timeline data. `?cursor=<id>&limit=50`.
- **Rate limiting:** Configurable per-endpoint. Default: 60 req/min for mutations, unlimited for reads.

### Key Endpoints

| Method | Path | Description |
|---|---|---|
| GET | `/api/v1/health` | Service health check |
| POST | `/api/v1/self-test` | Trigger hardware self-test |
| GET | `/api/v1/self-test/latest` | Latest self-test results |
| POST | `/api/v1/blame-check` | Run a blame check |
| GET | `/api/v1/incidents` | List incidents (paginated) |
| GET | `/api/v1/incidents/:id` | Incident detail with evidence |
| POST | `/api/v1/export/bundle` | Generate export bundle |
| GET | `/api/v1/probes/status` | Current probe schedule and status |
| POST | `/api/v1/speed-test` | Trigger a throughput test (LAN or WAN) |
| GET | `/api/v1/speed-test/latest` | Latest throughput test results |
| GET | `/api/v1/speed-test/history` | Historical throughput results (paginated) |
| GET | `/api/v1/schedules` | List all schedules |
| POST | `/api/v1/schedules` | Create a new schedule |
| GET | `/api/v1/schedules/:name` | Get schedule details |
| PUT | `/api/v1/schedules/:name` | Update a schedule |
| DELETE | `/api/v1/schedules/:name` | Delete a schedule |
| POST | `/api/v1/schedules/:name/enable` | Enable a schedule |
| POST | `/api/v1/schedules/:name/disable` | Disable a schedule |
| GET | `/api/v1/schedules/:name/history` | Execution history for a schedule |
| GET | `/api/v1/schedules/dry-run` | Preview upcoming scheduled runs |
| GET | `/api/v1/network/interfaces` | List network interfaces with speed capabilities |

---

## Observability

### Structured Logging

- Use `tracing` crate with `tracing-journald` subscriber in production.
- Use `tracing-subscriber` with `fmt` layer in development.
- Every log line includes: `timestamp`, `level`, `module`, `span` context.
- Log levels:
  - `ERROR`: Something is broken and needs attention.
  - `WARN`: Something unexpected but handled (e.g., probe timeout, missing optional hardware, thermal throttle during test).
  - `INFO`: Significant events (service start/stop, self-test complete, incident detected, scheduled test executed, throughput test result).
  - `DEBUG`: Detailed operation flow (probe results, SQL queries, scheduler decisions, iperf3 JSON output).
  - `TRACE`: Wire-level detail (packet contents, raw responses).

### Metrics (Future)

- Expose Prometheus-compatible metrics at `/metrics` (optional).
- Key metrics: probe latency histograms, incident count, disk usage, uptime, throughput test results, scheduler queue depth.

### Support Bundles

Always exportable, always working. A support bundle includes:
- Last 24h of journald logs (filtered to PacketParamedic units).
- Current config (redacted).
- Latest self-test report.
- Hardware inventory JSON (including 10GbE NIC status).
- Active schedule list and recent execution history.
- Disk and memory usage snapshot.

---

## Deployment & Updates

### Image Build

- Reproducible builds via a build script in `tools/`.
- Output: a `.img` file flashable to NVMe SSD (Pi 5 PCIe) or microSD, or a `.deb` package for existing installs.
- Build includes: compiled binaries, systemd units, default config, default schedule profiles, tmpfiles.d entries.
- Target: Pi 5 only. No multi-platform image builds.

### Update Strategy

- **Channels:** stable, beta, nightly.
- **Mechanism:** `packetparamedic-updater` systemd service checks for updates on a schedule.
- **Rollback:** Keep previous version on disk. If the new version fails health check within 5 minutes, roll back automatically.
- **A/B partitions (future):** For image-based updates, use two root partitions and swap on success.

### Systemd Integration

- `packetparamedic.service`: Main daemon (API + probes + scheduler + throughput + detection).
- `packetparamedic-updater.service`: Background update checker.
- Watchdog: systemd `WatchdogSec=30` with periodic health ping from the daemon.
- `RestartSec=5`, `Restart=on-failure` for all services.
- Throughput tests spawn iperf3 as a child process; never as a separate systemd service. The main daemon manages iperf3 lifecycle and enforces timeouts.
- The scheduler runs in-process within the main daemon (not a separate cron or systemd timer). This ensures bandwidth-aware coordination is centralized.

---

## Branch & Release Strategy

### Branches

| Branch | Purpose | Merges Into |
|---|---|---|
| `main` | Stable development trunk | -- |
| `feature/<name>` | Feature development | `main` via PR |
| `fix/<name>` | Bug fixes | `main` via PR |
| `release/<version>` | Release stabilization | `main` + tag |

### Versioning

- **Semantic versioning:** `MAJOR.MINOR.PATCH`
- `0.x.y` during initial development (breaking changes allowed on minor bumps).
- Pre-release: `-alpha.N`, `-beta.N`, `-rc.N`

### Release Checklist

- [ ] All CI checks pass
- [ ] Integration tests pass on Pi 5 hardware
- [ ] Soak test results reviewed (7+ days, no regressions)
- [ ] Scheduler runs without drift or collisions for full soak period
- [ ] Changelog updated
- [ ] Version bumped in `Cargo.toml`
- [ ] Release tag created and signed
- [ ] Release notes published

---

## Code Review Guidelines

### For Authors

1. **Keep PRs small.** Under 400 lines of diff is ideal. Split large features into stacked PRs.
2. **Write a clear description.** State what changed, why, and how to test it.
3. **Self-review first.** Read your own diff before requesting review.
4. **Link to the roadmap.** Reference the phase/checklist item this PR addresses.
5. **Pi 5 only.** Do not introduce compatibility shims for older hardware.

### For Reviewers

1. **Correctness first.** Does it do what it claims? Are edge cases handled?
2. **Security second.** Any new inputs validated? Any new network exposure? Is iperf3 spawned safely?
3. **Maintainability third.** Will a future developer understand this in 6 months?
4. **Style last.** `cargo fmt` and `clippy` handle most style issues. Don't nitpick what the tools already enforce.

### Approval Requirements

- 1 approval required for standard changes.
- 2 approvals required for: security-sensitive changes, database schema changes, systemd unit changes, dependency additions, scheduler coordination logic, BLE GATT protocol changes, iOS companion app releases.

<p align="center">
  <h1 align="center">PacketParamedic</h1>
  <p align="center">
    <strong>Appliance-grade network diagnostics for your home and small office.</strong>
  </p>
  <p align="center">
    <a href="https://blueoakcouncil.org/license/1.0.0"><img src="https://img.shields.io/badge/license-BlueOak--1.0.0-blue.svg" alt="License: Blue Oak 1.0.0"></a>
    <img src="https://img.shields.io/badge/platform-Raspberry%20Pi-c51a4a.svg" alt="Platform: Raspberry Pi">
    <img src="https://img.shields.io/badge/lang-Rust-orange.svg" alt="Language: Rust">
    <img src="https://img.shields.io/badge/UI-htmx-blueviolet.svg" alt="UI: htmx">
    <img src="https://img.shields.io/badge/storage-SQLite-green.svg" alt="Storage: SQLite">
    <img src="https://img.shields.io/badge/status-alpha-yellow.svg" alt="Status: Alpha">
  </p>
</p>

---

## What is PacketParamedic?

PacketParamedic is a Raspberry Pi-based network diagnostic appliance that answers one question with evidence:

> **"Is it my Wi-Fi, my router, or my ISP?"**

It runs unattended, collects structured measurements over time, detects anomalies, and produces shareable evidence bundles you can hand to your ISP or use to troubleshoot yourself.

### Key Capabilities

- **Blame attribution** -- Distinguishes LAN, Wi-Fi, router, DNS, and ISP issues with evidence.
- **Continuous monitoring** -- Scheduled ICMP, HTTP, DNS, and TCP probes build a baseline over days and weeks.
- **Incident detection** -- Statistical anomaly detection flags latency spikes, packet loss, route changes, and DNS shifts.
- **Evidence bundles** -- Export timestamped, redacted reports suitable for ISP support tickets.
- **Hardware self-test** -- Validates Pi hardware, Wi-Fi adapters, thermals, and power integrity before testing.
- **Appliance-grade reliability** -- Survives power cuts, manages disk space, and runs headless with zero maintenance.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  Raspberry Pi                   │
│                                                 │
│  ┌───────────┐  ┌───────────┐  ┌─────────────┐ │
│  │  Probes   │  │ Anomaly   │  │  Evidence    │ │
│  │  Engine   │→ │ Detector  │→ │  Builder    │ │
│  └─────┬─────┘  └───────────┘  └─────────────┘ │
│        │                                        │
│  ┌─────▼─────┐  ┌───────────┐  ┌─────────────┐ │
│  │  SQLite   │  │  axum     │  │  htmx UI    │ │
│  │  TSDB     │← │  API      │→ │  (SSR)      │ │
│  └───────────┘  └─────┬─────┘  └─────────────┘ │
│                       │                         │
│  ┌────────────────────┼───────────────────────┐ │
│  │ systemd units      │  journald logs        │ │
│  └────────────────────┴───────────────────────┘ │
│                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
│  │ Tailscale│  │   BLE    │  │  Cellular    │  │
│  │ (opt.)   │  │  (opt.)  │  │   (opt.)     │  │
│  └──────────┘  └──────────┘  └──────────────┘  │
└─────────────────────────────────────────────────┘
```

---

## Tech Stack

| Layer | Choice | Rationale |
|---|---|---|
| **OS** | Raspberry Pi OS Lite (Bookworm) | Best Pi hardware support; Wayland-ready for optional local UI |
| **Runtime** | Rust + Tokio + axum + tower | Lightweight async stack; low memory, safe concurrency |
| **Storage** | SQLite | Zero-ops local-first event store; crash-safe with WAL mode |
| **UI** | Server-rendered HTML + htmx | No SPA build pipeline; fast on low-power hardware |
| **Observability** | `tracing` + `tracing-journald` | Structured logs into journald; great for support bundles |
| **Services** | systemd units + tmpfiles.d | Appliance-grade supervision, easy rollback and diagnostics |
| **Remote admin** | Tailscale (optional) | Zero-trust, no inbound ports |
| **BLE** | BlueZ + bluer (optional) | Nearby provisioning and recovery |

---

## Getting Started

### Prerequisites

- Raspberry Pi 4 or 5 (2 GB+ RAM recommended)
- Raspberry Pi OS Lite (Bookworm, 64-bit)
- Rust toolchain (`rustup` -- see [rustup.rs](https://rustup.rs))
- SQLite 3.35+

### Build

```bash
# Clone the repository
git clone https://github.com/ParkWardRR/PacketParamedic.git
cd PacketParamedic

# Build in release mode
cargo build --release

# Run the self-test
./target/release/packetparamedic self-test
```

### Cross-compile for Raspberry Pi (from x86)

```bash
# Install the target
rustup target add aarch64-unknown-linux-gnu

# Build
cargo build --release --target aarch64-unknown-linux-gnu
```

### Run

```bash
# Start the daemon (uses systemd in production)
./target/release/packetparamedic serve

# Quick blame check from CLI
./target/release/packetparamedic blame-check
```

The web UI is available at `http://<pi-ip>:8080` once the server is running.

---

## Usage

### CLI

```bash
# Run hardware self-test
packetparamedic self-test

# Run a blame check ("Is it me or my ISP?")
packetparamedic blame-check

# Export an evidence bundle for your ISP
packetparamedic export-bundle --output report.zip

# Check service status
systemctl status packetparamedic
```

### API

All functionality is exposed via a local REST API:

```bash
# Trigger a blame check
curl http://localhost:8080/api/v1/blame-check

# Get the latest self-test report
curl http://localhost:8080/api/v1/self-test

# List recent incidents
curl http://localhost:8080/api/v1/incidents?limit=10
```

---

## Project Structure

```
PacketParamedic/
├── src/
│   ├── main.rs            # Entry point and CLI
│   ├── api/               # axum routes and handlers
│   ├── probes/            # ICMP, HTTP, DNS, TCP probe implementations
│   ├── storage/           # SQLite schema, queries, migrations
│   ├── detect/            # Anomaly detection and incident grouping
│   ├── evidence/          # Report and bundle generation
│   ├── selftest/          # Hardware and Wi-Fi self-test
│   └── accel/             # Acceleration manager (NEON/GPU fallback)
├── templates/             # HTML templates for htmx UI
├── static/                # CSS, minimal JS
├── systemd/               # Unit files for deployment
├── tests/                 # Integration and soak test harnesses
├── roadmap.md             # Development roadmap (checklist)
├── dev_plan.md            # Development plan and best practices
└── README.md
```

---

## Contributing

Contributions are welcome. Please read [`dev_plan.md`](dev_plan.md) for coding standards, branch conventions, and testing requirements before submitting a PR.

### Quick Guidelines

1. **Fork and branch** from `main`.
2. **Write tests** for new functionality.
3. **Run `cargo clippy` and `cargo fmt`** before committing.
4. **Keep PRs focused** -- one feature or fix per PR.
5. **Document "why"** in commit messages, not "what".

---

## Security

PacketParamedic is designed as a network appliance with a strong security posture:

- No default passwords; authentication required for all API access.
- Minimal open ports (only the local web UI port by default).
- All actions are auditable via journald.
- Optional features (monitor mode, injection testing) require explicit opt-in.
- Tailscale integration uses zero-trust networking with no inbound WAN ports.

To report a security issue, please open a private advisory on GitHub.

---

## License

[Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0) (SPDX: `BlueOak-1.0.0`)

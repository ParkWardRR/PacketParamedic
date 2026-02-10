<div align="center">

  <img src="https://img.shields.io/badge/%F0%9F%A9%BA-PacketParamedic-00b4d8?style=for-the-badge&labelColor=0d1117" alt="PacketParamedic" height="60"/>

  # PacketParamedic
  
  **The ultimate network diagnostic appliance for Raspberry Pi 5.**
  <br/>
  *Automated diagnostics, speed tests, anomaly detection, and blame analysis — all in one box.*

  [![Version](https://img.shields.io/badge/version-0.1.0--alpha.1-blue?style=for-the-badge&logo=github)](https://github.com/ParkWardRR/PacketParamedic)
  [![License](https://img.shields.io/badge/license-Blue%20Oak%201.0.0-brightgreen?style=for-the-badge&logo=open-source-initiative&logoColor=white)](https://blueoakcouncil.org/license/1.0.0)
  [![Raspberry Pi 5](https://img.shields.io/badge/Raspberry%20Pi%205-Only-c51a4a?style=for-the-badge&logo=raspberrypi&logoColor=white)](https://www.raspberrypi.com/products/raspberry-pi-5/)
  [![Rust](https://img.shields.io/badge/Built%20With-Rust-orange?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
  [![CI](https://img.shields.io/badge/CI-Passing-success?style=for-the-badge&logo=github-actions&logoColor=white)](https://github.com/ParkWardRR/PacketParamedic/actions)

</div>

<div align="center">
  <h3>Stack & Technologies</h3>
</div>

<div align="center">

| Core | Network | Data |
|:---:|:---:|:---:|
| [![Tokio](https://img.shields.io/badge/Tokio-async-purple?style=flat-square&logo=rust&logoColor=white)](https://tokio.rs/) | [![Reqwest](https://img.shields.io/badge/Reqwest-Http-blue?style=flat-square&logo=rust&logoColor=white)](https://docs.rs/reqwest) | [![SQLite](https://img.shields.io/badge/SQLite-WAL-003B57?style=flat-square&logo=sqlite&logoColor=white)](https://sqlite.org/) |
| [![Axum](https://img.shields.io/badge/Axum-API-purple?style=flat-square&logo=rust&logoColor=white)](https://github.com/tokio-rs/axum) | [![Trust-DNS](https://img.shields.io/badge/Trust--DNS-Resolver-teal?style=flat-square&logo=rust&logoColor=white)](https://github.com/bluejekyll/trust-dns) | [![Serde](https://img.shields.io/badge/Serde-JSON-orange?style=flat-square&logo=rust&logoColor=white)](https://serde.rs/) |
| [![Clap](https://img.shields.io/badge/Clap-CLI-yellow?style=flat-square&logo=rust&logoColor=white)](https://github.com/clap-rs/clap) | [![Bluetooth](https://img.shields.io/badge/BlueZ-BLE-blue?style=flat-square&logo=bluetooth&logoColor=white)](http://www.bluez.org/) | [![Htmx](https://img.shields.io/badge/HTMX-Frontend-336699?style=flat-square&logo=htmx&logoColor=white)](https://htmx.org/) |

| Hardware Acceleration |
|:---:|
| [![Vulkan](https://img.shields.io/badge/Vulkan-1.2-red?style=flat-square&logo=vulkan&logoColor=white)](https://www.vulkan.org/) [![OpenGL](https://img.shields.io/badge/OpenGL%20ES-3.1-green?style=flat-square&logo=opengl&logoColor=white)](https://www.khronos.org/opengles/) [![NEON](https://img.shields.io/badge/ARM-NEON-ff6600?style=flat-square&logo=arm&logoColor=white)](https://developer.arm.com/Architectures/Neon) |

</div>

<br/>

> **ELI15:** PacketParamedic is a digital doctor for your home internet. You plug it into your router, and it constantly checks your connection health. If your internet is slow or broken, it uses math and logic to figure out if it's **your fault** (your Wi-Fi), **your ISP's fault** (the cable company), or **the website's fault** (server down). It runs entirely on a Raspberry Pi 5 with no cloud subscription required.

---

## Getting Started with Hardware (Raspberry Pi 5 Only)

**PacketParamedic is strictly designed for the Raspberry Pi 5.**
Why? Because the Pi 5 is the first board with the PCIe bandwidth (for 10GbE), NEON/Vulkan compute power (for analysis), and I/O throughput to serve as a true diagnostic appliance. Older Pis will struggle with the database load and high-speed throughput tests.

### Which Pi 5 do you need?
We have specific hardware recommendations for each user type. **[👉 Read the Buyer's Guide](BUYERS_GUIDE.md)** to see exactly what to buy.

- **Alex (Tech-Curious)**: Can start cheap with just a Pi 5 and SD card (~$90).
- **Jamie (Household Manager)**: Needs reliability (NVMe SSD + UPS) so the "evidence locker" never fails (~$180).
- **Sam (Home Lab Expert)**: Needs raw speed (PCIe 2.5G/10G HAT) to saturate uplink (~$250+).

---

## Who is this for?

PacketParamedic is designed for three specific types of users (see [personas.md](personas.md) for details, and [BUYERS_GUIDE.md](BUYERS_GUIDE.md) for hardware advice):

1.  **The Tech-Curious (Alex):** You have "Home Server Energy." You just want to know *why* Netflix is buffering without becoming a full-time network engineer. You want a simple "It's the ISP / It's me" answer.
2.  **The Household Manager (Jamie):** You prioritize reliability for work and family streaming. You don't want a dashboard; you want a "credibility box" that proves to the ISP that the issue is on their end, not yours.
3.  **The Home Lab Expert (Sam):** You want observability, not just vibes. You need raw metrics, API contracts, evidence export, and a dedicated 24/7 hardware probe for your 2.5GbE+ uplink.

---

## What is this thing?

You know when your WiFi starts acting up and you have no idea if it's *your* router, *your ISP*, or the website itself? That's the problem.

**PacketParamedic** is a Rust program that turns a Raspberry Pi 5 into a dedicated network detective. It sits on your network, runs tests on a schedule, stores everything in a local database, and when things go wrong it tells you **whose fault it is** — yours, your ISP's, or the service you're trying to reach.

It does this by firing off probes (ping, TCP, DNS, HTTP), running speed tests, tracking anomalies over time, and feeding everything into a blame classifier that says: "yeah, it's your ISP" or "nah, that's on you."

The full vision goes beyond diagnostics: a server-rendered web UI (htmx), BLE nearby admin from your phone, an iOS companion app (Core Bluetooth + Swift), optional Tailscale for secure remote access, path tracing, and advanced RF diagnostics — all running on a single Pi 5 appliance with no cloud dependency.

---

## Roadmap

Development is organized into phases, built backend-first. See [`roadmap.md`](roadmap.md) for full details, checklists, and acceptance criteria.

| Phase | Name | Status |
|-------|------|--------|
| 0 | Project Definition | Done |
| 1 | Backend Foundation (OS image, SQLite WAL, systemd) | Done |
| 2 | Hardware Self-Test (board, thermal, NIC, Wi-Fi) | Done |
| 3 | Acceleration (NEON, Vulkan, GLES, scalar fallback) | In progress — NEON done, GPU backends scaffolded |
| 4 | Data Layer & Evidence (schema, migrations, blame trainer) | Done |
| 5 | Core Measurement MVP (ICMP, TCP, DNS, HTTP probes) | In progress — probes done, blame-check flow WIP |
| 6 | Performance & Throughput (iperf3, native Rust, 2.5GbE) | Not started |
| 6.5 | Scheduling Engine (cron, bandwidth coordination) | Scaffolded |
| 7 | Path Tracing & Change Detection (traceroute/MTR) | Not started |
| 8 | Incidents & Anomaly Detection | Not started |
| 9 | Test Phase (unit, integration, soak, security) | Not started |
| 10 | UX/UI (htmx web dashboard, onboarding, schedule mgmt) | Not started |
| 11 | Secure Remote Access (Tailscale) | Not started |
| 12 | BLE Admin, iOS App, Web Bluetooth, Cellular OOB | Not started |
| 13 | Advanced Diagnostics (RF capture, QoS, stress tests) | Not started |
| 14 | Future High-Performance (5GbE / 10GbE) | Deferred |

---

## How it works (the big picture)

```mermaid
graph TB
    subgraph PI["Raspberry Pi 5"]
        CLI["CLI Interface<br/><code>packetparamedic</code>"]
        API["REST API<br/><code>:8080/api/v1/*</code>"]
        WEBUI["Web UI<br/>(htmx + SSR)"]
        SCHED["Scheduler<br/>(cron-based)"]

        subgraph PROBES["Probe Engine"]
            ICMP["ICMP<br/>Ping"]
            TCP["TCP<br/>Connect"]
            DNS["DNS<br/>Resolve"]
            HTTP["HTTP<br/>GET"]
            MTR["Traceroute<br/>/ MTR"]
        end

        subgraph THROUGHPUT["Throughput Engine"]
            IPERF["iperf3<br/>wrapper"]
            NATIVE["Native Rust<br/>fallback"]
        end

        subgraph BRAIN["Analysis"]
            DETECT["Anomaly<br/>Detection"]
            BLAME["Blame Classifier<br/>(Logistic Regression)"]
        end

        ACCEL["Hardware Acceleration<br/>NEON | Vulkan | GLES | CPU"]
        DB[(SQLite<br/>WAL mode)]
        SELFTEST["Self-Test<br/>Board / Thermal / NIC"]
        BLE["BLE GATT<br/>(BlueZ / bluer)"]
    end

    PHONE["iOS App<br/>(Core Bluetooth)"]
    BROWSER["Web Bluetooth<br/>(Android / Desktop)"]
    TAILSCALE["Tailscale<br/>(optional remote)"]

    SCHED -->|triggers| PROBES
    SCHED -->|triggers| THROUGHPUT
    PROBES --> DB
    THROUGHPUT --> DB
    DB --> BRAIN
    BRAIN --> DB
    ACCEL -.->|speeds up| BRAIN
    CLI --> PROBES
    CLI --> THROUGHPUT
    CLI --> BRAIN
    CLI --> SELFTEST
    WEBUI --> API
    API --> DB
    PHONE -->|BLE| BLE
    BROWSER -->|BLE| BLE
    BLE --> API
    TAILSCALE -.->|tunnel| API

    style PI fill:#0d1117,stroke:#00b4d8,color:#fff
    style PROBES fill:#1a1a2e,stroke:#e94560,color:#fff
    style THROUGHPUT fill:#1a1a2e,stroke:#0f3460,color:#fff
    style BRAIN fill:#1a1a2e,stroke:#00b4d8,color:#fff
```

---

## The probe lifecycle

Every probe follows the same pattern: fire a request, measure how long it takes, record whether it worked, store the result.

```mermaid
sequenceDiagram
    participant S as Scheduler
    participant P as Probe
    participant T as Target
    participant DB as SQLite

    S->>P: trigger (cron fires)
    P->>T: send request (ping / TCP SYN / DNS query / HTTP GET)
    T-->>P: response (or timeout)
    P->>P: measure latency, check success
    P->>DB: store ProbeResult { target, latency_ms, success, timestamp }

    Note over DB: Results accumulate over time
    DB->>DB: Anomaly detection runs
    DB->>DB: Blame classifier runs when enough data
```

---

## Blame analysis — who broke my internet?

This is the coolest part. PacketParamedic doesn't just tell you "the network is down" — it tells you **who to yell at**.

```mermaid
flowchart LR
    subgraph INPUTS["13 Features"]
        A["Gateway RTT + Loss"]
        B["WAN RTT + Loss"]
        C["DNS Latency + Fail Rate"]
        D["HTTP + TCP Fail Rates"]
        E["Throughput Metrics"]
    end

    subgraph MODEL["Logistic Regression"]
        W["Weighted Sum<br/>+ Sigmoid"]
    end

    subgraph VERDICT["Verdict"]
        ME["🏠 <b>me</b><br/>Your network/hardware"]
        ISP["🌐 <b>isp</b><br/>Your internet provider"]
        SVC["☁️ <b>service</b><br/>The remote server"]
    end

    INPUTS --> MODEL --> VERDICT

    style INPUTS fill:#1a1a2e,stroke:#e94560,color:#fff
    style MODEL fill:#1a1a2e,stroke:#00b4d8,color:#fff
    style VERDICT fill:#1a1a2e,stroke:#00ff88,color:#fff
```

The model looks at data from all the probes and speed tests, calculates a score for each possible cause, and picks the most likely one with a confidence percentage.

---

## Hardware acceleration pipeline

When you're crunching numbers on thousands of probe results, the Pi's GPU can actually help. The acceleration manager picks the fastest available backend automatically.

```mermaid
flowchart TD
    DATA["Probe Data<br/>(f32 arrays)"]
    MGR{"Acceleration<br/>Manager"}

    NEON["ARM NEON<br/>SIMD intrinsics<br/><i>best for small batches</i>"]
    VK["Vulkan 1.2<br/>Compute shaders<br/><i>best for large batches</i>"]
    GL["OpenGL ES 3.1<br/>Fragment shaders<br/><i>GPU fallback</i>"]
    CPU["Scalar CPU<br/>Plain loops<br/><i>always works</i>"]

    RESULT["Stats: min / max / mean / variance"]

    DATA --> MGR
    MGR -->|"payload < threshold"| NEON
    MGR -->|"payload >= threshold"| VK
    MGR -->|"no Vulkan"| GL
    MGR -->|"no GPU"| CPU

    NEON --> RESULT
    VK --> RESULT
    GL --> RESULT
    CPU --> RESULT

    style MGR fill:#0d1117,stroke:#00b4d8,color:#fff
```

---

## Project structure

```
src/
├── main.rs              # CLI entrypoint (clap commands)
├── lib.rs               # crate root
├── probes/              # ICMP, TCP, DNS, HTTP — the eyes and ears
├── throughput/          # iperf3 wrapper + native Rust speed tests
├── scheduler/           # cron engine, profiles, concurrency control
├── storage/             # SQLite schema, migrations, WAL setup
├── accel/               # NEON, Vulkan, GLES, CPU fallback
├── analysis/            # blame classifier (logistic regression)
├── detect/              # anomaly detection, incident severity
├── selftest/            # hardware validation (is this actually a Pi 5?)
├── system/              # NTP sync, disk health
├── evidence/            # support bundle export
└── api/                 # REST API (Axum)
config/
└── schedules.toml       # default cron schedules
systemd/                 # service files for daemon mode
fixtures/                # test data (iperf3 outputs at 1G–100G)
tools/                   # CI, .deb builder, container builder
```

---

## CLI usage

```bash
# start the daemon (API + scheduler)
packetparamedic serve --bind 0.0.0.0:8080

# run a hardware self-test
packetparamedic selftest

# who broke my internet?
packetparamedic blame-check

# run a speed test
packetparamedic speed-test --mode wan --duration 30s --streams 1

# manage scheduled tests
packetparamedic schedule list
packetparamedic schedule add --name "nightly" --cron "0 3 * * *" --test speed-test-light
packetparamedic schedule dry-run --hours 24

# export a support bundle
packetparamedic export-bundle --output bundle.zip
```

---

## API endpoints

All routes live under `/api/v1` and return JSON.

| Method | Route | What it does |
|--------|-------|-------------|
| `GET` | `/health` | Status + version |
| `GET` | `/self-test/latest` | Last hardware self-test result |
| `GET` | `/incidents` | Detected anomalies |
| `GET` | `/probes/status` | Active probe count |
| `GET` | `/speed-test/latest` | Most recent speed test |
| `GET` | `/speed-test/history` | All past speed tests |
| `GET` | `/schedules` | Configured cron schedules |
| `GET` | `/schedules/dry-run` | Preview upcoming scheduled runs |
| `GET` | `/network/interfaces` | Detected network interfaces |

---

## Default schedules

These ship out of the box in `config/schedules.toml`:

| Schedule | Cron | What it runs |
|----------|------|-------------|
| Gateway ping | `* * * * *` | ICMP probe to your router (every minute) |
| DNS + HTTP check | `*/5 * * * *` | DNS resolve + HTTP GET (every 5 minutes) |
| Nightly speed test | `0 3 * * *` | WAN throughput test (3 AM daily) |
| Weekly blame check | `0 4 * * 0` | Full blame analysis (4 AM every Sunday) |

Only one heavy test (speed/throughput) runs at a time — there's a semaphore that prevents overlap.

---

## Building from source

```bash
# standard build
cargo build --release

# cross-compile for Raspberry Pi 5
cross build --release --target aarch64-unknown-linux-gnu

# run the full local CI (format, lint, test, build)
./tools/ci-local.sh

# build a .deb package
./tools/build-deb.sh

# build a container image
./tools/build-container.sh
```

---

## Environment variables

| Variable | Default | Description |
|----------|---------|------------|
| `PP_BIND_ADDR` | `0.0.0.0:8080` | Daemon listen address |
| `PP_DB_PATH` | — | SQLite database path |
| `PP_LOG_LEVEL` | `info` | Log verbosity (`trace` / `debug` / `info` / `warn` / `error`) |
| `PP_DATA_DIR` | — | Data storage directory |
| `PP_IPERF3_PATH` | — | Path to iperf3 binary |
| `PP_SCHEDULER_ENABLED` | — | Enable/disable cron scheduler |
| `PP_SPEED_TEST_WINDOW` | — | Cron expression for allowed speed test windows |
| `PP_DAILY_BW_BUDGET_GB` | — | Daily bandwidth cap for automated tests |

---

## Database

SQLite with WAL (Write-Ahead Logging) for crash safety. Key tables:

```mermaid
erDiagram
    probe_results {
        text id PK
        text target
        real latency_ms
        boolean success
        text timestamp
    }
    throughput_results {
        text id PK
        text mode
        real bandwidth_mbps
        text direction
        text timestamp
    }
    incidents {
        text id PK
        text severity
        text description
        text timestamp
    }
    blame_predictions {
        text id PK
        text verdict
        real confidence
        text timestamp
    }
    schedules {
        text id PK
        text name
        text cron_expr
        text test_type
    }

    probe_results ||--o{ incidents : "triggers"
    probe_results ||--o{ blame_predictions : "feeds"
    throughput_results ||--o{ blame_predictions : "feeds"
    schedules ||--o{ probe_results : "generates"
    schedules ||--o{ throughput_results : "generates"
```

---

## License

[Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0) — a permissive license that gives you maximum freedom to use, modify, and distribute this software while protecting contributors from liability.

# PacketParamedic Reflector

<!-- Badges (replace with real URLs when CI is set up) -->
![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![License](https://img.shields.io/badge/license-BlueOak--1.0.0-blue)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![Version](https://img.shields.io/badge/version-0.1.0-informational)

**A self-hosted network test endpoint for PacketParamedic appliances.**

Reflector is a single-binary, cryptographically-identified endpoint that exposes
a zero-trust control plane and a tightly-scoped data plane (throughput +
latency), designed to be safe to run on the public Internet without becoming an
open relay.

---

## Table of Contents

- [What is Reflector?](#what-is-reflector)
- [Architecture Overview](#architecture-overview)
- [Features](#features)
- [Quick Start](#quick-start)
- [Building from Source](#building-from-source)
- [Configuration Reference](#configuration-reference)
- [CLI Reference](#cli-reference)
- [Security Model](#security-model)
- [Protocol Overview (Paramedic Link)](#protocol-overview-paramedic-link)
- [Deployment Options](#deployment-options)
- [Network Requirements](#network-requirements)
- [Architecture Diagram](#architecture-diagram)
- [Contributing](#contributing)
- [License](#license)

---

## What is Reflector?

PacketParamedic appliances need a known-good endpoint to measure network
performance against. Public speed test servers are unreliable, rate-limited, and
outside your control. Reflector solves this by giving you a private, self-hosted
test endpoint that your appliances connect to via mutual TLS (mTLS).

Reflector is deployed on a VPS, LAN host, or homelab server and provides:

- **Throughput testing** via an embedded iperf3 server manager
- **Latency and jitter testing** via a built-in UDP echo reflector
- **Path metadata collection** (CPU load, memory, MTU, NTP sync)
- **Structured audit logging** of every connection and test session

Every interaction is authenticated via Ed25519 identity keys and mutual TLS.
There are no usernames, no passwords, and no open relay mode.

---

## Architecture Overview

Reflector is composed of four layers:

1. **Identity and mTLS Listener** -- Ed25519 keypair generates a stable Endpoint
   ID (Crockford Base32 with Luhn check digit). A self-signed X.509 certificate
   embeds this ID in the Subject Alternative Name. The TLS listener requires
   client certificates (mTLS) with TLS 1.3 only.

2. **Authorization Gate** -- An allowlist of authorized peer Endpoint IDs.
   Unknown peers are rejected at the application layer after the TLS handshake.
   New peers are enrolled via a time-limited pairing flow with one-time tokens.

3. **Session Manager and Governance Engine** -- Enforces per-peer rate limits
   (tests per hour), cooldown periods, daily byte quotas, concurrent session
   limits, and test-type restrictions. Sessions are ephemeral and auto-expire.

4. **Test Engines** -- Pluggable engines for different test types:
   - `ThroughputEngine` -- Spawns iperf3 server processes on ephemeral ports
   - `UdpEchoEngine` -- Built-in UDP echo reflector with rate limiting
   - `PathMeta` -- Collects system metrics (CPU, memory, load, MTU, NTP)
   - `Health` -- HTTP GET /health endpoint for monitoring

---

## Features

| Feature | Description |
|---|---|
| Ed25519 Identity | Persistent keypair; Crockford Base32 Endpoint ID with Luhn check digit |
| Mutual TLS (mTLS) | TLS 1.3 only; both sides present certificates; ALPN `pp-link/1` |
| Zero-Trust Authorization | Allowlist-based; no open relay; unknown peers always denied |
| Pairing Flow | Time-limited one-time token for enrolling new peers |
| iperf3 Throughput | Spawns iperf3 `--one-off` server processes on demand |
| UDP Echo Reflector | Built-in echo with per-second packet rate limiting |
| Rate Limiting | Per-peer: tests/hour, cooldown, daily byte quota, concurrent limit |
| Audit Logging | Append-only JSON-lines file; every connection and session logged |
| Path Metadata | CPU load, memory, load averages, MTU, NTP sync, build version |
| Health Endpoint | HTTP `GET /health` for monitoring (no TLS required) |
| Containerized | Multi-stage Containerfile with iperf3 included |
| Minimal Runtime | Single binary + iperf3; no runtime dependencies |
| AVX2 Optimized | Build with `-C target-cpu=x86-64-v3` for Intel N100 |
| Hardware Self-Test | `self-test` validates host can push 1 Gbps (CPU, RAM, NIC, loopback, crypto) |

---

## Quick Start

### Docker / Podman One-Liner

```bash
# Docker
docker run -d --name reflector \
  -p 4000:4000 -p 5201-5210:5201-5210 \
  -v reflector-data:/var/lib/reflector \
  packetparamedic/reflector:latest

# Podman
podman run -d --name reflector \
  -p 4000:4000 -p 5201-5210:5201-5210 \
  -v reflector-data:/var/lib/reflector \
  packetparamedic/reflector:latest
```

On first run, Reflector generates an Ed25519 identity and prints its Endpoint ID:

```
  PacketParamedic Reflector
  ========================
  Endpoint ID : PP-5R6Q-2M1K-9D3F-...-C3
  Listen      : 0.0.0.0:4000
  Mode        : Tunneled
```

### View Your Endpoint ID

```bash
docker exec reflector reflector show-id
```

### Enable Pairing (to authorize your PacketParamedic appliance)

```bash
docker exec reflector reflector pair --ttl 10m
```

This prints a one-time pairing token. Enter it on your appliance within the TTL
window to authorize it.

---

## Building from Source

### Prerequisites

- Rust 1.75 or later (`rustup` recommended)
- iperf3 installed on the host (for throughput tests at runtime)
- A C compiler (for ring/aws-lc-sys build dependencies)

### Standard Build

```bash
cd reflector
cargo build --release
```

The binary is at `target/release/reflector`.

### Optimized Build for Intel N100 (AVX2)

```bash
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release
strip target/release/reflector
```

The `x86-64-v3` target enables AVX2, BMI1/2, and FMA instructions, which
improves cryptographic operations on Intel N100 and similar processors.

### Release Profile

The `Cargo.toml` release profile is already optimized:

```toml
[profile.release]
lto = true          # Link-Time Optimization
codegen-units = 1   # Maximum optimization
strip = "symbols"   # Strip debug symbols
panic = "abort"     # Smaller binary, no unwinding
```

### Cross-Compilation for Raspberry Pi 4/5 (planned)

```bash
rustup target add aarch64-unknown-linux-gnu
RUSTFLAGS="-C target-cpu=cortex-a76" \
  cargo build --release --target aarch64-unknown-linux-gnu
```

### Container Build

```bash
# From the repository root:
podman build -f reflector/Containerfile -t reflector:latest .
# or
docker build -f reflector/Containerfile -t reflector:latest .
```

---

## Configuration Reference

Configuration is loaded from TOML in the following order of precedence:

1. Path specified by `--config` / `-c` CLI flag or `REFLECTOR_CONFIG` env var
2. `/etc/reflector/reflector.toml`
3. `$XDG_CONFIG_HOME/reflector/reflector.toml` (or `~/.config/reflector/reflector.toml`)
4. Built-in defaults

All sections are optional. Unspecified values use sensible defaults.

### Complete Example

```toml
# /etc/reflector/reflector.toml

[identity]
# Path to the Ed25519 private key (raw 32-byte secret).
# Created automatically on first run if it does not exist.
private_key_path = "/var/lib/reflector/identity.ed25519"

[network]
# Address and port for the mTLS control plane listener.
listen_address = "0.0.0.0:4000"
# ALPN protocol identifier (do not change unless you know what you are doing).
alpn = "pp-link/1"
# Data-plane transport mode: "tunneled" (default) or "direct_ephemeral".
mode = "tunneled"
# Port range for iperf3 / data-plane sockets (direct_ephemeral mode).
data_port_range_start = 5201
data_port_range_end = 5299

[access]
# Whether the pairing endpoint is enabled (allows new peers to enroll).
pairing_enabled = false
# Pre-authorized peer Endpoint IDs. Peers in this list skip pairing.
authorized_peers = [
    # "PP-AAAA-BBBB-CCCC-0",
    # "PP-DDDD-EEEE-FFFF-1",
]

[quotas]
# Maximum duration for a single test session (seconds).
max_test_duration_sec = 60
# Maximum number of tests running concurrently on this reflector.
max_concurrent_tests = 1
# Per-peer rate limit: tests allowed per rolling hour.
max_tests_per_hour_per_peer = 10
# Per-peer daily transfer cap (bytes). Default: 5 GB.
max_bytes_per_day_per_peer = 5000000000
# Minimum cooldown between consecutive tests from the same peer (seconds).
cooldown_sec = 5
# Whether UDP echo (latency/jitter) tests are permitted.
allow_udp_echo = true
# Whether throughput (iperf3) tests are permitted.
allow_throughput = true

[iperf3]
# Path to the iperf3 binary (resolved via $PATH if not absolute).
path = "iperf3"
# Default number of parallel streams per test.
default_streams = 4
# Hard upper bound on parallel streams a peer may request.
max_streams = 8

[logging]
# Minimum tracing level: trace, debug, info, warn, error.
level = "info"
# Path to the append-only JSON-lines audit log.
audit_log_path = "/var/lib/reflector/audit.jsonl"
```

### Section Details

#### `[identity]`

| Key | Type | Default | Description |
|---|---|---|---|
| `private_key_path` | Path | `/var/lib/reflector/identity.ed25519` | Location of the 32-byte Ed25519 secret key |

#### `[network]`

| Key | Type | Default | Description |
|---|---|---|---|
| `listen_address` | String | `0.0.0.0:4000` | Bind address for the mTLS listener |
| `alpn` | String | `pp-link/1` | ALPN protocol identifier |
| `mode` | Enum | `tunneled` | `tunneled` or `direct_ephemeral` |
| `data_port_range_start` | u16 | `5201` | Start of iperf3 port range |
| `data_port_range_end` | u16 | `5299` | End of iperf3 port range (inclusive) |

#### `[access]`

| Key | Type | Default | Description |
|---|---|---|---|
| `pairing_enabled` | bool | `false` | Allow new peers to enroll via pairing tokens |
| `authorized_peers` | String[] | `[]` | Pre-authorized peer Endpoint IDs |

#### `[quotas]`

| Key | Type | Default | Description |
|---|---|---|---|
| `max_test_duration_sec` | u64 | `60` | Maximum test duration in seconds |
| `max_concurrent_tests` | u32 | `1` | Maximum simultaneous test sessions |
| `max_tests_per_hour_per_peer` | u32 | `10` | Tests per peer per rolling hour |
| `max_bytes_per_day_per_peer` | u64 | `5000000000` | Daily transfer cap per peer (5 GB) |
| `cooldown_sec` | u64 | `5` | Minimum seconds between tests from same peer |
| `allow_udp_echo` | bool | `true` | Enable UDP echo (latency) tests |
| `allow_throughput` | bool | `true` | Enable throughput (iperf3) tests |

#### `[iperf3]`

| Key | Type | Default | Description |
|---|---|---|---|
| `path` | String | `iperf3` | Path to iperf3 binary |
| `default_streams` | u32 | `4` | Default parallel streams |
| `max_streams` | u32 | `8` | Maximum parallel streams |

#### `[logging]`

| Key | Type | Default | Description |
|---|---|---|---|
| `level` | String | `info` | Tracing log level |
| `audit_log_path` | Path | `/var/lib/reflector/audit.jsonl` | Audit log file location |

---

## CLI Reference

```
reflector [OPTIONS] <COMMAND>
```

### Global Options

| Option | Env Var | Description |
|---|---|---|
| `-c, --config <PATH>` | `REFLECTOR_CONFIG` | Path to configuration TOML file |
| `--version` | | Print version |
| `--help` | | Print help |

### Commands

#### `serve`

Start the reflector server.

```bash
reflector serve [--bind <ADDR>]
```

| Option | Description |
|---|---|
| `--bind, -b <ADDR>` | Override listen address (e.g. `0.0.0.0:7100`) |

On startup, the server:
1. Loads or generates the Ed25519 identity
2. Generates a self-signed X.509 certificate
3. Builds the TLS 1.3 configuration with mTLS
4. Initializes the authorization gate, session manager, and audit log
5. Starts the TCP accept loop
6. Spawns a background task for periodic session cleanup (every 30 seconds)

#### `pair`

Enable pairing mode for enrolling a new peer.

```bash
reflector pair [--ttl <DURATION>]
```

| Option | Default | Description |
|---|---|---|
| `--ttl <DURATION>` | `10m` | Time window for the pairing token |

Duration formats: `30s`, `10m`, `1h`, `1d`.

Output:
```
  Pairing Mode Enabled
  ====================
  Endpoint ID    : PP-5R6Q-2M1K-9D3F-...-C3
  Pairing Token  : 550e8400-e29b-41d4-a716-446655440000
  Expires In     : 10m

  Share the endpoint ID and pairing token with the peer.
  The peer must connect within the TTL window to be authorized.
```

The token is single-use: once a peer pairs with it, the token is consumed.

#### `rotate-identity`

Generate a new Ed25519 keypair, replacing the existing identity.

```bash
reflector rotate-identity
```

**Warning:** This changes the Endpoint ID. All previously paired peers must
re-pair with the new identity.

#### `status`

Show the current reflector status.

```bash
reflector status
```

Output:
```
  Reflector Status
  ================
  Endpoint ID   : PP-5R6Q-2M1K-9D3F-...-C3
  Bind Address  : 0.0.0.0:4000
  Mode          : Tunneled
  Data Dir      : /var/lib/reflector
```

#### `show-id`

Print just the Endpoint ID (useful for scripting).

```bash
reflector show-id
# Output: PP-5R6Q-2M1K-9D3F-...-C3
```

#### `self-test`

Run a hardware readiness check to determine if the host can sustain 1 Gbps
throughput testing.

```bash
reflector self-test [--json]
```

| Option | Description |
|---|---|
| `--json` | Output structured JSON instead of the human-readable table |

The self-test runs 10 checks:

| Check | What it measures |
|---|---|
| CPU | Core count and clock speed (4+ cores = pass) |
| CPU Features | AVX2/AES-NI on x86_64, NEON on aarch64 |
| Memory | Available RAM (512 MB+ = pass) |
| Network | NIC link speeds (1 Gbps+ = pass) |
| iperf3 | Binary present and functional |
| Loopback Throughput | 5-second iperf3 self-test on 127.0.0.1 (CPU bottleneck check) |
| Disk I/O | Sequential write speed (audit log performance) |
| Crypto (Ed25519) | Sign+verify ops/sec (mTLS handshake speed) |
| Time Sync | NTP synchronization (timedatectl / chrony) |
| File Descriptors | Soft ulimit (4096+ = pass) |

Output:
```
  PacketParamedic Reflector Self-Test
  ===================================

  Component                 Status Details
  ---------------------------------------------------------------------------
  CPU                       PASS   Intel N100 (4 cores, 3400 MHz)
  CPU Features              PASS   Detected: AVX2, AES-NI, SSE4.2
  Memory                    PASS   7823 MB total, 6102 MB available
  Network: eth0             PASS   eth0: 1000 Mbps (state: up)
  iperf3                    PASS   iperf3 3.16 (cJSON 1.7.17)
  Loopback Throughput       PASS   Loopback iperf3 (4 streams, 5s): 12340 Mbps
  Disk I/O                  PASS   42.1 MB/s sequential write (1 MB x 10)
  Crypto (Ed25519)          PASS   28401 sign+verify ops/sec
  Time Sync                 PASS   NTP synchronized (timedatectl)
  File Descriptors          PASS   Soft limit: 1048576

  Capabilities:
    + 1 Gbps Throughput Testing
    + 1 Gbps NIC Detected
    + mTLS Performance
    + Audit Log Performance
    + Accurate Timestamps

  Verdict: READY - Host can sustain 1 Gbps. Estimated max: 900 Mbps
```

Verdicts:
- **READY** -- All checks pass; host can sustain 1 Gbps.
- **DEGRADED** -- Some warnings; throughput may be limited.
- **NOT READY** -- Critical failures; exit code 2.

---

## Security Model

### No Passwords

Reflector uses Ed25519 public key cryptography for identity. There are no
usernames, passwords, API keys, or shared secrets. Identity is the keypair.

### Mutual TLS (mTLS)

Every connection requires both the server and client to present X.509
certificates. The TLS handshake enforces:

- **TLS 1.3 only** (no TLS 1.2 fallback)
- **Mandatory client certificates** (mTLS)
- **ALPN negotiation** (`pp-link/1`)
- **Ed25519 signatures** verified at the TLS layer

Certificate chain validation is intentionally permissive at the TLS layer.
Identity verification happens at the **application layer** by extracting the
peer's public key from the presented certificate and comparing it against the
authorized peers allowlist.

### Zero-Trust Authorization

After the TLS handshake completes, Reflector extracts the peer's Endpoint ID
from the certificate's Subject Alternative Name (`pp-id-PP-XXXX-...`) and checks
it against the allowlist. The authorization flow:

1. Peer connects with mTLS certificate
2. Reflector extracts Endpoint ID from the certificate SAN
3. Reflector checks the ID against the authorized peers set
4. If authorized: connection proceeds
5. If unknown and pairing mode is active: pairing handshake initiated
6. If unknown and pairing mode is inactive: connection denied and logged

### Pairing Mode

To authorize a new peer:

1. Run `reflector pair --ttl 10m` on the reflector
2. Share the Endpoint ID and pairing token with the peer
3. The peer connects within the TTL window and presents the token
4. On successful pairing, the peer is added to the authorized set
5. The token is consumed (single-use) and cannot be reused

### Audit Trail

Every security-relevant event is logged as a JSON line to the audit log:

- `connection_accepted` -- Peer identified and authorized
- `connection_denied` -- Peer not authorized
- `session_granted` -- Test session approved
- `session_denied` -- Test session denied (quota/rate/cooldown)
- `session_completed` -- Test session ended
- `pairing_enabled` -- Pairing mode activated
- `peer_paired` -- New peer enrolled
- `peer_removed` -- Peer removed from authorized set
- `identity_rotated` -- New identity keypair generated

### Key Storage

- Private keys are stored as raw 32-byte files with `0600` permissions
- Keys are zeroized in memory when dropped
- No key material is ever logged or transmitted

---

## Protocol Overview (Paramedic Link)

The Paramedic Link protocol is a length-prefixed JSON message protocol running
over the mTLS control channel.

### Frame Format

```
+--------+----------------------------+
| 4 bytes|       N bytes              |
| Length  |     JSON Payload           |
| (BE u32)|                            |
+--------+----------------------------+
```

- Length field: 4-byte big-endian unsigned integer
- Maximum frame size: 1 MB (1,048,576 bytes)
- Payload: JSON-encoded `LinkMessage`

### Message Envelope

```json
{
  "request_id": "unique-correlation-id",
  "payload": {
    "type": "hello",
    ...
  }
}
```

### Message Flow

```
Appliance                              Reflector
    |                                      |
    |--- TCP connect + mTLS handshake ---->|
    |                                      | (extract peer ID from cert)
    |                                      | (check authorization)
    |                                      |
    |--- Hello { version, features } ----->|
    |<-- ServerHello { version,            |
    |      features, policy_summary } -----|
    |                                      |
    |--- SessionRequest { test_type,       |
    |      params } ---------------------->|
    |                                      | (check governance: rate, quota,
    |                                      |  cooldown, concurrency)
    |<-- SessionGrant { test_id, mode,     |
    |      port, token, expires_at } ------|
    |                                      |
    |  ... data-plane test runs ...        |
    |                                      |
    |--- SessionClose { test_id } -------->|
    |<-- Ok -------------------------------|
    |                                      |
```

### Message Types

| Type | Direction | Description |
|---|---|---|
| `hello` | Client -> Server | Capability negotiation |
| `server_hello` | Server -> Client | Capabilities and policy summary |
| `session_request` | Client -> Server | Request a test session |
| `session_grant` | Server -> Client | Session approved with port and token |
| `session_deny` | Server -> Client | Session denied with reason |
| `session_close` | Client -> Server | End a test session |
| `get_status` | Client -> Server | Request reflector status |
| `status_snapshot` | Server -> Client | Current status |
| `get_path_meta` | Client -> Server | Request system metadata |
| `path_meta` | Server -> Client | CPU, memory, load, MTU, NTP info |
| `ok` | Server -> Client | Generic success |
| `error` | Server -> Client | Generic error with code and message |

### Test Types

| Type | Engine | Description |
|---|---|---|
| `throughput` | iperf3 `--one-off` | TCP/UDP bandwidth measurement |
| `udp_echo` | Built-in | Latency, jitter, and packet loss measurement |

### Deny Reasons

| Reason | Description |
|---|---|
| `unauthorized` | Peer not in the authorized set |
| `rate_limited` | Too many tests this hour or cooldown not elapsed |
| `busy` | Maximum concurrent tests reached |
| `invalid_params` | Test type not allowed or invalid parameters |
| `quota_exceeded` | Daily byte quota exhausted |

---

## Deployment Options

Reflector supports multiple deployment methods. See
[`deploy/DEPLOY.md`](deploy/DEPLOY.md) for detailed instructions.

| Method | Best For |
|---|---|
| **Podman Quadlet** | Production on Alma Linux / RHEL (recommended) |
| **Docker Compose** | Quick setup on any Docker host |
| **Kubernetes** | Multi-node clusters |
| **OrbStack** | macOS development |
| **Bare Metal (RPM)** | Alma Linux / RHEL without containers |
| **Bare Metal (deb)** | Debian / Ubuntu without containers |

---

## Network Requirements

### Ports

| Port | Protocol | Mode | Description |
|---|---|---|---|
| 4000/tcp | TLS 1.3 | Both | mTLS control plane (configurable) |
| 5201-5299/tcp | TCP | Direct Ephemeral | iperf3 data plane (configurable range) |
| 5201-5299/udp | UDP | Direct Ephemeral | UDP echo data plane (configurable range) |

In **Tunneled mode** (default), only port 4000/tcp needs to be open. All data
flows inside the mTLS tunnel.

In **Direct Ephemeral mode**, the configured data port range must also be open.

### Firewall Rules (Quick Reference)

```bash
# Tunneled mode (default) -- only control plane
sudo ufw allow 4000/tcp comment "Reflector control plane"

# Direct Ephemeral mode -- also open data ports
sudo ufw allow 4000/tcp comment "Reflector control plane"
sudo ufw allow 5201:5299/tcp comment "Reflector iperf3 data"
sudo ufw allow 5201:5299/udp comment "Reflector UDP echo data"
```

---

## Architecture Diagram

```
+---------------------------------------------------------------+
|                    PacketParamedic Appliance                   |
|                  (Raspberry Pi 5, read-only)                   |
+-------------------------------+-------------------------------+
                                |
                          mTLS (TLS 1.3)
                         ALPN: pp-link/1
                          Port 4000/tcp
                                |
+-------------------------------v-------------------------------+
|                      Reflector Server                         |
|                   (VPS / LAN / Homelab)                       |
|                                                               |
|  +------------------+  +-----------------+  +--------------+  |
|  |  TLS Acceptor    |  | Auth Gate       |  | Audit Log    |  |
|  |  (mTLS, TLS 1.3) |  | (Allowlist +    |  | (JSON-lines) |  |
|  |                  |  |  Pairing)       |  |              |  |
|  +--------+---------+  +-------+---------+  +------+-------+  |
|           |                    |                    |          |
|  +--------v--------------------v--------------------v-------+ |
|  |              Session Manager + Governance Engine          | |
|  |  (Rate limiting, Quotas, Cooldown, Concurrency control)  | |
|  +---+------------------+------------------+----------------+ |
|      |                  |                  |                  |
|  +---v---+       +------v------+    +------v------+          |
|  | UDP   |       | Throughput  |    | Path Meta   |          |
|  | Echo  |       | Engine      |    | Reporter    |          |
|  | Engine|       | (iperf3)    |    | (CPU/Mem/   |          |
|  |       |       |             |    |  MTU/NTP)   |          |
|  +-------+       +-------------+    +-------------+          |
|                                                               |
|  +-------------------+                                        |
|  | Health Endpoint   |  <-- HTTP GET /health (no TLS)         |
|  | (Axum, port 7301) |                                        |
|  +-------------------+                                        |
|                                                               |
|  /var/lib/reflector/                                          |
|    identity.ed25519       (Ed25519 private key, 0600)         |
|    audit.jsonl            (Append-only audit log)             |
|                                                               |
+---------------------------------------------------------------+
```

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Make your changes
4. Run the test suite: `cargo test`
5. Run clippy: `cargo clippy -- -D warnings`
6. Format code: `cargo fmt`
7. Submit a pull request

### Development Environment

- **Dev box:** `alfa@irww.alpina` (Alma Linux x86_64) -- build and test here
- **Appliance:** `alfa@PacketParamedic` (Raspberry Pi 5, read-only rootfs)
- **macOS:** OrbStack or Docker Desktop for local container testing

### Code Structure

```
reflector/
  Cargo.toml              # Package manifest
  Containerfile           # Multi-stage container build
  src/
    main.rs               # CLI entry point (clap)
    config.rs             # TOML configuration with defaults
    identity.rs           # Ed25519 keypair, Endpoint ID (Crockford Base32 + Luhn)
    cert.rs               # Self-signed X.509 certificate generation
    tls.rs                # mTLS server and client configuration (rustls)
    peer.rs               # Peer identity and authorized peers allowlist
    auth.rs               # Authorization gate with pairing flow
    rpc.rs                # Paramedic Link protocol message types
    wire.rs               # Length-prefixed frame codec
    server.rs             # Main mTLS server and connection handler
    session.rs            # Session manager (lifecycle, concurrency)
    governance.rs         # Rate limiting, quotas, cooldown enforcement
    audit.rs              # Structured JSON-lines audit logging
    selftest.rs           # Hardware self-test (1 Gbps readiness validation)
    engine/
      mod.rs              # Test engine trait and types
      udp_echo.rs         # Built-in UDP echo reflector
      throughput.rs       # iperf3 server spawner
      health.rs           # HTTP health check endpoint (Axum)
      path_meta.rs        # System metadata collector (CPU, memory, MTU, NTP)
  deploy/
    docker-compose.yml    # Docker Compose configuration
    k8s/
      deployment.yaml     # Kubernetes Deployment manifest
      service.yaml        # Kubernetes Service manifest
    DEPLOY.md             # Deployment guide
  quadlet/                # Podman Quadlet systemd unit files
  systemd/                # systemd service files for bare-metal
  rpm/                    # RPM build scripts
  debian/                 # Debian package build scripts
```

---

## License

PacketParamedic Reflector is licensed under the [Blue Oak Model License
1.0.0](https://blueoakcouncil.org/license/1.0.0).

```
SPDX-License-Identifier: BlueOak-1.0.0
```

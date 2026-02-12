# PacketParamedic Reflector -- Roadmap

> **Spec:** [`docs/specs/SELF_HOSTED_ENDPOINT_SPEC.md`](../docs/specs/SELF_HOSTED_ENDPOINT_SPEC.md)
>
> **License:** [Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0)
>
> **Primary target:** Intel N100 (x86_64, AVX2). Raspberry Pi 4/5 (aarch64) planned.

---

## Phase Summary

| Phase | Name | Status | Description |
|-------|------|--------|-------------|
| 1 | Identity + mTLS Handshake | Done | Ed25519 keypair, X.509 cert, TLS 1.3 mTLS, authorization gate |
| 2 | Sessions + UDP Echo + Governance | Done | Session lifecycle, rate limiting, quotas, UDP echo engine |
| 3 | Throughput + Health + PathMeta | Done | iperf3 spawner, GET /health, system metadata collection |
| 4 | Containerization + Packaging | Done | Containerfile, Podman Quadlet, Docker Compose, K8s, RPM, deb |
| 5 | Hardware Self-Test | Done | 10-check host readiness validator for 1 Gbps operation |
| 6 | Appliance Integration | Not started | Appliance-side client, end-to-end mTLS pairing flow |
| 7 | Tunneled Data Plane | Not started | Data-plane traffic inside the mTLS tunnel |
| 8 | Observability + Metrics | Not started | Prometheus metrics, Grafana dashboards, alerting |
| 9 | Multi-Peer + Scheduling | Not started | Concurrent peer support, scheduled test windows |
| 10 | ARM64 + Multi-Arch | Not started | Raspberry Pi 4/5 builds, multi-arch container images |
| 11 | Persistent Peer Store | Not started | On-disk peer database, peer management API |
| 12 | CI/CD + Release Pipeline | Not started | Automated builds, testing, container publishing |
| 13 | 10GbE + High Performance | Deferred | 10 Gbps targets, jumbo frames, kernel tuning |

---

## Phase 1: Identity + mTLS Handshake (Done)

**Goal:** `reflector serve` prints Endpoint ID, listens on port 4000, accepts mTLS, rejects unauthorized peers.

### Completed

- [x] `identity.rs` -- Ed25519 keypair generation, load/save with 0600 permissions, zeroize on drop
- [x] `identity.rs` -- Endpoint ID derivation: Crockford Base32 + Luhn mod-32 check digit (`PP-XXXX-XXXX-...-C`)
- [x] `cert.rs` -- Self-signed X.509 from Ed25519 via `rcgen`; SAN encodes Endpoint ID (`pp-id-PP-...`)
- [x] `cert.rs` -- `extract_peer_id_from_cert()` via `x509-parser` for inbound peer identification
- [x] `tls.rs` -- `build_server_config()`: TLS 1.3 only, mTLS mandatory, ALPN `pp-link/1`
- [x] `tls.rs` -- `build_client_config()`: client-side mTLS for appliance connections
- [x] `tls.rs` -- Custom `ClientCertVerifier` and `ServerCertVerifier` (auth at app layer)
- [x] `wire.rs` -- `LinkCodec`: u32 BE length-prefixed frames, max 1 MB, JSON payload
- [x] `wire.rs` -- `encode_message()` / `decode_message()` helpers
- [x] `rpc.rs` -- 12 message types: Hello, ServerHello, SessionRequest/Grant/Deny/Close, GetStatus, StatusSnapshot, GetPathMeta, PathMeta, Ok, Error
- [x] `rpc.rs` -- `TestType` enum (Throughput, UdpEcho), `DenyReason` enum, `PolicySummary`
- [x] `peer.rs` -- `PeerId` newtype with `from_cert()`, `is_authorized()`
- [x] `peer.rs` -- `AuthorizedPeers` set with add/remove/contains
- [x] `config.rs` -- TOML config with 6 sections, all with `#[serde(default)]`
- [x] `config.rs` -- `ReflectorConfig::load(path)` and `load_or_default()`
- [x] `auth.rs` -- `AuthGate` with allowlist + time-limited pairing (one-time tokens)
- [x] `audit.rs` -- Append-only JSON Lines audit log with 9 event types
- [x] `server.rs` -- TCP listener, TLS accept, peer ID extraction, authorization, message dispatch
- [x] `main.rs` -- Clap CLI: `serve`, `pair`, `rotate-identity`, `status`, `show-id`

### Test Coverage

- 116 unit tests covering all Phase 1-3 modules
- Identity: generation, save/load roundtrip, Endpoint ID format validation, Luhn check
- Config: defaults, TOML parsing (full, partial, empty), serialization roundtrip
- Auth: allowlist, pairing flow, token consumption, expiry
- Governance: rate limit, cooldown, byte quota, test type restriction, daily reset
- Session: grant/deny, concurrency limit, close, cleanup, byte tracking
- RPC: all 12 message types serialize/deserialize
- Wire: encode/decode roundtrip, oversized frame rejection
- TLS: server/client config build, cert verifier behavior

---

## Phase 2: Sessions + UDP Echo + Governance (Done)

**Goal:** `SessionRequest(udp_echo)` works end-to-end with rate limiting and accounting.

### Completed

- [x] `session.rs` -- `SessionManager`: max concurrent tests, UUID test IDs, expiry timer, auto-cleanup
- [x] `session.rs` -- `request_session()`, `close_session()`, `get_status()`, `cleanup_expired()`
- [x] `session.rs` -- Byte tracking per session (atomic u64)
- [x] `governance.rs` -- `GovernanceEngine`: sliding window rate limiter (tests/hour/peer)
- [x] `governance.rs` -- Cooldown enforcement, daily byte quota, test-type restrictions
- [x] `governance.rs` -- Daily reset of byte counters at UTC midnight
- [x] `engine/mod.rs` -- `TestHandle`, `EngineResult` types
- [x] `engine/udp_echo.rs` -- `UdpEchoEngine`: ephemeral UDP socket, echo loop, byte counting
- [x] `engine/udp_echo.rs` -- Per-second packet rate limiting (10,000 pps default)
- [x] `engine/udp_echo.rs` -- Auto-close after configured duration

---

## Phase 3: Throughput + Health + PathMeta (Done)

**Goal:** iperf3 sessions spawn/cleanup correctly. Health and meta endpoints respond.

### Completed

- [x] `engine/throughput.rs` -- `ThroughputEngine`: find free port in 5201-5299 range
- [x] `engine/throughput.rs` -- Spawn `iperf3 -s -p <port> --one-off`, return port in `SessionGrant`
- [x] `engine/throughput.rs` -- SIGTERM/SIGKILL cleanup on timeout/close, exit code logging
- [x] `engine/health.rs` -- Axum `GET /health` handler: `{"status":"ok","version":"...","uptime_sec":...}`
- [x] `engine/path_meta.rs` -- `collect_path_meta()`: CPU model, core count, frequency, memory total/available
- [x] `engine/path_meta.rs` -- Load averages (1m/5m/15m), OS info, hostname, build version
- [x] `engine/path_meta.rs` -- MTU detection per interface, NTP sync status

---

## Phase 4: Containerization + Packaging (Done)

**Goal:** Deployable via Podman Quadlet, Docker Compose, K8s, RPM, or deb.

### Completed

- [x] `Containerfile` -- Multi-stage build (rust:1.82-bookworm builder, debian:bookworm-slim runtime)
- [x] `Containerfile` -- AVX2 optimized (`RUSTFLAGS="-C target-cpu=x86-64-v3"`)
- [x] `Containerfile` -- iperf3 included, non-root user, healthcheck, <50 MB compressed target
- [x] `quadlet/reflector.container` -- Podman Quadlet systemd unit (AutoUpdate, NoNewPrivileges)
- [x] `quadlet/reflector.volume` -- Persistent storage for identity + audit log
- [x] `deploy/docker-compose.yml` -- Docker Compose with volume and port mapping
- [x] `deploy/k8s/deployment.yaml` -- Kubernetes Deployment (1 replica, resource limits)
- [x] `deploy/k8s/service.yaml` -- Kubernetes Service (ClusterIP/LoadBalancer)
- [x] `deploy/k8s/configmap.yaml` -- Kubernetes ConfigMap for reflector.toml
- [x] `deploy/k8s/pvc.yaml` -- Kubernetes PersistentVolumeClaim
- [x] `deploy/reflector.toml.example` -- Example configuration file
- [x] `systemd/reflector.service` -- Hardened systemd unit (DynamicUser, ProtectSystem, MemoryMax)
- [x] `rpm/reflector.spec` + `build-rpm.sh` -- RPM package for Alma/RHEL/Fedora
- [x] `debian/build-deb.sh` + `postinst` + `prerm` -- deb package for Debian/Ubuntu
- [x] `deploy/DEPLOY.md` -- Guides for all 6 deployment methods + OrbStack

---

## Phase 5: Hardware Self-Test (Done)

**Goal:** `reflector self-test` validates that the host can push 1 Gbps throughput.

### Completed

- [x] `selftest.rs` -- 10-check hardware readiness suite
- [x] CPU check: core count, clock speed, brand detection
- [x] CPU features: AVX2/AES-NI on x86_64, NEON on aarch64
- [x] Memory: total and available RAM (512 MB+ required for 1 Gbps)
- [x] Network interfaces: link speed detection via `/sys/class/net` (Linux) or `ifconfig` fallback
- [x] iperf3 availability: binary check with `--version`
- [x] Loopback throughput: 5-second iperf3 self-test on 127.0.0.1 (4 streams, JSON output parsed)
- [x] Disk I/O: sequential write speed (1 MB x 10 iterations)
- [x] Crypto performance: Ed25519 sign+verify benchmark (5000 iterations)
- [x] Time sync: NTP status via timedatectl/chrony
- [x] File descriptors: soft ulimit check (4096+ required)
- [x] Capability derivation: 1 Gbps Throughput, 1G NIC, mTLS, Audit Log, Timestamps
- [x] Estimated max throughput: min(loopback, NIC speed) * 0.9 efficiency factor
- [x] Verdict: READY / DEGRADED / NOT READY with exit code 2 on NOT READY
- [x] Human-readable colorized table output + `--json` flag for machine parsing
- [x] CLI integration: `reflector self-test [--json]`
- [x] 12 unit tests for parsing, verdicts, capabilities, and individual checks

---

## Phase 6: Appliance Integration (Not Started)

**Goal:** PacketParamedic appliance connects to the reflector and runs tests end-to-end.

### Planned

- [ ] Appliance-side client library (Rust) implementing the Paramedic Link protocol
- [ ] End-to-end mTLS handshake: appliance presents its cert, reflector validates
- [ ] Pairing flow integration: appliance sends pairing token, reflector enrolls peer
- [ ] Session lifecycle: appliance requests test, receives grant, runs data-plane, closes
- [ ] iperf3 client-side orchestration: appliance spawns `iperf3 -c <reflector> -p <port>`
- [ ] UDP echo client: appliance sends packets, measures RTT/jitter/loss
- [ ] PathMeta display: appliance fetches and displays reflector system info
- [ ] Error handling: reconnect on disconnect, handle denied/busy/rate-limited gracefully
- [ ] Integration tests: appliance <-> reflector over real mTLS

### Dependencies

- Appliance codebase (owned by another agent on `alfa@PacketParamedic`)
- Self-hosted endpoint provider kind (`SelfHostedWAN`, `SelfHostedLAN`) from Phase 6.2 of parent roadmap

---

## Phase 7: Tunneled Data Plane (Not Started)

**Goal:** All data-plane traffic flows inside the mTLS tunnel (no exposed iperf3 ports).

### Planned

- [ ] Bidirectional byte stream multiplexing over the mTLS connection
- [ ] Stream IDs for concurrent data flows (control + data on single port)
- [ ] iperf3 proxy: reflector relays iperf3 traffic through the tunnel
- [ ] UDP echo proxy: tunnel UDP echo packets over TLS with minimal overhead
- [ ] Bandwidth measurement accuracy validation (tunneled vs direct)
- [ ] Configurable: `mode = "tunneled"` (default) vs `mode = "direct_ephemeral"`
- [ ] Performance benchmarks: overhead of tunneled vs direct on N100 hardware

### Why

Tunneled mode means only port 4000/tcp needs to be open on the firewall. This is
the recommended mode for VPS deployments where opening a port range (5201-5299)
is undesirable or restricted.

---

## Phase 8: Observability + Metrics (Not Started)

**Goal:** Production monitoring with Prometheus metrics and alerting.

### Planned

- [ ] Prometheus metrics endpoint (`/metrics`): connections, sessions, bytes, errors, latency histograms
- [ ] Per-peer metrics: tests/hour, bytes/day, deny rate
- [ ] Governance counters: rate limit hits, quota exhaustions, cooldown blocks
- [ ] Test engine metrics: iperf3 spawn time, UDP echo packet rate, throughput achieved
- [ ] Self-test metrics: export last self-test results as Prometheus gauges
- [ ] Grafana dashboard template (JSON model)
- [ ] Alert rules: high deny rate, disk full, identity expiring, iperf3 unavailable
- [ ] Structured JSON log output option (for log aggregation pipelines)

---

## Phase 9: Multi-Peer + Scheduling (Not Started)

**Goal:** Support multiple concurrent peers and scheduled test windows.

### Planned

- [ ] Increase `max_concurrent_tests` default and test with 2-4 concurrent sessions
- [ ] Per-peer session isolation (separate iperf3 instances, separate UDP echo sockets)
- [ ] Fair queuing: round-robin between peers when at capacity
- [ ] Scheduled maintenance windows: reflector can decline tests during specified hours
- [ ] Scheduled self-test: periodic hardware validation (daily/weekly)
- [ ] Batch test orchestration: reflector coordinates a sequence of tests for a peer
- [ ] Integration with parent project's scheduling engine (Phase 6.5)

---

## Phase 10: ARM64 + Multi-Arch (Not Started)

**Goal:** Reflector runs on Raspberry Pi 4/5 and ships as multi-arch container images.

### Planned

- [ ] Cross-compile for `aarch64-unknown-linux-gnu` with `RUSTFLAGS="-C target-cpu=cortex-a76"`
- [ ] Raspberry Pi 4 target: `cortex-a72` (no NEON crypto extensions)
- [ ] Multi-arch Containerfile: `docker buildx build --platform linux/amd64,linux/arm64`
- [ ] ARM64 CI runner for automated testing
- [ ] Performance validation on Pi 5 hardware (target: 940 Mbps over 1GbE)
- [ ] Thermal monitoring integration (Pi 5 throttle detection during throughput tests)
- [ ] OrbStack ARM64 testing guide for macOS developers

---

## Phase 11: Persistent Peer Store (Not Started)

**Goal:** Peer authorizations survive restarts without manual config edits.

### Planned

- [ ] On-disk peer database (JSON or SQLite) at `/var/lib/reflector/peers.json`
- [ ] Peer management CLI: `reflector peers list`, `peers add <ID>`, `peers remove <ID>`
- [ ] Peer management API: RPC messages for add/remove/list peers
- [ ] Auto-save on pairing: newly paired peers are persisted immediately
- [ ] Peer metadata: enrollment timestamp, last seen, total bytes, test count
- [ ] Peer revocation: remove + deny list (block re-pairing)
- [ ] Config migration: existing `authorized_peers` in TOML merged with on-disk store

---

## Phase 12: CI/CD + Release Pipeline (Not Started)

**Goal:** Automated builds, testing, and container image publishing.

### Planned

- [ ] GitHub Actions workflow: `cargo test`, `cargo clippy`, `cargo fmt --check`
- [ ] Container image build + push to GitHub Container Registry (ghcr.io)
- [ ] Semantic versioning with automated changelog
- [ ] RPM and deb package builds in CI (COPR for Fedora/RHEL, PPA for Ubuntu)
- [ ] Release artifacts: binary tarballs (x86_64, aarch64), container images, packages
- [ ] Security scanning: `cargo audit`, Trivy container scan
- [ ] Integration test matrix: Alma Linux 9, Debian 12, Ubuntu 24.04, macOS (OrbStack)
- [ ] Automated self-test on release (run `reflector self-test` in CI container)

---

## Phase 13: 10GbE + High Performance (Deferred)

**Goal:** Support 10 Gbps targets and kernel-level performance tuning.

### Planned

- [ ] 10GbE NIC detection and validation in self-test
- [ ] Kernel tuning guide: IRQ affinity, ring buffer sizes, jumbo frames (MTU 9000)
- [ ] TCP window scaling and buffer auto-tuning for 10 Gbps
- [ ] Native Rust throughput engine (bypass iperf3 for lower overhead)
- [ ] Zero-copy I/O (`io_uring` or `sendfile`) for data-plane traffic
- [ ] Multi-stream optimization: 8-16 parallel streams for 10 Gbps saturation
- [ ] Hardware timestamping for sub-microsecond latency measurement
- [ ] Performance targets: 9.4 Gbps sustained TCP, <1 ms added latency

### Dependencies

- Parent project Phase 14 (10GbE hardware support)
- Hardware with 10 Gbps NIC and PCIe Gen 3+ (N100 has PCIe Gen 3 x2)

---

## Spec Traceability

Every phase traces back to the engineering specification:

| Spec Section | Phase | Coverage |
|---|---|---|
| Section 4: Identity | Phase 1 | Ed25519 keypair, Endpoint ID, key rotation |
| Section 5: Transport/mTLS | Phase 1 | TLS 1.3, mTLS, ALPN, self-signed X.509 |
| Section 6: Control Protocol | Phase 1 | Paramedic Link, length-prefixed JSON frames |
| Section 7: Test Engines | Phase 2-3 | UDP echo, iperf3 throughput, PathMeta, Health |
| Section 8: Governance | Phase 2 | Rate limiting, quotas, cooldown, audit logging |
| Section 9: Security Hardening | Phase 4 | systemd hardening, container security, NoNewPrivileges |
| Section 10: Deployment | Phase 4 | Containerfile, Quadlet, Compose, K8s, RPM, deb |
| Section 11: Tunneled Mode | Phase 7 | Data-plane multiplexing over mTLS |
| Section 12: Configuration | Phase 1 | TOML config, env var override, layered defaults |

---

## Hardware Targets

| Platform | CPU | Architecture | Status | Build Flags |
|---|---|---|---|---|
| Intel N100 | Alder Lake-N | x86_64 (AVX2) | Primary | `-C target-cpu=x86-64-v3` |
| Generic x86_64 | Any | x86_64 | Supported | default |
| Raspberry Pi 5 | Cortex-A76 | aarch64 | Planned (Phase 10) | `-C target-cpu=cortex-a76` |
| Raspberry Pi 4 | Cortex-A72 | aarch64 | Planned (Phase 10) | `-C target-cpu=cortex-a72` |

---

## Development Environment

| Host | Role | Access |
|---|---|---|
| `alfa@irww.alpina` | Dev/test box (Alma Linux x86_64, Podman) | Build, test, deploy |
| `alfa@PacketParamedic` | Appliance (Raspberry Pi 5, read-only) | Integration testing only |
| macOS (OrbStack) | Local development | Container builds, unit tests |

---

## Test Count

| Module | Tests |
|---|---|
| identity | 8 |
| cert | 5 |
| tls | 5 |
| wire | 4 |
| rpc | 16 |
| peer | 7 |
| config | 7 |
| auth | 10 |
| audit | 5 |
| governance | 6 |
| session | 6 |
| selftest | 12 |
| server | 5 |
| engine (all) | 7 |
| main (CLI) | 17 |
| **Total** | **130** |

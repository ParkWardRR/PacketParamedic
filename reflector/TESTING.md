# PacketParamedic Reflector -- Testing Guide

This document covers the testing strategy, test execution, and quality assurance
procedures for the PacketParamedic Reflector.

---

## Table of Contents

- [Test Architecture Overview](#test-architecture-overview)
- [Running Tests](#running-tests)
- [Unit Tests](#unit-tests)
- [Integration Tests](#integration-tests)
- [Container Testing](#container-testing)
- [Performance Testing](#performance-testing)
- [Security Testing Checklist](#security-testing-checklist)
- [Test Configuration](#test-configuration)
- [Test Matrix](#test-matrix)
- [CI/CD Integration](#cicd-integration)
- [Code Coverage](#code-coverage)
- [Troubleshooting Common Test Failures](#troubleshooting-common-test-failures)

---

## Test Architecture Overview

The Reflector test suite is organized into three tiers:

1. **Unit Tests** -- Embedded `#[cfg(test)]` modules in every source file. These
   test individual functions and types in isolation using `tempfile` for
   filesystem operations and `tokio-test` for async code.

2. **Integration Tests** -- Tests that exercise multiple subsystems together,
   including mTLS handshakes, session lifecycle, UDP echo round-trips, and rate
   limiting under governance rules.

3. **Container and Smoke Tests** -- Build the container image and verify that the
   binary starts, generates an identity, and responds to health checks.

All tests use the standard Rust test framework (`cargo test`) with `tokio::test`
for async tests. No external test frameworks are required.

### Test Dependencies

```toml
[dev-dependencies]
tower = { version = "0.5", features = ["util"] }   # Axum handler testing
tokio-test = "0.4"                                   # Async test utilities
tempfile = "3"                                       # Temporary directories
```

---

## Running Tests

### Quick Reference

```bash
# Run all tests (unit + integration)
cargo test

# Run all tests with output visible
cargo test -- --nocapture

# Run tests for a specific module
cargo test identity::tests
cargo test config::tests
cargo test auth::tests
cargo test audit::tests
cargo test tls::tests
cargo test wire::tests
cargo test session::tests
cargo test governance::tests
cargo test peer::tests
cargo test cert::tests
cargo test server::tests
cargo test rpc::tests

# Run tests for the engine modules
cargo test engine::udp_echo::tests
cargo test engine::throughput::tests
cargo test engine::health::tests
cargo test engine::path_meta::tests

# Run a single test by name
cargo test test_udp_echo_round_trip

# Run tests matching a pattern
cargo test test_parse_duration

# Run tests in release mode (faster execution, slower compilation)
cargo test --release
```

### Running Tests Locally (macOS)

On macOS, all unit tests and most integration tests run natively. A few
platform-specific behaviors to be aware of:

- **MTU detection** uses `ifconfig en0` on macOS instead of `/sys/class/net/`
- **NTP sync detection** falls back to a heuristic (year >= 2024) since
  `timedatectl` is not available
- **iperf3 throughput tests** require iperf3 installed: `brew install iperf3`
- **File permissions tests** use Unix mode 0600 checks

```bash
# Install iperf3 (if not already installed)
brew install iperf3

# Run the full test suite
cargo test

# Expected: all tests pass, with some platform-specific tests
# producing different output (e.g., MTU = None on some macOS configs)
```

### Running Tests on the Dev Box

The dev box (`alfa@irww.alpina`, Alma Linux x86_64) is the canonical test
environment. All tests, including Linux-specific ones (MTU via sysfs, NTP via
timedatectl), run here.

```bash
ssh alfa@irww.alpina
cd /path/to/reflector

# Build and test
cargo test

# Test with AVX2 flags (matching the production build)
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo test

# Run with verbose output for debugging
RUST_LOG=debug cargo test -- --nocapture
```

---

## Unit Tests

Every source module contains an embedded `#[cfg(test)] mod tests` with tests
covering its public API and key internal behavior.

### Module Coverage Summary

| Module | Tests | What They Cover |
|---|---|---|
| `main.rs` | 11 | CLI parsing (serve, pair, rotate-identity, status, show-id), duration parsing, config path resolution |
| `config.rs` | 6 | Default values, full TOML parsing, partial TOML with defaults, file loading, missing file error, serialization roundtrip |
| `identity.rs` | 8 | Key generation, Endpoint ID format, Luhn validation, corruption detection, save/load roundtrip, key permissions (0600), different keys produce different IDs, serialization |
| `cert.rs` | 5 | Self-signed cert generation, peer ID extraction from cert SAN, validity period (10 years), Common Name matches Endpoint ID, PKCS#8 DER structure, missing SAN detection |
| `tls.rs` | 5 | Server config build, client config build, ALPN negotiation, client cert mandatory, permissive verifier behavior |
| `peer.rs` | 7 | Peer ID from cert, display formatting, authorized peers add/remove, serialization, invalid cert handling, error display, equality and From conversions |
| `auth.rs` | 10 | Authorized peer allowed, unknown peer denied, pairing mode signaling, pairing disabled fallback, successful pairing, bad token rejection, token consumption (single-use), add/remove peer, pairing configured flag, expired pairing denial |
| `rpc.rs` | 15 | Round-trip serialization for every message type (Hello, ServerHello, SessionRequest, SessionGrant, SessionDeny, SessionClose, GetStatus, StatusSnapshot, GetPathMeta, PathMeta, Ok, Error), TestType serialization, DenyReason serialization, optional field handling |
| `wire.rs` | 4 | Encode/decode round-trip, codec default construction, oversized message rejection, empty struct handling |
| `server.rs` | 4 | Protocol version constant, max frame size constant, cleanup interval bounds, frame format verification |
| `session.rs` | 5 | Session request success, session denied (busy), close session, bytes recording, cleanup expired sessions, status snapshot |
| `governance.rs` | 6 | Rate limit enforcement (allows then denies), cooldown enforcement, byte quota enforcement, test type restriction, independent peer tracking, daily reset |
| `engine/udp_echo.rs` | 3 | UDP echo round-trip (send + receive), timeout behavior, packet rate limiting |
| `engine/throughput.rs` | 3 | Free port discovery, port range scanning, engine construction |
| `engine/health.rs` | 1 | HTTP GET /health returns 200 with JSON body containing status, version, load |
| `engine/path_meta.rs` | 3 | Metadata collection (non-negative values, valid memory, non-empty build info), MTU detection (no panic), NTP sync (no panic) |

### Key Unit Test Patterns

**Identity and cryptography:**
```bash
cargo test identity::tests
# Verifies:
# - Generated Endpoint IDs start with "PP-" and use valid Crockford Base32
# - Luhn check digit validates correctly
# - Corrupted IDs fail Luhn validation
# - Keys roundtrip through save/load with correct permissions
```

**Configuration:**
```bash
cargo test config::tests
# Verifies:
# - All defaults are sensible (correct ports, paths, limits)
# - Full TOML parses correctly with all fields
# - Partial TOML correctly inherits defaults for unspecified fields
# - Empty TOML produces all defaults
# - Missing config file returns an error
```

**Authorization:**
```bash
cargo test auth::tests
# Verifies:
# - Known peers are authorized immediately
# - Unknown peers are denied when pairing is disabled
# - Unknown peers get PairingRequired when pairing is active
# - Pairing tokens are single-use (consumed after first pair)
# - Expired pairing tokens are rejected
# - Zero-duration TTL expires immediately
```

---

## Integration Tests

Integration tests exercise multiple subsystems together. These are implemented
as `#[tokio::test]` functions within the source tree.

### mTLS Handshake Tests

Tests in `tls.rs` and `cert.rs` verify end-to-end certificate generation and
mTLS configuration:

```bash
cargo test tls::tests
cargo test cert::tests
```

These tests:
- Generate test Ed25519 identities
- Create self-signed certificates with Endpoint ID SANs
- Build server and client TLS configurations
- Verify ALPN negotiation (`pp-link/1`)
- Verify that client certificates are mandatory
- Verify that peer IDs can be extracted from certificates

### Session Lifecycle Tests

Tests in `session.rs` verify the full session lifecycle:

```bash
cargo test session::tests
```

These tests:
- Create sessions via the session manager
- Verify session grants contain valid test IDs and tokens
- Verify concurrency limits are enforced (max_concurrent_tests = 1)
- Record bytes and verify status snapshots
- Expire sessions and verify cleanup removes them

### UDP Echo Engine Tests

```bash
cargo test engine::udp_echo::tests
```

These tests:
- Start a UDP echo engine on an ephemeral port
- Send a UDP datagram and verify the echo response matches
- Verify timeout behavior when duration expires
- Verify packet rate limiting drops excess packets

### Rate Limiting and Governance Tests

```bash
cargo test governance::tests
```

These tests:
- Record multiple test starts and verify rate limit denial after the limit
- Verify cooldown period enforcement between consecutive tests
- Verify daily byte quota enforcement
- Verify test type restrictions (disable UDP echo or throughput)
- Verify that per-peer quotas are independent
- Verify daily counter reset

### Health Endpoint Tests

```bash
cargo test engine::health::tests
```

Uses the `tower::ServiceExt::oneshot` method to send a request to the Axum
health router and verify the JSON response contains `status: "ok"`, a version
string, and a numeric load value.

---

## Container Testing

### Build the Container Image

```bash
# From the repository root
podman build -f reflector/Containerfile -t reflector:test .
# or
docker build -f reflector/Containerfile -t reflector:test .
```

### Smoke Test

```bash
# Start the container
docker run -d --name reflector-test \
  -p 4000:4000 \
  -v reflector-test-data:/var/lib/reflector \
  reflector:test

# Wait for startup
sleep 3

# Verify the process is running
docker exec reflector-test reflector show-id
# Expected: PP-XXXX-XXXX-XXXX-...-X (a valid Endpoint ID)

# Verify status command works
docker exec reflector-test reflector status
# Expected: Reflector Status with Endpoint ID and bind address

# Verify the container health check
docker inspect --format='{{.State.Health.Status}}' reflector-test
# Expected: healthy (after the start period)

# Check logs for startup messages
docker logs reflector-test
# Expected: "identity ready", "reflector listening"

# Cleanup
docker stop reflector-test && docker rm reflector-test
docker volume rm reflector-test-data
```

### Container Security Verification

```bash
# Verify the container runs as non-root
docker exec reflector-test id
# Expected: uid=65534(reflector) or similar non-root user

# Verify read-only filesystem (if using docker-compose)
docker exec reflector-test touch /tmp/test 2>&1
# Expected: Read-only file system error (when read_only: true)

# Verify no new privileges
docker exec reflector-test cat /proc/1/status | grep NoNewPrivs
# Expected: NoNewPrivs: 1

# Verify capabilities are dropped
docker exec reflector-test cat /proc/1/status | grep CapEff
# Expected: 0000000000000000 (no capabilities)
```

### Docker Compose Test

```bash
cd reflector/deploy

# Start with compose
docker compose up -d

# Verify health
docker compose ps
# Expected: reflector running, healthy

# View logs
docker compose logs -f reflector

# Stop
docker compose down -v
```

---

## Performance Testing

### iperf3 Throughput Benchmarks

Prerequisites: iperf3 must be installed on both the reflector host and the
test client.

```bash
# On the reflector host: start the reflector
reflector serve

# On the test client: run an iperf3 test (after session is granted)
# The exact port comes from the SessionGrant message

# Direct test (for baseline comparison)
iperf3 -c <reflector-host> -p 5201 -t 10 -P 4
# Expected: Line-rate throughput for the given link

# Reverse mode (download test)
iperf3 -c <reflector-host> -p 5201 -t 10 -P 4 -R
```

### UDP Echo Latency Benchmarks

```bash
# Use a simple UDP echo test client (or the PacketParamedic appliance)
# Send 1000 packets at 100ms intervals
# Measure RTT, jitter, and packet loss

# Expected results on a LAN:
#   RTT: < 1ms
#   Jitter: < 0.1ms
#   Packet loss: 0%

# Expected results on a typical VPS:
#   RTT: varies by distance (10-100ms typical)
#   Jitter: < 5ms
#   Packet loss: < 0.1%
```

### Resource Usage Benchmarks

```bash
# Monitor idle resource usage
# On the reflector host:
reflector serve &
sleep 30
ps aux | grep reflector
# Expected: < 0.1% CPU, < 10 MB RSS at idle

# Monitor during a throughput test
# Run a 30-second iperf3 test and observe:
top -p $(pgrep reflector) -d 1
# Expected: CPU usage proportional to throughput; memory stable
```

---

## Security Testing Checklist

Use this checklist to verify the security properties of the reflector.

### Unauthorized Peer Rejection

- [ ] Connect with a certificate that has an Endpoint ID not in the authorized
      peers list
- [ ] Verify the connection is closed after the TLS handshake
- [ ] Verify a `connection_denied` audit log entry is created
- [ ] Verify no session is created for the unauthorized peer

### Pairing Mode Security

- [ ] Enable pairing with a short TTL (e.g., `--ttl 30s`)
- [ ] Verify a new peer can pair with the correct token within the TTL
- [ ] Verify the token is consumed after one use (second attempt fails)
- [ ] Wait for the TTL to expire and verify the token no longer works
- [ ] Verify a `peer_paired` audit log entry is created on success
- [ ] Verify pairing with an incorrect token is rejected

### Rate Limiting

- [ ] Configure `max_tests_per_hour_per_peer = 3`
- [ ] Run 3 tests from the same peer (all should succeed)
- [ ] Attempt a 4th test and verify it is denied with `rate_limited`
- [ ] Verify a different peer can still run tests

### Cooldown Enforcement

- [ ] Configure `cooldown_sec = 30`
- [ ] Run a test, then immediately attempt another from the same peer
- [ ] Verify the second test is denied with `rate_limited`
- [ ] Wait 30 seconds and verify the next test is allowed

### Byte Quota Enforcement

- [ ] Configure `max_bytes_per_day_per_peer = 1000000` (1 MB)
- [ ] Run a test that transfers more than 1 MB
- [ ] Verify subsequent tests are denied with `quota_exceeded`
- [ ] Verify a different peer is unaffected

### Concurrent Session Limit

- [ ] Configure `max_concurrent_tests = 1`
- [ ] Start a test from peer A
- [ ] Attempt a test from peer B while A is running
- [ ] Verify peer B's request is denied with `busy`

### Audit Log Verification

- [ ] Run a sequence of operations (connect, test, close)
- [ ] Read the audit log (`/var/lib/reflector/audit.jsonl`)
- [ ] Verify each line is valid JSON
- [ ] Verify timestamps are in ISO 8601 format
- [ ] Verify the correct event types appear in order
- [ ] Verify peer IDs and test IDs are present where applicable

### Identity Key Protection

- [ ] Verify the identity key file has 0600 permissions
- [ ] Verify the key file is exactly 32 bytes
- [ ] Verify the process does not log any key material
- [ ] Verify `rotate-identity` creates a new key and changes the Endpoint ID

### TLS Configuration

- [ ] Attempt a TLS 1.2 connection and verify it is rejected
- [ ] Attempt a connection without a client certificate and verify it is rejected
- [ ] Verify the ALPN protocol is `pp-link/1`
- [ ] Verify the server certificate contains the correct Endpoint ID in the SAN

---

## Test Configuration

### Environment Variables

| Variable | Description | Example |
|---|---|---|
| `REFLECTOR_CONFIG` | Path to configuration file | `/tmp/test-reflector.toml` |
| `RUST_LOG` | Tracing filter for test output | `debug`, `reflector=trace` |
| `RUST_BACKTRACE` | Show backtraces on panic | `1` or `full` |

### Test Fixtures

Tests use `tempfile::TempDir` for filesystem isolation. No shared fixtures or
external state is required. Each test creates its own temporary directory that is
automatically cleaned up when the test completes.

```rust
// Example pattern used throughout the test suite
let dir = tempfile::TempDir::new().unwrap();
let path = dir.path().join("identity.key");
// ... test with `path` ...
// `dir` is dropped and cleaned up automatically
```

### Test Configuration Overrides

For tests that need custom configurations:

```rust
fn test_config() -> QuotaConfig {
    QuotaConfig {
        max_concurrent_tests: 1,
        max_test_duration_sec: 30,
        max_tests_per_hour_per_peer: 10,
        max_bytes_per_day_per_peer: 10_000_000_000,
        cooldown_sec: 0,   // Disable cooldown for fast tests
        allow_udp_echo: true,
        allow_throughput: true,
    }
}
```

---

## Test Matrix

### Platforms

| Platform | Architecture | Status | Notes |
|---|---|---|---|
| Alma Linux 9 (x86_64) | x86-64-v3 (AVX2) | Primary | Dev box: `alfa@irww.alpina` |
| macOS (aarch64) | ARM64 | Supported | OrbStack / native cargo test |
| Debian Bookworm (x86_64) | x86-64 | Container | Containerfile base image |
| Raspberry Pi OS (aarch64) | ARM64 / Cortex-A76 | Planned | Appliance target |

### Rust Versions

| Version | Status | Notes |
|---|---|---|
| 1.75 (MSRV) | Required | Minimum Supported Rust Version from Cargo.toml |
| Stable (latest) | Recommended | Primary development toolchain |
| Nightly | Optional | For coverage tools and advanced diagnostics |

### Test Environment Matrix

| Test Type | macOS | Linux (native) | Container | CI |
|---|---|---|---|---|
| Unit tests | Yes | Yes | Yes | Yes |
| Identity/cert tests | Yes | Yes | Yes | Yes |
| UDP echo engine | Yes | Yes | Yes | Yes |
| iperf3 engine | Requires iperf3 | Yes | Yes | Yes |
| Health endpoint | Yes | Yes | Yes | Yes |
| MTU detection | Partial (ifconfig) | Full (sysfs) | Full | Yes |
| NTP sync detection | Heuristic | Full (timedatectl) | Partial | Yes |
| File permissions | Yes (Unix) | Yes | Yes | Yes |

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Reflector CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [1.75.0, stable]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: clippy, rustfmt

      - name: Install iperf3
        run: sudo apt-get update && sudo apt-get install -y iperf3

      - name: Check formatting
        run: cargo fmt --manifest-path reflector/Cargo.toml -- --check

      - name: Run clippy
        run: cargo clippy --manifest-path reflector/Cargo.toml -- -D warnings

      - name: Run tests
        run: cargo test --manifest-path reflector/Cargo.toml

      - name: Build release
        run: cargo build --release --manifest-path reflector/Cargo.toml

  container:
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v4
      - name: Build container
        run: docker build -f reflector/Containerfile -t reflector:ci .
      - name: Smoke test
        run: |
          docker run -d --name reflector-ci reflector:ci
          sleep 5
          docker exec reflector-ci reflector show-id
          docker stop reflector-ci
```

### Pre-Commit Checks

```bash
# Run before every commit
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

---

## Code Coverage

### Using cargo-tarpaulin (Linux)

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --manifest-path reflector/Cargo.toml --out Html --output-dir coverage/

# Open the report
open coverage/tarpaulin-report.html
```

### Using cargo-llvm-cov (macOS and Linux)

```bash
# Install llvm-cov
cargo install cargo-llvm-cov

# Run coverage
cargo llvm-cov --manifest-path reflector/Cargo.toml --html --output-dir coverage/

# Open the report
open coverage/html/index.html
```

### Coverage Targets

| Module | Target Coverage | Notes |
|---|---|---|
| `identity.rs` | > 90% | Core cryptographic identity |
| `config.rs` | > 95% | All config paths tested |
| `auth.rs` | > 90% | Authorization logic |
| `governance.rs` | > 85% | Rate limiting edge cases |
| `session.rs` | > 85% | Session lifecycle |
| `rpc.rs` | > 95% | All message types tested |
| `wire.rs` | > 90% | Frame codec |
| `audit.rs` | > 90% | Audit log I/O |
| `cert.rs` | > 85% | Certificate generation |
| `tls.rs` | > 80% | TLS configuration |
| `engine/*` | > 75% | Engine-specific tests |

---

## Troubleshooting Common Test Failures

### "Address already in use" on UDP echo tests

The UDP echo engine tests bind to ephemeral ports (port 0), so this should
rarely occur. If it does:

```bash
# Check for lingering processes
lsof -i :5201-5299
# Kill any orphaned iperf3 or test processes
```

### iperf3 tests fail with "command not found"

The throughput engine requires iperf3 to be installed and in `$PATH`:

```bash
# macOS
brew install iperf3

# Alma Linux / RHEL
sudo dnf install iperf3

# Debian / Ubuntu
sudo apt-get install iperf3
```

### TLS tests fail with ring/aws-lc-sys build errors

The `ring` and `aws-lc-sys` crates require a C compiler:

```bash
# macOS
xcode-select --install

# Alma Linux / RHEL
sudo dnf install gcc make

# Debian / Ubuntu
sudo apt-get install build-essential
```

### Tokio runtime panics in tests

Ensure you are using `#[tokio::test]` (not `#[test]`) for async test functions.
All async tests in the codebase use the multi-threaded tokio runtime.

### tempfile permission errors

On some systems, the default temp directory may have restrictive permissions.
The tests use `tempfile::TempDir::new()` which respects the `TMPDIR` environment
variable:

```bash
TMPDIR=/tmp cargo test
```

### Test output is too noisy

Tests produce tracing output that can be verbose. To suppress it:

```bash
RUST_LOG=error cargo test
```

To see only a specific module's output:

```bash
RUST_LOG=reflector::auth=debug cargo test auth::tests -- --nocapture
```

### Tests pass locally but fail in CI

Common causes:
- **Timing sensitivity:** Tests with short durations (e.g., `Duration::from_millis(100)`)
  may race on slow CI runners. The test suite avoids tight timing where possible.
- **iperf3 not installed:** Ensure the CI environment installs iperf3.
- **Port conflicts:** CI runners may have ports in use. Tests use ephemeral
  ports (port 0) to avoid this.
- **Platform differences:** MTU detection and NTP sync check behave differently
  on Linux vs macOS. The tests are written to handle both gracefully.

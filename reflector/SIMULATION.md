# PacketParamedic Reflector -- Simulation Guide

This guide walks through end-to-end simulation scenarios for testing the
Reflector before deploying it with production PacketParamedic appliances.

---

## Table of Contents

- [1. Prerequisites](#1-prerequisites)
- [2. Local Simulation (Single Machine)](#2-local-simulation-single-machine)
- [3. Two-Machine Simulation](#3-two-machine-simulation)
- [4. Container Simulation](#4-container-simulation)
- [5. Stress Testing](#5-stress-testing)
- [6. Verification Checklist](#6-verification-checklist)
- [7. Troubleshooting](#7-troubleshooting)

---

## 1. Prerequisites

### Software

| Component | Required | Install |
|---|---|---|
| Rust toolchain (1.75+) | Yes | `rustup update stable` |
| iperf3 | Yes | `dnf install iperf3` / `apt install iperf3` / `brew install iperf3` |
| OpenSSL CLI | For cert inspection | `dnf install openssl` / Already on macOS |
| jq | For audit log inspection | `dnf install jq` / `apt install jq` / `brew install jq` |
| Podman or Docker | For container tests | `dnf install podman` / Docker Desktop |
| netcat (nc) | For port testing | Usually pre-installed |

### Hardware

- **Minimum:** A single machine with two terminal sessions
- **Recommended:** Two machines on the same network (or accessible via VPN)
- **Dev box:** `alfa@irww.alpina` (Alma Linux x86_64) -- reflector host
- **Appliance:** `alfa@PacketParamedic` (Raspberry Pi 5) or any second machine

### Build the Reflector

```bash
cd reflector
cargo build --release
# Binary at target/release/reflector
```

---

## 2. Local Simulation (Single Machine)

This section walks through a complete protocol simulation on a single machine,
using a test configuration and a simple client script.

### 2.1 Generate Two Identities

The reflector and a simulated appliance each need their own Ed25519 identity.

```bash
# Create working directories
mkdir -p /tmp/sim/reflector
mkdir -p /tmp/sim/appliance

# Generate the reflector identity
./target/release/reflector \
  -c /dev/null \
  show-id 2>/dev/null || true

# Actually, let's use the reflector itself to generate identities
# by starting it with a test config pointing to our temp directory.
```

Create a test configuration for the reflector:

```bash
cat > /tmp/sim/reflector.toml << 'EOF'
[identity]
private_key_path = "/tmp/sim/reflector/identity.ed25519"

[network]
listen_address = "127.0.0.1:4000"
alpn = "pp-link/1"
mode = "tunneled"
data_port_range_start = 15201
data_port_range_end = 15210

[access]
pairing_enabled = true
authorized_peers = []

[quotas]
max_test_duration_sec = 30
max_concurrent_tests = 1
max_tests_per_hour_per_peer = 10
max_bytes_per_day_per_peer = 1000000000
cooldown_sec = 2
allow_udp_echo = true
allow_throughput = true

[iperf3]
path = "iperf3"
default_streams = 2
max_streams = 4

[logging]
level = "debug"
audit_log_path = "/tmp/sim/reflector/audit.jsonl"
EOF
```

### 2.2 Start the Reflector

```bash
# Terminal 1: Start the reflector
RUST_LOG=debug ./target/release/reflector \
  -c /tmp/sim/reflector.toml \
  serve
```

Expected output:

```
  PacketParamedic Reflector
  ========================
  Endpoint ID : PP-XXXX-XXXX-XXXX-...-X
  Listen      : 127.0.0.1:4000
  Mode        : Tunneled
```

Note the Endpoint ID -- you will need it for the pairing step.

### 2.3 Verify the Reflector is Listening

```bash
# Terminal 2: Check the port is open
nc -zv 127.0.0.1 4000
# Expected: Connection to 127.0.0.1 4000 port [tcp/*] succeeded!
```

### 2.4 Inspect the TLS Certificate

```bash
# Use openssl to connect and inspect the server certificate
echo | openssl s_client \
  -connect 127.0.0.1:4000 \
  -servername localhost \
  -showcerts \
  -brief 2>&1 | head -20

# Note: This will fail the handshake because the server requires
# a client certificate (mTLS), but you can see the server cert.
# Look for the CN and SAN containing the Endpoint ID.
```

### 2.5 Enable Pairing Mode

```bash
# Terminal 2: Enable pairing for the simulated appliance
./target/release/reflector \
  -c /tmp/sim/reflector.toml \
  pair --ttl 10m
```

Expected output:

```
  Pairing Mode Enabled
  ====================
  Endpoint ID    : PP-XXXX-XXXX-XXXX-...-X
  Pairing Token  : 550e8400-e29b-41d4-a716-446655440000
  Expires In     : 10m

  Share the endpoint ID and pairing token with the peer.
  The peer must connect within the TTL window to be authorized.
```

### 2.6 Understand the Protocol Exchange

A full session between an appliance and reflector follows this sequence:

```
Step 1: TCP connection + mTLS handshake
  - Appliance connects to 127.0.0.1:4000
  - Both sides present Ed25519 certificates
  - TLS 1.3 negotiated with ALPN "pp-link/1"
  - Reflector extracts peer's Endpoint ID from certificate SAN

Step 2: Authorization check
  - Reflector checks peer ID against authorized_peers list
  - If not found and pairing is active: PairingRequired signal
  - If not found and no pairing: ConnectionDenied (logged)
  - If found: ConnectionAccepted (logged)

Step 3: Hello -> ServerHello
  Client sends:
  {
    "request_id": "req-001",
    "payload": {
      "type": "hello",
      "version": "1.0",
      "features": ["throughput", "udp_echo", "path_meta"]
    }
  }

  Server responds:
  {
    "request_id": "req-001",
    "payload": {
      "type": "server_hello",
      "version": "1.0",
      "features": ["throughput", "udp_echo", "path_meta"],
      "policy_summary": {
        "max_test_duration_sec": 30,
        "max_concurrent_tests": 1,
        "max_tests_per_hour": 10,
        "allowed_test_types": ["throughput", "udp_echo"]
      }
    }
  }

Step 4: SessionRequest -> SessionGrant
  Client sends:
  {
    "request_id": "req-002",
    "payload": {
      "type": "session_request",
      "test_type": "udp_echo",
      "params": {
        "duration_sec": 10,
        "protocol": null,
        "streams": null,
        "reverse": null
      }
    }
  }

  Server responds (if approved):
  {
    "request_id": "req-002",
    "payload": {
      "type": "session_grant",
      "test_id": "550e8400-...",
      "mode": "direct_ephemeral",
      "port": 15201,
      "token": "auth-cookie-uuid",
      "expires_at": "2026-02-12T15:00:00Z"
    }
  }

Step 5: Data-plane test runs on the granted port

Step 6: SessionClose
  Client sends:
  {
    "request_id": "req-003",
    "payload": {
      "type": "session_close",
      "test_id": "550e8400-..."
    }
  }

  Server responds:
  {
    "request_id": "req-003",
    "payload": { "type": "ok" }
  }
```

### 2.7 Verify the Audit Log

After running a simulation, inspect the audit log:

```bash
# Pretty-print each audit entry
cat /tmp/sim/reflector/audit.jsonl | jq .

# Filter by event type
cat /tmp/sim/reflector/audit.jsonl | jq 'select(.event_type == "connection_accepted")'
cat /tmp/sim/reflector/audit.jsonl | jq 'select(.event_type == "session_granted")'
cat /tmp/sim/reflector/audit.jsonl | jq 'select(.event_type == "connection_denied")'

# Count events by type
cat /tmp/sim/reflector/audit.jsonl | jq -r '.event_type' | sort | uniq -c
```

### 2.8 Check Reflector Status

```bash
./target/release/reflector \
  -c /tmp/sim/reflector.toml \
  status
```

### 2.9 Cleanup

```bash
rm -rf /tmp/sim
```

---

## 3. Two-Machine Simulation

This section simulates a real deployment with the reflector on one machine and a
test client on another.

### 3.1 Set Up the Reflector

On the reflector host (`alfa@irww.alpina`):

```bash
# Build the reflector
cd reflector
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release

# Create the configuration
sudo mkdir -p /etc/reflector /var/lib/reflector
sudo chown $USER /var/lib/reflector

cat > /etc/reflector/reflector.toml << 'EOF'
[identity]
private_key_path = "/var/lib/reflector/identity.ed25519"

[network]
listen_address = "0.0.0.0:4000"
mode = "direct_ephemeral"
data_port_range_start = 5201
data_port_range_end = 5210

[access]
pairing_enabled = true

[quotas]
max_test_duration_sec = 60
max_concurrent_tests = 1
max_tests_per_hour_per_peer = 10
cooldown_sec = 5

[logging]
level = "info"
audit_log_path = "/var/lib/reflector/audit.jsonl"
EOF

# Open firewall ports (Alma Linux / firewalld)
sudo firewall-cmd --add-port=4000/tcp --permanent
sudo firewall-cmd --add-port=5201-5210/tcp --permanent
sudo firewall-cmd --add-port=5201-5210/udp --permanent
sudo firewall-cmd --reload

# Start the reflector
./target/release/reflector serve
```

### 3.2 Test mTLS Handshake

From the test client machine:

```bash
# Try connecting with openssl (will fail mTLS but shows the server cert)
echo | openssl s_client \
  -connect irww.alpina:4000 \
  -servername irww.alpina \
  -brief 2>&1

# Look for:
# - "TLSv1.3" in the protocol version
# - The PP-XXXX Endpoint ID in the certificate subject/SAN
# - "alert certificate required" (because no client cert was sent)
```

### 3.3 Test Unauthorized Peer Rejection

Connect with a valid TLS client certificate but one whose Endpoint ID is not in
the authorized list. The reflector should:

1. Accept the TLS handshake (mTLS succeeds at the TLS layer)
2. Extract the peer ID from the certificate
3. Check the allowlist and find no match
4. Close the connection
5. Log a `connection_denied` event in the audit log

Verify on the reflector host:

```bash
tail -f /var/lib/reflector/audit.jsonl | jq .
# Look for: "event_type": "connection_denied"
```

### 3.4 Test Pairing Flow

```bash
# On the reflector host:
./target/release/reflector \
  -c /etc/reflector/reflector.toml \
  pair --ttl 10m

# Note the pairing token.
# On the client: connect within 10 minutes using the token.
# After pairing succeeds, verify:

tail -1 /var/lib/reflector/audit.jsonl | jq .
# Expected: "event_type": "peer_paired" (or "connection_accepted" for the paired peer)
```

### 3.5 Run iperf3 Throughput Test End-to-End

After the peer is authorized and a `SessionGrant` is received with a port:

```bash
# On the client: run iperf3 against the granted port
iperf3 -c irww.alpina -p 5201 -t 10 -P 4

# Expected: throughput results matching your network capacity
# The reflector audit log should show session_granted and session_completed events
```

### 3.6 Run UDP Echo Test End-to-End

After receiving a `SessionGrant` for a UDP echo test:

```bash
# Simple UDP echo test using netcat (for verification)
# On the client:
echo "ping" | nc -u irww.alpina <granted-port>

# For proper latency measurement, use the PacketParamedic appliance
# or a dedicated UDP echo client that timestamps packets.
```

### 3.7 Verify Rate Limiting

Send 11 session requests in rapid succession from the same peer (with
`max_tests_per_hour_per_peer = 10`):

```bash
# After 10 successful sessions, the 11th should be denied:
# Expected response:
# {
#   "request_id": "req-011",
#   "payload": {
#     "type": "session_deny",
#     "reason": "rate_limited",
#     "message": "rate limit exceeded for this peer",
#     "retry_after_sec": 60
#   }
# }

# Verify in the audit log:
cat /var/lib/reflector/audit.jsonl | jq 'select(.event_type == "session_denied")'
```

---

## 4. Container Simulation

### 4.1 Build and Run the Reflector Container

```bash
# From the repository root
podman build -f reflector/Containerfile -t reflector:sim .

podman run -d --name reflector-sim \
  -p 4000:4000 \
  -p 5201-5210:5201-5210 \
  -v reflector-sim-data:/var/lib/reflector \
  reflector:sim

# Wait for startup
sleep 3

# Verify identity was generated
podman exec reflector-sim reflector show-id
```

### 4.2 Test Health Endpoint from Host

The health endpoint runs on a separate HTTP port (when configured). For the
default configuration, use the status command:

```bash
# Check status from the host
podman exec reflector-sim reflector status
```

### 4.3 Test mTLS from Another Container

```bash
# Create a network for container-to-container communication
podman network create reflector-net

# Start the reflector on the network
podman run -d --name reflector-srv \
  --network reflector-net \
  -p 4000:4000 \
  -v reflector-srv-data:/var/lib/reflector \
  reflector:sim

# Start a client container on the same network
podman run -it --rm \
  --network reflector-net \
  debian:bookworm-slim \
  bash -c "apt-get update && apt-get install -y openssl && \
    echo | openssl s_client -connect reflector-srv:4000 -brief 2>&1"

# Expected: TLSv1.3 handshake (will fail at client cert stage)
```

### 4.4 Docker Compose Simulation

```bash
cd reflector/deploy

# Start the full stack
docker compose up -d

# Check logs
docker compose logs -f reflector

# Verify the reflector is running
docker compose exec reflector reflector show-id
docker compose exec reflector reflector status

# Enable pairing
docker compose exec reflector reflector pair --ttl 10m

# Stop everything
docker compose down -v
```

### 4.5 Podman Pod Simulation

```bash
# Create a pod with port mapping
podman pod create --name reflector-pod -p 4000:4000 -p 5201-5210:5201-5210

# Run the reflector in the pod
podman run -d --pod reflector-pod \
  --name reflector \
  -v reflector-data:/var/lib/reflector \
  reflector:sim

# Verify
podman exec reflector reflector show-id

# Cleanup
podman pod rm -f reflector-pod
podman volume rm reflector-data
```

---

## 5. Stress Testing

### 5.1 Concurrent Connection Testing

Test the reflector's behavior under concurrent connection load.

```bash
# Use a loop to open multiple simultaneous connections
# (These will fail mTLS but test the TCP accept loop)
for i in $(seq 1 50); do
  (echo | openssl s_client -connect 127.0.0.1:4000 -brief 2>/dev/null) &
done
wait

# Monitor the reflector logs for errors or warnings
# Expected: 50 "TLS handshake failed" entries (no crash, no hang)
```

### 5.2 Rate Limit Verification Under Load

With `max_tests_per_hour_per_peer = 10` and `cooldown_sec = 0`:

```bash
# Simulate rapid session requests from the same peer
# After 10 grants, all subsequent requests should be denied

# Verify the audit log shows exactly 10 session_granted events
# and subsequent session_denied events for the same peer
cat /var/lib/reflector/audit.jsonl | \
  jq -r 'select(.peer_id == "PP-TEST-PEER") | .event_type' | \
  sort | uniq -c
# Expected:
#  10 session_granted
#   N session_denied
```

### 5.3 Byte Quota Exhaustion

Configure a low byte quota and run tests until it is exhausted:

```bash
# In reflector.toml:
# max_bytes_per_day_per_peer = 10000000  (10 MB)

# Run iperf3 tests until the quota is exhausted
# First test: 5 MB transfer -> success
# Second test: 5 MB transfer -> success
# Third test: should be denied with "quota_exceeded"

# Verify in audit log:
cat /var/lib/reflector/audit.jsonl | \
  jq 'select(.event_type == "session_denied" and .reason == "daily byte quota exceeded")'
```

### 5.4 Long-Duration Session Handling

Test behavior when a session exceeds its maximum duration:

```bash
# Configure max_test_duration_sec = 10
# Request a session with duration_sec = 30
# The session manager clamps to 10 seconds + 5 second grace period

# After 15 seconds, the session cleanup task should expire the session
# Verify with:
./target/release/reflector -c /tmp/sim/reflector.toml status
# Expected: no active test after expiry
```

### 5.5 UDP Echo Flood Test

Test the UDP echo engine's rate limiting under packet flood:

```bash
# Start a UDP echo session on a known port

# Send a flood of UDP packets (10,000 packets)
for i in $(seq 1 10000); do
  echo "ping-$i" | nc -u -w0 127.0.0.1 <echo-port>
done

# With max_packet_rate = 1000, the engine should drop excess packets
# silently. Verify the reflector does not crash or consume excessive memory.
```

---

## 6. Verification Checklist

Use this comprehensive checklist to verify all aspects of the reflector before
production deployment.

### Identity and Startup

| # | Scenario | Expected Result | How to Verify |
|---|---|---|---|
| 1 | First run (no existing identity) | New Ed25519 keypair generated, Endpoint ID printed | Check startup output and `/var/lib/reflector/identity.ed25519` exists (32 bytes, mode 0600) |
| 2 | Subsequent run (identity exists) | Same Endpoint ID as before | Compare `reflector show-id` output across restarts |
| 3 | Identity persists across restarts | Endpoint ID unchanged | Stop, start, verify `show-id` |
| 4 | `rotate-identity` generates new ID | Different Endpoint ID after rotation | Compare before and after |
| 5 | Identity key file permissions | 0600 (owner read/write only) | `stat -c %a /var/lib/reflector/identity.ed25519` |

### TLS and Connectivity

| # | Scenario | Expected Result | How to Verify |
|---|---|---|---|
| 6 | mTLS handshake with valid peer | Connection accepted | Audit log: `connection_accepted` |
| 7 | Connection without client cert | Handshake fails | `openssl s_client` shows alert |
| 8 | TLS 1.2 connection attempt | Rejected (TLS 1.3 only) | `openssl s_client -tls1_2` fails |
| 9 | ALPN mismatch | Handshake fails | Connect with wrong ALPN |
| 10 | Server cert contains Endpoint ID | SAN has `pp-id-PP-XXXX-...` | `openssl x509 -text` on the cert |

### Authorization

| # | Scenario | Expected Result | How to Verify |
|---|---|---|---|
| 11 | Authorized peer connects | Connection proceeds | Audit log: `connection_accepted` |
| 12 | Unauthorized peer (no pairing) | Connection closed | Audit log: `connection_denied` |
| 13 | Unknown peer with active pairing | Pairing handshake offered | Application layer response |
| 14 | Pairing with correct token | Peer added to authorized set | Subsequent connections succeed |
| 15 | Pairing with wrong token | Pairing rejected | Error response, peer not added |
| 16 | Pairing with expired token | Pairing rejected | Wait past TTL, then try |
| 17 | Pairing token is single-use | Second use fails | Try same token for two peers |

### Session Management

| # | Scenario | Expected Result | How to Verify |
|---|---|---|---|
| 18 | Session request within limits | `session_grant` response | Check grant contains test_id, port, token |
| 19 | Session request at max concurrency | `session_deny` with `busy` | Run two sessions simultaneously |
| 20 | Session request exceeding rate limit | `session_deny` with `rate_limited` | Send N+1 requests in one hour |
| 21 | Session request during cooldown | `session_deny` with `rate_limited` | Send two requests within cooldown_sec |
| 22 | Session request exceeding byte quota | `session_deny` with `quota_exceeded` | Transfer past daily limit |
| 23 | Session close | Session removed from active set | Status shows no active test |
| 24 | Expired session cleanup | Session removed automatically | Wait past expiry, check status |
| 25 | Duration clamped to maximum | Grant duration <= max_test_duration_sec | Request longer duration |

### Test Engines

| # | Scenario | Expected Result | How to Verify |
|---|---|---|---|
| 26 | UDP echo round-trip | Sent packet echoed back | Send UDP, receive same data |
| 27 | UDP echo rate limiting | Excess packets dropped | Send > rate limit, count responses |
| 28 | UDP echo timeout | Engine stops after duration | Wait for duration, check logs |
| 29 | iperf3 server starts | iperf3 process running on assigned port | `ss -tlnp` shows iperf3 |
| 30 | iperf3 `--one-off` exits after test | Process terminates | Check with `ps aux` |
| 31 | iperf3 killed on timeout | Process SIGTERM then SIGKILL | Check logs for termination |

### Audit Log

| # | Scenario | Expected Result | How to Verify |
|---|---|---|---|
| 32 | Every connection logged | `connection_accepted` or `connection_denied` | Count log entries vs connections |
| 33 | Every session logged | `session_granted` or `session_denied` | Count log entries vs requests |
| 34 | Session completion logged | `session_completed` event | Check after session close |
| 35 | Audit log survives restart | Entries preserved (append mode) | Restart reflector, check log |
| 36 | Log entries are valid JSON | Each line parses as JSON | `cat audit.jsonl \| jq .` (no errors) |
| 37 | Timestamps are ISO 8601 | RFC 3339 format | Inspect timestamp fields |
| 38 | Parent directories auto-created | Log created even in nested paths | Use a deep path in config |

### Configuration

| # | Scenario | Expected Result | How to Verify |
|---|---|---|---|
| 39 | No config file (defaults) | Starts with sensible defaults | Run without `-c` flag |
| 40 | Partial config (missing sections) | Missing sections use defaults | Omit `[quotas]` section |
| 41 | Full config file | All values applied | Set non-default values, verify in status |
| 42 | `REFLECTOR_CONFIG` env var | Config loaded from env path | `export REFLECTOR_CONFIG=/path/to/config.toml` |
| 43 | Invalid config file | Clear error message, process exits | Use malformed TOML |

---

## 7. Troubleshooting

### TLS Handshake Errors

**Symptom:** `TLS handshake failed: error:... certificate required`

**Cause:** The client did not present a certificate. Reflector requires mTLS
(mutual TLS) with mandatory client certificates.

**Solution:** Ensure the client presents an X.509 certificate containing its
Endpoint ID in the SAN. Use `build_client_config()` from `tls.rs` to configure
the client TLS stack.

---

**Symptom:** `TLS handshake failed: AlertReceived(ProtocolVersion)`

**Cause:** The client attempted TLS 1.2 or earlier. Reflector only supports
TLS 1.3.

**Solution:** Configure the client to use TLS 1.3. In openssl:
`openssl s_client -tls1_3 -connect host:4000`

---

### Port Conflicts

**Symptom:** `failed to bind TCP listener on 0.0.0.0:4000: Address already in use`

**Cause:** Another process (or a previous reflector instance) is using port 4000.

**Solution:**
```bash
# Find what is using the port
sudo lsof -i :4000
# or
sudo ss -tlnp sport eq 4000

# Kill the conflicting process, or use a different port:
reflector serve --bind 0.0.0.0:7100
```

---

**Symptom:** `no free port found in range 5201-5299`

**Cause:** All ports in the iperf3 data range are occupied.

**Solution:**
```bash
# Check which ports are in use
sudo ss -tlnp sport ge 5201 sport le 5299

# Expand the range in reflector.toml:
# data_port_range_start = 5201
# data_port_range_end = 5399
```

---

### Permission Denied

**Symptom:** `failed to write identity key: Permission denied`

**Cause:** The reflector process does not have write access to the identity
key directory.

**Solution:**
```bash
# Ensure the data directory exists and is writable
sudo mkdir -p /var/lib/reflector
sudo chown $(whoami) /var/lib/reflector
# or, if running as a systemd service:
sudo chown reflector:reflector /var/lib/reflector
```

---

## 8. NAT Environment Simulation (CGNAT / Double NAT)

To simulate strict NAT environments (like ISP CGNAT or Double NAT) using containers,
you can create nested networks with routing containers.

### 8.1 Simulating CGNAT

Architecture:
`[Client] --(100.64.0.x)--> [CGNAT Router] --(Public IP)--> [Reflector]`

```bash
# 1. Create WAN Network (Simulated Public Internet)
podman network create wan-net --subnet 198.51.100.0/24

# 2. Create CGNAT Network (Simulated ISP Private Network)
podman network create cgnat-net --subnet 100.64.0.0/24 --internal

# 3. Deploy Reflector on WAN
podman run -d --name reflector-wan \
  --network wan-net --ip 198.51.100.10 \
  reflector:sim serve

# 4. Deploy CGNAT Router (Alpine with iptables)
podman run -d --name cgnat-router \
  --network wan-net --ip 198.51.100.1 \
  --cap-add=NET_ADMIN \
  alpine sh -c "
    apk add iptables;
    echo 1 > /proc/sys/net/ipv4/ip_forward;
    iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE;
    ip addr add 100.64.0.1/24 dev eth0; # Secondary IP for internal side?
    # Actually, connect to both networks:
    sleep infinity"

# Connect Router to CGNAT Net
podman network connect cgnat-net cgnat-router --ip 100.64.0.1

# 5. Deploy Client in CGNAT
podman run -it --rm --name client-cgnat \
  --network cgnat-net --ip 100.64.0.10 \
  --cap-add=NET_ADMIN \
  debian:bookworm-slim bash -c "
    ip route del default;
    ip route add default via 100.64.0.1;
    apt-get update && apt-get install -y openssl iperf3;
    # Test connection to Public Reflector
    echo | openssl s_client -connect 198.51.100.10:4000 -brief"
```

### 8.2 Simulating Double NAT

Architecture:
`[Client] --(192.168.1.x)--> [Home Router] --(100.64.0.x)--> [CGNAT Router] --(Public)--> [Reflector]`

1.  Repeat steps for CGNAT.
2.  Create `lan-net` (192.168.1.0/24).
3.  Deploy "Home Router" connected to `lan-net` and `cgnat-net`.
4.  Configure Home Router to SNAT `lan-net` -> `cgnat-net`.
5.  Deploy Client in `lan-net` with default gateway = Home Router.

This setup verifies if the protocol (mTLS over TCP) survives multiple NAT layers.
(Note: It should, as it's standard TCP. The main issue is Peer-to-Peer pairing *inbound* to client, which Reflector architecture avoids by having Client dial out).

**Symptom:** `failed to open audit log: Permission denied`

**Cause:** The audit log directory is not writable by the reflector process.

**Solution:**
```bash
# Same fix as above -- ensure the directory is writable
sudo mkdir -p /var/lib/reflector
sudo chown $(whoami) /var/lib/reflector
```

---

### iperf3 Errors

**Symptom:** `failed to spawn iperf3: No such file or directory`

**Cause:** iperf3 is not installed or not in `$PATH`.

**Solution:**
```bash
# Install iperf3
sudo dnf install iperf3     # Alma/RHEL
sudo apt install iperf3     # Debian/Ubuntu
brew install iperf3          # macOS

# Or set the absolute path in reflector.toml:
# [iperf3]
# path = "/usr/bin/iperf3"
```

---

**Symptom:** `iperf3 exited with code 1`

**Cause:** iperf3 failed to bind its port, or encountered a runtime error.

**Solution:**
```bash
# Test iperf3 manually
iperf3 -s -p 5201 --one-off
# If this fails, check for port conflicts or permission issues
```

---

### Connection Issues

**Symptom:** Client cannot connect to the reflector

**Cause:** Firewall blocking port 4000, or the reflector is not listening on
the expected interface.

**Solution:**
```bash
# Verify the reflector is listening
ss -tlnp sport eq 4000
# Expected: LISTEN on 0.0.0.0:4000 (or the configured address)

# Check firewall rules
sudo firewall-cmd --list-all   # RHEL/Alma
sudo ufw status                 # Ubuntu
sudo iptables -L -n             # Generic

# Test connectivity from the client
nc -zv <reflector-host> 4000
telnet <reflector-host> 4000
```

---

### Audit Log Issues

**Symptom:** Audit log file is empty or missing events

**Cause:** The reflector process may not have write permission, or the log path
may be incorrect.

**Solution:**
```bash
# Verify the log file exists and is writable
ls -la /var/lib/reflector/audit.jsonl

# Verify the path in the configuration
grep audit_log_path /etc/reflector/reflector.toml

# Check the reflector stderr for write errors
journalctl -u reflector --no-pager | grep "audit"
```

---

### Memory or CPU Issues

**Symptom:** Reflector uses excessive memory or CPU at idle

**Cause:** This should not happen under normal conditions. The reflector has a
minimal idle footprint (< 10 MB RSS, < 0.1% CPU).

**Solution:**
```bash
# Check resource usage
ps aux | grep reflector
top -p $(pgrep reflector) -d 1

# If excessive, check for:
# - Stuck sessions (cleanup task runs every 30 seconds)
# - Very large audit log (consider log rotation)
# - iperf3 zombie processes
pgrep -la iperf3
```

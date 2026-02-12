# Engineering Spec (RFC): PacketParamedic Self‑Hosted Endpoint (“Reflector”)
Status: Draft / RFC  
Target audience: Development team (Rust/Go)  
Primary goal: A lightweight, secure, drop‑in endpoint users can deploy on a VPS or LAN host to produce **reliable, reproducible WAN/LAN benchmarking** for PacketParamedic.

---

## 0) One‑sentence pitch
Reflector is a single-binary, cryptographically-identified endpoint that exposes a **zero-trust control plane** and a **tightly-scoped data plane** (throughput + latency reflector), designed to be safe to run on the public Internet without becoming an open relay.

---

## 1) Product vision & non‑negotiables

### 1.1 Goals (what “done” looks like)
| Goal | Definition | Acceptance |
|---|---|---|
| Lean idle footprint | Idle CPU < 0.1%, RAM steady, minimal wakeups | Verified on a $5 VPS and a LAN host |
| Single binary | No runtime deps; static where practical | `reflector --version` runs on clean Linux install |
| Cryptographic identity | No usernames/passwords; identity is a keypair | Stable Endpoint ID printed on first run |
| Zero trust by default | Reject unknown peers; no open relay mode | Unknown peers always denied at app layer |
| Ephemeral tests | Tests run only on demand; auto-kill | No persistent iperf/echo server “just running” |
| Auditable | Every action produces structured logs and an audit trail | Audit log can be exported for support |

### 1.2 Non‑goals (explicitly out of scope for v1)
| Non‑goal | Reason |
|---|---|
| NAT traversal magic / hole punching | Keep v1 simple and deterministic |
| Global discovery network | Adds complexity + abuse surface; optional later |
| Long-running traffic generation | This is diagnostics, not a stress platform |
| Multi-tenant SaaS accounts | Avoid cloud dependency + account surface |

---

## 2) Terminology
| Term | Meaning |
|---|---|
| Endpoint | The Reflector instance (VPS or LAN host) |
| Appliance | PacketParamedic device initiating tests |
| Peer | A mutually-authenticated device (Endpoint or Appliance) |
| Control plane | mTLS connection + RPC for session creation and policy |
| Data plane | The actual throughput/latency traffic |

---

## 3) Architecture overview

### 3.1 High-level design
Reflector is composed of:
1) **Identity + mTLS listener** (TCP, fixed port)  
2) **Authorization gate** (allowlist; pairing mode optional)  
3) **Session manager** (quotas, concurrency, scheduling windows)  
4) **Test engines**:
   - Throughput engine (iperf3 server or native test engine later)
   - UDP echo reflector (built-in)
   - Path/meta reporting (load, MTU, link facts)

### 3.2 Two data-plane modes (choose per deployment)
| Mode | What’s open to the Internet | Pros | Cons | Recommended |
|---|---|---|---|---|
| **Tunneled** (default) | Only the mTLS control port | “Never open extra ports”; simplest firewall | Adds proxy overhead; highest-throughput may be limited by tunnel | Public VPS where safety > peak speed |
| **Direct ephemeral** (opt-in) | Short-lived ephemeral port(s) for tests | Best chance at max line-rate | Requires firewall choreography; higher abuse risk | Trusted environments / advanced users |

Design requirement: both modes share the same session policy + audit log.

---

## 4) Identity & Endpoint ID

### 4.1 Cryptographic identity
- Key type: **Ed25519** keypair (RFC 8032): https://www.rfc-editor.org/rfc/rfc8032
- Private key generated on first run, stored locally, never transmitted.
- Public key is used to derive a stable Endpoint ID and certificate identity.

### 4.2 Endpoint ID format (human-friendly)
Requirements:
| Property | Requirement |
|---|---|
| Human-safe | Easy to read over phone/chat |
| Error detection | Include check digits (Luhn) |
| Stable | Derived only from public key |
| No PII | Contains no IP/hostname |

Recommended encoding:
- Base32 (Crockford): https://www.crockford.com/base32.html  
- Check digits: Luhn algorithm: https://en.wikipedia.org/wiki/Luhn_algorithm

Proposed format (example):
`PP5R-6Q2M-1K9D-...-C3`  
(Exact length can be tuned; prioritize “copy/paste + verbal dictation”.)

### 4.3 Key storage & rotation
| Item | Best practice |
|---|---|
| At-rest protection | Restrict file perms (0600), dedicated user account |
| Backup | Document “how to back up identity” (optional but recommended) |
| Rotation | Provide `reflector rotate-identity` (creates new ID; requires re-pair) |
| Recovery | If key lost, endpoint is a new endpoint; explicit re-trust |

---

## 5) Transport, handshake, and authorization

### 5.1 Transport
- TLS: **TLS 1.3** (RFC 8446): https://www.rfc-editor.org/rfc/rfc8446
- Mutual TLS (mTLS) over TCP.
- Recommended Rust TLS stack: rustls https://github.com/rustls/rustls  
  (Go: `crypto/tls` with TLS1.3 + ed25519 certs.)

### 5.2 Certificates
- Each peer presents a self-signed certificate whose public key corresponds to its device identity.
- The certificate’s SubjectAltName contains:
  - `pp:id:<DEVICE_ID>` (string)
  - Optional: `pp:pk:<PUBKEY_HASH>` (pinning convenience)

### 5.3 Authorization (non-negotiable)
The TLS handshake can succeed cryptographically, but the application must enforce:
- **Allowlist**: connection is rejected unless peer ID is in `authorized_peers`.
- “Pairing mode” (optional): temporary window where a one-time token enables adding a new peer.

### 5.4 Pairing mode (user-presence required)
Pairing must require explicit user action on the endpoint host (to prevent drive-by enrollment).
Recommended pattern:
- `reflector pair --ttl 10m` prints a pairing token and temporarily allows a single enrollment.
- Enrollment binds the peer ID permanently into `authorized_peers` (append-only audit entry).

---

## 6) Control plane protocol (“Paramedic Link”)

### 6.1 Goals
| Goal | Approach |
|---|---|
| Small overhead | Binary framing, small messages |
| Easy debugging | Optional JSON mode or “trace decode” |
| Stable evolution | Versioned messages and capability negotiation |
| Safety | Strict input validation + bounded allocations |

### 6.2 Framing
- Single mTLS TCP connection, multiplexed request/response.
- Message: length-prefixed frames (u32 BE length + payload).
- Payload format options:
  - v1: JSON (fast to ship, easy debug)
  - v2: CBOR (smaller, faster) — optional later

### 6.3 RPC surface (v1)
All requests include `request_id` and `session_id` (where applicable).

#### Capability negotiation
- `Hello(version, client_features[]) -> ServerHello(version, server_features[], policy_summary)`

#### Session management
- `SessionRequest(test_type, params) -> SessionGrant(test_id, mode, ports, token, expires_at)`
- `SessionClose(test_id) -> Ok`
- `GetStatus() -> StatusSnapshot`

#### Meta
- `GetPathMeta() -> { cpu, mem, load, mtu, iface, time_sync, build }`

---

## 7) Test engines (“the tests”)
The Endpoint exposes a JSON-RPC / gRPC-lite interface over the mTLS tunnel.

### 7.1 Throughput (Wrapper for iperf3)
*   **Request:** `request_throughput { duration: 30s, protocol: tcp|udp, streams: 4, reverse: bool }`
*   **Endpoint Logic:**
    1.  Check quotas (is another test running?).
    2.  Find a free ephemeral port (e.g., 5201-5299).
    3.  Spawn `iperf3 -s -p <port> --one-off`.
    4.  Return: `{ status: "ok", port: <port>, token: "<auth_cookie>" }`.
*   **Bufferbloat Support:** This test is used as the "Load Generator" for Bufferbloat analysis.

### 7.2 Latency Reflector (UDP Echo / ICMP)
*   **Request:** `request_reflector { mode: "udp_echo", duration: 60s }`
*   **Endpoint Logic:**
    *   Starts a high-performance UDP echo socket on an ephemeral port.
    *   Returns the port number.
    *   *Why internal?* UDP Echo (RFC 862 style) allows accurate application-layer RTT measurement without root privileges (unlike ICMP).
*   **ICMP:** The host OS should respond to ICMP Echo Requests automatically. Ensure firewall allows `icmp-echo-request`.

### 7.3 Trace Target (MTR/Traceroute)
*   **Capability:** The endpoint must respond to:
    *   **ICMP Echo Request** (Traditional traceroute).
    *   **UDP to High Ports** (Unix traceroute) -> Respond with `ICMP Port Unreachable`.
    *   **TCP SYN to Port 80/443** (TCP traceroute) -> Respond with `SYN-ACK` or `RST`.

### 7.4 Reachability & Health (HTTP/TCP)
*   **Request:** `GET /health` (over mTLS tunnel or public HTTP port if enabled)
*   **Endpoint Logic:**
    *   Returns `{ status: "ok", version: "1.0.0", load: 0.05 }`.
    *   Used for "HttpProbe" and "TcpProbe" validation.

### 7.5 Path/Meta Helper
`GetPathMeta` returns facts to prevent mis-blame:
| Field | Purpose |
|---|---|
| CPU/load/mem | detect endpoint bottleneck |
| Network bytes/sec | detect endpoint congestion |
| MTU | catch PMTU issues affecting throughput |
| Time/clock status | avoid timeline corruption |
| Build info | correlate results to reflector version |

---

## 8) Resource governance (anti-abuse & correctness)

### 8.1 Quotas and caps
| Control | Default | Rationale |
|---|---:|---|
| Max concurrent tests | 1 | Prevent endpoint becoming a load generator |
| Max duration | 60s | Keeps exposure windows short |
| Per-peer tests/hour | 10 | Protect from automation/abuse |
| Per-peer bytes/day | configurable | Avoid surprise bills on metered hosts |
| Cooldown | configurable | Prevent repeated back-to-back tests |

### 8.2 Policy enforcement points
- Before granting a session: enforce allowlist + rate limits + “one test at a time”.
- During a session: enforce timeouts, kill child processes, clamp UDP echo rate.
- After session: record summary, update accounting, revoke token.

### 8.3 Audit log (required)
Every grant/deny must log:
- peer_id, endpoint_id
- test_type + params (redacted where relevant)
- decision + reason
- resource accounting deltas
- subprocess details (pid, exit code, runtime)

---

## 9) Security requirements (non-negotiable)

### 9.1 System hardening
Recommended systemd hardening references:
- systemd.exec security options: https://www.freedesktop.org/software/systemd/man/systemd.exec.html

Best practices (Linux):
| Practice | Target |
|---|---|
| Dedicated user | `reflector` user, no shell |
| No root | Run unprivileged; avoid CAPs |
| Read-only filesystem | Only writable state dir (`/var/lib/reflector`) |
| PrivateTmp / ProtectSystem | Enable systemd protections |
| Memory limits | Optional cgroup caps for VPS safety |
| Crash safety | Fail closed; do not auto-open ports on error |

### 9.2 Network posture
| Rule | Requirement |
|---|---|
| Default inbound | Only control plane port (e.g. 4000/tcp) |
| Direct ephemeral mode | Off by default; explicitly enabled |
| IPv6 | Supported; must enforce same allowlist logic |
| Pairing | Time-limited; must be explicit |

### 9.3 Supply chain & updates (optional but recommended)
Signed update options:
- minisign: https://jedisct1.github.io/minisign/
- Sigstore: https://www.sigstore.dev/

Recommendation:
- Provide `reflector self-update` that verifies a signed manifest and signed binary artifact.
- Allow disabling auto-update for locked-down environments.

---

## 10) Deployment guide (v1)

### 10.1 Supported platforms
| Platform | Support |
|---|---|
| Linux amd64 | Yes |
| Linux arm64 | Yes |
| macOS/Windows | Not required for v1 (optional for LAN users later) |

### 10.2 Minimal firewall guidance (examples)
Tunneled mode:
- Allow inbound TCP 4000 only.

Direct ephemeral mode:
- Allow inbound TCP 4000
- Allow a narrow ephemeral port range *only during test window* (implementation-defined; avoid leaving open)

(Exact `ufw`/`iptables` snippets should be provided in docs, but v1 must work without requiring complex firewall gymnastics.)

---

## 11) Implementation plan (engineering)

### Phase 1 — Identity + handshake (Week 1)
Deliverable:
- `reflector serve` prints:
  - Endpoint ID
  - Listen address
  - Pairing instructions (if pairing enabled)
- Accepts mTLS connection; rejects unauthorized peers with clear reason codes.

### Phase 2 — Sessions + UDP echo (Week 2)
Deliverable:
- `SessionRequest(udp_echo)` works end-to-end.
- Rate limiting + accounting enforced.

### Phase 3 — iperf3 integration (Week 3)
Deliverable:
- `SessionRequest(throughput)` grants a window and runs iperf3 (tunneled mode first).
- Parse iperf3 JSON and return normalized result.

### Phase 4 — Appliance integration (Week 4)
Deliverable:
- PacketParamedic adds “SelfHostedLAN/SelfHostedWAN” provider kind and uses Reflector sessions.
- Scheduler respects bandwidth windows and “one heavy test at a time” policy.

---

## 12) Configuration (TOML)

```toml
# reflector.toml

[identity]
private_key_path = "/var/lib/reflector/identity.ed25519"

[network]
listen_address = "0.0.0.0:4000"
alpn = "pp-link/1"
mode = "tunneled" # tunneled | direct_ephemeral
# If direct_ephemeral:
data_port_range_start = 5201
data_port_range_end = 5299

[access]
pairing_enabled = false
authorized_peers = [
  "PP....", # PacketParamedic Appliance IDs
]

[quotas]
max_test_duration_sec = 60
max_concurrent_tests = 1
max_tests_per_hour_per_peer = 10
max_bytes_per_day_per_peer = 5000000000 # 5 GB
allow_udp_echo = true
allow_throughput = true

[iperf3]
path = "/usr/bin/iperf3"
default_streams = 4
max_streams = 8

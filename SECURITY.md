# Security Policy

## Supported Versions

Since PacketParamedic is an appliance targeting specific hardware, we support the latest stable release and the current `main` branch.

| Version | Supported | Notes |
| ------- | ------------------ | ------------------------------------------------------------------ |
| `0.x.y` | :white_check_mark: | Active development. Breaking changes possible. |
| `main` | :white_check_mark: | Latest development snapshot. Includes passing integration tests. |

## Reporting a Vulnerability

Please do not report security vulnerabilities through public GitHub issues.

Instead, please report them via email to `security@packetparamedic.org` (or open a private GitHub security advisory).

We will acknowledge your report within 48 hours and provide an estimated timeline for a fix.

## Security Posture

PacketParamedic is designed as a secure network appliance with the following principles:

1.  **Minimal Attack Surface:**
    *   No open ports by default except the local web UI (typically port 8080).
    *   No inbound WAN ports ever opened.
    *   No default credentials. Authentication is mandatory for all API access.

2.  **Least Privilege:**
    *   System services run with minimal capabilities (`CAP_NET_RAW`, `CAP_NET_ADMIN` as needed), not as root where possible.
    *   Throughput tests (iperf3) run as ephemeral child processes with strict timeouts, never as persistent daemons.

3.  **Data Privacy:**
    *   Support bundles automatically redact sensitive information (internal IPs, MAC addresses, SSIDs).
    *   No telemetry is sent without explicit user opt-in.

4.  **Supply Chain Security:**
    *   Dependencies are audited using `cargo audit`.
    *   License compliance is enforced using `cargo deny`.

## Remote Access

We recommend using **Tailscale** for remote administration. PacketParamedic integrates with Tailscale to provide zero-trust, encrypted remote access without opening inbound firewall ports.

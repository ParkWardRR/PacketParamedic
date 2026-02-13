# PacketParamedic Reflector Validation Report

**Date:** 2026-02-13 08:00:29 UTC
**Client Host:** `packetparamedic.alpina` (Raspberry Pi 5)
**Scope:** Full Validation of LAN/WAN Reflector Integration & Public Benchmarks.

## 1. Executive Summary
This report confirms the successful operation of the PacketParamedic Reflector infrastructure. The system demonstrated near-line-rate performance on local LAN (940 Mbps) and functional connectivity to remote LAN endpoints. Public benchmarks confirm WAN uplink capacity.

**Key Findings:**
*   **Mac Reflector (LAN):** **939 Mbps** (Download) / **936 Mbps** (Upload). **PASS**.
*   **IRWW Reflector (LAN):** **88.5 Mbps** (Download) / **92.2 Mbps** (Upload). **WARNING**: Link negotiation appears capped at 100Mbps.
*   **Public WAN (Ookla):** **298 Mbps** (Download) / **40 Mbps** (Upload). **PASS**.

## 2. Environment Configuration
| Role | Hostname | IP Address | OS | Deployment |
|---|---|---|---|---|
| **Client** | `packetparamedic.alpina` | 172.16.x.x | Debian 12 | Native Binary |
| **LAN Target 1** | `172.16.16.222` (Mac) | 172.16.16.222 | macOS (OrbStack) | Docker Container |
| **LAN Target 2** | `irww.alpina` | 172.16.19.199 | AlmaLinux 9 | Podman Quadlet* |
| **WAN Target** | `ookla` | Public | N/A | SaaS |

*\*Note: IRWW currently running manual container for validation.*

## 3. Test Results

### 3.1 Local LAN (Client -> Mac OrbStack)
*Objective: Validation of high-speed LAN throughput.*
*   **Timestamp:** 2026-02-13T07:58:58Z
*   **Target:** `172.16.16.222:4000`
*   **Download:** **939.35 Mbps**
*   **Upload:** **936.21 Mbps**
*   **Verdict:** **EXCELLENT**. Saturates 1GbE interface.

### 3.2 Remote LAN (Client -> IRWW)
*Objective: Validation of remote Reflector reliability.*
*   **Timestamp:** 2026-02-13T08:00:06Z
*   **Target:** `172.16.19.199:4000`
*   **Download:** **88.50 Mbps**
*   **Upload:** **92.19 Mbps**
*   **Verdict:** **WARN**. Throughput inconsistent with 1GbE hardware. Suggests cabling fault or switch port set to 100Mbps/Half-Duplex on IRWW segment.

### 3.3 Public Internet (Client -> Speedtest.net)
*Objective: ISP Uplink verification.*
*   **Timestamp:** 2026-02-13T08:00:29Z
*   **Provider:** Ookla (GigabitNow, Los Angeles)
*   **Download:** **298 Mbps**
*   **Upload:** **40 Mbps**
*   **Latency:** 20ms
*   **Verdict:** **PASS**. Consistent with typical cable/fiber mid-tier plans.

## 4. Observations & Recommendations
1.  **IRWW Link Speed:** Investigate physical cabling or switch port configuration for `irww.alpina`. Attempt `ethtool eth0` locally to verify negotiation (likely 100Mbps).
2.  **Reflector Stability:** Mac instance is stable. IRWW instance required manual start (Quadlet user mapping needs adjustment for `reflector` vs `root` user inside container relative to host volume permissions).
3.  **Deployment:** Full Git Sync strategy proved successful for reliable builds.

## 5. Next Steps
*   Fix Quadlet `User=0` mapping in `deploy_fix.sh`.
*   Debug IRWW L1 physical connection.
*   Enable scheduled nightly tests.

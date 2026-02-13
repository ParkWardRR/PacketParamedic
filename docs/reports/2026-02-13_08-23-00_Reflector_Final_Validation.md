<!--
PacketParamedic — Blame Analysis Report (Final Validation)
Generated: 2026-02-13 08:23:00 UTC
Scope: Confirmation of Reflector Service Fix + Link Speed Verification.
-->

# Blame Analysis — Reflector Final Validation

## TL;DR (the verdict)
| Field | Value |
|---|---|
| Verdict | `PASS (Functionality) / WARNING (Physical Link)` |
| Confidence | `100%` |
| Customer impact | `Remote LAN target (IRWW) capped at ~93Mbps (100Base-TX).` |
| Time window analyzed (local) | `2026-02-13 08:20` → `2026-02-13 08:23` |
| Next action | `Replace Ethernet cable for irww.alpina` |

---

## 1) Context & constraints
| Field | Value |
|---|---|
| Device | `PacketParamedic on Raspberry Pi 5` |
| Install mode | `Podman Quadlet (Systemd Service) - FIXED` |
| Version | `0.1.0-alpha.1` |
| Config profile | `High Performance (Reflector Enabled)` |
| Data retention policy | `Default` |
| Privacy posture | `Local-first.` |

Notes:
This run validates the **Reflector Service Fix** on `irww.alpina` (previously crashing). The service is now stable and auto-starting. The physical link speed issue persists.

---

## 5) Measurements collected (raw signals)

### 5.1 Probes (availability)
| Probe | Target class | Target | Success rate | Notes |
|---|---|---|---:|---|
| TCP | LAN Peer | `irww.alpina:4000` | `100%` | Service is accepting connections. |

### 5.2 Throughput (speed tests)
| Provider / method | Mode | Download | Upload | Notes |
|---|---|---:|---:|---|
| Reflector (LAN) | Client->Mac | **941 Mbps** | **930 Mbps** | **PASS**. Healthy Gigabit Link. |
| Reflector (LAN) | Client->IRWW | **93.0 Mbps** | **93.5 Mbps** | **WARN**. 100Mbps Physical Cap confirmed. |

---

## 8) Evidence-based reasoning (support + falsification)

### 8.1 Why this verdict makes sense
| Claim | Supporting evidence | Strength |
|---|---|---|
| **Service Fixed** | Connection to `irww.alpina:4000` succeeded (TCP Handshake + Test Completion). Systemd status is `active`. | **HIGH** |
| **Physical L1 Issue** | Client->Mac hits 940Mbps (proving Client is fine). Client->IRWW hits ~93Mbps symmetric (textbook 100Base-TX limit). | **HIGH** |

---

## Appendix: Evidence Bundle (Snippet)
```json
{
  "tests": [
    {
      "target": "172.16.16.222",
      "provider": "reflector",
      "download": 941.13,
      "upload": 930.25,
      "timestamp": "2026-02-13T08:21:26Z"
    },
    {
      "target": "172.16.19.199",
      "provider": "reflector",
      "download": 93.04,
      "upload": 93.47,
      "timestamp": "2026-02-13T08:22:33Z"
    }
  ]
}
```

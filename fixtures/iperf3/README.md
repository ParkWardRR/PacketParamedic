# iperf3 Test Fixtures

Realistic iperf3 `--json` output samples for testing the PacketParamedic throughput parser.

All fixtures simulate a 30-second test on a Pi 5 (aarch64) system.

## Speed Tiers

| File | Speed | Protocol | Streams | Status |
|---|---|---|---|---|
| `250m-tcp.json` | 250 Mbps | TCP | 1 | Current |
| `250m-udp.json` | 250 Mbps | UDP | 1 | Current |
| `500m-tcp.json` | 500 Mbps | TCP | 1 | Current |
| `500m-udp.json` | 500 Mbps | UDP | 1 | Current |
| `1g-tcp.json` | 1 Gbps | TCP | 1 | Current (onboard NIC) |
| `1g-udp.json` | 1 Gbps | UDP | 1 | Current (onboard NIC) |
| `2.5g-tcp.json` | 2.5 Gbps | TCP | 1 | Future |
| `2.5g-udp.json` | 2.5 Gbps | UDP | 1 | Future |
| `5g-tcp.json` | 5 Gbps | TCP | 4 | Future |
| `5g-udp.json` | 5 Gbps | UDP | 4 | Future |
| `10g-tcp.json` | 10 Gbps | TCP | 8 | Future |
| `10g-udp.json` | 10 Gbps | UDP | 8 | Future |
| `40g-tcp.json` | 40 Gbps | TCP | 16 | Future |
| `40g-udp.json` | 40 Gbps | UDP | 16 | Future |
| `100g-tcp.json` | 100 Gbps | TCP | 16 | Future |
| `100g-udp.json` | 100 Gbps | UDP | 16 | Future |

## Notes

- **Current** tiers are achievable on Pi 5 hardware today (onboard 1GbE and common sub-gigabit links).
- **Future** tiers (2.5G, 5G, 10G, 40G, 100G) are for forward-looking parser testing; requires PCIe NIC or future hardware.
- TCP fixtures include `retransmits`; UDP fixtures include `jitter_ms`, `lost_packets`, and `lost_percent`.
- CPU utilization scales realistically: ~3% at 250M, ~13% at 1G, ~79% at 10G, ~99% at 100G.
- Throughput values are slightly below theoretical maximum (93-97% efficiency).
- Higher-speed fixtures use more parallel streams (realistic for saturating high-bandwidth links).

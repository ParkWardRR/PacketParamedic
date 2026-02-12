# PacketParamedic Buyer's Guide

This guide helps you choose the right hardware for your PacketParamedic appliance based on which [persona](personas.md) matches your needs.

> **Why Pi 5 Only?** We rely on specific silicon features (Cortex-A76, VideoCore VII, PCIe). See our [Hardware Optimization Strategy](docs/HARDWARE_OPTIMIZATION.md) for details.

---

## Quick Summary Table

| Feature | **Simple Troubleshooting** | **Reliability & Uptime** | **High Performance** |
| :--- | :--- | :--- | :--- |
| **Compute** | Pi 5 (4GB) | Pi 5 (4GB/8GB) | Pi 5 (8GB) |
| **Storage** | 32GB SD (A1/A2) | 128GB NVMe (Reliability) | 256GB+ NVMe (Permormance) |
| **Network** | Built-in 1GbE | Built-in 1GbE | PCIe 2.5G/10G HAT |
| **Cooling** | Active Fan | Silent/Passive | Active + PCIe Cooling |
| **Cost** | Entry Level | Mid Range | High End |

---

## 1. Simple Troubleshooting
*“I just want to know why Netflix is buffering and learn a bit about networking.”*

This use case needs the minimum viable setup to get the software running and performing basic health checks (gateway, DNS, WAN ping). The built-in hardware on the Pi 5 is more than enough.

**Recommended Bill of Materials:**
*   **Raspberry Pi 5 (4GB)**: Sufficient for the daemon, web UI, and basic probes.
*   **Power Supply**: Official 27W USB-C Power Supply.
*   **Storage**: 32GB SanDisk Extreme microSD card (A2 app performance class).
*   **Case**: Official Raspberry Pi 5 Case (includes fan).
*   **Network Cable**: Cat5e or Cat6 patch cable (connect directly to router LAN port).

**Why this works:** You aren't doing heavy 10Gbps throughput testing. You need the "doctor" functionality (logic/analysis), which runs fine on the CPU.

---

## 2. Reliability & Uptime
*“I need this to work reliably for years so I can prove to Comcast it’s their fault.”*

 This use case prioritizes reliability and "set-and-forget" operation. SD cards can wear out with constant database writes (SQLite WAL mode). An NVMe SSD is highly recommended to ensure the "black box" evidence recorder survives for the long haul.

**Recommended Bill of Materials:**
*   **Raspberry Pi 5 (4GB or 8GB)**.
*   **Power Supply**: Official 27W USB-C Power Supply.
*   **Storage**: **NVMe SSD Base (e.g., Pimoroni or Pineberry)** + **128GB M.2 2230/2242 NVMe SSD**.
    *   *Reason*: Much higher reliability than SD cards; faster database queries for historical/blame reports.
*   **Case**: Look for a case that accommodates the NVMe base.
*   **Network Cable**: Cat6 patch cable.

**Why this works:** The NVMe drive ensures the database (the "evidence locker") doesn't get corrupted or slow down over months of logging.

---

## 3. High Performance
*“I want to saturate my Gigabit uplink and feed metrics to Prometheus.”*

This use case uses PacketParamedic as a high-performance probe. The built-in 1GbE port is perfect for the current feature set (Phase 6), but you can add a faster NIC now to be ready for future multi-gigabit updates (Phase 14).

**Recommended Bill of Materials:**
*   **Raspberry Pi 5 (8GB)**: Extra RAM for large in-memory buffers during high-throughput tests.
*   **Network Upgrade (Future Proofing)**: **PCIe Network HAT (Pineberry Pi HatNET! or similar)**.
    *   Option A: 2.5GbE (Intel I225/I226 chipset) - *Note: Full 2.5Gbps throughput planned for Phase 14.*
    *   Option B: 10GbE (Aquantia AQC107 chipset) - *Note: Planned for Phase 14. Requires external power or careful thermal management.*
*   **Storage**: High-end NVMe SSD (Samsung PM991a or Sabrent Rocket 2230) 256GB+.
*   **Cooling**: Active cooler is mandatory. The PCIe NIC + NVMe + CPU load will generate heat.
*   **Switching**: Must be connected to a Multi-Gig (2.5G/10G) capable provider switch or router port.

**Why this works:** This unlocks the throughput capabilities to validate ISP speeds up to 1Gbps today, with a hardware path to 2.5Gbps+ in the future.

---

## Optional Upgrades

### Power over Ethernet (PoE+)
If you have a PoE+ switch, you can power the Pi 5 via a PoE+ HAT, eliminating the need for a separate USB-C power supply. This is great for cleaner racking but may conflict with some PCIe HATs (check clearance).

### Uninterruptible Power Supply (UPS)
For maximum uptime, a small USB-C UPS or HAT-based UPS can keep the device running during short power outages, ensuring your "evidence locker" captures the exact moment power returns or fails.

### Wi-Fi Capture (Standardized Add-on)
For advanced RF diagnostics, monitor mode, and frame injection, we standardize on the **MT7612U** chipset. It is fully supported in the main Linux kernel (`mt76`), avoiding the instability often associated with proprietary drivers.

*   **Single-Radio Capture Kit**: **1× ALFA AWUS036ACM**.
    *   *Best for*: Monitor mode, frame injection, and general sniffing on one channel at a time.
*   **Dual-Radio Concurrent Kit**: **2× ALFA AWUS036ACM**.
    *   *Best for*: Simultaneous capture/injection on different channels (e.g., park one radio, hop the other).
    *   *Why*: A single Wi-Fi radio cannot physically monitor multiple channels at once. If you need to watch two bands or channels simultaneously, you need two physical radios.

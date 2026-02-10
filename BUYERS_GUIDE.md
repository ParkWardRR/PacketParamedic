# PacketParamedic Buyer's Guide

This guide helps you choose the right hardware for your PacketParamedic appliance based on which [persona](personas.md) matches your needs.

---

## Quick Summary Table

| Feature | **Alex** (Tech-Curious) | **Jamie** (Household Manager) | **Sam** (Home Lab Expert) |
| :--- | :--- | :--- | :--- |
| **Compute** | Raspberry Pi 5 (4GB) | Raspberry Pi 5 (4GB/8GB) | Raspberry Pi 5 (8GB recommended) |
| **Storage** | 32GB microSD (A2 class) | 64GB+ NVMe SSD (Reliability) | 256GB+ NVMe SSD (Performance/Log retention) |
| **Network** | Built-in Gigabit Ethernet | Built-in Gigabit Ethernet | PCIe 2.5GbE or 10GbE Card (e.g., Pineberry Pi Hat) |
| **Power** | Official 27W USB-C | Official 27W USB-C (UPS recommended) | Official 27W + PoE+ HAT (optional) |
| **Case** | Official Case with Fan | Passive Cooling / Silent Case | Active Cooling / Armor Case |
| **Est. Cost** | ~$90 - $120 | ~$150 - $200 | ~$200 - $300 |

---

## 1. For Alex (The Tech-Curious)
*“I just want to know why Netflix is buffering and learn a bit about networking.”*

Alex needs the minimum viable setup to get the software running and performing basic health checks (gateway, DNS, WAN ping). The built-in hardware on the Pi 5 is more than enough.

**Recommended Bill of Materials:**
*   **Raspberry Pi 5 (4GB)**: Sufficient for the daemon, web UI, and basic probes.
*   **Power Supply**: Official 27W USB-C Power Supply.
*   **Storage**: 32GB SanDisk Extreme microSD card (A2 app performance class).
*   **Case**: Official Raspberry Pi 5 Case (includes fan).
*   **Network Cable**: Cat5e or Cat6 patch cable (connect directly to router LAN port).

**Why this works:** Alex isn't doing heavy 10Gbps throughput testing. They need the "doctor" functionality (logic/analysis), which runs fine on the CPU.

---

## 2. For Jamie (The Household Manager)
*“I need this to work reliably for years so I can prove to Comcast it’s their fault.”*

Jamie prioritizes reliability and "set-and-forget" operation. SD cards can wear out with constant database writes (SQLite WAL mode). An NVMe SSD is highly recommended to ensure the "black box" evidence recorder survives for the long haul.

**Recommended Bill of Materials:**
*   **Raspberry Pi 5 (4GB or 8GB)**.
*   **Power Supply**: Official 27W USB-C Power Supply. **Strongly consider a small USB-C UPS** to keep logging during power blips.
*   **Storage**: **NVMe SSD Base (e.g., Pimoroni or Pineberry)** + **128GB M.2 2230/2242 NVMe SSD**.
    *   *Reason*: Much higher reliability than SD cards; faster database queries for historical/blame reports.
*   **Case**: Look for a case that accommodates the NVMe base.
*   **Network Cable**: Cat6 patch cable.

**Why this works:** The NVMe drive ensures the database (the "evidence locker") doesn't get corrupted or slow down over months of logging.

---

## 3. For Sam (The Home Lab Expert)
*“I want to saturate my 2.5GbE uplink and feed metrics to Prometheus.”*

Sam wants to use PacketParamedic as a high-performance probe. The built-in 1GbE port is a bottleneck. Sam needs to tap into the Pi 5's PCIe lane to add a faster NIC.

**Recommended Bill of Materials:**
*   **Raspberry Pi 5 (8GB)**: Extra RAM for large in-memory buffers during high-throughput tests.
*   **Network Upgrade**: **PCIe Network HAT (Pineberry Pi HatNET! or similar)**.
    *   Option A: 2.5GbE (Intel I225/I226 chipset).
    *   Option B: 10GbE (Aquantia AQC107 chipset) - *Note: Requires external power or careful thermal management.*
*   **Storage**: High-end NVMe SSD (Samsung PM991a or Sabrent Rocket 2230) 256GB+.
*   **Cooling**: Active cooler is mandatory. The PCIe NIC + NVMe + CPU load will generate heat.
*   **Switching**: Must be connected to a Multi-Gig (2.5G/10G) capable provider switch or router port.

**Why this works:** This unlocks the "Phase 6" throughput capabilities, allowing Sam to validate ISP speeds beyond 1Gbps and analyze bufferbloat at line rate.

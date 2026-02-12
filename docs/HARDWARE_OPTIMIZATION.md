# Raspberry Pi 5 Hardware Optimization Strategy ⚡️

> **Philosophy:** Assume the hardware is a Raspberry Pi 5. If it's not, fail fast. Build for specific silicon, not generic compatibility.

## 1. The Silicon Target: BCM2712

PacketParamedic is exclusively tuned for the Broadcom BCM2712 SoC found in the Pi 5. We leverage every subsystem:

| Subsystem | Feature | PacketParamedic Usage |
|---|---|---|
| **CPU** | 4x Arm Cortex-A76 @ 2.4GHz | Out-of-order execution pipeline; heavy lifting. |
| **SIMD** | Arm NEON / ASIMD (128-bit) | Statistical analysis (mean/variance/stddev) on large datasets. |
| **GPU** | VideoCore VII (800MHz) | Vulkan 1.2 Compute Shaders for massive parallel log analysis. |
| **PCIe** | Gen 2.0 x1 (User-accessible) | Low-latency NVMe storage and High-Bandwidth NICs (avoiding USB bus). |
| **Crypto** | Armv8 Cryptographic Extensions | Hardware-accelerated HTTPS (TLS 1.3) and SSH via OpenSSL/BoringSSL. |

---

## 2. Active Optimizations

### 2.1 CPU Topology & Pinning (Core Isolation)
The OS and API server are bursty but latency-sensitive. Throughput tests (`iperf3`) are sustained and cache-thrashing.
**Optimization:** We strictly pin `iperf3` processes to **Cores 2 and 3** using `taskset`.
*   **Cores 0, 1:** Reserved for OS interrupts, API server (Tokio runtime), and Scheduler.
*   **Cores 2, 3:** Dedicated to `iperf3` measurement threads.
*   **Benefit:** Prevents the measurement tool from starving the metrics collection engine.

### 2.2 NEON Intrinsics (Data Plane)
We do not rely on compiler auto-vectorization alone. Critical loops verify `#[cfg(target_arch = "aarch64")]` and use `std::arch::aarch64` intrinsics.
*   **Implementation:** `src/accel/neon.rs`
*   **Mechanism:** 128-bit registers process 4x `f32` or 2x `f64` values per cycle.
*   **Fallback:** Removed. If NEON is missing, the binary panics. (Pi 5 guarantees NEON).

### 2.3 GPU Compute (Vulkan 1.2)
We bypass the GL stack for compute-heavy tasks using Vulkan.
*   **Driver:** Mesa V3DV (VideoCore VII).
*   **Shaders:** SPIR-V compute shaders dispatch via `ash` crate.
*   **Benefit:** Offloads "Blame Analysis" (Model Inference) from CPU, allowing the Cortex cores to stay responsive for real-time packet capture.

---

## 3. Hardware Scaling Strategy & Future Migration
The Raspberry Pi 5 is our "Tier 1" platform for home Gigabit networking.

### 3.1 The 10GbE Ceiling
The Pi 5 exposes a single PCIe Gen 2.0 x1 lane.
*   **Physics:** 5.0 GT/s = ~4 Gbps theoretical max (unidirectional).
*   **Reality:** We can saturate a **2.5GbE** link, but **10GbE cards will capped at ~3.5-4.0 Gbps**.
*   **Conclusion:** The Pi 5 is physically incapable of line-rate 10Gbps throughput.

### 3.2 The Strategic Pivot (Phase 14)
For the "Future High-Performance" phase (True 10Gbps), we will likely migrate to a new hardware target rather than fighting the Pi's limitations.
*   **Target:** x86-64 N100/N305 or RK3588-based boards with PCIe 3.0 x4 lanes.
*   **Focus Now:** We are laser-focused on continuous monitoring for **1Gbps and below**, which covers 99% of residential use cases.
*   **Why:** Better to be the perfect 1Gbps appliance than a mediocre 10Gbps one.

# Acceleration Strategy: The "Overuse" Architecture

PacketParamedic leverages a multi-tiered acceleration strategy designed to maximize throughput and responsiveness on the Raspberry Pi 5. We treat hardware acceleration as a first-class citizen, implementing critical data-parallel operations across three distinct backends.

## Philosophy: "Overuse is Non-Negotiable"

For computationally intensive backend tasks—particularly those involving batch data analytical processing—we implement robust hardware acceleration paths. We do not rely solely on the CPU.

Every accelerated operation MUST have three implementations:
1.  **`vk_compute` (Preferred)**: Vulkan 1.2 Compute Shaders.
    *   *Ideal for:* Large throughput, highly parallel tasks, complex kernels.
    *   *Target:* VideoCore VII (V3DV driver).
    *   *Pros:* First-class compute, explicit synchronization, shared memory.
2.  **`gles3_computeish` (Fallback #1)**: OpenGL ES 3.1 Render Passes.
    *   *Ideal for:* Image-like or grid-like transforms, devices where Vulkan is unavailable.
    *   *Target:* VideoCore VII (Mesa V3D driver).
    *   *Pros:* Ubiquitous driver support.
    *   *Cons:* Requires "render-to-texture" emulation for general compute; higher overhead.
3.  **`neon_cpu` (Fallback #2 / Latency-Sensitive)**: ARM NEON SIMD (ASIMD).
    *   *Ideal for:* Scanning, parsing, small batches, latency-critical loops where GPU dispatch overhead is prohibitive.
    *   *Target:* Cortex-A76 (Guaranteed on Pi 5).
    *   *Pros:* Zero transfer overhead, low latency, always available.

Plus a **Scalar Reference** (CPU) implementation for verification and debugging.

## Backend Task Mapping

We map specific PacketParamedic workloads to these backends based on their characteristics:

### 1. Statistical Anomaly Detection (Histograms & Percentiles)
*   **Task:** Analyzing thousands of latency samples to compute p95, p99, and jitter distribution.
*   **Best Backend:** **`vk_compute`** (Vulkan)
    *   *Reasoning:* Highly parallel reduction and sorting operations. Large batches of historical data can be uploaded to a storage buffer and processed in one dispatch.
*   **Alternative:** `neon_cpu` for real-time, per-packet sliding windows where batch size is small (< 1000 samples).

### 2. Throughput Pattern Analysis (Jitter/Loss Heatmaps)
*   **Task:** Generating a heatmap of packet arrival times vs. sequence numbers to visualize jitter.
*   **Best Backend:** **`gles3_computeish`** (OpenGL ES 3)
    *   *Reasoning:* This is fundamentally an "image generation" task (scattering points onto a 2D grid). Render passes are perfectly suited for this coordinate-space mapping.
*   **Alternative:** `vk_compute` if the heatmap is used for further compute analysis rather than just display.

### 3. Payload Pattern Matching / Filtering (DPI-lite)
*   **Task:** Scanning packet payloads for specific byte signatures or anomalies.
*   **Best Backend:** **`neon_cpu`** (NEON)
    *   *Reasoning:* Branch-heavy, pointer-chasing logic often stalls GPUs. NEON's vector instructions (e.g., `vld1`, `vcgt`, `vand`) are excellent for SIMD scanning over linear buffers without the overhead of moving data across the PCIe bus to the GPU.

### 4. Encryption/Hashing (if applicable)
*   **Task:** Verifying checksums or cryptographic signatures on updates/bundles.
*   **Best Backend:** **`neon_cpu`** (NEON)
    *   *Reasoning:* The Cortex-A76 has specific cryptographic extensions (AES/SHA) separate from NEON, but NEON is still vastly superior to scalar code for general hashing.

## Implementation Guidelines

### The Acceleration Manager (`crate::accel`)
A central `Accelerator` struct manages the lifecycle of these backends.
*   **Initialization:** Detects available runtimes (Vulkan, GLES, NEON).
*   **Selection:** heuristic-based dispatch.
    *   *Small payload?* -> NEON.
    *   *Large batch?* -> Vulkan.
    *   *Visual output?* -> GLES3.
*   **Verification:** periodically runs the *Reference* implementation alongside the accelerated path to ensure correctness (Debug builds: 100% check; Release builds: 0.1% sampling).

### Safety
*   **Vulkan/GLES:** All GPU interactions must be safe (no segfaults from malformed shaders). Synchronization primitives (fences/semaphores) must be used correctly.
*   **NEON:** Use `std::arch::aarch64` intrinsics wrapped in `unsafe` blocks, with strict bounds checking before entry.
*   **Fallback:** If a GPU hang is detected (watchdog), the Manager must transparently downgrade to NEON or Scalar instantly.

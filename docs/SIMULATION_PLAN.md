# PacketParamedic Simulation & Monitoring Plan

## Objective
Verify that the PacketParamedic backend fully utilizes the Raspberry Pi 5 hardware (Cortex-A76 cores, NEON acceleration, VideoCore VII compute) without thermal throttling or service instability during peak diagnostic loads.

## Monitoring Tools (Pi 5)
Run these commands in separate SSH terminals or tmux panes during simulation:

1.  **System Load & Resources**:
    ```bash
    htop
    ```
    *Expectation:* CPU specific cores (2,3) hit 100% during throughput/analysis tasks; Cores (0,1) remain responsive for API.

2.  **Hardware Health (Thermal/Voltage)**:
    ```bash
    watch -n 1 "vcgencmd measure_temp && vcgencmd get_throttled && vcgencmd measure_clock arm"
    ```
    *Expectation:* Temp < 80°C (with active cooling). Throttled hex should stay `0x0`. Clock should stay at max (2.4GHz).

3.  **Service Logs**:
    ```bash
    journalctl -u packetparamedic -f
    ```
    *Expectation:* "Bandwidth permit acquired", "MTR trace complete", "Incident recorded". No "Panic" or "OOM".

## Simulation Scenarios

### 1. Reliability Soak (The "Quiet Guardian" Persona)
*   **Action**: Schedule 24h worth of hourly checks.
*   **Run**: `cargo test --release --test persona_simulation test_persona_reliability_soak`
*   **Monitor**: Verify scheduler DB persistence and dry-run accuracy.

### 2. High Performance Burst (The "Gamer/Pro" Persona)
*   **Action**: Execute `ookla-cli` speed test (if installed) or simulated heavy load.
*   **Run**: Trigger `src/throughput/provider/ookla.rs` via API (future endpoint) or test suite.
*   **Monitor**: Watch CPU core pinning. The logic binds to cores 2-3.

### 3. Simple Troubleshooting (The "Is It Just Me?" Persona)
*   **Action**: Rapid-fire `mtr` traces to `8.8.8.8` whilst loading the CPU.
*   **Run**: `cargo test --release --test trace_tests`
*   **Monitor**: Check for latency spikes in `mtr` results caused by local CPU load (should be minimal due to core isolation).

## Test Execution
1.  Deploy latest binary: `cargo build --release`
2.  Restart service: `sudo systemctl restart packetparamedic`
3.  Run simulations: `cargo test --release --tests`
4.  Gather metrics.

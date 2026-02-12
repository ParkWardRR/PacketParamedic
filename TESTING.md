<div align="center">

  <img src="https://img.shields.io/badge/%F0%9F%A7%AA-Testing-green?style=for-the-badge&labelColor=0d1117" alt="Testing" height="60"/>

  # Testing Strategy & Roadmap
  
  **From Unit Tests to Agentic Chaos Engineering.**
  <br/>
  *How we ensure PacketParamedic is truly "Appliance-Grade".*

</div>

---

## Current Testing Architecture

We employ a multi-layered testing strategy designed to catch issues before they reach your network.

### 1. Unit & Logic Tests (Core)
*   **What:** Validates the internal logic of individual modules (scheduler calculations, blame analysis math, cron parsing).
*   **Tooling:** Standard Rust `cargo test`.
*   **Coverage:** 
    *   `src/scheduler`: Cron expression parsing and next-run calculations.
    *   `src/analysis`: Blame classification algorithms (logistic regression weights).
    *   `src/detect`: Anomaly detection thresholds.

### 2. CLI Smoke Tests (Integration)
*   **What:** Ensures the binary compiles, runs, and exposes the expected command-line interface.
*   **Tooling:** `assert_cmd` and `predicates` in `tests/smoke.rs`.
*   **Coverage:**
    *   Verifies `--help` and `--version` flags.
    *   Confirms all subcommands (`self-test`, `speed-test`, `schedule`) are registered and accept arguments.

### 3. Agentic Deployment Validation (E2E)
*   **What:** A Python-based integration harness (`tests/validate_deployment.py`) that runs against a live, deployed appliance.
*   **Execution:** Automated via SSH to the target Raspberry Pi.
*   **Capabilities:**
    *   **API Health:** Hits `GET /api/v1/health` to verify the HTTP server is up.
    *   **Hardware Check:** Queries `/self-test/latest` to ensure physical sensors (GPU, Thermal, NIC) are readable.
    *   **CRUD Operations:** Creates, Lists, and Deletes a test schedule to verify database persistence and scheduler logic.
    *   **CLI Verification:** Executes the remote binary via SSH to confirm version match.

### 4. Hardware Self-Test (Built-in)
*   **What:** The detailed `packetparamedic self-test` command.
*   **Scope:** 
    *   **GPU:** Checks if the VideoCore VII driver is loaded.
    *   **Power:** Detects under-voltage or thermal throttling events.
    *   **Storage:** Warns if running on slow microSD vs NVMe.
    *   **Network:** Validates link speed (1GbE vs 10GbE) and Wi-Fi capability.

---

## P5: Production Public Prod Pro Release To The Max (Roadmap)

Our goal is fully automated, agent-driven QA that simulates real-world network chaos. We are moving from "CI runs tests" to "Agents run the network."

### Phase 1: Zero-Touch CI/CD Pipeline 🚧
*   **Objective:** No human merges code without passing the gauntlet.
*   **Actions:**
    *   **Strict Linting:** `cargo clippy -- -D warnings` on every push.
    *   **Format Check:** `cargo fmt --check` enforcement.
    *   **Cross-Compilation Verification:** Automated builds for `aarch64-unknown-linux-gnu` (Pi 5) and `x86_64` (Dev) on every commit.
    *   **Dependency Audit:** `cargo audit` runs daily to catch vulnerable crates.

### Phase 2: Agentic E2E Verification 🤖
*   **Objective:** Coding agents continuously validate the user experience.
*   **Concept:** A "QA Agent" is triggered on every deployment.
*   **Workflow:**
    1.  Agent SSHs into a dedicated test Pi.
    2.  Deploys the new binary.
    3.  **Acts as the User:** 
        *   Configures a nightly speed test via the CLI.
        *   Waits for the cron job to fire (simulated via `dry-run`).
        *   Verifies the result appears in the database API.
    4.  **Reports:** Posts a summary to the pull request.

### Phase 3: Chaos & Resilience Engineering 💥
*   **Objective:** Break the network to prove we can debug it.
*   **Tools:** `tc-netem` (Linux Traffic Control), `toxiproxy`.
*   **Scenarios:**
    *   **"The Bad ISP":** Agent injects 5% packet loss on the WAN interface. PacketParamedic must correctly blame "ISP".
    *   **"The Flaky Wi-Fi":** Agent adds 200ms jitter to the LAN interface. PacketParamedic must blame "Local Network".
    *   **"The Power Outage":** Hard reboot the Pi during a database write. Verify SQLite WAL integrity upon recovery.

### Phase 4: Hardware-in-the-Loop (HITL) Fleet 🏎️
*   **Objective:** Performance testing on real silicon.
*   **Setup:** A rack of Raspberry Pi 5s with 10GbE HATs connected to a traffic generator.
*   **Tests:**
    *   **Thermal Soak:** Run `iperf3` at 9.4 Gbps for 24 hours. Ensure CPU stays < 80°C and no throttling occurs.
    *   **Memory Leak Detection:** continuous API hammering for 7 days.
    *   **Fan Control:** Verify active cooling ramps up correctly under load.

### Phase 5: Security Fuzzing & Hardening 🛡️
*   **Objective:** Unbreakable appliance status.
*   **Actions:**
    *   **API Fuzzing:** Run `cargo fuzz` against Axum endpoints to find panic-inducing inputs.
    *   **Protocol Fuzzing:** Send malformed ICMP/DNS packets to the probe engine.
    *   **Privilege Separation:** Verify `systemd` constraints (capabilities, read-only paths) prevent lateral movement if compromised.

---

## How to Run Tests Locally

### Unit & Smoke Tests
```bash
# Run standard test suite
cargo test

# Run with output captured
cargo test -- --nocapture
```

### Integration Test (Requires Running Appliance)
```bash
# Set target host
export PP_HOST="packetparamedic.local"

# Run the python validation harness
python3 tests/validate_deployment.py
```

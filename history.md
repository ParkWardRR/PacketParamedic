# Project History

## 2026-02-09
- Initialized development session.
- Verified repository structure and initial scaffolding.
- Created `SECURITY.md` outlining project security posture.
- Updated `roadmap.md` to mark Phase 0 items as complete:
    - Repo scaffolding created.
    - Versioning scheme defined.
    - Security posture documented.
    - Pi 5 hardware requirements documented.
- **Phase 1 Progress:**
    - Designed systemd unit layout (checked existing files).
    - Set up reproducible build pipeline via `tools/Dockerfile.build` and `tools/build-container.sh` (produces `.deb`).
- **Phase 3 Planning:**
    - Documented "Overuse" architecture in `docs/ACCELERATION_STRATEGY.md`.
    - Updated `roadmap.md` with detailed tasks for Vulkan 1.2, OpenGL ES 3, and NEON backends.
    - **Update:** Phase 3 marked as **MANDATORY**.
- **Phase 1 Progress:**
    - Verified `src/storage/mod.rs` (WAL mode enabled).
    - Updated `src/storage/schema.rs` with `measurements`, `spool`, and `acceleration_logs` support (columns `backend`, `duration_us`).
    - Implemented `src/storage/spool.rs` for crash-safe metric buffering.
    - Added `src/system/ntp.rs` (timedatectl wrapper) and `src/system/disk.rs` (simple df wrapper).
    - Added `systemd/journald.conf.d/packetparamedic-retention.conf` for 1GB/7d log retention.
    - Verified build passes `cargo check`.
    - **Note:** Phase 1.1 and 1.2 logic complete. Acceptance testing (soak) pending physical hardware.

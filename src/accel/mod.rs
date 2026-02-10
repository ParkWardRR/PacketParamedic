//! Acceleration manager -- NEON SIMD / VideoCore VII GPU / CPU fallback.
//!
//! Pi 5 guarantees Cortex-A76 NEON (ASIMD). GPU acceleration via VideoCore VII
//! (Vulkan 1.2 / OpenGL ES 3.1) is used where beneficial.

/// Which acceleration path was used for a given computation.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum AccelPath {
    /// Cortex-A76 NEON SIMD (always available on Pi 5)
    Neon,
    /// VideoCore VII GPU compute
    Gpu,
    /// Scalar CPU reference implementation
    CpuReference,
}

/// Metadata recording which acceleration path was used.
#[derive(Debug, serde::Serialize)]
pub struct AccelMetadata {
    pub path_used: AccelPath,
    pub duration_us: u64,
}

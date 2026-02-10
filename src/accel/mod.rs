//! Acceleration manager -- NEON SIMD / VideoCore VII GPU / CPU fallback.
//!
//! Pi 5 guarantees Cortex-A76 NEON (ASIMD). GPU acceleration via VideoCore VII
//! (Vulkan 1.2 / OpenGL ES 3.1) is used where beneficial.
//!
//! # Architecture: "Overuse"
//!
//! Every accelerated operation has implementations for:
//! - `vk_compute` (Vulkan 1.2)
//! - `gles_computeish` (OpenGL ES 3.1)
//! - `neon_cpu` (ARM NEON)
//! - `scalar_cpu` (Reference)

pub mod manager;
pub mod vulkan;
pub mod gles;
pub mod neon;
pub mod cpu;

pub mod ops;

// Re-export key types
pub use manager::{AccelerationManager, Backend, AcceleratedOp};

/// Metadata recording which acceleration path was used.
#[derive(Debug, serde::Serialize)]
pub struct AccelMetadata {
    pub backend: Backend,
    pub duration_us: u64,
}

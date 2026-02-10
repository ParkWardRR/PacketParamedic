use anyhow::Result;
use tracing::{debug, info, warn};

/// Backend types for acceleration
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Backend {
    /// Vulkan 1.2 Compute Shader (VideoCore VII)
    Vulkan,
    /// OpenGL ES 3.1 Render Pass using Fragment Shaders (VideoCore VII)
    Gles,
    /// ARM NEON SIMD (Cortex-A76) - Latency optimized
    Neon,
    /// Scalar CPU Reference - Verification only
    Scalar,
}

/// Trait that all accelerated operations must implement.
/// This enforces the "Overuse" architecture: every op must have 3+1 implementations.
pub trait AcceleratedOp<Input, Output> {
    /// Vulkan implementation (Large batch, Compute Shader)
    fn run_vulkan(&self, input: &Input, manager: &AccelerationManager) -> Result<Output>;

    /// GLES implementation (Render Pass)
    fn run_gles(&self, input: &Input, manager: &AccelerationManager) -> Result<Output>;

    /// NEON implementation (SIMD)
    fn run_neon(&self, input: &Input) -> Result<Output>;

    /// Scalar implementation (Reference)
    fn run_scalar(&self, input: &Input) -> Result<Output>;
}

/// Manager to handle backend selection and dispatch
pub struct AccelerationManager {
    vulkan_available: bool,
    gles_available: bool,
    #[allow(dead_code)]
    neon_available: bool,
}

impl AccelerationManager {
    pub fn new() -> Self {
        // Attempt runtime detection
        let vulkan_available = unsafe { crate::accel::vulkan::VulkanBackend::new().is_ok() };
        let gles_available = crate::accel::gles::GlesBackend::new().is_ok();
        let neon_available = true;    // Always true on Pi 5 (Cortex-A76)

        info!(
            "AccelerationManager initialized. Vulkan: {}, GLES: {}, NEON: {}",
            vulkan_available, gles_available, neon_available
        );

        Self {
            vulkan_available,
            gles_available,
            neon_available,
        }
    }

    /// Select the best backend for a given operation and payload size
    pub fn select_backend(&self, payload_size_bytes: usize) -> Backend {
        // Simple heuristic:
        // Tiny payloads (< 4KB) -> NEON (transfer overhead dominates)
        // Medium/Large payloads -> Vulkan if available, else GLES if available, else NEON
        const THRESHOLD_NEON_MAX: usize = 4096;

        if payload_size_bytes < THRESHOLD_NEON_MAX {
            return Backend::Neon;
        }

        if self.vulkan_available {
            return Backend::Vulkan;
        }

        if self.gles_available {
            return Backend::Gles;
        }

        Backend::Neon
    }

    /// Execute an operation using the best available backend.
    /// Includes optional verification against scalar reference (debug mode).
    pub fn execute<Op, Input, Output>(
        &self,
        op: &Op,
        input: &Input,
        payload_size: usize,
    ) -> Result<Output>
    where
        Op: AcceleratedOp<Input, Output>,
        Output: PartialEq + std::fmt::Debug,
    {
        let backend = self.select_backend(payload_size);
        let result = match backend {
            Backend::Vulkan => op.run_vulkan(input, self),
            Backend::Gles => op.run_gles(input, self),
            Backend::Neon => op.run_neon(input),
            Backend::Scalar => op.run_scalar(input),
        }?;

        // In debug builds, verify against scalar reference
        #[cfg(debug_assertions)]
        {
            if backend != Backend::Scalar {
                if let Ok(reference) = op.run_scalar(input) {
                    if result != reference {
                        warn!(
                            "Acceleration mismatch! Backend {:?} produced different result than Scalar.",
                            backend
                        );
                        // In strict mode, we might panic here. For now, just warn.
                    } else {
                        debug!("Acceleration verification passed for {:?}", backend);
                    }
                }
            }
        }

        Ok(result)
    }
}

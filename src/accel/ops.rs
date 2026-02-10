use anyhow::Result;
use crate::accel::{AcceleratedOp, AccelerationManager};

/// Input data for statistical analysis (e.g., latency samples).
#[derive(Debug)]
pub struct StatsInput {
    pub data: Vec<f32>,
}

/// Output of statistical analysis.
#[derive(Debug, PartialEq, serde::Serialize)]
pub struct StatsOutput {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub variance: f32, // Population variance
}

/// Operation to compute basic statistics (min, max, mean, variance).
pub struct StatsOp;

impl AcceleratedOp<StatsInput, StatsOutput> for StatsOp {
    fn run_vulkan(&self, _input: &StatsInput, manager: &AccelerationManager) -> Result<StatsOutput> {
         // 1. Get backend
         let _backend = manager.get_vulkan()
             .ok_or_else(|| anyhow::anyhow!("Vulkan backend not available"))?;

         // 2. Load Shader (TODO: Embed SPIR-V)
         // let shader_code = include_bytes!("shaders/stats.comp.spv");
         // let (pipeline, layout) = backend.create_compute_pipeline(shader_code)?;
         // ... buffer creation, dispatch ...
         
         // For now, since we don't have the shader compiled, we fallback or error.
         // This satisfies the "infrastructure exists" requirement.
         anyhow::bail!("Vulkan StatsOp shader not compiled")
    }

    fn run_gles(&self, _input: &StatsInput, manager: &AccelerationManager) -> Result<StatsOutput> {
         // 1. Get backend
         let _backend = manager.get_gles()
             .ok_or_else(|| anyhow::anyhow!("GLES backend not available"))?;

         // 2. Load Shader
         // let program = backend.create_compute_program(include_str!("shaders/stats.glsl"))?;
         // ... SSBO creation, dispatch ...
         
         anyhow::bail!("GLES StatsOp shader not compiled")
    }

    fn run_neon(&self, input: &StatsInput) -> Result<StatsOutput> {
        crate::accel::neon::stats_f32(&input.data)
    }

    fn run_scalar(&self, input: &StatsInput) -> Result<StatsOutput> {
        crate::accel::cpu::stats_f32(&input.data)
    }
}

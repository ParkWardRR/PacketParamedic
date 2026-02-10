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
    fn run_vulkan(&self, _input: &StatsInput, _manager: &AccelerationManager) -> Result<StatsOutput> {
         // Placeholder: In real life, upload buffer to GPU, run compute shader reduction
        anyhow::bail!("Vulkan backend not implemented for StatsOp")
    }

    fn run_gles(&self, _input: &StatsInput, _manager: &AccelerationManager) -> Result<StatsOutput> {
         // Placeholder: Render pass reduction
        anyhow::bail!("GLES backend not implemented for StatsOp")
    }

    fn run_neon(&self, input: &StatsInput) -> Result<StatsOutput> {
        crate::accel::neon::stats_f32(&input.data)
    }

    fn run_scalar(&self, input: &StatsInput) -> Result<StatsOutput> {
        crate::accel::cpu::stats_f32(&input.data)
    }
}

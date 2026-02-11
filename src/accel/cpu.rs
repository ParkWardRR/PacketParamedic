use crate::accel::ops::StatsOutput;
use anyhow::Result;

/// Scalar CPU reference implementation for F32 statistics.
/// Calculates Min, Max, Mean, Variance.
pub fn stats_f32(data: &[f32]) -> Result<StatsOutput> {
    if data.is_empty() {
        return Ok(StatsOutput {
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            variance: 0.0,
        });
    }

    let mut min = f32::MAX;
    let mut max = f32::MIN;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;

    for &val in data {
        if val < min {
            min = val;
        }
        if val > max {
            max = val;
        }
        sum += val;
        sum_sq += val * val;
    }

    let n = data.len() as f32;
    let mean = sum / n;
    let variance = (sum_sq / n) - (mean * mean); // Population variance E[x^2] - (E[x])^2

    Ok(StatsOutput {
        min,
        max,
        mean,
        variance: variance.max(0.0), // Avoid negative float precision errors
    })
}

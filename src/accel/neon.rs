use crate::accel::ops::StatsOutput;
use anyhow::Result;
use std::arch::aarch64::*;

/// NEON-optimized statistics calculation for F32 buffer.
/// Computes Min, Max, Sum, SumSq in parallel using 128-bit vector registers.
pub fn stats_f32(data: &[f32]) -> Result<StatsOutput> {
    if data.is_empty() {
        return Ok(StatsOutput {
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            variance: 0.0,
        });
    }

    let len = data.len();
    let mut i = 0;

    // Initialize accumulators
    // Using unsafe because these are intrinsic types
    unsafe {
        // Init vector registers with identity values for operations
        let mut v_min = vdupq_n_f32(f32::MAX);
        let mut v_max = vdupq_n_f32(f32::MIN);
        let mut v_sum = vdupq_n_f32(0.0);
        let mut v_sum_sq = vdupq_n_f32(0.0);

        // Process 4 floats (128 bits) at a time
        while i + 4 <= len {
            let ptr = data.as_ptr().add(i);
            let val = vld1q_f32(ptr); // Load 4 floats

            // Min/Max
            v_min = vminq_f32(v_min, val);
            v_max = vmaxq_f32(v_max, val);

            // Sum: accumulate directly
            v_sum = vaddq_f32(v_sum, val);

            // SumSq: Multiply-Accumulate (FMLA)
            // v_sum_sq += val * val
            v_sum_sq = vfmaq_f32(v_sum_sq, val, val);

            i += 4;
        }

        // Horizontal reduction across lanes
        // Reduce min/max/sum vectors to scalars
        let min = vminvq_f32(v_min);
        let max = vmaxvq_f32(v_max);
        let sum = vaddvq_f32(v_sum);
        let sum_sq = vaddvq_f32(v_sum_sq);

        // Handle remainder (scalar loop)
        let mut rem_min = min;
        let mut rem_max = max;
        let mut rem_sum = sum;
        let mut rem_sum_sq = sum_sq;

        while i < len {
            let val = *data.get_unchecked(i);
            if val < rem_min {
                rem_min = val;
            }
            if val > rem_max {
                rem_max = val;
            }
            rem_sum += val;
            rem_sum_sq += val * val;
            i += 1;
        }

        let n = len as f32;
        let mean = rem_sum / n;
        let variance = (rem_sum_sq / n) - (mean * mean);

        Ok(StatsOutput {
            min: rem_min,
            max: rem_max,
            mean,
            variance: variance.max(0.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accel::cpu;

    #[test]
    fn test_neon_parity_small() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let neon_res = stats_f32(&input).unwrap();
        let scalar_res = cpu::stats_f32(&input).unwrap();

        // Assert equality (strict for simple ints, but floats might drift)
        assert_eq!(neon_res, scalar_res);
    }

    #[test]
    fn test_neon_parity_large() {
        // 1024 elements (multiple of 4)
        let input: Vec<f32> = (0..1024).map(|i| i as f32 * 0.1).collect();
        let neon_res = stats_f32(&input).unwrap();
        let scalar_res = cpu::stats_f32(&input).unwrap();

        // Check closeness
        let epsilon = 1e-4;
        assert!(
            (neon_res.mean - scalar_res.mean).abs() < epsilon,
            "Mean mismatch: {} vs {}",
            neon_res.mean,
            scalar_res.mean
        );
        assert!(
            (neon_res.variance - scalar_res.variance).abs() < epsilon,
            "Var mismatch: {} vs {}",
            neon_res.variance,
            scalar_res.variance
        );
    }

    #[test]
    fn test_neon_parity_unaligned() {
        // 1001 elements (remainder case)
        let input: Vec<f32> = (0..1001).map(|i| (i % 100) as f32).collect();
        let neon_res = stats_f32(&input).unwrap();
        let scalar_res = cpu::stats_f32(&input).unwrap();

        let epsilon = 1e-5;
        assert!((neon_res.mean - scalar_res.mean).abs() < epsilon);
    }
}

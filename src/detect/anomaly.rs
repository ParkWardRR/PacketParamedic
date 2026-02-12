use crate::detect::DetectError;
use anyhow::Result;

/// A simple time series for statistical analysis.
pub struct TimeSeries {
    values: Vec<f64>,
}

impl TimeSeries {
    pub fn new(values: Vec<f64>) -> Self {
        Self { values }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    pub fn variance(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        let mean = self.mean();
        let sum_sq_diff: f64 = self
            .values
            .iter()
            .map(|&x| (x - mean).powi(2))
            .sum();
        sum_sq_diff / self.values.len() as f64
    }

    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Calculate the Z-score of a value relative to this series.
    /// Z = (value - mean) / std_dev
    pub fn z_score(&self, value: f64) -> Result<f64, DetectError> {
        if self.values.len() < 3 {
            return Err(DetectError::InsufficientBaseline {
                needed: 3,
                have: self.values.len(),
            });
        }
        let std = self.std_dev();
        if std == 0.0 {
            // If checking for deviation from a constant baseline, any difference is infinite Z
            if (value - self.mean()).abs() > f64::EPSILON {
                return Ok(f64::INFINITY); // Or a large number
            }
            return Ok(0.0);
        }
        Ok((value - self.mean()) / std)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats() {
        let ts = TimeSeries::new(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(ts.mean(), 3.0);
        // Variance of 1..5 is 2.0
        // StdDev is sqrt(2) ~ 1.414
        let z = ts.z_score(10.0).unwrap();
        // (10 - 3) / 1.414 = 7 / 1.414 ~ 4.95
        assert!(z > 4.9);
    }
}

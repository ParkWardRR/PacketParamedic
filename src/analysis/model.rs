use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::fs;
use tracing::{info, warn};

// Embed default model for fallback
const DEFAULT_MODEL_JSON: &str = include_str!("blame_lr.json");

/// Blame Classifier Model (Logistic Regression)
#[derive(Debug, Deserialize)]
pub struct LogisticModel {
    pub feature_names: Vec<String>,
    pub class_names: Vec<String>,
    pub weights: Vec<Vec<f64>>, // [n_classes][n_features]
    pub bias: Vec<f64>,         // [n_classes]
    pub means: Vec<f64>,        // For standardization
    pub stds: Vec<f64>,         // For standardization
}

/// Feature vector for Blame Prediction
#[derive(Debug, Default, Clone, Serialize)]
pub struct BlameFeatures {
    pub gw_rtt_p50_ms: f64,
    pub gw_rtt_p95_ms: f64,
    pub gw_loss_pct: f64,
    
    pub wan_rtt_p50_ms: f64,
    pub wan_rtt_p95_ms: f64,
    pub wan_loss_pct: f64,
    
    pub delta_rtt_p50_ms: f64,
    
    pub dns_ms_p50: f64,
    pub dns_fail_rate: f64,
    
    pub http_fail_rate: f64,
    pub tcp_fail_rate: f64,
    
    pub wan_down_mbps: f64,
    pub wan_up_mbps: f64,
}

impl BlameFeatures {
    pub fn to_vector(&self) -> Vec<f64> {
        vec![
            self.gw_rtt_p50_ms, self.gw_rtt_p95_ms, self.gw_loss_pct,
            self.wan_rtt_p50_ms, self.wan_rtt_p95_ms, self.wan_loss_pct,
            self.delta_rtt_p50_ms,
            self.dns_ms_p50, self.dns_fail_rate,
            self.http_fail_rate, self.tcp_fail_rate,
            self.wan_down_mbps, self.wan_up_mbps,
        ]
    }
}

/// Result of a blame prediction
#[derive(Debug, Serialize)]
pub struct Prediction {
    pub verdict: String,
    pub confidence: f64,
    pub probabilities: std::collections::HashMap<String, f64>,
    pub is_preliminary: bool, // Always false for raw model, managed by state machine
}

impl LogisticModel {
    /// Load model from JSON file, falling back to embedded default if missing/invalid.
    pub fn load(path: &str) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(model) = serde_json::from_str(&content) {
                info!("Loaded blame model from {}", path);
                return model;
            } else {
                warn!("Failed to parse model at {}. Using embedded default.", path);
            }
        } else {
            warn!("Model file not found at {}. Using embedded default.", path);
        }
        
        serde_json::from_str(DEFAULT_MODEL_JSON).expect("Embedded default model is invalid JSON")
    }

    /// Predict class probabilities for a given feature set
    pub fn predict(&self, features: &BlameFeatures) -> Result<Prediction> {
        let raw = features.to_vector();
        if raw.len() != self.means.len() {
            anyhow::bail!("Feature vector length mismatch. Expected {}, got {}", self.means.len(), raw.len());
        }

        // 1. Standardize
        let mut norm = Vec::with_capacity(raw.len());
        for (i, val) in raw.iter().enumerate() {
            norm.push((val - self.means[i]) / self.stds[i]);
        }

        // 2. Compute Scores (Dot Product + Bias)
        let n_classes = self.class_names.len();
        let mut scores = Vec::with_capacity(n_classes);
        let mut max_score = f64::NEG_INFINITY;

        for k in 0..n_classes {
            let mut dot = 0.0;
            for (j, &w) in self.weights[k].iter().enumerate() {
                dot += w * norm[j];
            }
            let score = dot + self.bias[k];
            scores.push(score);
            if score > max_score {
                max_score = score;
            }
        }

        // 3. Softmax
        let mut probs = Vec::with_capacity(n_classes);
        let mut sum_exp = 0.0;
        for &s in &scores {
            let p = (s - max_score).exp();
            probs.push(p);
            sum_exp += p;
        }

        let mut prob_map = std::collections::HashMap::new();
        let mut best_class = "unknown".to_string();
        let mut best_prob = 0.0;

        for (k, &p) in probs.iter().enumerate() {
            let probability = p / sum_exp;
            let class_name = &self.class_names[k];
            prob_map.insert(class_name.clone(), probability);

            if probability > best_prob {
                best_prob = probability;
                best_class = class_name.clone();
            }
        }

        Ok(Prediction {
            verdict: best_class,
            confidence: best_prob,
            probabilities: prob_map,
            is_preliminary: false, // Default stateless result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_model_sanity() {
        let model = LogisticModel::load("non_existent_path.json");
        assert_eq!(model.class_names.len(), 3);
        assert_eq!(model.feature_names.len(), 13);
    }

    #[test]
    fn test_prediction_isp_failure() {
        let model = LogisticModel::load("non_existent_path.json"); // Uses embedded default

        // Synthetic ISP failure case: GW ok, WAN bad, Delta bad
        let features = BlameFeatures {
            gw_rtt_p50_ms: 2.0,
            gw_rtt_p95_ms: 3.0,
            gw_loss_pct: 0.0,
            
            wan_rtt_p50_ms: 100.0,
            wan_rtt_p95_ms: 150.0,
            wan_loss_pct: 5.0,
            
            delta_rtt_p50_ms: 98.0,
            
            dns_ms_p50: 50.0,
            dns_fail_rate: 0.05,
            
            http_fail_rate: 0.05,
            tcp_fail_rate: 0.0,
            
            wan_down_mbps: 100.0,
            wan_up_mbps: 10.0,
        };

        let prediction = model.predict(&features).unwrap();
        
        println!("Prediction: {:?}", prediction);
        
        assert_eq!(prediction.verdict, "isp");
        assert!(prediction.probabilities.get("isp").unwrap() > &0.9);
    }
}

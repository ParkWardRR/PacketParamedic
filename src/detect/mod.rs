//! Anomaly detection and incident grouping.

pub mod anomaly;
pub mod incident;
pub mod engine;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DetectError {
    #[error("insufficient baseline data: need {needed} samples, have {have}")]
    InsufficientBaseline { needed: usize, have: usize },
}

/// Severity levels for detected incidents.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// A detected incident with verdict and evidence.
#[derive(Debug, serde::Serialize)]
pub struct Incident {
    pub id: uuid::Uuid,
    pub severity: Severity,
    pub verdict: String,
    pub evidence: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

//! Execution history tracking for scheduled runs.

/// A record of a scheduled test execution.
#[derive(Debug, serde::Serialize)]
pub struct HistoryEntry {
    pub schedule_name: String,
    pub status: RunStatus,
    pub result_summary: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, serde::Serialize)]
pub enum RunStatus {
    Success,
    Failed,
    Aborted,
    Missed,
}

//! Priority queue with bandwidth-aware coordination.
//!
//! Priority order: blame-check > probes > speed tests > stress tests.
//! User-triggered tests preempt scheduled background tests.

/// Test priority levels (lower number = higher priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Priority {
    BlameCheck = 0,
    Probe = 1,
    SpeedTest = 2,
    StressTest = 3,
}

/// A queued test job.
#[derive(Debug)]
pub struct Job {
    pub id: uuid::Uuid,
    pub test_type: String,
    pub priority: Priority,
    pub user_triggered: bool,
}

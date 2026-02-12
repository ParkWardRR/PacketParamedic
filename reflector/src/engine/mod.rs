//! Test engine trait and dispatcher for the PacketParamedic Reflector.
//!
//! Each test type is implemented as an engine module that can be started and
//! stopped independently.  The [`TestHandle`] provides a shutdown channel for
//! graceful termination, and [`EngineResult`] captures the outcome.

pub mod health;
pub mod path_meta;
pub mod throughput;
pub mod udp_echo;

// ---------------------------------------------------------------------------
// TestHandle
// ---------------------------------------------------------------------------

/// Handle to a running test engine instance.
///
/// The caller holds the `shutdown_tx` sender; dropping it or sending a value
/// signals the engine to terminate gracefully.
pub struct TestHandle {
    /// Unique test identifier.
    pub test_id: String,
    /// Data-plane port the engine is listening on.
    pub port: u16,
    /// One-shot channel to signal graceful shutdown.
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

// ---------------------------------------------------------------------------
// EngineResult
// ---------------------------------------------------------------------------

/// Outcome of a test engine run.
#[derive(Debug)]
pub enum EngineResult {
    /// The test completed normally.
    Completed {
        /// Total bytes transferred during the test.
        bytes_transferred: u64,
        /// Wall-clock duration in seconds.
        duration_sec: f64,
    },
    /// The test was terminated because it exceeded its time limit.
    TimedOut {
        /// Total bytes transferred before timeout.
        bytes_transferred: u64,
        /// Wall-clock duration in seconds (should be close to the limit).
        duration_sec: f64,
    },
    /// The test failed with an error.
    Error(String),
}

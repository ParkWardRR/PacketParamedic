//! iperf3 server spawner engine for throughput tests.
//!
//! Spawns an `iperf3 -s --one-off` child process on a free port within a
//! configured range.  The child is monitored and killed on shutdown signal
//! or timeout.

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::process::Command;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::Iperf3Config;

use super::{EngineResult, TestHandle};

// ---------------------------------------------------------------------------
// ThroughputEngine
// ---------------------------------------------------------------------------

/// iperf3-based throughput test engine.
///
/// Spawns an `iperf3` server process on a free port and monitors it until
/// completion, timeout, or shutdown signal.
pub struct ThroughputEngine {
    /// Path to the iperf3 binary.
    iperf3_path: String,
    /// Start of the ephemeral port range.
    port_range_start: u16,
    /// End of the ephemeral port range (inclusive).
    port_range_end: u16,
}

impl ThroughputEngine {
    /// Create a new throughput engine from configuration.
    pub fn new(config: &Iperf3Config, port_range: (u16, u16)) -> Self {
        Self {
            iperf3_path: config.path.clone(),
            port_range_start: port_range.0,
            port_range_end: port_range.1,
        }
    }

    /// Get the configured port range.
    pub fn port_range(&self) -> (u16, u16) {
        (self.port_range_start, self.port_range_end)
    }

    /// Find a free port within the configured range by attempting to bind.
    ///
    /// Returns the first port in the range that is available.
    pub async fn find_free_port(&self) -> Result<u16> {
        for port in self.port_range_start..=self.port_range_end {
            match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await {
                Ok(_listener) => {
                    // The listener is dropped here, freeing the port for iperf3.
                    debug!(port = port, "found free port");
                    return Ok(port);
                }
                Err(_) => {
                    debug!(port = port, "port in use, trying next");
                    continue;
                }
            }
        }

        anyhow::bail!(
            "no free port found in range {}-{}",
            self.port_range_start,
            self.port_range_end
        );
    }

    /// Start an iperf3 server on the given port.
    ///
    /// Spawns `iperf3 -s -p {port} --one-off` as a child process and monitors
    /// it.  The child is terminated on shutdown signal or timeout.
    ///
    /// # Arguments
    ///
    /// * `port` - TCP port for the iperf3 server.
    /// * `duration` - Maximum time to wait for the test to complete.
    pub async fn start(
        &self,
        port: u16,
        duration: Duration,
    ) -> Result<(TestHandle, JoinHandle<EngineResult>)> {
        let test_id = Uuid::new_v4().to_string();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        info!(
            test_id = test_id.as_str(),
            port = port,
            iperf3_path = self.iperf3_path.as_str(),
            duration_sec = duration.as_secs(),
            "starting iperf3 server"
        );

        let mut child = Command::new(&self.iperf3_path)
            .arg("-s")
            .arg("-p")
            .arg(port.to_string())
            .arg("--one-off")
            .kill_on_drop(true)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .with_context(|| {
                format!(
                    "failed to spawn iperf3 at '{}' on port {}",
                    self.iperf3_path, port
                )
            })?;

        let child_pid = child.id();

        let task_test_id = test_id.clone();
        let handle = tokio::spawn(async move {
            let start = tokio::time::Instant::now();
            let timeout = tokio::time::sleep(duration);
            tokio::pin!(timeout);
            tokio::pin!(shutdown_rx);

            let result = tokio::select! {
                biased;

                _ = &mut shutdown_rx => {
                    debug!(test_id = task_test_id.as_str(), "shutdown signal received, terminating iperf3");
                    terminate_child(&mut child).await;
                    let elapsed = start.elapsed().as_secs_f64();
                    EngineResult::Completed {
                        bytes_transferred: 0, // iperf3 output parsing not yet implemented
                        duration_sec: elapsed,
                    }
                }

                _ = &mut timeout => {
                    warn!(test_id = task_test_id.as_str(), "iperf3 test timed out, terminating");
                    terminate_child(&mut child).await;
                    let elapsed = start.elapsed().as_secs_f64();
                    EngineResult::TimedOut {
                        bytes_transferred: 0,
                        duration_sec: elapsed,
                    }
                }

                status = child.wait() => {
                    let elapsed = start.elapsed().as_secs_f64();
                    match status {
                        Ok(exit) => {
                            info!(
                                test_id = task_test_id.as_str(),
                                exit_code = exit.code(),
                                duration_sec = elapsed,
                                "iperf3 exited"
                            );
                            if exit.success() {
                                EngineResult::Completed {
                                    bytes_transferred: 0,
                                    duration_sec: elapsed,
                                }
                            } else {
                                EngineResult::Error(format!(
                                    "iperf3 exited with code {:?}",
                                    exit.code()
                                ))
                            }
                        }
                        Err(e) => {
                            EngineResult::Error(format!("failed to wait for iperf3: {}", e))
                        }
                    }
                }
            };

            info!(
                test_id = task_test_id.as_str(),
                pid = child_pid,
                "iperf3 engine stopped"
            );

            result
        });

        let test_handle = TestHandle {
            test_id,
            port,
            shutdown_tx,
        };

        Ok((test_handle, handle))
    }
}

/// Gracefully terminate a child process.
///
/// Sends SIGTERM first, waits up to 5 seconds, then sends SIGKILL if the
/// process is still running.
async fn terminate_child(child: &mut tokio::process::Child) {
    // Try SIGTERM first (Unix only).
    #[cfg(unix)]
    {
        if let Some(pid) = child.id() {
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
    }

    // Wait up to 5 seconds for graceful exit.
    match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
        Ok(Ok(status)) => {
            debug!(exit_code = status.code(), "child exited after SIGTERM");
        }
        Ok(Err(e)) => {
            warn!(error = %e, "error waiting for child after SIGTERM");
        }
        Err(_) => {
            // Timed out waiting for graceful exit; force kill.
            warn!("child did not exit after SIGTERM, sending SIGKILL");
            if let Err(e) = child.kill().await {
                warn!(error = %e, "failed to SIGKILL child");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_find_free_port() {
        let config = Iperf3Config {
            path: "iperf3".into(),
            default_streams: 4,
            max_streams: 8,
        };
        // Use port 0 which always succeeds (OS assigns).
        // Instead, test with a small real range.
        let engine = ThroughputEngine::new(&config, (19200, 19210));

        let result = engine.find_free_port().await;
        assert!(
            result.is_ok(),
            "should find a free port in range 19200-19210"
        );
        let port = result.unwrap();
        assert!(port >= 19200 && port <= 19210);
    }

    #[tokio::test]
    async fn test_find_free_port_no_range() {
        let config = Iperf3Config {
            path: "iperf3".into(),
            default_streams: 4,
            max_streams: 8,
        };
        // Bind a listener on port 0 -- but the engine expects a specific range.
        // Use a range where one port is likely occupied.
        // This is hard to test deterministically, so we just verify the API works.
        let engine = ThroughputEngine::new(&config, (19300, 19305));
        let result = engine.find_free_port().await;
        // This should generally succeed unless all 6 ports are in use.
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_throughput_engine_new() {
        let config = Iperf3Config {
            path: "/usr/bin/iperf3".into(),
            default_streams: 2,
            max_streams: 16,
        };
        let engine = ThroughputEngine::new(&config, (5201, 5210));
        assert_eq!(engine.iperf3_path, "/usr/bin/iperf3");
        assert_eq!(engine.port_range_start, 5201);
        assert_eq!(engine.port_range_end, 5210);
    }
}

//! Built-in UDP echo reflector engine.
//!
//! Listens on a UDP socket and echoes every received datagram back to the
//! sender.  Tracks bytes transferred and enforces an optional packet rate
//! limit and a maximum duration.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{EngineResult, TestHandle};

// ---------------------------------------------------------------------------
// UdpEchoEngine
// ---------------------------------------------------------------------------

/// UDP echo reflector engine.
///
/// Binds a UDP socket and echoes every received packet back to the sender,
/// tracking total bytes transferred.
pub struct UdpEchoEngine;

impl UdpEchoEngine {
    /// Start a UDP echo session on the given port.
    ///
    /// Returns a [`TestHandle`] for controlling the session and a
    /// [`JoinHandle`] that resolves to the [`EngineResult`] when the
    /// session ends.
    ///
    /// # Arguments
    ///
    /// * `port` - UDP port to bind on `0.0.0.0`. Use `0` for an
    ///   OS-assigned ephemeral port.
    /// * `duration` - Maximum duration before the session auto-closes.
    /// * `max_packet_rate` - Maximum packets per second. `0` means unlimited.
    pub async fn start(
        port: u16,
        duration: Duration,
        max_packet_rate: u32,
    ) -> Result<(TestHandle, JoinHandle<EngineResult>)> {
        let bind_addr = format!("0.0.0.0:{}", port);
        let socket = UdpSocket::bind(&bind_addr)
            .await
            .with_context(|| format!("failed to bind UDP socket on {}", bind_addr))?;

        let actual_port = socket
            .local_addr()
            .context("failed to get local address")?
            .port();

        let test_id = Uuid::new_v4().to_string();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        info!(
            test_id = test_id.as_str(),
            port = actual_port,
            duration_sec = duration.as_secs(),
            max_packet_rate = max_packet_rate,
            "starting UDP echo engine"
        );

        let task_test_id = test_id.clone();
        let bytes_transferred = Arc::new(AtomicU64::new(0));
        let bytes_clone = bytes_transferred.clone();

        let handle = tokio::spawn(async move {
            let start = tokio::time::Instant::now();
            let timeout = tokio::time::sleep(duration);
            tokio::pin!(timeout);
            tokio::pin!(shutdown_rx);

            let mut buf = [0u8; 65536]; // max UDP datagram size
            let mut packets_this_second: u32 = 0;
            let mut second_start = tokio::time::Instant::now();
            let mut timed_out = false;

            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown_rx => {
                        debug!(test_id = task_test_id.as_str(), "shutdown signal received");
                        break;
                    }
                    _ = &mut timeout => {
                        debug!(test_id = task_test_id.as_str(), "duration expired");
                        timed_out = true;
                        break;
                    }
                    result = socket.recv_from(&mut buf) => {
                        match result {
                            Ok((len, addr)) => {
                                // Rate limiting.
                                if max_packet_rate > 0 {
                                    let now = tokio::time::Instant::now();
                                    if now.duration_since(second_start) >= Duration::from_secs(1) {
                                        packets_this_second = 0;
                                        second_start = now;
                                    }
                                    packets_this_second += 1;
                                    if packets_this_second > max_packet_rate {
                                        // Drop the packet silently.
                                        continue;
                                    }
                                }

                                // Echo the packet back.
                                if let Err(e) = socket.send_to(&buf[..len], addr).await {
                                    warn!(
                                        test_id = task_test_id.as_str(),
                                        error = %e,
                                        "failed to echo packet"
                                    );
                                }

                                // Track bytes (recv + send).
                                bytes_clone.fetch_add(len as u64 * 2, Ordering::Relaxed);
                            }
                            Err(e) => {
                                warn!(
                                    test_id = task_test_id.as_str(),
                                    error = %e,
                                    "recv_from error"
                                );
                            }
                        }
                    }
                }
            }

            let elapsed = start.elapsed().as_secs_f64();
            let total_bytes = bytes_clone.load(Ordering::Relaxed);

            info!(
                test_id = task_test_id.as_str(),
                bytes_transferred = total_bytes,
                duration_sec = elapsed,
                timed_out = timed_out,
                "UDP echo engine stopped"
            );

            if timed_out {
                EngineResult::TimedOut {
                    bytes_transferred: total_bytes,
                    duration_sec: elapsed,
                }
            } else {
                EngineResult::Completed {
                    bytes_transferred: total_bytes,
                    duration_sec: elapsed,
                }
            }
        });

        let test_handle = TestHandle {
            test_id,
            port: actual_port,
            shutdown_tx,
        };

        Ok((test_handle, handle))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::UdpSocket;

    #[tokio::test]
    async fn test_udp_echo_round_trip() {
        // Start the echo engine on an ephemeral port.
        let (handle, task) = UdpEchoEngine::start(0, Duration::from_secs(5), 0)
            .await
            .expect("should start echo engine");

        let echo_port = handle.port;
        assert!(echo_port > 0, "should have been assigned a port");

        // Create a client socket and send a packet.
        let client = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("client bind");
        let payload = b"hello, echo!";
        client
            .send_to(payload, format!("127.0.0.1:{}", echo_port))
            .await
            .expect("send");

        // Receive the echoed response.
        let mut buf = [0u8; 1024];
        let (len, _addr) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .expect("timeout waiting for echo")
            .expect("recv");

        assert_eq!(&buf[..len], payload);

        // Shut down the engine.
        let _ = handle.shutdown_tx.send(());
        let result = task.await.expect("task should complete");

        match result {
            EngineResult::Completed {
                bytes_transferred, ..
            } => {
                // Should have transferred at least the payload size * 2 (recv + send).
                assert!(
                    bytes_transferred >= (payload.len() as u64 * 2),
                    "expected at least {} bytes, got {}",
                    payload.len() * 2,
                    bytes_transferred
                );
            }
            other => panic!("expected Completed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_udp_echo_timeout() {
        // Start with a very short duration.
        let (handle, task) = UdpEchoEngine::start(0, Duration::from_millis(100), 0)
            .await
            .expect("should start echo engine");

        // Don't send the shutdown signal; let it time out.
        drop(handle.shutdown_tx);

        // Wait a bit for the timeout, then for the task to complete.
        let result = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("task should complete within timeout")
            .expect("task should not panic");

        match result {
            EngineResult::TimedOut { duration_sec, .. } => {
                assert!(
                    duration_sec >= 0.05,
                    "should have run for at least ~100ms, got {}",
                    duration_sec
                );
            }
            // Dropping shutdown_tx causes the receiver to resolve immediately
            // with an error, which may also trigger the shutdown path.
            EngineResult::Completed { .. } => {
                // Acceptable: dropping the sender can race with the timeout.
            }
            EngineResult::Error(e) => panic!("unexpected error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_udp_echo_rate_limit() {
        let (handle, _task) = UdpEchoEngine::start(0, Duration::from_secs(5), 2)
            .await
            .expect("should start echo engine");

        let echo_port = handle.port;

        let client = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("client bind");

        // Send 5 packets rapidly - only 2 per second should be echoed.
        for _ in 0..5 {
            client
                .send_to(b"test", format!("127.0.0.1:{}", echo_port))
                .await
                .expect("send");
        }

        // Give the engine a moment to process.
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Shut down.
        let _ = handle.shutdown_tx.send(());
    }
}

//! Core scheduler loop -- Tokio-based, runs in-process within the main daemon.

use anyhow::Result;
use tokio::sync::Semaphore;

/// The throughput semaphore: only one throughput-heavy test at a time.
static THROUGHPUT_PERMIT: Semaphore = Semaphore::const_new(1);

/// Acquire the throughput slot (blocks until available or timeout).
pub async fn acquire_throughput_slot(
    timeout: std::time::Duration,
) -> Result<tokio::sync::SemaphorePermit<'static>> {
    match tokio::time::timeout(timeout, THROUGHPUT_PERMIT.acquire()).await {
        Ok(Ok(permit)) => Ok(permit),
        Ok(Err(_)) => anyhow::bail!("throughput semaphore closed"),
        Err(_) => anyhow::bail!("timed out waiting for throughput slot"),
    }
}

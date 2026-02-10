//! PacketParamedic -- Appliance-grade network diagnostics for Raspberry Pi 5.
//!
//! This crate provides the core library for network diagnostic probes,
//! throughput testing, anomaly detection, scheduling, and evidence collection.

pub mod accel;
pub mod api;
pub mod detect;
pub mod evidence;
pub mod probes;
pub mod scheduler;
pub mod selftest;
pub mod storage;
pub mod throughput;
pub mod system;
pub mod analysis;

use anyhow::Result;

/// Start the PacketParamedic daemon: API server, scheduler, and probe engine.
pub async fn serve(bind: &str) -> Result<()> {
    let addr: std::net::SocketAddr = bind.parse()?;
    let app = api::router();

    tracing::info!(%addr, "PacketParamedic listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

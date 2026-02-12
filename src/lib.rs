//! PacketParamedic -- Appliance-grade network diagnostics for Raspberry Pi 5.
//!
//! This crate provides the core library for network diagnostic probes,
//! throughput testing, anomaly detection, scheduling, and evidence collection.

pub mod accel;
pub mod analysis;
pub mod api;
pub mod detect;
pub mod evidence;
pub mod probes;
pub mod scheduler;
pub mod selftest;
pub mod storage;
pub mod system;
pub mod throughput;

use anyhow::Result;

/// Start the PacketParamedic daemon: API server, scheduler, and probe engine.
pub async fn serve(bind: &str, db_path: &str) -> Result<()> {
    // 1. Initialize Storage
    tracing::info!(%db_path, "Initializing database");
    let pool = storage::open_pool(db_path)?;

    // 2. Initialize Scheduler
    let scheduler = scheduler::Scheduler::new(pool.clone());
    scheduler.ensure_defaults().await?;

    // 3. Start Scheduler Engine (background task)
    let scheduler_engine = scheduler.clone();
    tokio::spawn(async move {
        scheduler::run_scheduler_loop(scheduler_engine).await;
    });

    // 4. Start API Server
    let addr: std::net::SocketAddr = bind.parse()?;

    let app_state = api::state::AppState {
        pool: pool.clone(),
        scheduler: scheduler.clone(),
    };

    let app = api::router(app_state);

    tracing::info!(%addr, "PacketParamedic listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

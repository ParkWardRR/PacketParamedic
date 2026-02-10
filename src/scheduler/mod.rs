
pub mod cron;
// pub mod engine;

// Re-export common types
pub use self::cron::Scheduler;

// TODO: Refactor legacy stubs to use the real Scheduler instance

/// List all configuredschedules (Legacy stub refactored)
pub async fn list_schedules() -> anyhow::Result<()> {
    tracing::info!("Use the API or Scheduler struct for real operations.");
    Ok(())
}

/// Add a new schedule (Legacy stub refactored)
pub async fn add_schedule(_name: &str, _cron_expr: &str, _test_type: &str) -> anyhow::Result<()> {
    tracing::info!("Use the API or Scheduler struct for real operations.");
    Ok(())
}

/// Remove a schedule by name (Legacy stub refactored)
pub async fn remove_schedule(_name: &str) -> anyhow::Result<()> {
    tracing::info!("Use the API or Scheduler struct for real operations.");
    Ok(())
}

/// Preview what will run (Legacy stub refactored)
pub async fn dry_run(_hours: u64) -> anyhow::Result<()> {
    tracing::info!("Use the API or Scheduler struct for real operations.");
    Ok(())
}

//! Scheduling engine -- cron-like recurring schedules with bandwidth-aware coordination.

pub mod cron;
pub mod engine;
pub mod history;
pub mod profiles;
pub mod queue;

use anyhow::Result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("invalid cron expression: {expr}")]
    InvalidCron { expr: String },

    #[error("schedule '{name}' already exists")]
    DuplicateSchedule { name: String },

    #[error("resource conflict: {resource} is in use by '{holder}'")]
    ResourceConflict { resource: String, holder: String },
}

/// A scheduled test definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Schedule {
    pub name: String,
    pub cron_expr: String,
    pub test_type: String,
    pub enabled: bool,
}

/// List all configured schedules.
pub async fn list_schedules() -> Result<()> {
    // TODO: Read from SQLite
    tracing::info!("Listing schedules (stub)");
    println!("No schedules configured yet.");
    Ok(())
}

/// Add a new schedule.
pub async fn add_schedule(name: &str, cron_expr: &str, test_type: &str) -> Result<()> {
    // TODO: Validate cron expression
    // TODO: Store in SQLite
    tracing::info!(%name, %cron_expr, %test_type, "Adding schedule (stub)");
    println!("Schedule '{}' added: {} -> {}", name, cron_expr, test_type);
    Ok(())
}

/// Remove a schedule by name.
pub async fn remove_schedule(name: &str) -> Result<()> {
    // TODO: Delete from SQLite
    tracing::info!(%name, "Removing schedule (stub)");
    println!("Schedule '{}' removed.", name);
    Ok(())
}

/// Preview what will run in the next N hours.
pub async fn dry_run(hours: u64) -> Result<()> {
    // TODO: Compute next-run times for all enabled schedules
    tracing::info!(%hours, "Dry run preview (stub)");
    println!("Dry run for next {} hours: (no schedules configured)", hours);
    Ok(())
}

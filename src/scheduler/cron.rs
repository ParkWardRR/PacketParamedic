use crate::storage::Pool;
use anyhow::{Context, Result};
use chrono::Utc;
use cron::Schedule as CronSchedule;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// A scheduler that persists tasks in SQLite and checks for runnable tasks.
#[derive(Clone)]
pub struct Scheduler {
    pool: Pool,
    bandwidth_permit: Arc<Semaphore>,
}

impl Scheduler {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            bandwidth_permit: Arc::new(Semaphore::new(1)), // Only 1 bandwidth-heavy test at a time
        }
    }

    pub fn get_pool(&self) -> &Pool {
        &self.pool
    }

    pub fn get_bandwidth_permit(&self) -> Arc<Semaphore> {
        self.bandwidth_permit.clone()
    }

    /// Ensure default schedules exist (idempotent).
    pub async fn ensure_defaults(&self) -> Result<()> {
        let defaults = crate::scheduler::profiles::defaults();
        for sched in defaults {
            // Attempt to add. If it exists (name constraint), we ignore error.
            // Ideally we check specific error, but for now ignore any error on defaults.
            match self.add_schedule(&sched.name, &sched.cron_expr, &sched.test_type).await {
                Ok(_) => tracing::info!("Initialized default schedule: {}", sched.name),
                Err(e) => tracing::debug!("Default schedule '{}' skipped (exists or invalid): {}", sched.name, e),
            }
        }
        Ok(())
    }

    /// Add a new schedule to the database
    pub async fn add_schedule(&self, name: &str, cron_expr: &str, test_type: &str) -> Result<()> {
        // Normalize Cron: parse 5-field (standard) as 6-field (quartz with 0 seconds)
        let parts: Vec<&str> = cron_expr.split_whitespace().collect();
        let effective_cron = if parts.len() == 5 {
            format!("0 {}", cron_expr)
        } else {
            cron_expr.to_string()
        };

        // Validate Cron
        let _ = CronSchedule::from_str(&effective_cron)
            .map_err(|e| anyhow::anyhow!("Invalid cron expression '{}': {}", effective_cron, e))?;

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO schedules (name, cron_expr, test_type, enabled) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![name, effective_cron, test_type],
        )
        .context("Failed to insert schedule")?;

        Ok(())
    }

    /// Calculate next run times for all enabled schedules
    /// This is strictly a dry-run preview, not the execution loop.
    pub async fn preview_next_runs(&self, hours: u64) -> Result<Vec<(String, String, String)>> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT name, cron_expr, test_type FROM schedules WHERE enabled = 1")?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let now = Utc::now();
        let end = now + chrono::Duration::hours(hours as i64);
        let mut preview = Vec::new();

        for r in rows {
            let (name, cron_expr, test_type) = r?;
            if let Ok(schedule) = CronSchedule::from_str(&cron_expr) {
                for next_time in schedule.after(&now) {
                    if next_time > end {
                        break;
                    }
                    preview.push((next_time.to_rfc3339(), name.clone(), test_type.clone()));
                }
            }
        }

        // Sort by time
        preview.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(preview)
    }

    /// List all schedules
    pub async fn list(&self) -> Result<Vec<(String, String, String, bool)>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT name, cron_expr, test_type, enabled FROM schedules")?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? != 0,
            ))
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let changed = conn.execute(
            "DELETE FROM schedules WHERE name = ?1",
            rusqlite::params![name],
        )?;
        if changed == 0 {
            anyhow::bail!("Schedule '{}' not found", name);
        }
        Ok(())
    }

    /// Check for tasks that are due to run.
    /// Returns list of (name, test_type)
    pub async fn check_due_tasks(&self) -> Result<Vec<(String, String)>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT name, cron_expr, last_run_at, test_type FROM schedules WHERE enabled = 1",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let now = Utc::now();
        let mut due_tasks = Vec::new();

        for r in rows {
            let (name, cron_expr, last_run_at, test_type) = r?;

            let should_run = match last_run_at {
                Some(last_run_str) => {
                    // Parse last run time
                    if let Ok(last_run) = chrono::DateTime::parse_from_rfc3339(&last_run_str) {
                        let last_run_utc = last_run.with_timezone(&Utc);
                        if let Ok(schedule) = CronSchedule::from_str(&cron_expr) {
                            // Find next occurrence after last run
                            if let Some(next_run) = schedule.after(&last_run_utc).next() {
                                next_run <= now
                            } else {
                                false // Invalid schedule or finished?
                            }
                        } else {
                            false // Invalid cron
                        }
                    } else {
                        // Mangled date, run immediately and fix
                        true
                    }
                }
                None => true, // Never ran, run now
            };

            if should_run {
                due_tasks.push((name, test_type));
            }
        }

        Ok(due_tasks)
    }

    /// Mark a task as just run
    pub async fn update_last_run(&self, name: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE schedules SET last_run_at = ?1, updated_at = ?1 WHERE name = ?2",
            rusqlite::params![now, name],
        )?;
        Ok(())
    }
}

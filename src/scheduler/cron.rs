use anyhow::{Result, Context};
use std::str::FromStr;
use cron::Schedule as CronSchedule;
use chrono::Utc;
use crate::storage::Pool;

/// A scheduler that persists tasks in SQLite and checks for runnable tasks.
pub struct Scheduler {
    pool: Pool,
}

impl Scheduler {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Add a new schedule to the database
    pub async fn add_schedule(&self, name: &str, cron_expr: &str, test_type: &str) -> Result<()> {
        // Validate Cron
        let _ = CronSchedule::from_str(cron_expr)
            .map_err(|e| anyhow::anyhow!("Invalid cron expression '{}': {}", cron_expr, e))?;

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO schedules (name, cron_expr, test_type, enabled) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![name, cron_expr, test_type],
        ).context("Failed to insert schedule")?;

        Ok(())
    }

    /// Calculate next run times for all enabled schedules
    /// This is strictly a dry-run preview, not the execution loop.
    pub async fn preview_next_runs(&self, hours: u64) -> Result<Vec<(String, String, String)>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT name, cron_expr, test_type FROM schedules WHERE enabled = 1")?;
        
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
                    if next_time > end { break; }
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
        let changed = conn.execute("DELETE FROM schedules WHERE name = ?1", rusqlite::params![name])?;
        if changed == 0 {
            anyhow::bail!("Schedule '{}' not found", name);
        }
        Ok(())
    }
}

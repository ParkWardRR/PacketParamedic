//! Database schema and migrations.

use anyhow::Result;
use rusqlite::Connection;

/// Run all pending migrations.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS probe_results (
            id INTEGER PRIMARY KEY,
            probe_type TEXT NOT NULL,
            target TEXT NOT NULL,
            result_json TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS incidents (
            id INTEGER PRIMARY KEY,
            severity TEXT NOT NULL,
            verdict TEXT NOT NULL,
            evidence_json TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS throughput_results (
            id INTEGER PRIMARY KEY,
            mode TEXT NOT NULL,
            direction TEXT NOT NULL,
            link_speed_mbps INTEGER,
            streams INTEGER NOT NULL DEFAULT 1,
            throughput_mbps REAL,
            jitter_ms REAL,
            loss_percent REAL,
            result_json TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS schedules (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            cron_expr TEXT NOT NULL,
            test_type TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            last_run_at TEXT,
            next_run_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS measurements (
            id INTEGER PRIMARY KEY,
            probe_type TEXT NOT NULL,
            target TEXT NOT NULL,
            value REAL NOT NULL,
            unit TEXT NOT NULL,
            backend TEXT NOT NULL DEFAULT 'scalar',
            duration_us INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS spool (
            id INTEGER PRIMARY KEY,
            payload_json TEXT NOT NULL,
            dispatched INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS schedule_history (
            id INTEGER PRIMARY KEY,
            schedule_name TEXT NOT NULL,
            status TEXT NOT NULL,
            result_summary TEXT,
            backend_used TEXT,
            duration_us INTEGER,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            FOREIGN KEY (schedule_name) REFERENCES schedules(name)
        );

        CREATE INDEX IF NOT EXISTS idx_probe_results_created ON probe_results(created_at);
        CREATE INDEX IF NOT EXISTS idx_incidents_created ON incidents(created_at);
        CREATE INDEX IF NOT EXISTS idx_throughput_created ON throughput_results(created_at);
        CREATE INDEX IF NOT EXISTS idx_schedule_history_name ON schedule_history(schedule_name);

        CREATE TABLE IF NOT EXISTS blame_predictions (
            id INTEGER PRIMARY KEY,
            verdict TEXT NOT NULL,
            confidence REAL NOT NULL,
            probabilities_json TEXT NOT NULL,
            features_json TEXT NOT NULL,
            is_preliminary INTEGER NOT NULL DEFAULT 0,
            analysis_window_start TEXT,
            analysis_window_end TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_blame_created ON blame_predictions(created_at);

        CREATE TABLE IF NOT EXISTS trace_results (
            id INTEGER PRIMARY KEY,
            target TEXT NOT NULL,
            hop_count INTEGER,
            max_latency_ms REAL,
            avg_loss_percent REAL,
            result_json TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_trace_results_created ON trace_results(created_at);",
    )?;

    // Migration: Add 'status' to incidents if missing
    let has_status: i32 = conn.query_row(
        "SELECT count(*) FROM pragma_table_info('incidents') WHERE name='status'",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    if has_status == 0 {
         conn.execute("ALTER TABLE incidents ADD COLUMN status TEXT NOT NULL DEFAULT 'Open'", [])?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();

        // Verify tables exist by querying them
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM probe_results", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schedules", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_migrate_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap(); // Should not error
    }
}

//! SQLite storage layer -- schema, queries, migrations.

pub mod schema;

use anyhow::Result;
use r2d2::Pool as R2D2Pool;
use r2d2_sqlite::SqliteConnectionManager;

/// Connection Pool type
pub type Pool = R2D2Pool<SqliteConnectionManager>;

/// Open (or create) the SQLite database and return a connection pool.
pub fn open_pool(path: &str) -> Result<Pool> {
    let manager = SqliteConnectionManager::file(path).with_init(|c| {
        c.execute_batch(
            "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = NORMAL;
                 PRAGMA temp_store = MEMORY;
                 PRAGMA mmap_size = 30000000000;
                 PRAGMA foreign_keys = ON;
                 PRAGMA busy_timeout = 5000;",
        )
    });

    let pool = R2D2Pool::new(manager)?;

    // Run migrations on a single connection
    let conn = pool.get()?;
    schema::migrate(&conn)?;

    Ok(pool)
}

use crate::probes::Measurement;
use chrono::{DateTime, Utc};

/// Save a probe measurement RESULT to the database.
pub fn save_measurement(pool: &Pool, m: &Measurement) -> Result<()> {
    let conn = pool.get()?;

    // Convert SystemTime to RFC3339 string
    let dt: DateTime<Utc> = m.timestamp.into();
    let created_at = dt.to_rfc3339();

    conn.execute(
        "INSERT INTO measurements (probe_type, target, value, unit, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            m.probe_type.to_string(),
            m.target,
            m.value,
            m.unit,
            created_at
        ],
    )?;

    Ok(())
}

//! SQLite storage layer -- schema, queries, migrations.

pub mod schema;

use anyhow::Result;
use rusqlite::Connection;

/// Open (or create) the SQLite database at the given path with WAL mode.
pub fn open(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )?;
    schema::migrate(&conn)?;
    Ok(conn)
}

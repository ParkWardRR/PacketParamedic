//! SQLite storage layer -- schema, queries, migrations.

pub mod schema;

use anyhow::Result;
use r2d2::Pool as R2D2Pool;
use r2d2_sqlite::SqliteConnectionManager;

/// Connection Pool type
pub type Pool = R2D2Pool<SqliteConnectionManager>;

/// Open (or create) the SQLite database and return a connection pool.
pub fn open_pool(path: &str) -> Result<Pool> {
    let manager = SqliteConnectionManager::file(path)
        .with_init(|c| {
            c.execute_batch(
                "PRAGMA journal_mode = WAL;
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

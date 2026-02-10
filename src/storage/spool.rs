//! Spooling mechanism for write-heavy/crash-safe operations.
//!
//! The spool acts as a persistent write-ahead log. Metrics are written here immediately
//! upon collection. A background worker (or the same process during idle time)
//! aggregates these into the `measurements` and `findings` tables.

use anyhow::Result;
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SpoolEntry {
    pub probe_type: String,
    pub target: String,
    pub value: f64,
    pub unit: String,
    pub backend: String, // 'vk', 'gles', 'neon', 'scalar'
    pub duration_us: Option<u64>,
}

/// Write a measurement to the spool immediately.
/// This is the "fast path" for probes.
pub fn write(conn: &Connection, entry: &SpoolEntry) -> Result<()> {
    let payload = serde_json::to_string(entry)?;
    conn.execute(
        "INSERT INTO spool (payload_json) VALUES (?1)",
        params![payload],
    )?;
    Ok(())
}

/// Process pending items from the spool.
/// Usually called by a background maintenance task.
pub fn process_pending(conn: &mut Connection, limit: usize) -> Result<usize> {
    let tx = conn.transaction()?;
    
    // Select batch
    let mut stmt = tx.prepare(
        "SELECT id, payload_json FROM spool WHERE dispatched = 0 ORDER BY id LIMIT ?1"
    )?;
    
    let rows: Vec<(i64, String)> = stmt.query_map(params![limit], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?
    .collect::<Result<_, _>>()?;

    if rows.is_empty() {
        return Ok(0);
    }

    // Insert into measurements
    let mut insert_stmt = tx.prepare(
        "INSERT INTO measurements (probe_type, target, value, unit, backend, duration_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )?;

    let mut ids_to_delete = Vec::new();

    for (id, json) in rows {
        if let Ok(entry) = serde_json::from_str::<SpoolEntry>(&json) {
            insert_stmt.execute(params![
                entry.probe_type,
                entry.target,
                entry.value,
                entry.unit,
                entry.backend,
                entry.duration_us
            ])?;
            ids_to_delete.push(id);
        } else {
            // Log error but mark as processed/failed to avoid infinite loop
            // In a real app, maybe move to a 'dead_letter' table
            tracing::error!("Failed to parse spool entry id={}", id);
            ids_to_delete.push(id); 
        }
    }

    // Delete processed from spool
    if !ids_to_delete.is_empty() {
        // Simple deletion optimization
        // For appliance reliability, keeping spool history might be nice for debugging,
        // but disk space is precious. We delete processed rows.
        let ids_str = ids_to_delete.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
        tx.execute(
            &format!("DELETE FROM spool WHERE id IN ({})", ids_str),
            [],
        )?;
    }

    tx.commit()?;
    Ok(ids_to_delete.len())
}

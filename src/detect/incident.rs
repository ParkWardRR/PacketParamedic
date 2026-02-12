use crate::detect::{Incident, Severity};
use crate::storage::Pool;
use anyhow::Result;
use rusqlite::params;
use uuid::Uuid;

pub struct IncidentManager {
    pool: Pool,
}

impl IncidentManager {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub fn record_incident(&self, verdict: &str, severity: Severity, evidence: serde_json::Value) -> Result<Uuid> {
        let conn = self.pool.get()?;
        
        // Anti-spam: Check for existing OPEN incident with same verdict within last 30m
        let mut stmt = conn.prepare("SELECT id FROM incidents WHERE verdict = ?1 AND status = 'Open' AND updated_at > datetime('now', '-30 minutes')")?;
        let existing: Result<String, _> = stmt.query_row([verdict], |row| row.get(0));
        
        if let Ok(existing_id) = existing {
             // Update existing incident
             // We update 'updated_at' to keep it alive
             let uuid = Uuid::parse_str(&existing_id).unwrap_or_default();
             conn.execute("UPDATE incidents SET updated_at = datetime('now') WHERE id = ?1", params![existing_id])?;
             return Ok(uuid);
        }

        let id = Uuid::new_v4();
        let severity_str = format!("{:?}", severity); // Info, Warning, Critical
        let evidence_json = serde_json::to_string(&evidence)?;

        conn.execute(
            "INSERT INTO incidents (id, severity, verdict, evidence_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'Open', datetime('now'), datetime('now'))",
            params![id.to_string(), severity_str, verdict, evidence_json],
        )?;

        Ok(id)
    }

    pub fn resolve_incident(&self, incident_id: Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute("UPDATE incidents SET status = 'Resolved', updated_at = datetime('now') WHERE id = ?1", params![incident_id.to_string()])?;
        Ok(())
    }


    pub fn list_recent(&self, limit: usize) -> Result<Vec<Incident>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT id, severity, verdict, evidence_json, created_at FROM incidents ORDER BY created_at DESC LIMIT ?1")?;
        
        let rows = stmt.query_map([limit], |row| {
            let id_str: String = row.get(0)?;
            let sev_str: String = row.get(1)?;
            let severity = match sev_str.as_str() {
                "Critical" => Severity::Critical,
                "Warning" => Severity::Warning,
                _ => Severity::Info,
            };
            let evidence_str: String = row.get(3)?;

            Ok(Incident {
                id: Uuid::parse_str(&id_str).unwrap_or_default(),
                severity,
                verdict: row.get(2)?,
                evidence: serde_json::from_str(&evidence_str).unwrap_or_default(),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?).unwrap_or_default().with_timezone(&chrono::Utc),
            })
        })?;

        let mut incidents = Vec::new();
        for r in rows {
            if let Ok(i) = r { incidents.push(i); }
        }
        Ok(incidents)
    }
}

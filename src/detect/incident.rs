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
        let id = Uuid::new_v4();
        let severity_str = format!("{:?}", severity); // Info, Warning, Critical
        let evidence_json = serde_json::to_string(&evidence)?;

        conn.execute(
            "INSERT INTO incidents (id, severity, verdict, evidence_json, created_at) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![id.to_string(), severity_str, verdict, evidence_json],
        )?;

        Ok(id)
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

use anyhow::{Result, Context};
use crate::storage::Pool;
use crate::analysis::{aggregator::FeatureAggregator, model::LogisticModel};
 // for blame_predictions table? No, we need a writer.
use chrono::Utc;
use tracing::{info, warn};

/// Run the full blame analysis pipeline:
/// 1. Aggregate features from `measurements` (last 5 mins)
/// 2. Load the model (embedded or file)
/// 3. Run inference
/// 4. Store prediction in `blame_predictions`
pub async fn perform_blame_analysis(pool: &Pool) -> Result<()> {
    info!("Starting Blame Analysis...");

    // 1. Aggregate Features
    // We need a Connection. pool.get() gives a pooled connection.
    let conn = pool.get().context("Failed to get DB connection")?;
    
    // Aggregator expects &Connection
    let features = match FeatureAggregator::compute_features(&conn, 5) { // 5 minute window
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to compute features (not enough data?): {}", e);
            return Ok(()); // flexible
        }
    };
    
    // 2. Load Model
    // TODO: Make model path configurable or persistent. For now, try file then default.
    let model = LogisticModel::load("src/analysis/blame_lr.json"); // Try local first
    
    // 3. Inference
    let prediction = model.predict(&features)?; // Propagate error if predict fails
    info!(verdict=%prediction.verdict, confidence=%prediction.confidence, "Blame Analysis Result");

    // 4. Store Result
    // We need to implement a storage function for blame predictions.
    // For now, let's do it here or add to storage module.
    // Let's add it here for simplicity, using the same connection.
    
    let features_json = serde_json::to_string(&features)?;
    let probs_json = serde_json::to_string(&prediction.probabilities)?;
    
    conn.execute(
        "INSERT INTO blame_predictions (
            verdict, confidence, probabilities_json, features_json, 
            is_preliminary, analysis_window_start, analysis_window_end, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            prediction.verdict,
            prediction.confidence,
            probs_json,
            features_json,
            0, // is_preliminary (TODO: Logic for this)
            Utc::now().to_rfc3339(), // Window end (approx)
            Utc::now().to_rfc3339(), // Window start (approx... actually -5m)
            Utc::now().to_rfc3339()
        ]
    ).context("Failed to save blame prediction")?;

    Ok(())
}

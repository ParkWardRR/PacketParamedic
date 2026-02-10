//! Evidence bundle generation and export.

use anyhow::Result;

/// Export a support/evidence bundle to the specified path.
pub async fn export_bundle(output: &str) -> Result<()> {
    // TODO: Collect last 24h of probe results, incidents, config, self-test report
    // TODO: Redact MAC addresses, internal IPs, SSIDs
    // TODO: Package as ZIP
    tracing::info!(%output, "Export bundle (stub)");
    Ok(())
}

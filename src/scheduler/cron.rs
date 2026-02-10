//! Cron expression parser and next-run calculator.

use anyhow::Result;

/// Validate a cron expression string.
pub fn validate(expr: &str) -> Result<()> {
    // TODO: Use the `cron` crate to parse and validate
    if expr.split_whitespace().count() != 5 {
        anyhow::bail!("cron expression must have exactly 5 fields: {}", expr);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cron() {
        assert!(validate("0 3 * * *").is_ok());
        assert!(validate("*/5 * * * *").is_ok());
    }

    #[test]
    fn test_invalid_cron() {
        assert!(validate("bad").is_err());
        assert!(validate("1 2 3").is_err());
    }
}

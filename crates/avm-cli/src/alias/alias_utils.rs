//! Alias helpers: which names an alias may take.
use crate::common::manager::is_reserved;
use anyhow::{anyhow, Result};

/// Rejects alias names that `avm <name>` could never reach: built-in command
/// words, empty names, and names that look like flags.
pub fn validate_alias_name(name: &str) -> Result<()> {
    if name.is_empty() || name.starts_with('-') {
        return Err(anyhow!("invalid alias name '{name}'"));
    }
    if is_reserved(name) {
        return Err(anyhow!(
            "'{name}' is reserved for a built-in command, so it cannot be an alias"
        ));
    }
    Ok(())
}

//! `avm init`.
use crate::config;
use crate::config::{ConfigLoadResult, CONFIG_FILE};
use ::fs_err as fs;
use anyhow::{anyhow, Context, Result};
use std::io;

/// `avm init`: creates `.avm.json` in the cwd and ensures `.avm.local.json` is git-ignored.
/// Errors if the config already exists.
pub fn cmd_init() -> Result<()> {
    let root = std::env::current_dir().context("failed to read current directory")?;
    let _guard = avm_plugin_sdk::storage::lock(&root.join(CONFIG_FILE))?;
    if root.join(CONFIG_FILE).exists() {
        return Err(anyhow!("{CONFIG_FILE} already exists"));
    }
    let ignore = root.join(".gitignore");
    let mut raw = match fs::read_to_string(&ignore) {
        Ok(raw) => raw,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).context("read .gitignore"),
    };
    if !raw
        .lines()
        .any(|line| matches!(line.trim(), ".avm.local.json" | "/.avm.local.json"))
    {
        if !raw.is_empty() && !raw.ends_with('\n') {
            raw.push('\n');
        }
        raw.push_str(".avm.local.json\n");
        avm_plugin_sdk::storage::atomic_write(&ignore, raw.as_bytes())
            .context("update .gitignore")?;
    }
    config::save(&root, &ConfigLoadResult::default())?;
    crate::trust::trust(&root.join(CONFIG_FILE))?;
    done!("Created {CONFIG_FILE} in current directory");
    Ok(())
}

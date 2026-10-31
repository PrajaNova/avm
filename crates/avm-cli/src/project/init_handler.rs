//! `avm init`.
use crate::common::ui;
use crate::config;
use crate::config::{ConfigLoadResult, CONFIG_FILE};
use crate::constants::msg::{self, fill};
use ::fs_err as fs;
use anyhow::{anyhow, Context, Result};
use std::io;

/// `avm init`: creates `.avm.json` in the cwd and ensures `.avm.local.json` is git-ignored.
/// Errors if the config already exists.
pub fn cmd_init() -> Result<()> {
    let root = std::env::current_dir().context(msg::PROJECT_CWD_FAILED)?;
    let _guard = avm_plugin_sdk::storage::lock(&root.join(CONFIG_FILE))?;
    if root.join(CONFIG_FILE).exists() {
        return Err(anyhow!(fill(msg::INIT_EXISTS, &[("file", CONFIG_FILE)])));
    }
    let ignore = root.join(".gitignore");
    let mut raw = match fs::read_to_string(&ignore) {
        Ok(raw) => raw,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).context(msg::INIT_READ_GITIGNORE),
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
            .context(msg::INIT_UPDATE_GITIGNORE)?;
    }
    config::save(&root, &ConfigLoadResult::default())?;
    crate::trust::trust(&root.join(CONFIG_FILE))?;
    ui::success(fill(msg::INIT_CREATED, &[("file", CONFIG_FILE)]));
    Ok(())
}

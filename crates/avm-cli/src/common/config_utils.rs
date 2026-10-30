//! Helpers shared by the config-editing features: where the config lives, how to edit it, and how to print it.
use crate::common::state::home_dir;
use crate::config;
use crate::config::{ConfigLoadResult, CONFIG_FILE};
use crate::resolver::AliasSource;
use anyhow::{anyhow, Context, Result};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

/// `.avm.json`'s directory: `$HOME` for global, else the current directory.
pub fn config_root(global: bool) -> Result<PathBuf> {
    if global {
        home_dir()
    } else {
        std::env::current_dir().context("failed to read current directory")
    }
}

/// Load the local/global config, apply `f`, save it back. A missing file
/// starts empty when `create`, otherwise it's an error.
pub fn edit_config(
    global: bool,
    create: bool,
    f: impl FnOnce(&mut ConfigLoadResult) -> Result<()>,
) -> Result<()> {
    let root = config_root(global)?;
    let _guard = avm_plugin_sdk::storage::lock(&root.join(CONFIG_FILE))?;
    if !create && !root.join(CONFIG_FILE).exists() {
        return Err(if global {
            anyhow!("no {CONFIG_FILE} found in {}", root.display())
        } else {
            anyhow!("no {CONFIG_FILE} found in current directory. Run `avm init` first")
        });
    }
    // Editing through avm keeps an already-trusted (or brand new) local file
    // trusted; it never promotes an untrusted one.
    let file = root.join(CONFIG_FILE);
    let keep_trusted = !global && (!file.exists() || crate::trust::is_trusted(&file, &[]));
    let mut cfg = config::load(&root)?;
    f(&mut cfg)?;
    config::save(&root, &cfg)?;
    if keep_trusted {
        crate::trust::trust(&file)?;
    }
    Ok(())
}

/// Human-readable scope name (`global` or `local`) for confirmation messages.
pub fn scope(global: bool) -> &'static str {
    if global {
        "global"
    } else {
        "local"
    }
}

/// Print local ∪ global entries sorted by key, local winning.
pub fn print_layered(
    local: &HashMap<String, String>,
    global: &HashMap<String, String>,
    indent: &str,
    sep: &str,
    origins: &HashMap<String, String>,
) {
    let keys: BTreeSet<&String> = local.keys().chain(global.keys()).collect();
    for key in keys {
        let value = local
            .get(key)
            .or_else(|| global.get(key))
            .map(String::as_str)
            .unwrap_or("");
        let origin = origins.get(key).map(String::as_str).unwrap_or("unknown");
        let override_label = if local.contains_key(key) && global.contains_key(key) {
            " [override global]"
        } else {
            ""
        };
        println!("{indent}{key}{sep}{value} (from {origin}){override_label}");
    }
}

/// Label (`local`, `global` or `plugin`) for where an alias came from.
pub fn alias_source_label(source: &AliasSource) -> &'static str {
    match source {
        AliasSource::Local => "local",
        AliasSource::Global => "global",
        AliasSource::Plugin => "plugin",
    }
}

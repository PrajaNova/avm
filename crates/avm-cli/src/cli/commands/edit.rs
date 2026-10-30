//! Editing `.avm.json`: add, remove, alias and env.
use crate::cli::args::{AddArgs, AliasCommands, RemoveArgs};
use crate::cli::env::merge_env;
use crate::cli::state::{home_dir, load_state};
use crate::config;
use crate::config::{ConfigLoadResult, CONFIG_FILE};
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
    let _guard = avm_plugin_api::storage::lock(&root.join(CONFIG_FILE))?;
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
fn scope(global: bool) -> &'static str {
    if global {
        "global"
    } else {
        "local"
    }
}

/// Print local ∪ global entries sorted by key, local winning.
fn print_layered(
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

/// `avm add`: stores an alias in the local or global config.
pub fn cmd_add(args: AddArgs) -> Result<()> {
    edit_config(args.global, args.global, |cfg| {
        cfg.aliases.insert(args.key.clone(), args.value.join(" "));
        Ok(())
    })?;
    done!("Added {} alias '{}'", scope(args.global), args.key);
    Ok(())
}

/// `avm remove`: deletes an alias from the chosen config; errors when the alias is not defined there.
pub fn cmd_remove(args: RemoveArgs) -> Result<()> {
    edit_config(args.global, false, |cfg| {
        match cfg.aliases.remove(&args.key) {
            Some(_) => Ok(()),
            None => Err(anyhow!("alias '{}' not found", args.key)),
        }
    })?;
    done!("Removed {} alias '{}'", scope(args.global), args.key);
    Ok(())
}

/// `avm alias add/remove/list` — same shape as `avm plugin`. `add`/`remove`
/// reuse the exact same handlers as the top-level `avm add`/`avm remove`.
pub fn cmd_alias(command: AliasCommands) -> Result<()> {
    match command {
        AliasCommands::Add(args) => cmd_add(args),
        AliasCommands::Remove(args) => cmd_remove(args),
        AliasCommands::List => cmd_alias_list(),
    }
}

/// Lists local and global aliases merged, with local winning, or a hint when none exist.
fn cmd_alias_list() -> Result<()> {
    let cfg = load_state()?;
    if cfg.local_aliases.is_empty() && cfg.global_aliases.is_empty() {
        println!("No aliases configured.");
        return Ok(());
    }
    print_layered(
        &cfg.local_aliases,
        &cfg.global_aliases,
        "",
        " → ",
        &cfg.alias_origins,
    );
    Ok(())
}

/// `avm env add/remove/list` — same shape as `avm plugin`, targeting the
/// custom `env` map in `.avm.json` rather than `aliases`.
pub fn cmd_env_add(key: String, value: String, global: bool) -> Result<()> {
    edit_config(global, global, |cfg| {
        cfg.env.insert(key.clone(), value.clone());
        Ok(())
    })?;
    done!("Added {} env var '{key}={value}'", scope(global));
    Ok(())
}

/// Removes an env var from the chosen config; errors when it is not defined there.
pub fn cmd_env_remove(key: String, global: bool) -> Result<()> {
    edit_config(global, false, |cfg| match cfg.env.remove(&key) {
        Some(_) => Ok(()),
        None => Err(anyhow!("env var '{key}' not found")),
    })?;
    done!("Removed {} env var '{key}'", scope(global));
    Ok(())
}

/// `avm env list`: prints merged env vars sorted by key, each with the file it came from.
pub fn cmd_env_list() -> Result<()> {
    let cfg = load_state()?;
    let merged = merge_env(&cfg);
    if merged.is_empty() {
        println!("No env vars configured.");
        return Ok(());
    }
    let mut keys: Vec<_> = merged.keys().collect();
    keys.sort();
    for key in keys {
        println!(
            "{key}={} (from {})",
            merged[key],
            cfg.env_origins
                .get(key)
                .map(String::as_str)
                .unwrap_or("unknown")
        );
    }
    Ok(())
}

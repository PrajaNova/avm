//! Env commands: the export list, and add/remove/list of custom env vars.
use crate::common::config_utils::{edit_config, scope};
use crate::common::env_utils::{merge_env, resolved_tool_env};
use crate::common::shell::Shell;
use crate::common::state::{
    env_baseline, load_state, untrusted_files, untrusted_notice, ENV_BASELINE, INHERITED_ENV,
};
use crate::common::ui;
use crate::config;
use crate::constants::msg::{self, fill};
use crate::env::EnvCommands;
use anyhow::{anyhow, Result};

/// `avm env`: dispatches the env subcommands; with none, prints the shell statements that apply the resolved environment.
pub fn cmd_env(command: Option<EnvCommands>, shell: Shell) -> Result<()> {
    match command {
        None => cmd_env_print(shell),
        Some(EnvCommands::Add { key, value, global }) => cmd_env_add(key, value, global),
        Some(EnvCommands::Remove { key, global }) => cmd_env_remove(key, global),
        Some(EnvCommands::List) => cmd_env_list(),
    }
}

/// Prints export/unset statements for `shell` so the resolved tool and config env replaces the inherited baseline.
/// Errors if the config env is invalid.
fn cmd_env_print(shell: Shell) -> Result<()> {
    let cfg = load_state()?;
    let mut env = resolved_tool_env(&cfg)?;
    env.extend(merge_env(&cfg));
    config::validate_env(&env)?;
    let mut baseline = env_baseline()?;
    for (key, value) in &baseline {
        if !env.contains_key(key) {
            println!(
                "{}",
                match value {
                    Some(value) => shell.export(key, value),
                    None => shell.unset(key),
                }
            );
        }
    }
    baseline.retain(|key, _| env.contains_key(key));
    // No PATH here: this output is `eval`'d straight into the interactive
    // shell on every `avm` invocation (see shell-init's `_avm_apply_env`).
    // Exporting a resolved-tool PATH prefix here would re-clobber the
    // shims-first ordering shell-init just set up, on every single call —
    // shims already resolve the right managed version per directory, so
    // there's nothing this needs to add for shell use. The equivalent
    // PATH prefix is still applied for real, scoped only to that one
    // subprocess, in `exec-shim`.
    let mut keys: Vec<_> = env.keys().collect();
    keys.sort();
    for key in keys {
        baseline
            .entry(key.clone())
            .or_insert_with(|| INHERITED_ENV.get().and_then(|env| env.get(key)).cloned());
        println!("{}", shell.export(key, &env[key]));
    }
    if baseline.is_empty() {
        println!("{}", shell.unset(ENV_BASELINE));
    } else {
        println!(
            "{}",
            shell.export(ENV_BASELINE, &serde_json::to_string(&baseline)?)
        );
    }
    // This output is eval'd with stderr discarded, so the untrusted notice
    // is emitted as shell code, once per shell session per set of files.
    let untrusted = untrusted_files(&cfg);
    let marker: Vec<String> = untrusted.iter().map(|f| f.display().to_string()).collect();
    let marker = marker.join(":");
    if !untrusted.is_empty() && std::env::var("_AVM_TRUST_NOTICE").ok().as_deref() != Some(&marker)
    {
        println!("{}", shell.warn(&untrusted_notice(&untrusted)));
        println!("{}", shell.export("_AVM_TRUST_NOTICE", &marker));
    }
    Ok(())
}

/// `avm env add/remove/list` — same shape as `avm plugin`, targeting the
/// custom `env` map in `.avm.json` rather than `aliases`.
pub fn cmd_env_add(key: String, value: String, global: bool) -> Result<()> {
    edit_config(global, global, |cfg| {
        cfg.env.insert(key.clone(), value.clone());
        Ok(())
    })?;
    ui::success(fill(
        msg::ENV_ADDED,
        &[("scope", scope(global)), ("key", &key), ("value", &value)],
    ));
    Ok(())
}

/// Removes an env var from the chosen config; errors when it is not defined there.
pub fn cmd_env_remove(key: String, global: bool) -> Result<()> {
    edit_config(global, false, |cfg| match cfg.env.remove(&key) {
        Some(_) => Ok(()),
        None => Err(anyhow!(fill(msg::ENV_NOT_FOUND, &[("key", &key)]))),
    })?;
    ui::success(fill(
        msg::ENV_REMOVED,
        &[("scope", scope(global)), ("key", &key)],
    ));
    Ok(())
}

/// `avm env list`: prints merged env vars sorted by key, each with the file it came from.
pub fn cmd_env_list() -> Result<()> {
    let cfg = load_state()?;
    let merged = merge_env(&cfg);
    if merged.is_empty() {
        ui::info(msg::ENV_NONE);
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

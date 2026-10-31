use crate::common::env_utils::extend_env;
use crate::common::ui;
use crate::config;
use crate::constants::msg::{self, fill};
use crate::resolver::ResolvedConfig;
use ::fs_err as fs;
use anyhow::{Context, Result};
use avm_plugin_sdk::ResolvedAlias;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Config for env, shims and exec: never touches `package.json`, so a broken
/// one can't stop `node` or the prompt.
pub fn load_state() -> Result<ResolvedConfig> {
    load_state_with(false)
}

/// Like `load_state`, plus `package.json` scripts as aliases (for `run`, `resolve`, `which`, `list`).
pub fn load_state_with_aliases() -> Result<ResolvedConfig> {
    load_state_with(true)
}

/// Loads the resolved config for the cwd, optionally including `package.json` scripts as aliases.
/// A broken `package.json` is ignored with a warning.
fn load_state_with(aliases: bool) -> Result<ResolvedConfig> {
    let cwd = std::env::current_dir().context(msg::CWD_READ_FAILED)?;
    let plugin_aliases = if aliases {
        package_json_aliases(&cwd).unwrap_or_else(|err| {
            ui::error(fill(
                msg::STATE_SCRIPTS_IGNORED,
                &[("err", &format!("{err:#}"))],
            ));
            HashMap::new()
        })
    } else {
        HashMap::new()
    };
    let cfg = crate::resolver::load(&cwd, &home_dir()?, plugin_aliases)?;
    if !cfg.local_tools.is_empty() {
        crate::core::tracked::track(&cwd); // so `avm plugin prune` keeps this project's versions
    }
    Ok(cfg)
}

/// `.env` files skipped at startup because they aren't trusted.
pub static UNTRUSTED_DOTENV: std::sync::OnceLock<Vec<PathBuf>> = std::sync::OnceLock::new();
pub const ENV_BASELINE: &str = "_AVM_ENV_BASELINE";
pub static INHERITED_ENV: std::sync::OnceLock<HashMap<String, String>> = std::sync::OnceLock::new();

/// Reads the saved env baseline from `_AVM_ENV_BASELINE`; empty if unset, errors if it is invalid.
pub fn env_baseline() -> Result<BTreeMap<String, Option<String>>> {
    let baseline: BTreeMap<String, Option<String>> = match std::env::var(ENV_BASELINE) {
        Ok(raw) => serde_json::from_str(&raw).context(msg::STATE_BASELINE_INVALID)?,
        Err(_) => BTreeMap::new(),
    };
    config::validate_env(
        &baseline
            .keys()
            .map(|key| (key.clone(), String::new()))
            .collect(),
    )?;
    Ok(baseline)
}

/// Reverts the process env to the saved baseline and records the resulting inherited env for later child processes.
pub fn restore_inherited_env() -> Result<()> {
    for (key, value) in env_baseline()? {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    let mut inherited = HashMap::new();
    extend_env(&mut inherited, std::env::vars());
    let _ = INHERITED_ENV.set(inherited);
    Ok(())
}

/// Project files whose aliases/env are currently ignored for lack of trust.
pub fn untrusted_files(cfg: &ResolvedConfig) -> Vec<PathBuf> {
    cfg.untrusted
        .iter()
        .cloned()
        .chain(UNTRUSTED_DOTENV.get().into_iter().flatten().cloned())
        .collect()
}

/// Message telling the user which config files are ignored for lack of trust and how to trust them.
pub fn untrusted_notice(files: &[PathBuf]) -> String {
    let names: Vec<String> = files.iter().map(|f| f.display().to_string()).collect();
    fill(msg::UNTRUSTED_NOTICE, &[("files", &names.join(", "))])
}

pub use avm_plugin_sdk::home_dir;

/// `package.json` scripts as plugin aliases, run through whichever package
/// manager the lockfile points at.
fn package_json_aliases(cwd: &Path) -> Result<HashMap<String, ResolvedAlias>> {
    let package_json = cwd.join("package.json");
    if !package_json.exists() {
        return Ok(HashMap::new());
    }
    let raw = fs::read_to_string(&package_json).context(msg::PACKAGE_JSON_READ_FAILED)?;
    // Windows editors often save a BOM, which JSON parsers reject.
    let parsed: serde_json::Value = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .context(msg::PACKAGE_JSON_PARSE_FAILED)?;

    let manager = if cwd.join("bun.lockb").exists() || cwd.join("bun.lock").exists() {
        "bun run"
    } else if cwd.join("pnpm-lock.yaml").exists() {
        "pnpm run"
    } else if cwd.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm run"
    };

    let scripts = parsed.get("scripts").and_then(|v| v.as_object());
    Ok(scripts
        .into_iter()
        .flatten()
        .filter_map(|(name, script)| {
            let alias = ResolvedAlias {
                command: format!("{manager} {}", avm_plugin_sdk::sh_quote(name)),
                description: Some(script.as_str()?.to_string()),
                plugin_name: "node".to_string(),
                section_name: "Node Scripts".to_string(),
                source: Some(manager.to_string()),
            };
            Some((name.clone(), alias))
        })
        .collect())
}

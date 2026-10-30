//! The environment a child process (exec, run, shim) is started with.
use crate::global::provider_by_name;
use crate::plugins::is_installed_plugin;
use crate::resolver::ResolvedConfig;
use crate::shims;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Merges global and local env vars, with local values overriding global ones.
pub fn merge_env(cfg: &ResolvedConfig) -> HashMap<String, String> {
    let mut merged = HashMap::new();
    extend_env(&mut merged, cfg.global_env.clone());
    extend_env(&mut merged, cfg.local_env.clone());
    merged
}

/// Inserts `values` into `env`, uppercasing keys on Windows where variable names are case-insensitive.
pub fn extend_env(
    env: &mut HashMap<String, String>,
    values: impl IntoIterator<Item = (String, String)>,
) {
    for (key, value) in values {
        env.insert(crate::platform::normalize_env_key(key), value);
    }
}

/// Env for a child process: inherited env, then provider env, then the
/// managed-tool PATH prefix, then `.avm.json` env (which wins).
pub fn child_env(cfg: &ResolvedConfig) -> Result<HashMap<String, String>> {
    let mut env = HashMap::new();
    extend_env(&mut env, std::env::vars());
    env.extend(resolved_tool_env(cfg)?);
    if let Some(path) = resolved_tool_path_prefix(cfg) {
        env.insert("PATH".to_string(), path);
    }
    env.extend(merge_env(cfg));
    Ok(env)
}

/// Env vars contributed by the selected tools' providers (e.g. `ANDROID_HOME`).
/// These are provider defaults — callers must apply them *before* `merge_env`
/// so user `.avm.json` `env` still wins.
pub fn resolved_tool_env(cfg: &ResolvedConfig) -> Result<HashMap<String, String>> {
    let mut env = HashMap::new();
    for (tool, (version, _)) in cfg.resolve_tools_with_source() {
        // Unknown/uninstalled providers just contribute nothing. Check locally first:
        // `provider_by_name` hits the marketplace for its hint, and this runs per shim/prompt.
        if !is_installed_plugin(&tool) {
            continue;
        }
        if let Ok(provider) = provider_by_name(&tool) {
            if let Ok(vars) = provider.env_vars(&version) {
                extend_env(&mut env, vars);
            }
        }
    }
    Ok(env)
}

/// Builds a PATH prefix of the bin directories of the selected tool versions, in tool-name order without duplicates; `None` when none are installed.
fn resolved_tool_path_prefix(cfg: &ResolvedConfig) -> Option<String> {
    let selections = cfg.resolve_tools_with_source();
    let mut tools: Vec<_> = selections.iter().collect();
    tools.sort_unstable_by_key(|(tool, _)| *tool);

    let mut paths: Vec<PathBuf> = Vec::new();
    for (tool, (version, _)) in tools {
        if let Some(dir) = managed_tool_bin_path(tool, version, tool)
            .and_then(|bin| bin.parent().map(Path::to_path_buf))
        {
            if !paths.contains(&dir) {
                paths.push(dir);
            }
        }
    }
    if paths.is_empty() {
        return None;
    }
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths)
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Path to `binary` inside the managed `tool`/`version` bin dir, or `None` if it is absent or the names are invalid.
pub fn managed_tool_bin_path(tool: &str, version: &str, binary: &str) -> Option<PathBuf> {
    avm_plugin_sdk::download::validate_version(version).ok()?;
    avm_plugin_sdk::download::validate_version(binary).ok()?;
    let bin = avm_plugin_sdk::tool_dir(tool)
        .ok()?
        .join(version)
        .join("bin");
    // `node` → `node.exe` / `npm.cmd` on Windows.
    shims::command_names(binary)
        .into_iter()
        .map(|name| bin.join(name))
        .find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_environment_keys_merge_case_insensitively() {
        let mut env = HashMap::new();
        extend_env(&mut env, [("Path".into(), "system".into())]);
        extend_env(&mut env, [("PATH".into(), "managed".into())]);
        assert_eq!(env["PATH"], "managed");
        assert_eq!(env.len(), if cfg!(windows) { 1 } else { 2 });
    }
}

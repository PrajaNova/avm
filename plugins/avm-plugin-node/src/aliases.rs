use anyhow::{Context, Result};
use avm_plugin_api::ResolvedAlias;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// `package.json` scripts as aliases, run through the lockfile's package manager.
pub fn aliases_from_package_json(cwd: &Path) -> Result<HashMap<String, ResolvedAlias>> {
    let package_json = cwd.join("package.json");
    if !package_json.exists() {
        return Ok(HashMap::new());
    }

    let raw = fs::read_to_string(&package_json).context("failed to read package.json")?;
    let parsed: Value = serde_json::from_str(&raw).context("failed to parse package.json")?;
    let Some(scripts) = parsed.get("scripts").and_then(Value::as_object) else {
        return Ok(HashMap::new());
    };

    let run_prefix = detect_manager(cwd);
    Ok(scripts
        .iter()
        .filter_map(|(name, value)| {
            let alias = ResolvedAlias {
                command: format!("{run_prefix} {name}"),
                description: Some(value.as_str()?.to_string()),
                plugin_name: "node".to_string(),
                section_name: "Node Scripts".to_string(),
                source: Some(run_prefix.to_string()),
            };
            Some((name.clone(), alias))
        })
        .collect())
}

fn detect_manager(cwd: &Path) -> &'static str {
    if cwd.join("bun.lockb").exists() || cwd.join("bun.lock").exists() {
        "bun run"
    } else if cwd.join("pnpm-lock.yaml").exists() {
        "pnpm run"
    } else if cwd.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm run"
    }
}

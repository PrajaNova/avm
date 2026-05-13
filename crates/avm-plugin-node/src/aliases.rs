use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct NodeAlias {
    pub command: String,
    pub description: Option<String>,
    pub manager: String,
}

pub fn aliases_from_package_json(cwd: &Path) -> Result<HashMap<String, NodeAlias>> {
    let package_json = cwd.join("package.json");
    if !package_json.exists() {
        return Ok(HashMap::new());
    }

    let raw = fs::read_to_string(&package_json).context("failed to read package.json")?;
    let parsed: Value = serde_json::from_str(&raw).context("failed to parse package.json")?;
    let scripts = parsed
        .get("scripts")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let run_prefix = detect_manager(cwd);
    let mut aliases = HashMap::new();
    for (name, value) in scripts {
        let Some(cmd) = value.as_str() else {
            continue;
        };
        let command = format!("{run_prefix} {name}");
        aliases.insert(
            name,
            NodeAlias {
                command,
                description: Some(cmd.to_string()),
                manager: run_prefix.to_string(),
            },
        );
    }
    Ok(aliases)
}

fn detect_manager(cwd: &Path) -> String {
    if cwd.join("bun.lockb").exists() || cwd.join("bun.lock").exists() {
        return "bun run".to_string();
    }
    if cwd.join("pnpm-lock.yaml").exists() {
        return "pnpm run".to_string();
    }
    if cwd.join("yarn.lock").exists() {
        return "yarn".to_string();
    }
    "npm run".to_string()
}

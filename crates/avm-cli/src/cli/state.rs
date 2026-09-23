use super::*;

pub fn load_state() -> Result<ResolvedConfig> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let plugin_aliases = package_json_aliases(&cwd)?;
    crate::resolver::load(&cwd, &home_dir()?, plugin_aliases)
}

pub fn home_dir() -> Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow!("HOME not set"))
}

/// `package.json` scripts as plugin aliases, run through whichever package
/// manager the lockfile points at.
fn package_json_aliases(cwd: &Path) -> Result<HashMap<String, ResolvedAlias>> {
    let package_json = cwd.join("package.json");
    if !package_json.exists() {
        return Ok(HashMap::new());
    }
    let raw = fs::read_to_string(&package_json).context("failed to read package.json")?;
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).context("failed to parse package.json")?;

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
                command: format!("{manager} {name}"),
                description: Some(script.as_str()?.to_string()),
                plugin_name: "node".to_string(),
                section_name: "Node Scripts".to_string(),
                source: Some(manager.to_string()),
            };
            Some((name.clone(), alias))
        })
        .collect())
}

fn load_state() -> Result<ResolvedConfig> {
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let home = home_dir()?;
    let mut plugin_aliases = load_runtime_aliases(&cwd)?;
    for (name, alias) in load_node_aliases(&cwd)? {
        plugin_aliases.entry(name).or_insert(alias);
    }
    let resolver = Resolver::new(cwd, home);
    resolver.load(plugin_aliases)
}

fn home_dir() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home));
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        return Ok(PathBuf::from(home));
    }
    Err(anyhow!("HOME not set"))
}

fn load_config_for_root(root: &Path) -> Result<ConfigLoadResult> {
    load_with_env(root, CONFIG_FILE)
}

fn load_runtime_aliases(cwd: &Path) -> Result<HashMap<String, ResolvedAlias>> {
    let plugin_manager = PluginManager::new(None)?;
    plugin_manager.list_aliases(cwd)
}

fn load_node_aliases(cwd: &Path) -> Result<HashMap<String, ResolvedAlias>> {
    let node = NodeProvider::new();
    let aliases = node.aliases_from_package_json(cwd)?;
    let mut resolved = HashMap::new();
    for (name, alias) in aliases {
        resolved.insert(
            name,
            resolve_node_alias(alias),
        );
    }
    Ok(resolved)
}

fn resolve_node_alias(alias: NodeAlias) -> ResolvedAlias {
    let manager = if alias.manager.is_empty() {
        "npm".to_string()
    } else {
        alias.manager.clone()
    };
    ResolvedAlias {
        command: alias.command,
        description: alias.description,
        plugin_name: "node".to_string(),
        section_name: "Node Scripts".to_string(),
        source: Some(manager),
    }
}

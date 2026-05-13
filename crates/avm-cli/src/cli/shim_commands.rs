fn cmd_shims(command: ShimsCommands) -> Result<()> {
    match command {
        ShimsCommands::Install => {
            install_shims()?;
            println!("avm shims installed.");
            Ok(())
        }
        ShimsCommands::Remove { tool } => {
            remove_shim(&tool)?;
            println!("Removed shim for {tool}");
            Ok(())
        }
        ShimsCommands::Path => {
            println!("{}", shim_path_env()?);
            Ok(())
        }
    }
}

fn cmd_exec_shim(args: ExecShimArgs) -> Result<()> {
    let cfg = load_state()?;
    let node = NodeProvider::new();
    let effective_tool = normalize_shim_tool(&args.tool);
    let configured_version = cfg.resolve_tool("node", &cfg).map(|(version, _)| version);
    let selected = configured_version.as_deref().and_then(|version| {
        node.bin_path_for(version, &args.tool)
            .ok()
            .flatten()
            .or_else(|| {
                if effective_tool == "node" {
                    node.bin_path_for(version, "node").ok().flatten()
                } else {
                    None
                }
            })
    });

    let executable = if let Some(executable) = selected {
        executable
    } else {
        if let Some(version) = configured_version {
            eprintln!(
                "warning: managed node {version} is not installed; falling back to system {}",
                args.tool
            );
        }
        which_in_path_excluding_shims(&args.tool).ok_or_else(|| anyhow!("command '{}' not found in PATH", args.tool))?
    };

    let mut env = std::env::vars().collect::<HashMap<String, String>>();
    if let Some(path_prefix) = resolved_tool_path_prefix(&cfg)? {
        env.insert("PATH".to_string(), path_prefix);
    }
    for (key, value) in merge_env(&cfg) {
        env.insert(key, value);
    }

    let status = Command::new(&executable)
        .args(&args.args)
        .envs(env)
        .status()
        .context("failed to run shim target")?;
    std::process::exit(status.code().unwrap_or(1));
}

fn merge_env(cfg: &ResolvedConfig) -> HashMap<String, String> {
    let mut merged = HashMap::new();
    for (key, value) in &cfg.global_env {
        merged.insert(key.clone(), value.clone());
    }
    for (key, value) in &cfg.local_env {
        merged.insert(key.clone(), value.clone());
    }
    merged
}

fn resolved_tool_path_prefix(cfg: &ResolvedConfig) -> Result<Option<String>> {
    let node = NodeProvider::new();
    let selections = cfg.resolve_tools_with_source(&cfg);
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    let sep = if cfg!(windows) { ";" } else { ":" };

    if let Some((version, _)) = selections.get("node") {
        if let Some(path) = node.bin_path_for(version, "node").ok().flatten() {
            if let Some(bin) = path.parent() {
                let candidate = bin.to_string_lossy().to_string();
                if seen.insert(candidate.clone()) {
                    paths.push(candidate);
                }
            }
        }
    }

    if paths.is_empty() {
        return Ok(None);
    }

    let existing = std::env::var("PATH").unwrap_or_else(|_| String::new());
    let injected = paths.join(sep);
    if existing.is_empty() {
        Ok(Some(injected))
    } else {
        Ok(Some(format!("{injected}{sep}{existing}")))
    }
}
fn which_in_path_excluding_shims(binary: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    let shim_dir = avm_shims::shim_dir().ok();

    for entry in std::env::split_paths(&paths) {
        let primary = entry.join(binary);
        if is_safe_executable(&primary).is_some() && !is_shim_path(&primary, &shim_dir) {
            return Some(primary);
        }

        if cfg!(windows) {
            let exe = entry.join(format!("{binary}.exe"));
            if is_safe_executable(&exe).is_some() && !is_shim_path(&exe, &shim_dir) {
                return Some(exe);
            }
        }
    }

    None
}

#[cfg(unix)]
fn is_safe_executable(path: &Path) -> Option<()> {
    if !path.exists() || !path.is_file() {
        return None;
    }

    use std::os::unix::fs::PermissionsExt;
    let meta = path.metadata().ok()?;
    let mode = meta.permissions().mode();
    if mode & 0o002 != 0 {
        return None;
    }
    if mode & 0o111 == 0 {
        return None;
    }
    Some(())
}

#[cfg(not(unix))]
fn is_safe_executable(path: &Path) -> Option<()> {
    if path.exists() && path.is_file() {
        Some(())
    } else {
        None
    }
}

fn is_shim_path(candidate: &Path, shim_dir: &Option<PathBuf>) -> bool {
    let Some(shim_dir) = shim_dir.as_ref() else {
        return false;
    };

    if let (Ok(candidate), Ok(shim_dir)) = (candidate.canonicalize(), shim_dir.canonicalize()) {
        return candidate.starts_with(shim_dir);
    }

    false
}

fn alias_source_label(source: &AliasSource) -> &'static str {
    match source {
        AliasSource::Local => "local",
        AliasSource::Global => "global",
        AliasSource::Plugin => "plugin",
    }
}

fn normalize_shim_tool(tool: &str) -> &str {
    match tool {
        "npm" | "npx" | "pnpm" | "yarn" | "bun" => "node",
        value => value,
    }
}

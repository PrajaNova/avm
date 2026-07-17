fn cmd_shims(command: ShimsCommands) -> Result<()> {
    match command {
        ShimsCommands::Install => {
            reshim()?;
            println!("avm shims installed.");
            Ok(())
        }
        ShimsCommands::Reshim => {
            reshim()?;
            println!("avm shims regenerated.");
            Ok(())
        }
        ShimsCommands::Activate => {
            let written = activate_profiles()?;
            if written.is_empty() {
                println!("avm shims already active in your shell startup files.");
            } else {
                for path in &written {
                    println!("Added avm shims to {}", path.display());
                }
                println!("Open a new shell (or `source` the file) to apply.");
            }
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
    let effective_tool = normalize_shim_tool(&args.tool);

    let executable = match resolve_managed_binary(&cfg, effective_tool, &args.tool) {
        Some(executable) => executable,
        None => {
            // A pinned tool that isn't installed is worth warning about; an
            // unknown binary (e.g. a global package) just falls through.
            if let Some((version, _)) = cfg.resolve_tool(effective_tool, &cfg) {
                eprintln!(
                    "warning: managed {effective_tool} {version} is not installed; falling back to system {}",
                    args.tool
                );
            }
            which_in_path_excluding_shims(&args.tool)
                .ok_or_else(|| anyhow!("command '{}' not found in PATH", args.tool))?
        }
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

    // A global package install adds new binaries to a version's bin dir. Reshim
    // so `tsc`/`eslint`/etc. are runnable immediately, no manual step.
    // ponytail: reshims on any pkg-manager install verb; prune stale shims later if it matters.
    if status.success() && is_node_pkg_install(&args.tool, &args.args) {
        let _ = reshim();
    }

    std::process::exit(status.code().unwrap_or(1));
}

/// Find `binary` inside a managed version's bin dir. Search order: the pinned
/// version of the tool the binary maps to (local pin first, then global), then
/// every other managed tool. This resolves core tools *and* global-package
/// binaries, and lets a package installed under the global version still run in
/// a project pinned to a different version.
fn resolve_managed_binary(
    cfg: &ResolvedConfig,
    effective_tool: &str,
    binary: &str,
) -> Option<PathBuf> {
    let mut candidates: Vec<(String, String)> = Vec::new();
    let push = |tool: &str, candidates: &mut Vec<(String, String)>| {
        if let Some(v) = cfg.local_tools.get(tool) {
            candidates.push((tool.to_string(), v.clone()));
        }
        if let Some(v) = cfg.global_tools.get(tool) {
            candidates.push((tool.to_string(), v.clone()));
        }
    };

    push(effective_tool, &mut candidates);
    let mut others: Vec<&String> = cfg.local_tools.keys().chain(cfg.global_tools.keys()).collect();
    others.sort_unstable();
    others.dedup();
    for tool in others {
        if tool != effective_tool {
            push(tool, &mut candidates);
        }
    }

    candidates
        .into_iter()
        .find_map(|(tool, version)| managed_tool_bin_path(&tool, &version, binary))
}

fn is_node_pkg_install(tool: &str, args: &[String]) -> bool {
    if !matches!(tool, "npm" | "npx" | "pnpm" | "yarn" | "bun") {
        return false;
    }
    args.iter().any(|a| {
        matches!(
            a.as_str(),
            "install" | "i" | "add" | "ci" | "remove" | "uninstall" | "rm" | "link" | "unlink"
        )
    })
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
            push_bin_dir(&mut paths, &mut seen, path.parent());
        }
    }

    let mut tools: Vec<_> = selections.keys().collect();
    tools.sort_unstable();
    for tool in tools {
        if tool == "node" {
            continue;
        }
        if let Some((version, _)) = selections.get(tool) {
            if let Some(path) = managed_tool_bin_path(tool, version, tool) {
                push_bin_dir(&mut paths, &mut seen, path.parent());
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

fn managed_tool_bin_path(tool: &str, version: &str, binary: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let candidate = home
        .join(".avm")
        .join("tools")
        .join(tool)
        .join(version)
        .join("bin")
        .join(binary_name(binary));
    candidate.exists().then_some(candidate)
}

fn push_bin_dir(paths: &mut Vec<String>, seen: &mut HashSet<String>, bin: Option<&Path>) {
    if let Some(bin) = bin {
        let candidate = bin.to_string_lossy().to_string();
        if seen.insert(candidate.clone()) {
            paths.push(candidate);
        }
    }
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
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
        "java" | "javac" | "jar" | "javadoc" | "jshell" | "jarsigner" | "keytool" => "java",
        value => value,
    }
}

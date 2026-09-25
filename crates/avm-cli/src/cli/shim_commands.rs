use super::*;
use crate::shims::TOOL_BINS;

pub fn cmd_shims(command: ShimsCommands) -> Result<()> {
    match command {
        ShimsCommands::Install => {
            shims::reshim()?;
            println!("avm shims installed.");
            Ok(())
        }
        ShimsCommands::Activate => {
            let written = shims::activate_profiles()?;
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
            shims::remove_shim(&tool)?;
            println!("Removed shim for {tool}");
            Ok(())
        }
        ShimsCommands::Path => {
            println!("{}", shims::shim_dir()?.display());
            Ok(())
        }
    }
}

pub fn cmd_exec_shim(args: ExecShimArgs) -> Result<()> {
    let cfg = load_state()?;
    let effective_tool = normalize_shim_tool(&args.tool);

    let executable = match resolve_managed_binary(&cfg, effective_tool, &args.tool) {
        Some(executable) => executable,
        None => {
            // A pinned tool that isn't installed is worth warning about; an
            // unknown binary (e.g. a global package) just falls through.
            if let Some((version, _)) = cfg.resolve_tool(effective_tool) {
                eprintln!(
                    "warning: managed {effective_tool} {version} is not installed; falling back to system {}",
                    args.tool
                );
            }
            shims::which(&args.tool, true)
                .ok_or_else(|| anyhow!("command '{}' not found in PATH", args.tool))?
        }
    };

    let status = Command::new(&executable)
        .args(&args.args)
        .envs(child_env(&cfg)?)
        .status()
        .context("failed to run shim target")?;

    // A global package install adds new binaries to a version's bin dir. Reshim
    // so `tsc`/`eslint`/etc. are runnable immediately, no manual step.
    // ponytail: reshims on any pkg-manager install verb; prune stale shims later if it matters.
    if status.success() && is_node_pkg_install(&args.tool, &args.args) {
        let _ = shims::reshim();
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
    let others: BTreeSet<&str> = cfg
        .local_tools
        .keys()
        .chain(cfg.global_tools.keys())
        .map(String::as_str)
        .filter(|tool| *tool != effective_tool)
        .collect();
    std::iter::once(effective_tool)
        .chain(others)
        .flat_map(|tool| {
            [cfg.local_tools.get(tool), cfg.global_tools.get(tool)]
                .into_iter()
                .flatten()
                .map(move |version| (tool, version))
        })
        .find_map(|(tool, version)| managed_tool_bin_path(tool, version, binary))
}

fn is_node_pkg_install(tool: &str, args: &[String]) -> bool {
    tool != "node"
        && normalize_shim_tool(tool) == "node"
        && args.iter().any(|a| {
            matches!(
                a.as_str(),
                "install" | "i" | "add" | "ci" | "remove" | "uninstall" | "rm" | "link" | "unlink"
            )
        })
}

/// The tool whose pinned version a shimmed binary runs under (`npm` → `node`).
fn normalize_shim_tool(tool: &str) -> &str {
    TOOL_BINS
        .iter()
        .find(|(_, bins)| bins.contains(&tool))
        .map_or(tool, |(owner, _)| owner)
}

pub fn merge_env(cfg: &ResolvedConfig) -> HashMap<String, String> {
    let mut merged = cfg.global_env.clone();
    merged.extend(cfg.local_env.clone());
    merged
}

/// Env for a child process: inherited env, then provider env, then the
/// managed-tool PATH prefix, then `.avm.json` env (which wins).
pub fn child_env(cfg: &ResolvedConfig) -> Result<HashMap<String, String>> {
    let mut env: HashMap<String, String> = std::env::vars().collect();
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
        // Unknown/uninstalled providers just contribute nothing.
        if let Ok(provider) = provider_by_name(&tool) {
            if let Ok(vars) = provider.env_vars(&version) {
                env.extend(vars);
            }
        }
    }
    Ok(env)
}

fn resolved_tool_path_prefix(cfg: &ResolvedConfig) -> Option<String> {
    let selections = cfg.resolve_tools_with_source();
    let mut tools: Vec<_> = selections.iter().collect();
    tools.sort_unstable_by_key(|(tool, _)| *tool);

    let mut paths: Vec<String> = Vec::new();
    for (tool, (version, _)) in tools {
        if let Some(bin) = managed_tool_bin_path(tool, version, tool) {
            if let Some(dir) = bin.parent().map(|d| d.to_string_lossy().to_string()) {
                if !paths.contains(&dir) {
                    paths.push(dir);
                }
            }
        }
    }
    if paths.is_empty() {
        return None;
    }
    if let Ok(existing) = std::env::var("PATH") {
        if !existing.is_empty() {
            paths.push(existing);
        }
    }
    Some(paths.join(":"))
}

fn managed_tool_bin_path(tool: &str, version: &str, binary: &str) -> Option<PathBuf> {
    let candidate = avm_plugin_api::tool_dir(tool)
        .ok()?
        .join(version)
        .join("bin")
        .join(binary);
    candidate.exists().then_some(candidate)
}

pub fn alias_source_label(source: &AliasSource) -> &'static str {
    match source {
        AliasSource::Local => "local",
        AliasSource::Global => "global",
        AliasSource::Plugin => "plugin",
    }
}

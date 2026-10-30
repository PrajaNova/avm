use crate::common::env_utils::{child_env, managed_tool_bin_path};
use crate::common::state::load_state;
use crate::project::mark_used;
use crate::resolver::ResolvedConfig;
use crate::shims;
use crate::shims::TOOL_BINS;
use crate::shims::{ExecShimArgs, ShimsCommands};
use anyhow::{anyhow, Context, Result};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

/// `avm shims`: installs shims or activates them in the user's shell startup files.
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

/// Runs a shimmed binary with the project's managed environment, falling back to the system binary.
/// Reshims after node package installs and exits with the child's status.
pub fn cmd_exec_shim(args: ExecShimArgs) -> Result<()> {
    let cfg = load_state()?;
    let effective_tool = normalize_shim_tool(&args.tool);

    let executable = match resolve_managed_binary(&cfg, effective_tool, &args.tool) {
        Some(executable) => {
            mark_used(&executable);
            executable
        }
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
        if let Err(err) = shims::reshim() {
            eprintln!(
                "avm: couldn't refresh shims after the install ({err:#}); run `avm shims install`"
            );
        }
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

/// True when a non-`node` binary that maps to node (such as `npm`) is run with a package install or removal verb.
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

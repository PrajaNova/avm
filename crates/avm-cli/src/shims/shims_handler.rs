use crate::common::env_utils::{child_env, managed_tool_bin_path};
use crate::common::state::load_state;
use crate::common::ui;
use crate::constants::msg::{self, fill};
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
            ui::info(msg::SHIM_INSTALLED);
            Ok(())
        }
        ShimsCommands::Activate => {
            let written = shims::activate_profiles()?;
            if written.is_empty() {
                ui::info(msg::SHIM_ALREADY_ACTIVE);
            } else {
                for path in &written {
                    ui::info(fill(
                        msg::SHIM_ACTIVATED,
                        &[("path", &path.display().to_string())],
                    ));
                }
                ui::info(msg::SHIM_OPEN_NEW_SHELL);
            }
            Ok(())
        }
        ShimsCommands::Remove { tool } => {
            shims::remove_shim(&tool)?;
            ui::info(fill(msg::SHIM_REMOVED, &[("tool", &tool)]));
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
                ui::notice(fill(
                    msg::SHIM_NOT_INSTALLED,
                    &[
                        ("tool", effective_tool),
                        ("version", &version.to_string()),
                        ("command", &args.tool),
                    ],
                ));
            }
            shims::which(&args.tool, true)
                .ok_or_else(|| anyhow!(fill(msg::COMMAND_NOT_FOUND, &[("program", &args.tool)])))?
        }
    };

    let status = Command::new(&executable)
        .args(&args.args)
        .envs(child_env(&cfg)?)
        .status()
        .context(msg::SHIM_RUN_FAILED)?;

    // A global package install adds new binaries to a version's bin dir. Reshim
    // so `tsc`/`eslint`/etc. are runnable immediately, no manual step.
    // ponytail: reshims on any pkg-manager install verb; prune stale shims later if it matters.
    if status.success() && is_node_pkg_install(&args.tool, &args.args) {
        if let Err(err) = shims::reshim() {
            ui::error(fill(
                msg::SHIM_RESHIM_FAILED,
                &[("err", &format!("{err:#}"))],
            ));
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

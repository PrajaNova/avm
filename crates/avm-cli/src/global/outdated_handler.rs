//! `avm outdated` and `avm upgrade`.
use crate::common::config_utils::alias_source_label;
use crate::common::state::load_state;
use crate::common::ui;
use crate::config::CONFIG_FILE;
use crate::global::{ensure_provider_version_installed, provider_by_name, set_tool_version};
use crate::resolver::{AliasSource, ResolvedConfig};
use crate::shims;
use crate::version_files::{numbers, pick};
use anyhow::{anyhow, Context, Result};
use avm_plugin_sdk::ToolVersionQuery;
use std::io;
use std::io::Write;
use std::path::Path;

struct Status {
    tool: String,
    requested: String,
    current: String,
    in_range: Option<String>,
    latest: Option<String>,
    /// `local`, `global`, or the version file (`./.nvmrc`).
    source: String,
    /// Pinned by a version file we won't rewrite.
    from_file: bool,
}

/// Fetches recent version names from the tool's plugin; `None` when no plugin exists, `Some(Err)` when listing fails.
fn available_versions(tool: &str) -> Option<Result<Vec<String>>> {
    let provider = provider_by_name(tool).ok()?;
    Some(
        provider
            .available_versions(ToolVersionQuery::Recent)
            .map(|versions| versions.into_iter().map(|v| v.version).collect())
            .with_context(|| format!("couldn't list {tool} versions")),
    )
}

/// Builds one status row per pinned tool (restricted to `only` if non-empty), sorted by tool name.
/// Version lists are fetched in parallel; tools without a usable plugin are skipped.
fn statuses(cfg: &ResolvedConfig, only: &[String]) -> Result<Vec<Status>> {
    let mut rows = Vec::new();
    let mut tools: Vec<_> = cfg.resolve_tools_with_source().into_iter().collect();
    tools.sort_by(|a, b| a.0.cmp(&b.0));
    tools.retain(|(tool, _)| only.is_empty() || only.contains(tool));
    // `None` = no usable plugin for the tool, which is skipped.
    let lists: Vec<Option<Result<Vec<String>>>> = std::thread::scope(|scope| {
        let handles: Vec<_> = tools
            .iter()
            .map(|(tool, _)| scope.spawn(|| available_versions(tool)))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap_or(None))
            .collect()
    });
    for ((tool, (current, source)), available) in tools.into_iter().zip(lists) {
        let Some(available) = available else {
            continue;
        };
        let available = available?;
        let origin = matches!(source, AliasSource::Local)
            .then(|| cfg.tool_origins.get(&tool))
            .flatten()
            .filter(|file| {
                std::env::current_dir()
                    .ok()
                    .is_none_or(|cwd| Path::new(file.as_str()) != cwd.join(CONFIG_FILE))
            });
        let requested = match source {
            AliasSource::Local => cfg
                .tool_specs
                .get(&tool)
                .cloned()
                .unwrap_or_else(|| current.clone()),
            _ => cfg
                .global_tool_specs
                .get(&tool)
                .cloned()
                .unwrap_or_else(|| current.clone()),
        };
        // An exact pin's "range" is its major line; a spec (`20`, `>=18`) is its own range.
        let range = if requested == current {
            numbers(&current)
                .and_then(|n| n.first().map(u64::to_string))
                .unwrap_or(requested.clone())
        } else {
            requested.clone()
        };
        rows.push(Status {
            in_range: pick(&tool, &range, available.clone()),
            latest: pick(&tool, "*", available),
            source: origin
                .cloned()
                .unwrap_or_else(|| alias_source_label(&source).to_string()),
            from_file: origin.is_some(),
            tool,
            requested,
            current,
        });
    }
    Ok(rows)
}

/// Prints `rows` as a left-aligned table with column widths sized to the longest cell.
fn print_table(header: &[&str], rows: &[Vec<String>]) {
    let widths: Vec<usize> = (0..header.len())
        .map(|i| {
            rows.iter()
                .map(|r| r[i].len())
                .chain([header[i].len()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |cells: Vec<&str>| {
        let padded: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(c, w)| format!("{c:<w$}"))
            .collect();
        println!("{}", padded.join("  ").trim_end());
    };
    line(header.to_vec());
    for row in rows {
        line(row.iter().map(String::as_str).collect());
    }
}

/// `avm outdated`: shows requested, current and latest versions per tool, as JSON when `json` is set.
pub fn cmd_outdated(json: bool) -> Result<()> {
    let cfg = load_state()?;
    let rows = statuses(&cfg, &[])?;
    if json {
        let out: Vec<_> = rows
            .iter()
            .map(|s| {
                serde_json::json!({
                    "tool": s.tool, "requested": s.requested, "current": s.current,
                    "latest_in_range": s.in_range, "latest": s.latest, "source": s.source,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }
    if rows.is_empty() {
        println!("No tools pinned here.");
        return Ok(());
    }
    let dash = || "-".to_string();
    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|s| {
            vec![
                s.tool.clone(),
                s.requested.clone(),
                s.current.clone(),
                s.in_range.clone().unwrap_or_else(dash),
                s.latest.clone().unwrap_or_else(dash),
                s.source.clone(),
            ]
        })
        .collect();
    print_table(
        &[
            "TOOL",
            "REQUESTED",
            "CURRENT",
            "LATEST IN RANGE",
            "LATEST",
            "PINNED BY",
        ],
        &table,
    );
    Ok(())
}

/// Asks a yes/no question; `yes` skips the prompt.
/// Errors when there is no terminal to prompt on and `yes` was not given.
pub fn confirm(question: &str, yes: bool) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    if !ui::can_select() {
        return Err(anyhow!(
            "{question} Re-run with -y to confirm (no terminal to ask)."
        ));
    }
    print!("{question} [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

/// `avm upgrade`: moves pinned tools to the newest in-range version, or the newest overall with `bump`.
/// `dry_run` only prints the plan; `yes` skips the confirmation.
pub fn cmd_upgrade(tools: Vec<String>, bump: bool, dry_run: bool, yes: bool) -> Result<()> {
    let cfg = load_state()?;
    let rows = statuses(&cfg, &tools)?;
    if rows.is_empty() {
        println!("No tools pinned here.");
        return Ok(());
    }
    let mut plan = Vec::new();
    for s in rows {
        let target = if bump {
            s.latest.clone()
        } else {
            s.in_range.clone()
        };
        match target {
            Some(t) if t != s.current => {
                if bump && s.from_file {
                    println!("{}: pinned by {}; edit it to move to {t}", s.tool, s.source);
                } else {
                    plan.push((s, t));
                }
            }
            _ => println!("{}: {} is up to date", s.tool, s.current),
        }
    }
    if plan.is_empty() {
        return Ok(());
    }
    for (s, target) in &plan {
        let pin = if s.from_file {
            "installs; the version file already covers it".to_string()
        } else {
            format!("{} pin", s.source)
        };
        println!("{}: {} → {target} ({pin})", s.tool, s.current);
    }
    if dry_run
        || (bump
            && !confirm(
                "Move these pins to the latest release (may cross a major version)?",
                yes,
            )?)
    {
        return Ok(());
    }
    for (s, target) in plan {
        let provider = provider_by_name(&s.tool)?;
        ensure_provider_version_installed(&s.tool, provider.as_ref(), &target)?;
        if !s.from_file {
            set_tool_version(&s.tool, &target, s.source == "global")?;
        }
    }
    shims::reshim()?;
    Ok(())
}

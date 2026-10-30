//! `avm plugin list`.
use crate::common::config_utils::alias_source_label;
use crate::common::state::{home_dir, load_state_with_aliases, untrusted_files, untrusted_notice};
use crate::plugins::PluginRegistry;
use crate::resolver::ResolvedConfig;
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// `~/x`, `./x` or `../x` instead of an absolute path (a file directly in home is `~/x`).
fn short_path(path: &str) -> String {
    let path = Path::new(path);
    let home = home_dir().ok();
    let under_home = home.as_deref().and_then(|h| path.strip_prefix(h).ok());
    if let Some(rest) = under_home.filter(|r| r.components().count() == 1) {
        return format!("~/{}", rest.display());
    }
    let cwd = std::env::current_dir().ok();
    for (depth, dir) in cwd.iter().flat_map(|c| c.ancestors()).enumerate() {
        if let Ok(rest) = path.strip_prefix(dir) {
            let up = if depth == 0 {
                "./".to_string()
            } else {
                "../".repeat(depth)
            };
            return format!("{up}{}", rest.display());
        }
    }
    match under_home {
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// One-line summary of which project config files supply local settings, or a note that only global settings apply.
fn project_line(cfg: &ResolvedConfig) -> String {
    let files: BTreeSet<String> = cfg
        .local_aliases
        .keys()
        .filter_map(|k| cfg.alias_origins.get(k))
        .chain(cfg.local_env.keys().filter_map(|k| cfg.env_origins.get(k)))
        .chain(
            cfg.local_tools
                .keys()
                .filter_map(|k| cfg.tool_origins.get(k)),
        )
        .map(|o| short_path(o))
        .collect();
    if files.is_empty() {
        "Project: none here (no .avm.json). Showing global only.".to_string()
    } else {
        format!(
            "Project: {}",
            files.into_iter().collect::<Vec<_>>().join(", ")
        )
    }
}

/// Installed versions with the active one starred; flags a pin that is not installed.
pub(crate) fn installed_cell(installed: &[String], active: Option<&str>) -> String {
    let mut cells: Vec<String> = installed
        .iter()
        .map(|v| format!("{v}{}", if Some(v.as_str()) == active { "*" } else { "" }))
        .collect();
    if active.is_some_and(|a| !installed.iter().any(|v| v == a)) {
        cells.push("(not installed)".into());
    }
    if cells.is_empty() {
        "-".into()
    } else {
        cells.join("  ")
    }
}

/// TOOL / ACTIVE / SCOPE / SET BY / INSTALLED, one row per pinned or installed tool.
fn print_tool_table(cfg: &ResolvedConfig, installed: &BTreeMap<String, Vec<String>>) {
    let names: BTreeSet<&String> = cfg
        .local_tools
        .keys()
        .chain(cfg.global_tools.keys())
        .chain(installed.keys())
        .collect();
    let mut rows = vec![["TOOL", "ACTIVE", "SCOPE", "SET BY", "INSTALLED"].map(String::from)];
    for name in names {
        let pin = cfg.resolve_tool(name);
        let active = pin.as_ref().map(|(v, _)| v.as_str());
        let versions = installed.get(name).map(Vec::as_slice).unwrap_or(&[]);
        rows.push([
            name.to_string(),
            active.unwrap_or("-").to_string(),
            pin.as_ref()
                .map_or("-", |(_, s)| alias_source_label(s))
                .to_string(),
            cfg.tool_origins
                .get(name)
                .map_or("-".to_string(), |o| short_path(o)),
            installed_cell(versions, active),
        ]);
    }
    let widths: Vec<usize> = (0..4)
        .map(|i| rows.iter().map(|r| r[i].len()).max().unwrap_or(0) + 2)
        .collect();
    for row in rows {
        let lead: String = (0..4)
            .map(|i| format!("{:<w$}", row[i], w = widths[i]))
            .collect();
        println!("{lead}{}", row[4]);
    }
}

/// Installed versions per plugin, numerically sorted; plugins with none are left out.
fn installed_by_tool() -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    let Ok(plugin_manager) = PluginRegistry::new() else {
        return out;
    };
    for name in plugin_manager.installed_tool_names() {
        let provider = plugin_manager.provider(&name);
        if let Some(Ok(mut versions)) = provider.map(|p| p.installed_versions()) {
            if !versions.is_empty() {
                crate::version_files::sort_versions(&mut versions);
                out.insert(name, versions);
            }
        }
    }
    out
}

type PluginAliasSections<'a> = BTreeMap<&'a str, BTreeMap<&'a str, (&'a str, &'a str)>>;

/// Plugin-provided aliases not shadowed by a user alias, by section then name.
fn visible_plugin_aliases(cfg: &ResolvedConfig) -> PluginAliasSections<'_> {
    let mut sections = PluginAliasSections::new();
    for (name, alias) in &cfg.plugin_aliases {
        if cfg.local_aliases.contains_key(name) || cfg.global_aliases.contains_key(name) {
            continue;
        }
        sections.entry(&alias.section_name).or_default().insert(
            name,
            (&alias.command, alias.source.as_deref().unwrap_or_default()),
        );
    }
    sections
}

/// `avm plugin list`: prints each tool's active version, where it is pinned and its installed versions, then plugin-provided aliases.
/// Warns about untrusted config files that are being ignored.
pub fn cmd_plugin_list() -> Result<()> {
    let cfg = load_state_with_aliases()?;
    let untrusted = untrusted_files(&cfg);
    if !untrusted.is_empty() {
        eprintln!("{}", untrusted_notice(&untrusted));
    }
    let installed = installed_by_tool();
    let plugin_aliases = visible_plugin_aliases(&cfg);
    let has_tools =
        !cfg.local_tools.is_empty() || !cfg.global_tools.is_empty() || !installed.is_empty();
    let has_aliases = !plugin_aliases.is_empty();

    if has_tools || has_aliases {
        println!("{}", project_line(&cfg));
        println!();
    }
    if has_tools {
        print_tool_table(&cfg, &installed);
        println!();
    }
    if !plugin_aliases.is_empty() {
        println!("Plugin aliases:");
        for (section, items) in plugin_aliases {
            println!("  {section}:");
            for (name, (command, source)) in items {
                if source.is_empty() {
                    println!("    {name} → {command}");
                } else {
                    println!("    {name} → {command} ({source})");
                }
            }
        }
    }
    if !has_tools && !has_aliases {
        println!("No plugins or tools yet.");
        println!();
        println!("Get started:");
        println!("  avm plugin install <plugin>");
        println!("  avm node use 20.11.1");
    }
    Ok(())
}

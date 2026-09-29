//! `avm outdated`, `avm upgrade`, `avm plugin outdated` (#26) and `avm prune` (#27).
use super::*;
use crate::version_files::{numbers, pick};
use std::time::{Duration, SystemTime};

// ponytail: version lists are fetched live (one plugin call per tool); add a
// ~/.avm/cache with a TTL if outdated/upgrade ever feel slow.

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

fn statuses(cfg: &ResolvedConfig, only: &[String]) -> Result<Vec<Status>> {
    let mut rows = Vec::new();
    let mut tools: Vec<_> = cfg.resolve_tools_with_source().into_iter().collect();
    tools.sort_by(|a, b| a.0.cmp(&b.0));
    for (tool, (current, source)) in tools {
        if !only.is_empty() && !only.contains(&tool) {
            continue;
        }
        let Ok(provider) = provider_by_name(&tool) else { continue };
        let available: Vec<String> = provider
            .available_versions(ToolVersionQuery::Recent)
            .with_context(|| format!("couldn't list {tool} versions"))?
            .into_iter()
            .map(|v| v.version)
            .collect();
        let origin = matches!(source, AliasSource::Local).then(|| cfg.tool_origins.get(&tool)).flatten();
        let requested = match source {
            AliasSource::Local => cfg.tool_specs.get(&tool).cloned().unwrap_or_else(|| current.clone()),
            _ => current.clone(),
        };
        // An exact pin's "range" is its major line; a spec (`20`, `>=18`) is its own range.
        let range = if requested == current {
            numbers(&current).and_then(|n| n.first().map(u64::to_string)).unwrap_or(requested.clone())
        } else {
            requested.clone()
        };
        rows.push(Status {
            in_range: pick(&tool, &range, available.clone()),
            latest: pick(&tool, "*", available),
            source: origin.cloned().unwrap_or_else(|| alias_source_label(&source).to_string()),
            from_file: origin.is_some(),
            tool,
            requested,
            current,
        });
    }
    Ok(rows)
}

fn print_table(header: &[&str], rows: &[Vec<String>]) {
    let widths: Vec<usize> = (0..header.len())
        .map(|i| rows.iter().map(|r| r[i].len()).chain([header[i].len()]).max().unwrap_or(0))
        .collect();
    let line = |cells: Vec<&str>| {
        let padded: Vec<String> = cells.iter().zip(&widths).map(|(c, w)| format!("{c:<w$}")).collect();
        println!("{}", padded.join("  ").trim_end());
    };
    line(header.to_vec());
    for row in rows {
        line(row.iter().map(String::as_str).collect());
    }
}

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
    print_table(&["TOOL", "REQUESTED", "CURRENT", "LATEST IN RANGE", "LATEST", "PINNED BY"], &table);
    Ok(())
}

fn confirm(question: &str, yes: bool) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    if !ui::can_select() {
        return Err(anyhow!("{question} Re-run with -y to confirm (no terminal to ask)."));
    }
    print!("{question} [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

pub fn cmd_upgrade(tools: Vec<String>, bump: bool, dry_run: bool, yes: bool) -> Result<()> {
    let cfg = load_state()?;
    let mut plan = Vec::new();
    for s in statuses(&cfg, &tools)? {
        let target = if bump { s.latest.clone() } else { s.in_range.clone() };
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
        let pin = if s.from_file { "installs; the version file already covers it".to_string() } else { format!("{} pin", s.source) };
        println!("{}: {} → {target} ({pin})", s.tool, s.current);
    }
    if dry_run || (bump && !confirm("Move these pins to the latest release (may cross a major version)?", yes)?) {
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

pub fn cmd_plugin_outdated(json: bool) -> Result<()> {
    let manager = PluginManager::new()?;
    let mut rows = Vec::new();
    let mut names: Vec<String> = manager.list_plugins().into_values().map(|m| m.name).collect();
    names.sort();
    names.dedup();
    for name in names {
        // Marketplace installs record their release in meta.json.
        let meta = manager.plugin_dir().join(format!("avm-plugin-{name}")).join("meta.json");
        let installed = fs::read(&meta)
            .ok()
            .and_then(|raw| serde_json::from_slice::<serde_json::Value>(&raw).ok())
            .and_then(|m| m["version"].as_str().map(str::to_string));
        let latest = runtime::marketplace_lookup(&name)
            .ok()
            .flatten()
            .and_then(|entry| runtime::github_release(&entry.repo, None).ok())
            .map(|r| r.tag_name);
        rows.push((name, installed, latest));
    }
    if json {
        let out: Vec<_> = rows.iter().map(|(n, i, l)| serde_json::json!({"plugin": n, "installed": i, "latest": l})).collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }
    let table: Vec<Vec<String>> = rows
        .into_iter()
        .map(|(name, installed, latest)| {
            let state = match (&installed, &latest) {
                (Some(i), Some(l)) if i == l => "up to date".to_string(),
                (Some(_), Some(_)) => format!("run `avm plugin update {name}`"),
                (Some(_), None) => "couldn't reach GitHub (set GITHUB_TOKEN if rate-limited)".to_string(),
                _ => "not from the marketplace".to_string(),
            };
            vec![name, installed.unwrap_or_else(|| "-".into()), latest.unwrap_or_else(|| "-".into()), state]
        })
        .collect();
    print_table(&["PLUGIN", "INSTALLED", "LATEST", ""], &table);
    Ok(())
}

// ---- #27 prune ----

fn tracked_path() -> Result<PathBuf> {
    Ok(shims::avm_home()?.join("tracked-configs.json"))
}

fn tracked_dirs() -> BTreeSet<String> {
    tracked_path().ok().and_then(|p| fs::read(p).ok()).and_then(|raw| serde_json::from_slice(&raw).ok()).unwrap_or_default()
}

/// Remember a directory whose config pins tools, so `avm prune` keeps them.
/// Called on every resolution, so it only writes when the directory is new.
pub fn track_config_dir(dir: &Path) {
    let key = dir.to_string_lossy().into_owned();
    let mut dirs = tracked_dirs();
    if dirs.insert(key) {
        if let Ok(path) = tracked_path() {
            let _ = path.parent().map(fs::create_dir_all);
            let _ = fs::write(path, serde_json::to_vec_pretty(&dirs).unwrap_or_default());
        }
    }
}

/// Shims mark `<version>/.last_used`, at most once a day.
pub fn mark_used(executable: &Path) {
    let Some(version_dir) = executable.parent().and_then(Path::parent) else { return };
    let marker = version_dir.join(".last_used");
    let fresh = fs::metadata(&marker)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < Duration::from_secs(24 * 60 * 60));
    if !fresh {
        let _ = fs::write(marker, b"");
    }
}

fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else { return 0 };
    if !meta.is_dir() {
        return meta.len();
    }
    fs::read_dir(path).into_iter().flatten().flatten().map(|e| dir_size(&e.path())).sum()
}

fn human(bytes: u64) -> String {
    let mb = bytes as f64 / 1_048_576.0;
    if mb >= 1024.0 { format!("{:.1} GB", mb / 1024.0) } else { format!("{mb:.0} MB") }
}

fn parse_days(spec: &str) -> Result<u64> {
    spec.strip_suffix('d')
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| anyhow!("--older-than takes days, like 90d"))
}

/// Global npm packages in a node version's bin (anything beyond node's own tools).
fn global_packages(tool: &str, version_dir: &Path) -> Vec<String> {
    if tool != "node" {
        return Vec::new();
    }
    let own = ["node", "npm", "npx", "corepack", "nodevars", "install_tools"];
    let mut pkgs: BTreeSet<String> = BTreeSet::new();
    for entry in fs::read_dir(version_dir.join("bin")).into_iter().flatten().flatten() {
        let path = entry.path();
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if !own.contains(&stem.as_str()) && !stem.starts_with('.') && path.extension().is_none_or(|e| e != "ps1") {
            pkgs.insert(stem);
        }
    }
    pkgs.into_iter().collect()
}

pub fn cmd_prune(older_than: Option<String>, include_unrecorded: bool, dry_run: bool, yes: bool) -> Result<()> {
    let home = home_dir()?;
    let cutoff = older_than.as_deref().map(parse_days).transpose()?.map(|d| Duration::from_secs(d * 24 * 60 * 60));

    // Versions still wanted: global pins, plus pins of every tracked project that still exists.
    let mut keep: HashSet<(String, String)> = config::load(&home)?.tools.into_iter().collect();
    let mut live_dirs = BTreeSet::new();
    for dir in tracked_dirs() {
        let path = PathBuf::from(&dir);
        if path.is_dir() {
            if let Ok(cfg) = crate::resolver::load(&path, &home, HashMap::new()) {
                keep.extend(cfg.local_tools);
            }
            live_dirs.insert(dir);
        }
    }

    let tools_root = shims::avm_home()?.join("tools");
    let mut doomed = Vec::new();
    let mut unrecorded = 0;
    for tool in fs::read_dir(&tools_root).into_iter().flatten().flatten() {
        let tool_name = tool.file_name().to_string_lossy().into_owned();
        for version in fs::read_dir(tool.path()).into_iter().flatten().flatten() {
            let name = version.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || !version.path().is_dir() || keep.contains(&(tool_name.clone(), name.clone())) {
                continue;
            }
            let last = fs::metadata(version.path().join(".last_used")).and_then(|m| m.modified()).ok();
            // avm only learns which projects use a version as it runs, so a version
            // it has never seen used may still be needed. Don't guess unless asked.
            if last.is_none() && !include_unrecorded {
                unrecorded += 1;
                continue;
            }
            if let Some(cutoff) = cutoff {
                let age = last.or_else(|| version.metadata().and_then(|m| m.modified()).ok()).and_then(|t| t.elapsed().ok());
                if age.is_some_and(|a| a < cutoff) {
                    continue;
                }
            }
            doomed.push((tool_name.clone(), name, version.path(), last));
        }
    }
    doomed.sort();

    let skipped_note = || {
        if unrecorded > 0 {
            println!(
                "Skipped {unrecorded} version(s) avm hasn't seen used yet; add --include-unrecorded to consider them (check the list with --dry-run first)."
            );
        }
    };
    if doomed.is_empty() {
        println!("Nothing to prune.");
        skipped_note();
        return Ok(());
    }
    let mut total = 0;
    for (tool, version, path, last) in &doomed {
        let size = dir_size(path);
        total += size;
        let used = last
            .map(|t| {
                let days = SystemTime::now().duration_since(t).map(|d| d.as_secs() / 86_400).unwrap_or(0);
                format!("last used {days}d ago")
            })
            .unwrap_or_else(|| "no recorded use".to_string());
        println!("  {tool} {version}  {}  ({used})", human(size));
        let pkgs = global_packages(tool, path);
        if !pkgs.is_empty() {
            println!("    warning: holds global packages ({}); other versions fall back to them", pkgs.join(", "));
        }
    }
    println!("{} version(s), {} to reclaim.", doomed.len(), human(total));
    skipped_note();
    println!("Kept: global pins and pins of {} project(s) avm has seen (it records them as you use them).", live_dirs.len());
    if dry_run || !confirm("Remove them?", yes)? {
        return Ok(());
    }
    for (tool, version, path, _) in doomed {
        fs::remove_dir_all(&path).with_context(|| format!("failed to remove {tool} {version}"))?;
        println!("✓ Removed {tool} {version}");
    }
    // Forget projects that no longer exist.
    if let Ok(path) = tracked_path() {
        let _ = fs::write(path, serde_json::to_vec_pretty(&live_dirs).unwrap_or_default());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_helpers() {
        assert_eq!(parse_days("90d").unwrap(), 90);
        assert!(parse_days("90").is_err() && parse_days("3w").is_err());
        assert_eq!(human(5 * 1_048_576), "5 MB");
        assert_eq!(human(3 * 1024 * 1_048_576), "3.0 GB");
    }
}

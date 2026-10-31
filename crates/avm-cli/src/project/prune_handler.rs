//! `avm plugin prune`, plus the `.last_used` marker shims write.
use crate::common::state::home_dir;
use crate::common::ui;
use crate::constants::msg::{self, fill};
use crate::core::tracked;
use crate::{config, shims};
use ::fs_err as fs;
use anyhow::{anyhow, Context, Result};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Config and plugin-env caches are one file per directory or plugin build and are
/// never read again once stale; drop the ones untouched for a month.
fn prune_stale_caches() {
    let Ok(dir) = shims::avm_home().map(|home| home.join("cache")) else {
        return;
    };
    let month = Duration::from_secs(30 * 24 * 60 * 60);
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let stale = (name.starts_with("config-") || name.starts_with("env-"))
            && entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > month);
        if stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Shims mark `<version>/.last_used`, at most once a day.
pub fn mark_used(executable: &Path) {
    let Some(version_dir) = executable.parent().and_then(Path::parent) else {
        return;
    };
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

/// Total size in bytes of `path`, recursing into directories; unreadable entries count as 0 and symlinks are not followed.
fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| dir_size(&e.path()))
        .sum()
}

/// Formats a byte count as MB, or GB from 1024 MB up.
fn human(bytes: u64) -> String {
    let mb = bytes as f64 / 1_048_576.0;
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{mb:.0} MB")
    }
}

/// Parses an `--older-than` value such as `90d` into days; errors on any other form.
fn parse_days(spec: &str) -> Result<u64> {
    spec.strip_suffix('d')
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| anyhow!(msg::PRUNE_DAYS_FORMAT))
}

/// Global npm packages in a node version's bin (anything beyond node's own tools).
fn global_packages(tool: &str, version_dir: &Path) -> Vec<String> {
    if tool != "node" {
        return Vec::new();
    }
    let own = [
        "node",
        "npm",
        "npx",
        "corepack",
        "nodevars",
        "install_tools",
    ];
    let mut pkgs: BTreeSet<String> = BTreeSet::new();
    for entry in fs::read_dir(version_dir.join("bin"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let path = entry.path();
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !own.contains(&stem.as_str())
            && !stem.starts_with('.')
            && path.extension().is_none_or(|e| e != "ps1")
        {
            pkgs.insert(stem);
        }
    }
    pkgs.into_iter().collect()
}

/// `avm plugin prune`: removes installed tool versions that no global pin or tracked project still wants.
/// `older_than` (days) spares versions used more recently, `include_unrecorded` also removes versions with no
/// usage record, `dry_run` only reports and `yes` skips the confirmation.
pub fn cmd_prune(
    older_than: Option<String>,
    include_unrecorded: bool,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    let home = home_dir()?;
    if !dry_run {
        prune_stale_caches();
    }
    let tracked = tracked::path()?;
    // Not locked while resolving or asking: every shim and prompt takes that lock to track its project.
    let tracked_snapshot = tracked::dirs()?;
    let cutoff = older_than
        .as_deref()
        .map(parse_days)
        .transpose()?
        .map(|d| Duration::from_secs(d * 24 * 60 * 60));

    // Versions still wanted: global pins, plus pins of every tracked project that still exists.
    let mut keep: HashSet<(String, String)> = config::load(&home)?
        .tools
        .into_iter()
        .flat_map(|(tool, spec)| {
            [
                (tool.clone(), crate::version_files::resolve(&tool, &spec)),
                (tool, spec),
            ]
        })
        .collect();
    let mut live_dirs = BTreeSet::new();
    for dir in &tracked_snapshot {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            let cfg = crate::resolver::load(&path, &home, HashMap::new()).with_context(|| {
                fill(
                    msg::PRUNE_RESOLVE_FAILED,
                    &[("path", &path.display().to_string())],
                )
            })?;
            keep.extend(
                cfg.local_tools
                    .into_iter()
                    .chain(cfg.global_tools)
                    .flat_map(|(tool, spec)| {
                        [
                            (tool.clone(), crate::version_files::resolve(&tool, &spec)),
                            (tool, spec),
                        ]
                    }),
            );
            // Keep pins in inactive profiles too: switching AVM_ENV must not
            // make prune delete a runtime another profile still selects.
            for ancestor in path.ancestors().take_while(|p| *p != home) {
                let mut files: Vec<_> = fs::read_dir(ancestor)?
                    .map(|e| e.map(|e| e.path()))
                    .collect::<io::Result<_>>()?;
                files.sort();
                for file in files.into_iter().filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with(".avm.") && n.ends_with(".json"))
                }) {
                    for (tool, spec) in config::load_file(&file)?.tools {
                        avm_plugin_sdk::validate_name(&tool)?;
                        avm_plugin_sdk::validate_version_spec(&spec)?;
                        keep.insert((tool.clone(), crate::version_files::resolve(&tool, &spec)));
                    }
                }
                if config::load(ancestor)?.root {
                    break;
                }
            }
            live_dirs.insert(dir.clone());
        }
    }

    let tools_root = shims::avm_home()?.join("tools");
    let mut doomed = Vec::new();
    let mut unrecorded = 0;
    for tool in fs::read_dir(&tools_root).into_iter().flatten().flatten() {
        let tool_name = tool.file_name().to_string_lossy().into_owned();
        for version in fs::read_dir(tool.path()).into_iter().flatten().flatten() {
            let name = version.file_name().to_string_lossy().into_owned();
            if name.starts_with('.')
                || !version.path().is_dir()
                || keep.contains(&(tool_name.clone(), name.clone()))
            {
                continue;
            }
            let last = fs::metadata(version.path().join(".last_used"))
                .and_then(|m| m.modified())
                .ok();
            // avm only learns which projects use a version as it runs, so a version
            // it has never seen used may still be needed. Don't guess unless asked.
            if last.is_none() && !include_unrecorded {
                unrecorded += 1;
                continue;
            }
            if let Some(cutoff) = cutoff {
                let age = last
                    .or_else(|| version.metadata().and_then(|m| m.modified()).ok())
                    .and_then(|t| t.elapsed().ok());
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
            ui::info(fill(
                msg::PRUNE_SKIPPED,
                &[("count", &unrecorded.to_string())],
            ));
        }
    };
    if doomed.is_empty() {
        ui::info(msg::PRUNE_NOTHING);
        skipped_note();
        return Ok(());
    }
    let mut total = 0;
    for (tool, version, path, last) in &doomed {
        let size = dir_size(path);
        total += size;
        let used = last
            .map(|t| {
                let days = SystemTime::now()
                    .duration_since(t)
                    .map(|d| d.as_secs() / 86_400)
                    .unwrap_or(0);
                fill(msg::PRUNE_LAST_USED, &[("days", &days.to_string())])
            })
            .unwrap_or_else(|| msg::PRUNE_NO_USE.to_string());
        let entry = [
            ("tool", tool.as_str()),
            ("version", version),
            ("size", &human(size)),
            ("used", &used),
        ];
        ui::info(fill(msg::PRUNE_ENTRY, &entry));
        let pkgs = global_packages(tool, path);
        if !pkgs.is_empty() {
            ui::info(fill(
                msg::PRUNE_GLOBAL_PACKAGES,
                &[("packages", &pkgs.join(", "))],
            ));
        }
    }
    ui::info(fill(
        msg::PRUNE_SUMMARY,
        &[
            ("count", &doomed.len().to_string()),
            ("size", &human(total)),
        ],
    ));
    skipped_note();
    ui::info(fill(
        msg::PRUNE_KEPT,
        &[("count", &live_dirs.len().to_string())],
    ));
    if dry_run || !ui::confirm(msg::PRUNE_CONFIRM, yes)? {
        return Ok(());
    }
    // A project tracked while we waited may need a version we're about to delete.
    let _guard = avm_plugin_sdk::storage::lock(&tracked)?;
    if tracked::dirs()? != tracked_snapshot {
        return Err(anyhow!(msg::PRUNE_RACE));
    }
    for (tool, version, path, _) in doomed {
        let vars = [("tool", tool.as_str()), ("version", version.as_str())];
        fs::remove_dir_all(&path).with_context(|| fill(msg::PRUNE_REMOVE_FAILED, &vars))?;
        ui::success(fill(msg::PRUNE_REMOVED, &vars));
    }
    // Forget projects that no longer exist.
    tracked::replace(&live_dirs)?;
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

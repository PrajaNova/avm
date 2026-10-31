use crate::constants::msg::{self, fill};
use crate::resolver::ResolvedConfig;
use anyhow::{anyhow, bail, Context, Result};
use avm_plugin_sdk::{
    download::{validate_asset, validate_platform, validate_version},
    protocol::LockedAsset,
    ToolProvider, ToolVersionQuery,
};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

pub const LOCK_FILE: &str = ".avm.lock";

#[derive(Debug, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: u32,
    pub tools: BTreeMap<String, LockedTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedTool {
    pub requested: String,
    pub resolved: String,
    pub plugin: PluginPin,
    pub assets: BTreeMap<String, LockedAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPin {
    pub name: String,
    pub version: String,
    // shortcut: only host executable hashes are available; fetch other platform binaries when plugin artifact pinning is added.
    pub binaries: BTreeMap<String, String>,
}

/// The spec each local tool was requested with (its version-file/config spec when known, else its pinned version), sorted by tool.
pub fn requested_tools(cfg: &ResolvedConfig) -> BTreeMap<String, String> {
    cfg.local_tools
        .iter()
        .map(|(tool, version)| {
            (
                tool.clone(),
                cfg.tool_specs.get(tool).unwrap_or(version).clone(),
            )
        })
        .collect()
}

/// The nearest `.avm.lock`'s entry for `tool`, kept only if `matches` accepts it.
pub fn entry_for(tool: &str, matches: impl Fn(&LockedTool) -> bool) -> Result<Option<LockedTool>> {
    Ok(load(&std::env::current_dir()?)?
        .and_then(|mut lock| lock.tools.remove(tool))
        .filter(matches))
}

/// Find the nearest `.avm.lock` walking up from `dir` and validate it (version, names, resolved versions satisfy requests, plugin and asset shape).
/// Returns `None` when no lockfile exists; errors on unreadable, malformed, or inconsistent files.
pub fn load(dir: &Path) -> Result<Option<Lockfile>> {
    let mut found = None;
    for ancestor in dir.ancestors() {
        let path = ancestor.join(LOCK_FILE);
        match fs::read(&path) {
            Ok(raw) => {
                found = Some((path, raw));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e).context(msg::LOCK_READ_FAILED),
        }
    }
    let Some((path, raw)) = found else {
        return Ok(None);
    };
    let lock: Lockfile = serde_json::from_slice(&raw)
        .with_context(|| fill(msg::LOCK_INVALID, &[("path", &path.display().to_string())]))?;
    if lock.version != 1 {
        bail!(fill(
            msg::LOCK_UNSUPPORTED_VERSION,
            &[("version", &lock.version.to_string())]
        ));
    }
    for (tool, entry) in &lock.tools {
        validate_tool(tool)?;
        validate_version(&entry.resolved)?;
        if entry.requested != "latest"
            && entry.requested != "lts"
            && !entry.requested.starts_with("lts/")
            && crate::version_files::pick(tool, &entry.requested, vec![entry.resolved.clone()])
                .as_ref()
                != Some(&entry.resolved)
        {
            bail!(unsatisfied(tool, entry));
        }
        if let Some((_, build)) = entry.requested.split_once('+') {
            if entry.resolved.split_once('+').map(|(_, b)| b) != Some(build) {
                bail!(unsatisfied(tool, entry));
            }
        }
        if entry.plugin.name != *tool || entry.plugin.version.is_empty() || entry.assets.is_empty()
        {
            bail!(fill(msg::LOCK_INVALID_ENTRY, &[("tool", tool)]));
        }
        for (platform, hash) in &entry.plugin.binaries {
            validate_platform(platform)?;
            if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
                bail!(fill(msg::LOCK_INVALID_HASH, &[("tool", tool)]));
            }
        }
        for (platform, asset) in &entry.assets {
            validate_platform(platform)?;
            validate_asset(asset)?;
        }
    }
    Ok(Some(lock))
}

/// The error for a locked version that no longer satisfies what the lock requested.
fn unsatisfied(tool: &str, entry: &LockedTool) -> String {
    fill(
        msg::LOCK_UNSATISFIED,
        &[
            ("tool", tool),
            ("resolved", &entry.resolved),
            ("requested", &entry.requested),
        ],
    )
}

/// Reject empty tool names or names with characters other than ASCII alphanumerics, `-` and `_`.
pub fn validate_tool(tool: &str) -> Result<()> {
    if tool.is_empty()
        || !tool
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        bail!(fill(msg::TOOL_INVALID_NAME, &[("tool", tool)]));
    }
    Ok(())
}

impl Lockfile {
    /// Error unless the lock covers exactly the tools requested by `cfg` with identical specs.
    pub fn validate_config(&self, cfg: &ResolvedConfig) -> Result<()> {
        let requested = requested_tools(cfg);
        if requested.len() != self.tools.len()
            || requested.iter().any(|(tool, spec)| {
                self.tools
                    .get(tool)
                    .is_none_or(|entry| entry.requested != *spec)
            })
        {
            bail!(msg::LOCK_OUT_OF_DATE);
        }
        Ok(())
    }

    /// Replace each locally pinned tool version in `cfg` with the locked resolved version, after checking the lock is current.
    pub fn apply(&self, cfg: &mut ResolvedConfig) -> Result<()> {
        self.validate_config(cfg)?;
        for (tool, entry) in &self.tools {
            if let Some(version) = cfg.local_tools.get_mut(tool) {
                *version = entry.resolved.clone();
            }
        }
        Ok(())
    }

    /// Write the lockfile as pretty JSON with a trailing newline into `dir`, atomically.
    pub fn save(&self, dir: &Path) -> Result<()> {
        let mut raw = serde_json::to_vec_pretty(self)?;
        raw.push(b'\n');
        avm_plugin_sdk::storage::atomic_write(&dir.join(LOCK_FILE), &raw)
    }
}

/// Use the existing version-range matcher over the provider's release list.
pub fn resolve_spec(provider: &dyn ToolProvider, spec: &str) -> Result<String> {
    if spec.trim().is_empty() {
        bail!(fill(
            msg::LOCK_VERSION_REQUIRED,
            &[("tool", provider.name())]
        ));
    }
    let numeric = crate::version_files::numbers(spec);
    let literal = validate_version(spec).is_ok();
    // Exact pins need no release index, including historical asdf versions.
    if literal && (spec.contains('+') || numeric.as_ref().is_some_and(|n| n.len() >= 3)) {
        return Ok(if provider.name() == "node" {
            spec.trim_start_matches('v')
        } else {
            spec
        }
        .to_string());
    }
    let query = if literal {
        numeric
            .as_ref()
            .and_then(|n| n.first())
            .map(|m| ToolVersionQuery::Major(*m))
            .unwrap_or(ToolVersionQuery::Recent)
    } else {
        ToolVersionQuery::Recent
    };
    let releases = provider.available_versions(query)?;
    let range = if spec == "latest" { "*" } else { spec };
    let mut candidates: Vec<String> = if spec == "lts" || spec.starts_with("lts/") {
        let channel = spec.strip_prefix("lts/").filter(|c| *c != "*");
        releases
            .into_iter()
            .filter(|v| {
                v.is_lts
                    && channel.is_none_or(|c| {
                        v.channel
                            .as_deref()
                            .is_some_and(|name| name.eq_ignore_ascii_case(c))
                    })
            })
            .map(|v| v.version)
            .collect()
    } else {
        releases.into_iter().map(|v| v.version).collect()
    };
    if let Some((_, build)) = spec.split_once('+') {
        candidates.retain(|v| v.split_once('+').map(|(_, b)| b) == Some(build));
    }
    let range = if spec == "lts" || spec.starts_with("lts/") {
        "*"
    } else {
        range
    };
    // Preserve exact vendor/build strings when the provider publishes them.
    let version = if candidates.iter().any(|v| v == spec) {
        spec.to_string()
    } else {
        crate::version_files::pick(provider.name(), range, candidates).ok_or_else(|| {
            anyhow!(fill(
                msg::LOCK_NO_MATCH,
                &[("tool", provider.name()), ("spec", spec)]
            ))
        })?
    };
    validate_version(&version)?;
    Ok(version)
}

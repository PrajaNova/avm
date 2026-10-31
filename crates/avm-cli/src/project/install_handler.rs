use crate::common::state::home_dir;
use crate::common::ui;
use crate::constants::msg::{self, fill};
use crate::global::provider_by_name;
use crate::lockfile::{LockedTool, Lockfile, PluginPin};
use crate::plugins::PluginRegistry;
use crate::resolver::ResolvedConfig;
use crate::{config, lockfile, plugins as runtime, shims};
use ::fs_err as fs;
use anyhow::{anyhow, Context, Result};
use avm_plugin_sdk::download::{
    host_platform, validate_asset, validate_platform, validate_version,
};
use avm_plugin_sdk::protocol::LockedAsset;
use avm_plugin_sdk::ToolProvider;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Loads the project config without applying `.avm.lock`, so locked versions do not mask what the config requests.
fn unlocked_state() -> Result<ResolvedConfig> {
    crate::resolver::load_unlocked(&std::env::current_dir()?, &home_dir()?, HashMap::new())
}

/// Records the plugin's name, version and executable checksum for `platform`, as stored in the lockfile.
fn plugin_pin(process: &runtime::PluginProcess, platform: &str) -> Result<PluginPin> {
    let manifest = process.manifest()?;
    Ok(PluginPin {
        name: manifest.name,
        version: manifest.version,
        binaries: BTreeMap::from([(
            platform.to_string(),
            avm_plugin_sdk::sha256_file(process.executable())?,
        )]),
    })
}

/// `avm plugin lock`: resolves the project's tools and writes `.avm.lock` for the host plus any extra `platforms`.
/// Prints a hint instead when no tools are configured.
pub fn cmd_lock(platforms: Vec<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let home = home_dir()?;
    let root = lock_root(&cwd, &home)?;
    let cfg = crate::resolver::load_unlocked(&root, &home, HashMap::new())?;
    if lockfile::requested_tools(&cfg).is_empty() {
        ui::info(msg::LOCK_NOTHING);
        return Ok(());
    }
    write_lock(&root, &cfg, platforms)
}

/// Resolve every project tool and write `<root>/.avm.lock`.
fn write_lock(root: &Path, cfg: &ResolvedConfig, mut platforms: Vec<String>) -> Result<()> {
    let host = host_platform()?;
    if platforms.is_empty() {
        platforms.push(host.clone());
    }
    platforms.sort();
    platforms.dedup();
    for platform in &platforms {
        validate_platform(platform)?;
    }
    let manager = PluginRegistry::new()?;
    let mut lock = Lockfile {
        version: 1,
        tools: BTreeMap::new(),
    };
    for (tool, requested) in lockfile::requested_tools(cfg) {
        lockfile::validate_tool(&tool)?;
        let provider = manager
            .protocol_provider(&tool)
            .ok_or_else(|| anyhow!(fill(msg::LOCK_LEGACY_PLUGIN, &[("tool", &tool)])))?;
        let resolved = lockfile::resolve_spec(&provider, &requested)?;
        let downloads = provider
            .resolve_downloads(&resolved, &platforms)
            .with_context(|| fill(msg::LOCK_CANNOT_DOWNLOADS, &[("tool", &tool)]))?;
        if downloads.version != resolved {
            return Err(anyhow!(fill(
                msg::LOCK_RESOLVE_VERSION,
                &[
                    ("tool", &tool),
                    ("returned", &downloads.version),
                    ("resolved", &resolved)
                ]
            )));
        }
        if downloads.assets.keys().cloned().collect::<Vec<_>>() != platforms {
            return Err(anyhow!(fill(
                msg::LOCK_RESOLVE_PLATFORMS,
                &[("tool", &tool)]
            )));
        }
        for platform in &platforms {
            validate_asset(downloads.assets.get(platform).ok_or_else(|| {
                anyhow!(fill(
                    msg::LOCK_NO_DOWNLOAD,
                    &[("tool", &tool), ("platform", platform)]
                ))
            })?)?;
        }
        let plugin = plugin_pin(&provider, &host)?;
        if plugin.name != tool || plugin.version.is_empty() {
            return Err(anyhow!(fill(msg::LOCK_BAD_MANIFEST, &[("tool", &tool)])));
        }
        lock.tools.insert(
            tool,
            LockedTool {
                requested,
                resolved,
                plugin,
                assets: downloads.assets,
            },
        );
    }
    lock.save(root)?;
    let count = lock.tools.len().to_string();
    let path = root.join(lockfile::LOCK_FILE).display().to_string();
    ui::info(fill(
        msg::LOCK_DONE,
        &[
            ("count", &count),
            ("path", &path),
            ("platforms", &platforms.join(", ")),
        ],
    ));
    Ok(())
}

/// Finds the nearest ancestor of `cwd` holding a lockfile or project config (ignoring the home directory's config); falls back to `cwd`.
/// Errors if a candidate exists but is not a file.
fn lock_root(cwd: &Path, home: &Path) -> Result<PathBuf> {
    let names = config::file_names(&std::env::var("AVM_ENV").unwrap_or_default())?;
    for dir in cwd.ancestors() {
        let files = std::iter::once(lockfile::LOCK_FILE)
            .chain(names.iter().filter(|_| dir != home).map(String::as_str));
        for file in files {
            match fs::metadata(dir.join(file)) {
                Ok(meta) if meta.is_file() => return Ok(dir.to_path_buf()),
                Ok(_) => {
                    let path = dir.join(file).display().to_string();
                    return Err(anyhow!(fill(msg::LOCK_NOT_A_FILE, &[("path", &path)])));
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => return Err(err).context(msg::LOCK_FIND_DIR),
            }
        }
    }
    Ok(cwd.to_path_buf())
}

/// `avm install`: installs the tools in `.avm.lock`, locking first when allowed.
/// With `frozen`, errors rather than creating or changing the lockfile.
pub fn cmd_install(frozen: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cfg = unlocked_state()?;
    let project_tools = !lockfile::requested_tools(&cfg).is_empty();
    if frozen && !project_tools {
        ui::info(msg::INSTALL_NOTHING_FROZEN);
        return Ok(());
    }
    let mut lock = lockfile::load(&cwd)?;
    if frozen && lock.is_none() {
        return Err(anyhow!(msg::INSTALL_FROZEN_NO_LOCK));
    }
    if let Some(current) = &lock {
        if let Err(err) = current.validate_config(&cfg) {
            if frozen {
                return Err(err);
            }
            ui::error(fill(
                msg::INSTALL_STALE_LOCK,
                &[("err", &format!("{err:#}"))],
            ));
            lock = None;
        }
    }
    if !project_tools {
        ui::info(msg::INSTALL_NO_TOOLS);
    }
    let platform = host_platform()?;
    let mut plan = Vec::new();
    let mut requested = lockfile::requested_tools(&cfg);
    if lock.is_none() {
        for (tool, spec) in &cfg.global_tool_specs {
            requested
                .entry(tool.clone())
                .or_insert_with(|| spec.clone());
        }
    }
    // Validate the whole plan before downloading or installing any tool.
    for (tool, requested) in requested {
        lockfile::validate_tool(&tool)?;
        let provider = provider_by_name(&tool)?;
        let (version, asset) = if let Some(lock) = &lock {
            let entry = lock
                .tools
                .get(&tool)
                .ok_or_else(|| anyhow!(fill(msg::INSTALL_MISSING_FROM_LOCK, &[("tool", &tool)])))?;
            (
                entry.resolved.clone(),
                Some(locked_asset(&tool, entry, &platform)?),
            )
        } else {
            let version = if provider.is_installed(&requested) {
                requested.clone()
            } else {
                lockfile::resolve_spec(provider.as_ref(), &requested)?
            };
            (version, None)
        };
        validate_version(&version)?;
        plan.push((tool, version, provider, asset));
    }
    // Android's sdkmanager requires a JDK; install Java first when selected.
    plan.sort_by(|a, b| (a.0 != "java", &a.0).cmp(&(b.0 != "java", &b.0)));
    for (tool, version, provider, asset) in plan {
        let global = !project_tools || !cfg.local_tools.contains_key(&tool);
        let label = if global { " (global)" } else { "" };
        let vars = [
            ("tool", tool.as_str()),
            ("version", &version),
            ("label", label),
        ];
        if provider.is_installed(&version) {
            ui::success(fill(msg::INSTALL_ALREADY, &vars));
        } else {
            if let Some(asset) = asset {
                provider.install_locked(&version, &asset)?;
            } else {
                provider.install(&version)?;
            }
            if !provider.is_installed(&version) {
                return Err(anyhow!(fill(msg::INSTALL_NOT_INSTALLED, &vars)));
            }
            ui::success(fill(msg::INSTALL_DONE, &vars));
        }
    }
    shims::reshim()?;
    if project_tools && lock.is_none() {
        // Best-effort: an old or asdf plugin must not turn a good install into a failure.
        let root = lock_root(&cwd, &home_dir()?)?;
        if let Err(err) = write_lock(&root, &cfg, Vec::new()) {
            ui::error(fill(
                msg::INSTALL_LOCK_WRITE_FAILED,
                &[("err", &format!("{err:#}"))],
            ));
        }
    }
    Ok(())
}

/// After a project pin changes, regenerate an existing `.avm.lock` so it never goes stale.
pub fn refresh_lock() {
    let refresh = || -> Result<()> {
        let home = home_dir()?;
        let root = lock_root(&std::env::current_dir()?, &home)?;
        let Some(old) = lockfile::load(&root)? else {
            return Ok(());
        };
        let mut platforms: Vec<String> = old
            .tools
            .values()
            .flat_map(|tool| tool.assets.keys().cloned())
            .collect();
        platforms.sort();
        platforms.dedup();
        let cfg = crate::resolver::load_unlocked(&root, &home, HashMap::new())?;
        write_lock(&root, &cfg, platforms)
    };
    if let Err(err) = refresh() {
        ui::error(fill(
            msg::INSTALL_REFRESH_FAILED,
            &[("err", &format!("{err:#}"))],
        ));
    }
}

/// Checks the installed plugin against the lockfile entry for `tool` and returns the locked asset to install.
/// Errors when the plugin is missing or its version or checksum differs.
fn locked_asset(tool: &str, entry: &LockedTool, platform: &str) -> Result<LockedAsset> {
    let process = PluginRegistry::new()?
        .protocol_provider(tool)
        .ok_or_else(|| anyhow!(fill(msg::INSTALL_NO_NATIVE_PLUGIN, &[("tool", tool)])))?;
    let installed = plugin_pin(&process, platform)?;
    if installed.name != entry.plugin.name || installed.version != entry.plugin.version {
        return Err(anyhow!(fill(
            msg::INSTALL_PLUGIN_DIFFERS,
            &[("tool", tool), ("version", &entry.plugin.version)]
        )));
    }
    if let Some(expected) = entry.plugin.binaries.get(platform) {
        if installed
            .binaries
            .get(platform)
            .is_none_or(|actual| !actual.eq_ignore_ascii_case(expected))
        {
            return Err(anyhow!(fill(
                msg::INSTALL_CHECKSUM_DIFFERS,
                &[("tool", tool)]
            )));
        }
    } else {
        ui::warn(fill(
            msg::INSTALL_PLUGIN_UNPINNED,
            &[("tool", tool), ("platform", platform)],
        ));
    }
    entry.assets.get(platform).cloned().ok_or_else(|| {
        anyhow!(fill(
            msg::INSTALL_NO_LOCKED_DOWNLOAD,
            &[("tool", tool), ("platform", platform)]
        ))
    })
}

/// Existing per-tool install/use commands must verify locked selections too.
pub fn install_provider_version(provider: &dyn ToolProvider, version: &str) -> Result<()> {
    match lockfile::entry_for(provider.name(), |entry| entry.resolved == version)? {
        Some(entry) => {
            let asset = locked_asset(provider.name(), &entry, &host_platform()?)?;
            provider.install_locked(version, &asset)
        }
        None => provider.install(version),
    }
}

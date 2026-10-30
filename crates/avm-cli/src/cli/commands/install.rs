use crate::cli::commands::provider_by_name;
use crate::cli::state::home_dir;
use crate::plugins::PluginManager;
use crate::resolver::ResolvedConfig;
use crate::{config, plugins as runtime, shims};
use anyhow::{anyhow, Context, Result};
use avm_plugin_api::ToolProvider;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use crate::lockfile::{self, LockedTool, Lockfile, PluginPin};
use avm_plugin_api::{
    download::{host_platform, validate_asset, validate_platform, validate_version},
    protocol::LockedAsset,
};

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
            avm_plugin_api::sha256_file(process.executable())?,
        )]),
    })
}

/// `avm lock`: resolves the project's tools and writes `.avm.lock` for the host plus any extra `platforms`.
/// Prints a hint instead when no tools are configured.
pub fn cmd_lock(platforms: Vec<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let home = home_dir()?;
    let root = lock_root(&cwd, &home)?;
    let cfg = crate::resolver::load_unlocked(&root, &home, HashMap::new())?;
    if lockfile::requested_tools(&cfg).is_empty() {
        println!("Nothing to lock. Add a tool first: avm node use 20");
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
    let manager = PluginManager::new()?;
    let mut lock = Lockfile {
        version: 1,
        tools: BTreeMap::new(),
    };
    for (tool, requested) in lockfile::requested_tools(cfg) {
        lockfile::validate_tool(&tool)?;
        let provider = manager.protocol_provider(&tool)
            .ok_or_else(|| anyhow!("{tool} requires a native plugin with lockfile support; legacy asdf plugins can use `avm -p install` without a lockfile"))?;
        let resolved = lockfile::resolve_spec(&provider, &requested)?;
        let downloads = provider
            .resolve_downloads(&resolved, &platforms)
            .with_context(|| {
                format!(
                    "{tool} cannot lock downloads; its plugin is too old, run `avm plugin install {tool}`"
                )
            })?;
        if downloads.version != resolved {
            return Err(anyhow!(
                "{tool} resolve returned {} instead of {resolved}",
                downloads.version
            ));
        }
        if downloads.assets.keys().cloned().collect::<Vec<_>>() != platforms {
            return Err(anyhow!(
                "{tool} resolve returned downloads for a different set of platforms"
            ));
        }
        for platform in &platforms {
            validate_asset(
                downloads
                    .assets
                    .get(platform)
                    .ok_or_else(|| anyhow!("{tool} did not return a download for {platform}"))?,
            )?;
        }
        let plugin = plugin_pin(&provider, &host)?;
        if plugin.name != tool || plugin.version.is_empty() {
            return Err(anyhow!("invalid plugin manifest for {tool}"));
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
    println!(
        "✓ Locked {} tools in {} ({})",
        lock.tools.len(),
        root.join(lockfile::LOCK_FILE).display(),
        platforms.join(", ")
    );
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
                Ok(_) => return Err(anyhow!("{} is not a file", dir.join(file).display())),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => return Err(err).context("find project lock directory"),
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
        println!("Nothing to install: no tools in .avm.json.");
        return Ok(());
    }
    let mut lock = lockfile::load(&cwd)?;
    if frozen && lock.is_none() {
        return Err(anyhow!(
            "--frozen requires .avm.lock; run `avm lock` and commit it first"
        ));
    }
    if let Some(current) = &lock {
        if let Err(err) = current.validate_config(&cfg) {
            if frozen {
                return Err(err);
            }
            eprintln!("avm: {err:#}; installing from .avm.json and refreshing the lock");
            lock = None;
        }
    }
    if !project_tools {
        println!("No tools in .avm.json. Add one: avm node use 20");
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
                .ok_or_else(|| anyhow!("{tool} missing from .avm.lock"))?;
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
        if provider.is_installed(&version) {
            done!("{tool} {version} already installed{label}");
        } else {
            if let Some(asset) = asset {
                provider.install_locked(&version, &asset)?;
            } else {
                provider.install(&version)?;
            }
            if !provider.is_installed(&version) {
                return Err(anyhow!(
                    "{tool} {version} installer succeeded but the tool is not installed"
                ));
            }
            done!("Installed {tool} {version}{label}");
        }
    }
    shims::reshim()?;
    if project_tools && lock.is_none() {
        // Best-effort: an old or asdf plugin must not turn a good install into a failure.
        let root = lock_root(&cwd, &home_dir()?)?;
        if let Err(err) = write_lock(&root, &cfg, Vec::new()) {
            eprintln!("avm: installed, but could not write .avm.lock: {err:#}");
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
        eprintln!("avm: could not refresh .avm.lock ({err:#}); run `avm lock`");
    }
}

/// Checks the installed plugin against the lockfile entry for `tool` and returns the locked asset to install.
/// Errors when the plugin is missing or its version or checksum differs.
fn locked_asset(tool: &str, entry: &LockedTool, platform: &str) -> Result<LockedAsset> {
    let process = PluginManager::new()?
        .protocol_provider(tool)
        .ok_or_else(|| anyhow!("{tool} requires a native plugin for locked installs"))?;
    let installed = plugin_pin(&process, platform)?;
    if installed.name != entry.plugin.name || installed.version != entry.plugin.version {
        return Err(anyhow!(
            "{tool} plugin differs from .avm.lock; install plugin {} or regenerate with `avm lock`",
            entry.plugin.version
        ));
    }
    if let Some(expected) = entry.plugin.binaries.get(platform) {
        if installed
            .binaries
            .get(platform)
            .is_none_or(|actual| !actual.eq_ignore_ascii_case(expected))
        {
            return Err(anyhow!("{tool} plugin checksum differs from .avm.lock; restore the locked plugin or run `avm lock`"));
        }
    } else {
        warn!("{tool} plugin binary not pinned for {platform}; only plugin name and version are checked");
    }
    entry.assets.get(platform).cloned().ok_or_else(|| {
        anyhow!(
            ".avm.lock has no {tool} download for {platform}; run `avm lock --platform {platform}`"
        )
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

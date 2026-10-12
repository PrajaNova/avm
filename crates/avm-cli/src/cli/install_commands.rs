use super::*;
use crate::lockfile::{self, LockedTool, Lockfile, PluginPin};
use avm_plugin_api::{
    download::{
        download_verified, host_platform, validate_asset, validate_platform, validate_version,
    },
    protocol::LockedAsset,
};

fn unlocked_state() -> Result<ResolvedConfig> {
    crate::resolver::load_unlocked(&std::env::current_dir()?, &home_dir()?, HashMap::new())
}

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

pub fn cmd_lock(mut platforms: Vec<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cfg = unlocked_state()?;
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
    for (tool, requested) in lockfile::requested_tools(&cfg) {
        lockfile::validate_tool(&tool)?;
        let provider = manager.protocol_provider(&tool)
            .ok_or_else(|| anyhow!("{tool} requires a native plugin with lockfile support; legacy asdf plugins can use `avm install` without a lockfile"))?;
        let resolved = lockfile::resolve_spec(&provider, &requested)?;
        let downloads = provider
            .resolve_downloads(&resolved, &platforms)
            .with_context(|| {
                format!(
                    "{tool} cannot lock downloads; update its plugin to one supporting `resolve`"
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
    lock.save(&cwd)?;
    println!(
        "✓ Locked {} tools in .avm.lock ({})",
        lock.tools.len(),
        platforms.join(", ")
    );
    Ok(())
}

pub fn cmd_install(frozen: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cfg = unlocked_state()?;
    let lock = lockfile::load(&cwd)?;
    if frozen && lock.is_none() {
        return Err(anyhow!(
            "--frozen requires .avm.lock; run `avm lock` and commit it first"
        ));
    }
    if let Some(lock) = &lock {
        lock.validate_config(&cfg)?;
    }
    let platform = host_platform()?;
    let mut plan = Vec::new();
    // Validate the whole plan before downloading or installing any tool.
    for (tool, requested) in lockfile::requested_tools(&cfg) {
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
        if !provider.is_installed(&version) {
            if let Some(asset) = asset {
                install_locked(provider.as_ref(), &version, &asset)?;
            } else {
                provider.install(&version)?;
            }
            if !provider.is_installed(&version) {
                return Err(anyhow!(
                    "{tool} {version} installer succeeded but the tool is not installed"
                ));
            }
        }
        println!("✓ Installed {tool} {version}");
    }
    shims::reshim()?;
    Ok(())
}

fn install_locked(provider: &dyn ToolProvider, version: &str, asset: &LockedAsset) -> Result<()> {
    let staging = std::env::temp_dir().join(format!(
        "avm-locked-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    fs::create_dir(&staging).context("create locked download staging dir")?;
    let archive = staging.join("archive");
    let result = download_verified(asset, &archive).and_then(|_| {
        provider.install_locked(
            version,
            &LockedAsset {
                url: archive.to_string_lossy().into_owned(),
                sha256: asset.sha256.clone(),
            },
        )
    });
    let _ = fs::remove_dir_all(&staging);
    result
}

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
    }
    entry.assets.get(platform).cloned().ok_or_else(|| {
        anyhow!(
            ".avm.lock has no {tool} download for {platform}; run `avm lock --platform {platform}`"
        )
    })
}

/// Existing per-tool install/use commands must verify locked selections too.
pub fn install_provider_version(provider: &dyn ToolProvider, version: &str) -> Result<()> {
    if let Some(lock) = lockfile::load(&std::env::current_dir()?)? {
        if let Some(entry) = lock
            .tools
            .get(provider.name())
            .filter(|entry| entry.resolved == version)
        {
            let asset = locked_asset(provider.name(), entry, &host_platform()?)?;
            return install_locked(provider, version, &asset);
        }
    }
    provider.install(version)
}

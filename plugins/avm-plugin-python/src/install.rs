use crate::versions;
use anyhow::{Context, Result};
use avm_plugin_sdk::protocol::LockedAsset;
use avm_plugin_sdk::tool_dir;
use fs_err as fs;

const TAR_TIMEOUT_MS: u64 = 300_000;

/// Installs a CPython build with no locked asset.
pub fn install_python(version: &str) -> Result<()> {
    install(version, None)
}

/// Installs Python `version` under a per-version lock, doing nothing if already installed; uses `locked` when given and errors on download or extract failure.
pub fn install(version: &str, locked: Option<&LockedAsset>) -> Result<()> {
    avm_plugin_sdk::download::validate_version(version)?;
    let tools_root = tool_dir("python")?;
    let _guard = avm_plugin_sdk::storage::lock(&tools_root.join(format!(".install-{version}")))?;
    let target = tools_root.join(version);
    if target.join("bin").join(crate::PYTHON_BIN).exists() {
        return Ok(());
    }

    // Staged inside tools_root: never counts as installed and the final rename
    // stays on one filesystem.
    let tmp = tools_root.join(format!(".tmp-{version}"));
    let root = tmp.join("python");
    if tmp.exists() {
        fs::remove_dir_all(&tmp).context("failed to clean previous python install temp dir")?;
    }
    fs::create_dir_all(&root).context("failed to create python install temp dir")?;

    let archive = tmp.join("python.tar.gz");
    let download = (|| {
        let resolved;
        let asset = match locked {
            Some(asset) => asset,
            None => {
                let platform = versions::host_platform()?;
                resolved = versions::resolve_downloads(version, std::slice::from_ref(&platform))?;
                &resolved.assets[&platform]
            }
        };
        avm_plugin_sdk::download::download_verified(asset, &archive)
    })();
    if let Err(e) = download {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }

    // The archive's single root dir is `python/`; strip it.
    avm_plugin_sdk::download::extract_archive(
        &archive,
        &root,
        1,
        TAR_TIMEOUT_MS,
        "AVM_PYTHON_TAR_TIMEOUT",
        "Python archive extraction",
    )?;

    if target.exists() {
        fs::remove_dir_all(&target).context("failed to replace existing python install")?;
    }
    // Windows builds keep python.exe at the archive root; nest it under `bin`
    // so every platform shares the `<version>/bin/<python>` layout.
    if cfg!(windows) {
        fs::create_dir_all(&target).context("failed to create python install dir")?;
        fs::rename(&root, target.join("bin")).context("failed to move Python into place")?;
    } else {
        fs::rename(&root, &target).context("failed to move Python into place")?;
    }
    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

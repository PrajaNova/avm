use crate::versions;
use anyhow::{Context, Result};
use avm_plugin_api::protocol::LockedAsset;
use avm_plugin_api::{run_timed, tool_dir};
use std::fs;
use std::process::Command;

const TAR_TIMEOUT_MS: u64 = 300_000;

pub fn install_python(version: &str) -> Result<()> {
    install(version, None)
}

pub fn install(version: &str, locked: Option<&LockedAsset>) -> Result<()> {
    avm_plugin_api::download::validate_version(version)?;
    let tools_root = tool_dir("python")?;
    let _guard = avm_plugin_api::storage::lock(&tools_root.join(format!(".install-{version}")))?;
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
        avm_plugin_api::download::download_verified(asset, &archive)
    })();
    if let Err(e) = download {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }

    // The archive's single root dir is `python/`; strip it.
    let mut tar = Command::new("tar");
    tar.arg("-xzf")
        .arg(&archive)
        .arg("--strip-components=1")
        .arg("-C")
        .arg(&root);
    run_timed(
        tar,
        TAR_TIMEOUT_MS,
        "Python archive extraction",
        "AVM_PYTHON_TAR_TIMEOUT",
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

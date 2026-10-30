//! Windows shims: `<tool>.exe` hard-linked (or copied) from avm-bin.
use super::{invoked_as_shim, path_exts, which};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// A regular file whose extension is in `PATHEXT` (`.exe`, `.cmd`, ...).
pub(super) fn is_executable(path: &Path) -> bool {
    let exts = path_exts();
    path.is_file()
        && path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            exts.iter()
                .any(|x| x.trim_start_matches('.').eq_ignore_ascii_case(e))
        })
}

/// Windows: `<tool>.exe` is avm-bin itself (hard link, else a copy), so IDEs
/// and debuggers that start `node.exe` directly still go through avm; avm-bin
/// sees the name it was started as and dispatches to `exec-shim <tool>`.
pub(super) fn write_shim(shims_dir: &Path, tool: &str) -> Result<()> {
    let _ = fs::remove_file(shims_dir.join(format!("{tool}.cmd"))); // older .cmd shims
    let path = shims_dir.join(format!("{tool}.exe"));
    let avm = avm_exe()?;
    if path.exists() && fs::remove_file(&path).is_err() {
        // In use (a running shim): Windows lets a running exe be renamed, so
        // move it aside; remove_stale_shims deletes it on a later reshim.
        let aside = shims_dir.join(format!("{tool}.exe.stale-{}", std::process::id()));
        if fs::rename(&path, &aside).is_err() {
            return Ok(()); // keep the working one
        }
    }
    fs::hard_link(&avm, &path)
        .or_else(|_| fs::copy(&avm, &path).map(|_| ()))
        .with_context(|| format!("write shim for {tool}"))
}

/// The real avm-bin.exe to link shims to. When this process is itself a shim
/// (reshim after `npm i -g` runs inside `npm.exe`), that's the one on PATH.
fn avm_exe() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("locate avm-bin")?;
    if invoked_as_shim().is_none() {
        return Ok(exe);
    }
    which("avm-bin", true).context("avm-bin not found on PATH")
}

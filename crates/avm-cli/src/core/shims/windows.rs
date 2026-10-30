//! Windows shims: `<tool>.exe` hard-linked (or copied) from avm-bin.
use super::which;
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

/// The shim name for a tool's `bin/` entry: `tsc.cmd` → `tsc`.
pub(super) fn shim_name(entry: &fs::DirEntry) -> Option<std::ffi::OsString> {
    entry.path().file_stem().map(|s| s.to_os_string())
}

/// Executable extensions from `PATHEXT`, defaulting to `.COM;.EXE;.BAT;.CMD`.
fn path_exts() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter(|e| !e.is_empty())
        .map(str::to_string)
        .collect()
}

/// Candidate file names for a bare command: `node.exe`, `node.cmd`, ... per
/// `PATHEXT` unless it already has an extension.
pub fn command_names(bin: &str) -> Vec<String> {
    if Path::new(bin).extension().is_some() {
        return vec![bin.to_string()];
    }
    path_exts()
        .iter()
        .map(|ext| format!("{bin}{}", ext.to_ascii_lowercase()))
        .collect()
}

/// When avm-bin runs as a Windows shim (`node.exe`), the tool it stands for.
pub fn invoked_as_shim() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let stem = exe.file_stem()?.to_str()?.to_string();
    (!stem.eq_ignore_ascii_case("avm-bin") && !stem.eq_ignore_ascii_case("avm")).then_some(stem)
}

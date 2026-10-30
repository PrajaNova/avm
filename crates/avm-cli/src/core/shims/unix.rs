//! Unix shims: a small `sh` script per tool.
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// A regular file with an exec bit that isn't world-writable.
pub(super) fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let mode = meta.permissions().mode();
    meta.is_file() && mode & 0o111 != 0 && mode & 0o002 == 0
}

/// Write the `sh` shim for `tool` into `shims_dir` and mark it executable; leaves an identical existing shim untouched.
pub(super) fn write_shim(shims_dir: &Path, tool: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let path = shims_dir.join(tool);
    let contents = format!(
        r#"#!/usr/bin/env sh
# avm generated shim
if [ -z "$(command -v avm-bin)" ]; then
  echo "avm: avm-bin not found in PATH" >&2
  exit 1
fi

exec "$(command -v avm-bin)" exec-shim {tool} -- "$@"
"#
    );

    // Unchanged shims are left alone: shell startup reshims, and a truncating
    // rewrite can be read half-written by a process running the shim right now.
    if fs::read_to_string(&path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    // Write beside it and rename, so readers see the old or the new script, never a partial one.
    let staged = shims_dir.join(format!(".{tool}.{}.tmp", std::process::id()));
    fs::write(&staged, contents).with_context(|| format!("write shim for {tool}"))?;
    let mut perms = fs::metadata(&staged)
        .context("shim metadata")?
        .permissions();
    perms.set_mode(perms.mode() | 0o755);
    fs::set_permissions(&staged, perms).context("chmod shim")?;
    fs::rename(&staged, &path).with_context(|| format!("install shim for {tool}"))?;
    Ok(())
}

/// The shim name for a tool's `bin/` entry: the file name as is.
pub(super) fn shim_name(entry: &fs::DirEntry) -> Option<std::ffi::OsString> {
    Some(entry.file_name())
}

/// Candidate file names for a bare command: itself.
pub fn command_names(bin: &str) -> Vec<String> {
    vec![bin.to_string()]
}

/// Unix shims are `sh` scripts, so avm-bin is never started under a tool's name.
pub fn invoked_as_shim() -> Option<String> {
    None
}

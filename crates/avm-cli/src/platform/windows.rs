use crate::common::shell::Shell;
use anyhow::{anyhow, Context, Result};
use fs_err as fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Archive extension of released binaries and plugins.
pub const ARCHIVE_EXT: &str = "zip";
/// A protocol plugin's executable inside its dir.
pub const PLUGIN_BIN: &str = "avm-plugin.exe";

/// Env var names are case-insensitive here: normalised to upper case.
pub fn normalize_env_key(key: String) -> String {
    key.to_ascii_uppercase()
}

/// Starts `cmd` with no console so it outlives the terminal session.
pub fn detach(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    cmd.creation_flags(DETACHED_PROCESS);
}

/// A running exe can't be overwritten on Windows, but it can be renamed.
pub fn replace_binary(exe: &Path, new: &Path) -> Result<()> {
    let old = old_binary(exe);
    let _ = fs::remove_file(&old);
    fs::rename(exe, &old).context("failed to move the running avm-bin aside")?;
    if let Err(err) = fs::rename(new, exe) {
        let _ = fs::rename(&old, exe);
        return Err(err).context("failed to swap in the new binary");
    }
    Ok(())
}

/// Where the previous binary is parked during an update.
fn old_binary(exe: &Path) -> PathBuf {
    exe.with_extension("old.exe")
}

/// The previous binary is left aside until the next run.
pub fn remove_old_binary(exe: &Path) {
    let _ = fs::remove_file(old_binary(exe));
}

/// Shims are hard links to the old binary; relink them.
pub fn after_update() {
    let _ = crate::shims::reshim();
}

/// Shell used when none is requested.
pub const DEFAULT_SHELL: Shell = Shell::Pwsh;
/// Program and flag that run an alias command string.
pub const ALIAS_SHELL: (&str, &str) = ("cmd", "/C");

/// Windows won't search the child's PATH for a bare name, so look it up in `path`.
pub fn resolve_program(program: &str, path: &str, skip_shims: bool) -> Result<PathBuf> {
    if Path::new(program).components().count() != 1 {
        return Ok(PathBuf::from(program));
    }
    crate::shims::which_in_path(program, std::ffi::OsStr::new(path), skip_shims)
        .ok_or_else(|| anyhow!("command '{program}' not found in PATH"))
}

/// Windows can't exec-replace: runs `command` and exits with its status.
pub fn run_replacing_process(mut command: Command) -> Result<()> {
    let status = command.status().context("failed to execute command")?;
    std::process::exit(status.code().unwrap_or(1));
}

/// Windows decides executability by extension, so there is nothing to set.
pub fn make_executable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Windows has no owner or mode bits to check.
pub fn check_plugin_source_perms(_meta: &std::fs::Metadata) -> Result<()> {
    Ok(())
}

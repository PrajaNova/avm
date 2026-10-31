use crate::common::shell::Shell;
use crate::constants::msg;
use anyhow::{anyhow, Context, Result};
use fs_err as fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Archive extension of released binaries and plugins.
pub const ARCHIVE_EXT: &str = "tar.gz";
/// A protocol plugin's executable inside its dir.
pub const PLUGIN_BIN: &str = "avm-plugin";

/// Config env keys are case-sensitive here: kept as written.
pub fn normalize_env_key(key: String) -> String {
    key
}

/// Puts `cmd` in its own process group so it outlives the terminal session.
pub fn detach(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

/// Make `new` executable and atomically rename it over `exe`.
pub fn replace_binary(exe: &Path, new: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(new, std::fs::Permissions::from_mode(0o755))?;
    fs::rename(new, exe).context(msg::BINARY_SWAP_FAILED) // atomic
}

/// Nothing is parked aside on Unix.
pub fn remove_old_binary(_exe: &Path) {}

/// Unix shims are `sh` scripts that don't point at the old binary.
pub fn after_update() {}

/// Shell used when none is requested.
pub const DEFAULT_SHELL: Shell = Shell::Sh;
/// Program and flag that run an alias command string.
pub const ALIAS_SHELL: (&str, &str) = ("sh", "-c");

/// Unix looks `program` up on the child's PATH itself, so it is used as given.
pub fn resolve_program(program: &str, _path: &str, _skip_shims: bool) -> Result<PathBuf> {
    Ok(PathBuf::from(program))
}

/// Replaces this process with `command`; only returns the error if that fails.
pub fn run_replacing_process(mut command: Command) -> Result<()> {
    use std::os::unix::process::CommandExt;
    Err(command.exec()).context(msg::COMMAND_EXEC_FAILED)
}

/// Marks `path` rwxr-xr-x.
pub fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

/// Rejects a plugin source owned by root (unless we are root) or world-writable.
pub fn check_plugin_source_perms(meta: &std::fs::Metadata) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if meta.uid() == 0 && current_euid() != 0 {
        return Err(anyhow!(msg::PLUGIN_SOURCE_ROOT_OWNED));
    }
    if meta.permissions().mode() & 0o002 != 0 {
        return Err(anyhow!(msg::PLUGIN_SOURCE_WORLD_WRITABLE));
    }
    Ok(())
}

/// The process's effective user id.
fn current_euid() -> u32 {
    // SAFETY:
    // Operation: `libc::geteuid()`.
    // Contract (POSIX `geteuid`): takes no arguments and always succeeds; it has
    // no memory-safety preconditions and does not touch Rust-managed memory.
    // The `libc` crate declares the return type as the platform's `uid_t`, so
    // there is no hand-written ABI declaration to get wrong.
    // Postcondition: none for Rust state; the returned id is a plain integer.
    unsafe { libc::geteuid() }
}

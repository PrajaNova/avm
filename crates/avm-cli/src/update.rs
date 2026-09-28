//! `avm self-update` and the once-a-day "new version" notice (#28).
use crate::runtime;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const REPO: &str = "PrajaNova/avm";
const CURRENT: &str = env!("CARGO_PKG_VERSION");
const DAY_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, PartialEq)]
enum InstallMethod {
    Homebrew,
    Npm,
    Cargo,
    Scoop,
    /// install.sh / install.ps1 / a copied binary: avm updates itself.
    Direct,
}

fn install_method(exe: &Path) -> InstallMethod {
    let path = exe.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
    if path.contains("/cellar/") || path.contains("/homebrew/") || path.contains("/linuxbrew/") {
        InstallMethod::Homebrew
    } else if path.contains("/node_modules/") {
        InstallMethod::Npm
    } else if path.contains("/.cargo/bin/") {
        InstallMethod::Cargo
    } else if path.contains("/scoop/apps/") {
        InstallMethod::Scoop
    } else {
        InstallMethod::Direct
    }
}

/// The package manager's own upgrade command; `None` when avm updates itself.
fn upgrade_command(method: &InstallMethod) -> Option<&'static str> {
    match method {
        InstallMethod::Homebrew => Some("brew upgrade avm"),
        InstallMethod::Npm => Some("npm install -g @prajanova/avm@latest"),
        InstallMethod::Cargo => Some("cargo install --git https://github.com/PrajaNova/avm.git avm-cli --force"),
        InstallMethod::Scoop => Some("scoop update avm"),
        InstallMethod::Direct => None,
    }
}

fn current_exe() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("failed to locate avm-bin")?;
    Ok(exe.canonicalize().unwrap_or(exe))
}

pub fn self_update(version: Option<String>) -> Result<()> {
    let exe = current_exe()?;
    let method = install_method(&exe);
    if let Some(cmd) = upgrade_command(&method) {
        println!("avm was installed with {method:?}, so update it the same way:\n  {cmd}");
        return Ok(());
    }
    remove_old_binary(&exe);

    let tag = version.map(|v| format!("v{}", v.trim_start_matches('v')));
    let release = runtime::github_release(REPO, tag.as_deref())?;
    let target = release.tag_name.trim_start_matches('v').to_string();
    if target == CURRENT || (tag.is_none() && !is_newer(&target, CURRENT)) {
        println!("✓ avm {CURRENT} is up to date");
        return Ok(());
    }

    let (os, arch) = runtime::marketplace_platform()?;
    let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
    let asset = format!("avm_{os}_{arch}.{ext}");
    let dir = exe.parent().ok_or_else(|| anyhow!("{} has no parent directory", exe.display()))?;
    // Extract next to the binary so the swap is a same-filesystem rename.
    let staging = dir.join(format!(".avm-update-{}", std::process::id()));
    let result = runtime::fetch_verified_archive(REPO, &release, &asset, &staging)
        .and_then(|_| replace_binary(&exe, &staging.join(format!("avm-bin{}", std::env::consts::EXE_SUFFIX))));
    let _ = fs::remove_dir_all(&staging);
    result.with_context(|| format!("couldn't update {}", exe.display()))?;

    // Windows shims are hard links to the old binary; relink them.
    if cfg!(windows) {
        let _ = crate::shims::reshim();
    }
    println!("✓ Updated avm {CURRENT} → {target}");
    Ok(())
}

#[cfg(unix)]
fn replace_binary(exe: &Path, new: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(new, fs::Permissions::from_mode(0o755))?;
    fs::rename(new, exe).context("failed to swap in the new binary") // atomic
}

/// A running exe can't be overwritten on Windows, but it can be renamed.
#[cfg(windows)]
fn replace_binary(exe: &Path, new: &Path) -> Result<()> {
    let old = old_binary(exe);
    let _ = fs::remove_file(&old);
    fs::rename(exe, &old).context("failed to move the running avm-bin aside")?;
    if let Err(err) = fs::rename(new, exe) {
        let _ = fs::rename(&old, exe);
        return Err(err).context("failed to swap in the new binary");
    }
    Ok(())
}

fn old_binary(exe: &Path) -> PathBuf {
    exe.with_extension("old.exe")
}

/// Windows leaves the previous binary aside until the next run.
fn remove_old_binary(exe: &Path) {
    if cfg!(windows) {
        let _ = fs::remove_file(old_binary(exe));
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct CheckCache {
    #[serde(default)]
    checked_at: u64,
    #[serde(default)]
    notified_at: u64,
    latest: Option<String>,
}

fn cache_path() -> Result<PathBuf> {
    Ok(crate::shims::avm_home()?.join("update-check.json"))
}

fn read_cache() -> CheckCache {
    cache_path().ok().and_then(|p| fs::read(p).ok()).and_then(|raw| serde_json::from_slice(&raw).ok()).unwrap_or_default()
}

fn write_cache(cache: &CheckCache) {
    if let Ok(path) = cache_path() {
        let _ = path.parent().map(fs::create_dir_all);
        let _ = fs::write(path, serde_json::to_vec(cache).unwrap_or_default());
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// After an interactive command: print a one-line notice (at most daily) if a
/// newer release is known, and refresh that knowledge in a detached background
/// process (at most daily). Never blocks, never in CI, pipes, or when opted out.
pub fn maybe_notify() {
    if std::env::var_os("AVM_NO_UPDATE_CHECK").is_some()
        || std::env::var_os("CI").is_some()
        || !std::io::stderr().is_terminal()
    {
        return;
    }
    let mut cache = read_cache();
    let now = now();
    let mut dirty = false;
    if let Some(latest) = cache.latest.as_deref().filter(|l| is_newer(l, CURRENT)) {
        if now.saturating_sub(cache.notified_at) >= DAY_SECS {
            let how = current_exe().ok().and_then(|exe| upgrade_command(&install_method(&exe))).unwrap_or("avm self-update");
            eprintln!("avm {latest} is available (you have {CURRENT}). Update with: {how}");
            cache.notified_at = now;
            dirty = true;
        }
    }
    let check = now.saturating_sub(cache.checked_at) >= DAY_SECS;
    if check {
        cache.checked_at = now; // even if the check fails, retry tomorrow, not every command
        dirty = true;
    }
    // Written before the background check starts, so it can't clobber the result.
    if dirty {
        write_cache(&cache);
    }
    if check {
        spawn_detached_check();
    }
}

/// `avm-bin __update-check`, detached so it outlives this command and the
/// terminal session (its own process group; no console on Windows).
fn spawn_detached_check() {
    let Ok(exe) = std::env::current_exe() else { return };
    let mut cmd = Command::new(exe);
    cmd.arg("__update-check").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(DETACHED_PROCESS);
    }
    let _ = cmd.spawn();
}

/// The background half of `maybe_notify`: record the latest release.
pub fn refresh_cache() -> Result<()> {
    let release = runtime::github_release(REPO, None)?;
    let mut cache = read_cache();
    cache.latest = Some(release.tag_name.trim_start_matches('v').to_string());
    cache.checked_at = now();
    write_cache(&cache);
    Ok(())
}

/// Semver-ish: `0.4.0 > 0.4.0-beta-2 > 0.4.0-beta-1 > 0.3.9`.
fn is_newer(candidate: &str, current: &str) -> bool {
    fn split(v: &str) -> (Vec<u64>, Option<&str>) {
        let v = v.trim_start_matches('v');
        let (core, pre) = v.split_once('-').map_or((v, None), |(c, p)| (c, Some(p)));
        (core.split('.').map(|n| n.parse().unwrap_or(0)).collect(), pre)
    }
    // Pre-release tags compared piecewise, numbers numerically: beta-10 > beta-9.
    fn pre_key(p: &str) -> Vec<(u64, String)> {
        p.split(['-', '.'])
            .map(|part| part.parse().map_or((0, part.to_string()), |n| (n, String::new())))
            .collect()
    }
    let (a, pa) = split(candidate);
    let (b, pb) = split(current);
    if a != b {
        return a > b;
    }
    match (pa, pb) {
        (None, Some(_)) => true,
        (Some(x), Some(y)) => pre_key(x) > pre_key(y),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(is_newer("0.4.0", "0.4.0-beta-1"));
        assert!(is_newer("0.4.0-beta-2", "0.4.0-beta-1"));
        assert!(is_newer("0.4.0-beta-10", "0.4.0-beta-9"));
        assert!(is_newer("v0.4.1", "0.4.0"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("0.4.0-beta-1", "0.4.0"));
        assert!(!is_newer("0.4.0", "0.4.0"));
        assert!(!is_newer("0.3.9", "0.4.0-beta-1"));
    }

    #[test]
    fn install_methods() {
        let m = |p: &str| install_method(Path::new(p));
        assert_eq!(m("/opt/homebrew/Cellar/avm/0.4.0/bin/avm-bin"), InstallMethod::Homebrew);
        assert_eq!(m("/home/linuxbrew/.linuxbrew/Cellar/avm/0.4.0/bin/avm-bin"), InstallMethod::Homebrew);
        assert_eq!(m("/usr/lib/node_modules/@prajanova/avm/bin/avm-bin"), InstallMethod::Npm);
        assert_eq!(m("/home/u/.cargo/bin/avm-bin"), InstallMethod::Cargo);
        assert_eq!(m(r"C:\Users\u\scoop\apps\avm\current\avm-bin.exe"), InstallMethod::Scoop);
        assert_eq!(m("/home/u/.local/bin/avm-bin"), InstallMethod::Direct);
        assert_eq!(m(r"C:\Users\u\AppData\Local\avm\bin\avm-bin.exe"), InstallMethod::Direct);
    }
}

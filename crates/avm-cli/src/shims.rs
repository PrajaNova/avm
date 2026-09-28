use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Core shims per managed tool: every binary here always gets a shim and
/// dispatches through its tool's pinned version.
pub const TOOL_BINS: &[(&str, &[&str])] = &[
    ("node", &["node", "npm", "npx", "pnpm", "yarn", "bun"]),
    (
        "java",
        &["java", "javac", "jar", "javadoc", "jshell", "jarsigner", "keytool"],
    ),
];

/// `$HOME/.avm`.
pub fn avm_home() -> Result<PathBuf> {
    Ok(avm_plugin_api::home_dir()?.join(".avm"))
}

pub fn shim_dir() -> Result<PathBuf> {
    Ok(avm_home()?.join("shims"))
}

/// Regenerate shims: keep the core set, then scan every installed managed
/// version's `bin/` (`~/.avm/tools/<tool>/<version>/bin`) and write a shim for
/// each executable found — so globally-installed package binaries like `tsc`
/// or `eslint` become runnable through the same dir-aware dispatch.
pub fn reshim() -> Result<()> {
    let shims_dir = shim_dir()?;
    fs::create_dir_all(&shims_dir).context("create shims dir")?;
    for tool in TOOL_BINS.iter().flat_map(|(_, bins)| bins.iter()) {
        write_shim(&shims_dir, tool)?;
    }

    let tools_root = avm_home()?.join("tools");
    let Ok(tools) = fs::read_dir(&tools_root) else {
        return Ok(());
    };
    for tool in tools.flatten() {
        let Ok(versions) = fs::read_dir(tool.path()) else {
            continue;
        };
        for version in versions.flatten() {
            let Ok(bins) = fs::read_dir(version.path().join("bin")) else {
                continue;
            };
            for entry in bins.flatten() {
                if !is_executable(&entry.path()) {
                    continue;
                }
                // Windows: `tsc.cmd` → shim `tsc`.
                let file_name = if cfg!(windows) { entry.path().file_stem().map(|s| s.to_os_string()) } else { Some(entry.file_name()) };
                if let Some(name) = file_name.as_deref().and_then(|n| n.to_str()) {
                    // Reject anything that isn't a plain command name.
                    if name.starts_with('.') || name.contains('/') || name.contains('\\') {
                        continue;
                    }
                    write_shim(&shims_dir, name)?;
                }
            }
        }
    }
    Ok(())
}

/// A regular file with an exec bit that isn't world-writable.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let mode = meta.permissions().mode();
    meta.is_file() && mode & 0o111 != 0 && mode & 0o002 == 0
}

/// A regular file whose extension is in `PATHEXT` (`.exe`, `.cmd`, ...).
#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    let exts = path_exts();
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| exts.iter().any(|x| x.trim_start_matches('.').eq_ignore_ascii_case(e)))
}

/// Candidate file names for a bare command: itself on Unix; `node.exe`,
/// `node.cmd`, ... per `PATHEXT` on Windows.
pub fn command_names(bin: &str) -> Vec<String> {
    if cfg!(windows) && Path::new(bin).extension().is_none() {
        path_exts().iter().map(|ext| format!("{bin}{}", ext.to_ascii_lowercase())).collect()
    } else {
        vec![bin.to_string()]
    }
}

fn path_exts() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter(|e| !e.is_empty())
        .map(str::to_string)
        .collect()
}

/// First executable `bin` on PATH, optionally skipping avm's own shims.
pub fn which(bin: &str, skip_shims: bool) -> Option<PathBuf> {
    let shim_dir = shim_dir().ok().and_then(|dir| dir.canonicalize().ok());
    let paths = std::env::var_os("PATH")?;
    let names = command_names(bin);
    std::env::split_paths(&paths)
        .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
        .filter(|candidate| is_executable(candidate))
        .find(|candidate| {
            !skip_shims
                || !matches!(
                    (candidate.canonicalize(), &shim_dir),
                    (Ok(real), Some(shims)) if real.starts_with(shims)
                )
        })
}

/// Persist `~/.avm/shims` onto PATH in shell startup files so dir-aware
/// resolution survives environments that reset PATH (GUI apps, sandboxed tools
/// like Codex/Claude). `.zshenv` is the key target — zsh sources it for every
/// invocation, including non-interactive `zsh -c` used by such tools.
pub fn activate_profiles() -> Result<Vec<PathBuf>> {
    let home = avm_plugin_api::home_dir()?;
    let block = "\n# >>> avm shims >>>\nexport PATH=\"$HOME/.avm/shims:$PATH\"\n# <<< avm shims <<<\n";
    let marker = "# >>> avm shims >>>";

    let mut written = Vec::new();
    for name in [".zshenv", ".bashrc", ".profile"] {
        let path = home.join(name);
        let existing = fs::read_to_string(&path).unwrap_or_default();
        if existing.contains(marker) {
            continue;
        }
        fs::write(&path, format!("{existing}{block}"))
            .with_context(|| format!("failed to update {}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

pub fn remove_shim(tool: &str) -> Result<()> {
    for name in [tool.to_string(), format!("{tool}.cmd"), format!("{tool}.exe")] {
        let path = shim_dir()?.join(name);
        if path.exists() {
            fs::remove_file(path).context("remove shim")?;
        }
    }
    Ok(())
}

/// Windows: `<tool>.exe` is avm-bin itself (hard link, else a copy), so IDEs
/// and debuggers that start `node.exe` directly still go through avm; avm-bin
/// sees the name it was started as and dispatches to `exec-shim <tool>`.
#[cfg(windows)]
fn write_shim(shims_dir: &Path, tool: &str) -> Result<()> {
    let _ = fs::remove_file(shims_dir.join(format!("{tool}.cmd"))); // older .cmd shims
    let path = shims_dir.join(format!("{tool}.exe"));
    let avm = std::env::current_exe().context("locate avm-bin")?;
    if path.exists() {
        // A running shim can't be replaced; the existing one still works.
        if fs::remove_file(&path).is_err() {
            return Ok(());
        }
    }
    fs::hard_link(&avm, &path)
        .or_else(|_| fs::copy(&avm, &path).map(|_| ()))
        .with_context(|| format!("write shim for {tool}"))
}

/// When avm-bin runs as a Windows shim (`node.exe`), the tool it stands for.
pub fn invoked_as_shim() -> Option<String> {
    if !cfg!(windows) {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let stem = exe.file_stem()?.to_str()?.to_string();
    (!stem.eq_ignore_ascii_case("avm-bin") && !stem.eq_ignore_ascii_case("avm")).then_some(stem)
}

#[cfg(unix)]
fn write_shim(shims_dir: &Path, tool: &str) -> Result<()> {
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

    fs::write(&path, contents).with_context(|| format!("write shim for {tool}"))?;
    let mut perms = fs::metadata(&path).context("shim metadata")?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    fs::set_permissions(&path, perms).context("chmod shim")?;
    Ok(())
}

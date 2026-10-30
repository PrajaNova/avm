use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
#[cfg(unix)]
use unix::{is_executable, write_shim};
#[cfg(windows)]
use windows::{is_executable, write_shim};

/// Core shims per managed tool: every binary here always gets a shim and
/// dispatches through its tool's pinned version.
pub const TOOL_BINS: &[(&str, &[&str])] = &[
    ("node", &["node", "npm", "npx", "pnpm", "yarn", "bun"]),
    (
        "java",
        &[
            "java",
            "javac",
            "jar",
            "javadoc",
            "jshell",
            "jarsigner",
            "keytool",
        ],
    ),
];

/// `$HOME/.avm`.
pub fn avm_home() -> Result<PathBuf> {
    Ok(avm_plugin_api::home_dir()?.join(".avm"))
}

/// The directory holding avm's shims (`~/.avm/shims`).
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
    remove_stale_shims(&shims_dir);
    // One shim failing (e.g. in use) mustn't stop the rest; report the first.
    let mut first_err = None;
    let mut write = |tool: &str| {
        if let Err(err) = write_shim(&shims_dir, tool) {
            first_err.get_or_insert(err);
        }
    };
    for tool in TOOL_BINS.iter().flat_map(|(_, bins)| bins.iter()) {
        write(tool);
    }

    let tools_root = avm_home()?.join("tools");
    let Ok(tools) = fs::read_dir(&tools_root) else {
        return first_err.map_or(Ok(()), Err);
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
                let file_name = if cfg!(windows) {
                    entry.path().file_stem().map(|s| s.to_os_string())
                } else {
                    Some(entry.file_name())
                };
                if let Some(name) = file_name.as_deref().and_then(|n| n.to_str()) {
                    // Reject anything that isn't a plain command name.
                    if name.starts_with('.') || name.contains('/') || name.contains('\\') {
                        continue;
                    }
                    write(name);
                }
            }
        }
    }
    first_err.map_or(Ok(()), Err)
}

/// Candidate file names for a bare command: itself on Unix; `node.exe`,
/// `node.cmd`, ... per `PATHEXT` on Windows.
pub fn command_names(bin: &str) -> Vec<String> {
    if cfg!(windows) && Path::new(bin).extension().is_none() {
        path_exts()
            .iter()
            .map(|ext| format!("{bin}{}", ext.to_ascii_lowercase()))
            .collect()
    } else {
        vec![bin.to_string()]
    }
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

/// First executable `bin` on PATH, optionally skipping avm's own shims.
pub fn which(bin: &str, skip_shims: bool) -> Option<PathBuf> {
    which_in_path(bin, &std::env::var_os("PATH")?, skip_shims)
}

/// Like `which`, but searching the given PATH-style list `paths`.
pub fn which_in_path(bin: &str, paths: &std::ffi::OsStr, skip_shims: bool) -> Option<PathBuf> {
    let shim_dir = shim_dir().ok().and_then(|dir| dir.canonicalize().ok());
    let names = command_names(bin);
    std::env::split_paths(paths)
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
    let block =
        "\n# >>> avm shims >>>\nexport PATH=\"$HOME/.avm/shims:$PATH\"\n# <<< avm shims <<<\n";
    let marker = "# >>> avm shims >>>";

    let mut written = Vec::new();
    for name in [".zshenv", ".bashrc", ".profile"] {
        let path = home.join(name);
        // Lossy read: a non-UTF-8 rc file must never be mistaken for an empty one.
        let existing = String::from_utf8_lossy(&fs::read(&path).unwrap_or_default()).into_owned();
        if existing.contains(marker) {
            continue;
        }
        // Append only; never rewrite what the user already has.
        let appended = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, block.as_bytes()));
        appended.with_context(|| format!("failed to update {}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

/// Delete the shim files (plain, `.cmd`, `.exe`) for `tool`; errors on an invalid tool name or failed removal.
pub fn remove_shim(tool: &str) -> Result<()> {
    avm_plugin_api::validate_name(tool)?;
    for name in [
        tool.to_string(),
        format!("{tool}.cmd"),
        format!("{tool}.exe"),
    ] {
        let path = shim_dir()?.join(name);
        if path.exists() {
            fs::remove_file(path).context("remove shim")?;
        }
    }
    Ok(())
}

/// Best-effort cleanup of shims moved aside while they were running.
fn remove_stale_shims(shims_dir: &Path) {
    for entry in fs::read_dir(shims_dir).into_iter().flatten().flatten() {
        if entry.file_name().to_string_lossy().contains(".exe.stale-") {
            let _ = fs::remove_file(entry.path());
        }
    }
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

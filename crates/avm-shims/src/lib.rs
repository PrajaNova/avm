use anyhow::{Context, Result};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const SHIMS: &[&str] = &[
    "node",
    "npm",
    "npx",
    "pnpm",
    "yarn",
    "bun",
    "java",
    "javac",
    "jar",
    "javadoc",
    "jshell",
    "jarsigner",
    "keytool",
];

pub fn avm_home() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".avm"))
}

pub fn shim_dir() -> Result<PathBuf> {
    Ok(avm_home()?.join("shims"))
}

pub fn install_shims() -> Result<()> {
    let shims_dir = shim_dir()?;
    fs::create_dir_all(&shims_dir).context("create shims dir")?;

    for tool in SHIMS {
        write_shim(&shims_dir, tool)?;
    }
    Ok(())
}

/// Regenerate shims: keep the core set, then scan every installed managed
/// version's `bin/` (`~/.avm/tools/<tool>/<version>/bin`) and write a shim for
/// each executable found — so globally-installed package binaries like `tsc`
/// or `eslint` become runnable through the same dir-aware dispatch.
pub fn reshim() -> Result<()> {
    let shims_dir = shim_dir()?;
    fs::create_dir_all(&shims_dir).context("create shims dir")?;
    for tool in SHIMS {
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
                let path = entry.path();
                if !is_executable_file(&path) {
                    continue;
                }
                if let Some(name) = entry.file_name().to_str() {
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

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    meta.is_file() && meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

/// Persist `~/.avm/shims` onto PATH in shell startup files so dir-aware
/// resolution survives environments that reset PATH (GUI apps, sandboxed tools
/// like Codex/Claude). `.zshenv` is the key target — zsh sources it for every
/// invocation, including non-interactive `zsh -c` used by such tools.
pub fn activate_profiles() -> Result<Vec<PathBuf>> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let home = PathBuf::from(home);

    // Each shim execs `avm-bin`, so the sandbox also needs avm-bin's own dir on
    // PATH — not just the shims dir. Add it unless it's already the shims dir.
    let shims_dir = home.join(".avm").join("shims");
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    let path_line = match exe_dir {
        Some(dir) if dir != shims_dir => format!(
            "export PATH=\"$HOME/.avm/shims:{}:$PATH\"",
            dir.display()
        ),
        _ => "export PATH=\"$HOME/.avm/shims:$PATH\"".to_string(),
    };
    let block = format!("\n# >>> avm shims >>>\n{path_line}\n# <<< avm shims <<<\n");
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
    let path = shim_dir()?.join(tool);
    if path.exists() {
        fs::remove_file(path).context("remove shim")?;
    }
    Ok(())
}

fn write_shim(shims_dir: &Path, tool: &str) -> Result<()> {
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
    #[cfg(unix)]
    {
        let metadata = fs::metadata(&path).context("shim metadata")?;
        let mut perms = metadata.permissions();
        perms.set_mode(perms.mode() | 0o755);
        fs::set_permissions(&path, perms).context("chmod shim")?;
    }
    Ok(())
}

pub fn shim_path_env() -> Result<String> {
    Ok(format!("{}", shim_dir()?.display()))
}

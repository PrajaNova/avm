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

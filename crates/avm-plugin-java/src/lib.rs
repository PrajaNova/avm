mod install;
mod versions;

use anyhow::Context;
use avm_plugin_api::{ToolProvider, ToolVersion, ToolVersionQuery};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct JavaProvider;

impl JavaProvider {
    pub fn new() -> Self {
        Self
    }

    fn bin_path_for(&self, version: &str, binary: &str) -> anyhow::Result<Option<PathBuf>> {
        let candidate = install::tools_root()?
            .join(version)
            .join("bin")
            .join(binary_name(binary));
        Ok(candidate.exists().then_some(candidate))
    }
}

impl ToolProvider for JavaProvider {
    fn name(&self) -> &str {
        "java"
    }

    fn is_installed(&self, version: &str) -> bool {
        self.bin_path_for(version, "java").ok().flatten().is_some()
    }

    fn installed_versions(&self) -> anyhow::Result<Vec<String>> {
        let root = install::tools_root()?;
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err).context("failed to read java tools dir"),
        };

        let mut versions: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|version| self.is_installed(version))
            .collect();
        versions.sort_unstable();
        Ok(versions)
    }

    fn available_versions(&self, query: ToolVersionQuery) -> anyhow::Result<Vec<ToolVersion>> {
        versions::available_versions(query)
    }

    fn executable_path(&self, version: &str) -> anyhow::Result<Option<PathBuf>> {
        self.bin_path_for(version, "java")
    }

    /// Deterministic, in-process — no subprocess, unlike the old asdf
    /// `exec-env`-diffing approach this replaces.
    fn env_vars(&self, version: &str) -> anyhow::Result<HashMap<String, String>> {
        let home = install::tools_root()?.join(version);
        let mut env = HashMap::new();
        env.insert("JAVA_HOME".to_string(), home.to_string_lossy().to_string());
        Ok(env)
    }

    fn install(&self, version: &str) -> anyhow::Result<()> {
        install::install_java(version)
    }

    fn uninstall(&self, version: &str) -> anyhow::Result<()> {
        let target = install::tools_root()?.join(version);
        if target.exists() {
            fs::remove_dir_all(&target).context("failed to remove managed java version")?;
        }
        Ok(())
    }
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

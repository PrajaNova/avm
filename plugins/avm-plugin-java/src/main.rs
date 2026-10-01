mod install;

mod versions;

/// `bin/java` (or `bin\java.exe`) inside each installed JDK.
pub(crate) const JAVA_BIN: &str = if cfg!(windows) { "java.exe" } else { "java" };

use avm_plugin_api::{runner, tool_dir, Manifest, ToolProvider, ToolVersion, ToolVersionQuery};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

struct JavaProvider;

impl JavaProvider {
    fn java_bin(&self, version: &str) -> anyhow::Result<Option<PathBuf>> {
        let candidate = tool_dir("java")?.join(version).join("bin").join(JAVA_BIN);
        Ok(candidate.exists().then_some(candidate))
    }
}

impl ToolProvider for JavaProvider {
    fn name(&self) -> &str {
        "java"
    }

    fn is_installed(&self, version: &str) -> bool {
        self.java_bin(version).ok().flatten().is_some()
    }

    fn installed_versions(&self) -> anyhow::Result<Vec<String>> {
        avm_plugin_api::list_installed("java", |v| self.is_installed(v))
    }

    fn available_versions(&self, query: ToolVersionQuery) -> anyhow::Result<Vec<ToolVersion>> {
        versions::available_versions(query)
    }

    fn executable_path(&self, version: &str) -> anyhow::Result<Option<PathBuf>> {
        self.java_bin(version)
    }

    fn env_vars(&self, version: &str) -> anyhow::Result<HashMap<String, String>> {
        let home = tool_dir("java")?.join(version);
        Ok(HashMap::from([("JAVA_HOME".to_string(), home.to_string_lossy().to_string())]))
    }

    fn install(&self, version: &str) -> anyhow::Result<()> {
        install::install_java(version)
    }

    fn uninstall(&self, version: &str) -> anyhow::Result<()> {
        avm_plugin_api::remove_version("java", version)
    }
}

fn main() -> ExitCode {
    let manifest = Manifest::new(
        "java",
        env!("CARGO_PKG_VERSION"),
        "Built-in OpenJDK provider (Eclipse Temurin builds via the foojay Disco API)",
        "OpenJDK",
    );
    runner::run(manifest, &JavaProvider)
}

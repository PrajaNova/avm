mod install;

mod versions;

/// `bin/python3` (or `bin\python.exe`) inside each installed version.
pub(crate) const PYTHON_BIN: &str = if cfg!(windows) {
    "python.exe"
} else {
    "python3"
};

use avm_plugin_sdk::{runner, tool_dir, Manifest, ToolProvider, ToolVersion, ToolVersionQuery};
use std::path::PathBuf;
use std::process::ExitCode;

struct PythonProvider;

impl PythonProvider {
    /// Returns the `python` binary for `version` if it is installed.
    fn python_bin(&self, version: &str) -> anyhow::Result<Option<PathBuf>> {
        let candidate = tool_dir("python")?
            .join(version)
            .join("bin")
            .join(PYTHON_BIN);
        Ok(candidate.exists().then_some(candidate))
    }
}

impl ToolProvider for PythonProvider {
    fn name(&self) -> &str {
        "python"
    }

    fn is_installed(&self, version: &str) -> bool {
        self.python_bin(version).ok().flatten().is_some()
    }

    fn installed_versions(&self) -> anyhow::Result<Vec<String>> {
        avm_plugin_sdk::list_installed("python", |v| self.is_installed(v))
    }

    fn available_versions(&self, query: ToolVersionQuery) -> anyhow::Result<Vec<ToolVersion>> {
        versions::available_versions(query)
    }

    fn executable_path(&self, version: &str) -> anyhow::Result<Option<PathBuf>> {
        self.python_bin(version)
    }

    fn install(&self, version: &str) -> anyhow::Result<()> {
        install::install_python(version)
    }

    fn resolve_downloads(
        &self,
        version: &str,
        platforms: &[String],
    ) -> anyhow::Result<avm_plugin_sdk::protocol::ResolveResponse> {
        versions::resolve_downloads(version, platforms)
    }

    fn install_locked(
        &self,
        version: &str,
        asset: &avm_plugin_sdk::protocol::LockedAsset,
    ) -> anyhow::Result<()> {
        install::install(version, Some(asset))
    }

    fn uninstall(&self, version: &str) -> anyhow::Result<()> {
        avm_plugin_sdk::remove_version("python", version)
    }
}

/// Plugin entry point: runs the Python provider through the plugin protocol.
fn main() -> ExitCode {
    let manifest = Manifest::new(
        "python",
        env!("CARGO_PKG_VERSION"),
        "Built-in CPython provider (python-build-standalone builds)",
        "Python",
    );
    runner::run(manifest, &PythonProvider)
}

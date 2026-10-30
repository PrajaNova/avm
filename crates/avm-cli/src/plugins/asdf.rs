use super::*;

#[derive(Debug, Clone)]
pub struct AsdfToolProvider {
    pub(super) name: String,
    pub(super) plugin_name: String,
    pub(super) plugin_path: PathBuf,
}

impl AsdfToolProvider {
    /// Install directory for `version`; errors if the version string is invalid.
    fn install_path(&self, version: &str) -> Result<PathBuf> {
        avm_plugin_api::download::validate_version(version)?;
        Ok(tool_dir(&self.name)?.join(version))
    }

    /// The installed tool binary for `version`, or `None` if it is not installed.
    fn bin_path(&self, version: &str) -> Option<PathBuf> {
        let candidate = self
            .install_path(version)
            .ok()?
            .join("bin")
            .join(&self.name);
        candidate.exists().then_some(candidate)
    }

    /// Run the asdf plugin's `bin/<command>` in a sandboxed environment under a timeout, returning stdout.
    /// Errors if the script is missing, fails, or times out.
    fn run_asdf_command(
        &self,
        command: &str,
        version: Option<&str>,
        timeout_ms: u64,
    ) -> Result<String> {
        let command_path = self.plugin_path.join("bin").join(command);
        if !command_path.exists() {
            return Err(anyhow!("asdf plugin missing bin/{command}"));
        }

        let mut cmd = Command::new(command_path);
        sandbox_asdf_command(&mut cmd, &self.plugin_path);
        if let Some(version) = version {
            cmd.env("ASDF_INSTALL_VERSION", version);
            cmd.env("ASDF_INSTALL_PATH", self.install_path(version)?);
        }
        run_with_timeout(cmd, timeout_ms)
    }

    /// Run bash, optionally sourcing `exec_env` first, and return its resulting
    /// environment as a map (NUL-delimited so values with newlines survive).
    fn capture_env(
        &self,
        exec_env: Option<&Path>,
        version: &str,
        install_path: &Path,
    ) -> Result<HashMap<String, String>> {
        let script = match exec_env {
            Some(path) => format!(
                ". {} >/dev/null 2>&1; env -0",
                avm_plugin_api::sh_quote(&path.to_string_lossy())
            ),
            None => "env -0".to_string(),
        };
        let mut cmd = Command::new("bash");
        cmd.arg("-c").arg(&script);
        cmd.env("ASDF_INSTALL_VERSION", version);
        cmd.env("ASDF_INSTALL_PATH", install_path);
        cmd.env("ASDF_INSTALL_TYPE", "version");
        let output = run_with_timeout(cmd, ASDF_ENV_TIMEOUT_MS)?;

        let mut map = HashMap::new();
        for entry in output.split('\0') {
            if let Some((key, value)) = entry.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            }
        }
        Ok(map)
    }
}

impl ToolProvider for AsdfToolProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_installed(&self, version: &str) -> bool {
        self.bin_path(version).is_some()
    }

    fn installed_versions(&self) -> Result<Vec<String>> {
        list_installed(&self.name, |_| true)
    }

    fn available_versions(&self, query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {
        let output = self.run_asdf_command("list-all", None, ASDF_LIST_TIMEOUT_MS)?;
        let mut all: Vec<ToolVersion> = output
            .split_whitespace()
            .map(|version| ToolVersion {
                version: version.to_string(),
                label: version.to_string(),
                channel: Some(self.plugin_name.clone()),
                is_lts: false,
                is_security: false,
            })
            .collect();
        // asdf list-all is usually oldest-first; protocol queries are newest-first.
        all.sort_by(|a, b| {
            use crate::version_files::{build, numbers};
            (numbers(&b.version), build(&b.version), &b.version).cmp(&(
                numbers(&a.version),
                build(&a.version),
                &a.version,
            ))
        });
        let mut versions = query.filter(all, |v| version_major(&v.version).unwrap_or(u64::MAX));
        if matches!(query, ToolVersionQuery::Recent) {
            versions.truncate(10);
        }
        Ok(versions)
    }

    fn executable_path(&self, version: &str) -> Result<Option<PathBuf>> {
        Ok(self.bin_path(version))
    }

    /// Env vars declared by the plugin's `bin/exec-env` (standard asdf contract:
    /// the script is sourced and exports vars like `ANDROID_HOME`). Returns only
    /// what the script adds or changes, by diffing against a control run.
    fn env_vars(&self, version: &str) -> Result<HashMap<String, String>> {
        let exec_env = self.plugin_path.join("bin").join("exec-env");
        if !exec_env.exists() {
            return Ok(HashMap::new());
        }
        let install_path = self.install_path(version)?;

        // Baseline env (control) vs env after sourcing exec-env. Same injected
        // ASDF_* vars in both so those cancel out in the diff.
        let baseline = self.capture_env(None, version, &install_path)?;
        let sourced = self.capture_env(Some(&exec_env), version, &install_path)?;

        let mut out = HashMap::new();
        for (key, value) in sourced {
            if baseline.get(&key) != Some(&value) {
                out.insert(key, value);
            }
        }
        Ok(out)
    }

    fn install(&self, version: &str) -> Result<()> {
        avm_plugin_api::download::validate_version(version)?;
        let _guard = avm_plugin_api::storage::lock(
            &tool_dir(&self.name)?.join(format!(".install-{version}")),
        )?;
        if self.is_installed(version) {
            return Ok(());
        }
        let install_path = self.install_path(version)?;
        fs::create_dir_all(&install_path).context("failed to create asdf install path")?;
        if let Err(err) = self.run_asdf_command("install", Some(version), ASDF_INSTALL_TIMEOUT_MS) {
            let _ = fs::remove_dir_all(&install_path);
            return Err(err);
        }
        Ok(())
    }

    fn uninstall(&self, version: &str) -> Result<()> {
        if self.plugin_path.join("bin").join("uninstall").exists() {
            self.run_asdf_command("uninstall", Some(version), ASDF_INSTALL_TIMEOUT_MS)?;
        }
        remove_version(&self.name, version)
    }
}

/// Reset `cmd` to a minimal environment (plugin dir as cwd and `ASDF_DIR`, default PATH, `HOME`, `TMPDIR`) so plugin scripts don't inherit the caller's.
pub(super) fn sandbox_asdf_command(cmd: &mut Command, plugin_path: &Path) {
    cmd.env_clear();
    cmd.current_dir(plugin_path);
    cmd.env("ASDF_DIR", plugin_path);
    cmd.env("PATH", DEFAULT_PLUGIN_PATH_ENV);
    if let Ok(home) = avm_plugin_api::home_dir() {
        cmd.env("HOME", home);
    }
    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        cmd.env("TMPDIR", tmpdir);
    }
}

/// Leading numeric component of a version, ignoring a `<name>-` prefix (`temurin-17.0.9` is 17); `None` if not numeric.
pub(super) fn version_major(version: &str) -> Option<u64> {
    let version = version
        .rsplit_once('-')
        .map(|(_, version)| version)
        .unwrap_or(version);
    version
        .split(['.', '+', '-'])
        .find(|part| !part.is_empty())
        .and_then(|part| part.parse::<u64>().ok())
}

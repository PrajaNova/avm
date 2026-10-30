use super::*;

/// Host-side runner for avm's native plugin protocol (see
/// `docs/plugins/CREATING_A_PLUGIN.md`): a plugin is a standalone executable
/// speaking a small JSON-on-stdout contract for "read" calls, and plain
/// exit-code + inherited stdio for `install`/`uninstall` so progress streams
/// live. This implements the same `ToolProvider` trait as `AsdfToolProvider`
/// and the in-process builtins, so every existing call site in `avm-cli`
/// works unchanged regardless of which tier resolved the provider.
#[derive(Debug, Clone)]
pub struct PluginProcess {
    pub(super) tool_name: String,
    pub(super) executable: PathBuf,
}

impl PluginProcess {
    /// Wrap the plugin executable at `executable` as a provider for `tool_name`.
    pub fn new(tool_name: impl Into<String>, executable: PathBuf) -> Self {
        Self {
            tool_name: tool_name.into(),
            executable,
        }
    }

    /// Ask the plugin for its own manifest (name/version/description) — real
    /// data from the installed executable, not guessed from disk layout.
    pub fn manifest(&self) -> Result<Manifest> {
        self.call_json(&["manifest"])
    }

    /// The plugin's own executable, for callers that need to invoke a
    /// command outside the fixed `ToolProvider` protocol surface (e.g. a
    /// plugin-specific subcommand like `avm android avd list`) rather than
    /// one of the standard `manifest`/`versions`/`install`/... calls.
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Run the plugin with `args` under the read timeout (`AVM_PLUGIN_READ_TIMEOUT`) and parse stdout as JSON.
    /// Errors on spawn failure, non-zero exit, timeout, or malformed output.
    fn call_json<T: serde::de::DeserializeOwned>(&self, args: &[&str]) -> Result<T> {
        let mut cmd = Command::new(&self.executable);
        cmd.args(args);
        let timeout = env_timeout_ms("AVM_PLUGIN_READ_TIMEOUT", PLUGIN_PROCESS_READ_TIMEOUT_MS);
        let output = run_with_timeout(cmd, timeout).with_context(|| {
            format!(
                "plugin '{}' ({}) failed on `{}`",
                self.tool_name,
                self.executable.display(),
                args.join(" ")
            )
        })?;
        serde_json::from_str(output.trim()).with_context(|| {
            format!(
                "plugin '{}' ({}) returned malformed output for `{}`: {}",
                self.tool_name,
                self.executable.display(),
                args.join(" "),
                output.trim()
            )
        })
    }

    /// `env-vars` is read on every prompt and shim call, and is a fixed answer for a
    /// given plugin binary and version, so it's cached under `~/.avm/cache`. The key
    /// includes the plugin executable's mtime and size, so updating the plugin
    /// invalidates it. `avm prune` removes old entries.
    fn env_cache_path(&self, version: &str) -> Option<PathBuf> {
        avm_plugin_api::download::validate_version(version).ok()?;
        let meta = fs::metadata(&self.executable).ok()?;
        let nanos = meta
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let name = format!(
            "env-{}-{version}-{nanos}-{}.json",
            self.tool_name,
            meta.len()
        );
        Some(crate::shims::avm_home().ok()?.join("cache").join(name))
    }

    /// Run the plugin with `args` under the install timeout (`AVM_PLUGIN_INSTALL_TIMEOUT`), inheriting stdio so progress streams live.
    fn run_status(&self, args: &[&str]) -> Result<()> {
        let mut cmd = Command::new(&self.executable);
        cmd.args(args);
        run_timed(
            cmd,
            PLUGIN_PROCESS_INSTALL_TIMEOUT_MS,
            &format!("plugin '{}' `{}`", self.tool_name, args.join(" ")),
            "AVM_PLUGIN_INSTALL_TIMEOUT",
        )
    }
}

impl ToolProvider for PluginProcess {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn is_installed(&self, version: &str) -> bool {
        self.call_json::<avm_plugin_api::protocol::IsInstalledResponse>(&["is-installed", version])
            .map(|r| r.installed)
            .unwrap_or(false)
    }

    fn installed_versions(&self) -> Result<Vec<String>> {
        Ok(self
            .call_json::<avm_plugin_api::protocol::InstalledVersionsResponse>(&[
                "installed-versions",
            ])?
            .versions)
    }

    fn available_versions(&self, query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {
        let query_arg = query.to_arg();
        Ok(self
            .call_json::<avm_plugin_api::protocol::VersionsResponse>(&[
                "versions", "--query", &query_arg,
            ])?
            .versions)
    }

    fn executable_path(&self, version: &str) -> Result<Option<PathBuf>> {
        Ok(self
            .call_json::<avm_plugin_api::protocol::ExecutablePathResponse>(&[
                "executable-path",
                version,
            ])?
            .path
            .map(PathBuf::from))
    }

    fn env_vars(&self, version: &str) -> Result<HashMap<String, String>> {
        let cache = self.env_cache_path(version);
        if let Some(env) = cache
            .as_ref()
            .and_then(|p| fs::read(p).ok())
            .and_then(|raw| serde_json::from_slice(&raw).ok())
        {
            return Ok(env);
        }
        let env = self
            .call_json::<avm_plugin_api::protocol::EnvVarsResponse>(&["env-vars", version])?
            .env;
        // Best effort: a cache that can't be written just means a spawn next time.
        if let Some((path, raw)) = cache.zip(serde_json::to_vec(&env).ok()) {
            if path
                .parent()
                .is_some_and(|dir| fs::create_dir_all(dir).is_ok())
            {
                let _ = avm_plugin_api::storage::atomic_write(&path, &raw);
            }
        }
        Ok(env)
    }

    fn install(&self, version: &str) -> Result<()> {
        self.run_status(&["install", version])
    }

    fn uninstall(&self, version: &str) -> Result<()> {
        self.run_status(&["uninstall", version])
    }

    fn resolve_downloads(
        &self,
        version: &str,
        platforms: &[String],
    ) -> Result<avm_plugin_api::protocol::ResolveResponse> {
        let request = serde_json::to_string(&avm_plugin_api::protocol::ResolveRequest {
            version: version.to_string(),
            platforms: platforms.to_vec(),
        })?;
        self.call_json(&["resolve", &request])
    }

    fn install_locked(
        &self,
        version: &str,
        asset: &avm_plugin_api::protocol::LockedAsset,
    ) -> Result<()> {
        let asset = serde_json::to_string(asset)?;
        self.run_status(&["install-locked", version, &asset])
    }
}

use anyhow::{anyhow, Context, Result};
use avm_plugin_api::{
    AliasDetail, AliasValue, ExportResponse, Manifest, ResolvedAlias, ToolProvider, ToolVersion,
    ToolVersionQuery,
};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

const PLUGIN_TIMEOUT_MS: u64 = 500;
const GLOBAL_TIMEOUT_MS: u64 = 1000;
const ASDF_LIST_TIMEOUT_MS: u64 = 20_000;
const ASDF_INSTALL_TIMEOUT_MS: u64 = 120_000;
const GIT_CLONE_TIMEOUT_MS: u64 = 120_000;
const GIT_PULL_TIMEOUT_MS: u64 = 60_000;

fn timeout_from_env(var: &str, default_ms: u64) -> u64 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|secs| secs.saturating_mul(1000))
        .unwrap_or(default_ms)
}

/// Wait for an inherited-stdio child with a timeout. On timeout, the child is
/// killed and a descriptive error is returned naming the env var that can
/// override the deadline.
fn status_with_timeout(
    mut child: std::process::Child,
    timeout_ms: u64,
    label: &str,
    env_override: &str,
) -> Result<std::process::ExitStatus> {
    let timeout = Duration::from_millis(timeout_ms);
    match child
        .wait_timeout(timeout)
        .with_context(|| format!("failed while waiting for {label}"))?
    {
        Some(status) => Ok(status),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            Err(anyhow!(
                "{label} timed out after {}s — set {env_override}=<seconds> to extend",
                timeout_ms / 1000
            ))
        }
    }
}

#[derive(Debug)]
pub struct PluginManager {
    plugin_dir: PathBuf,
}

impl PluginManager {
    pub fn new(plugin_dir: Option<PathBuf>) -> Result<Self> {
        let dir = plugin_dir.unwrap_or_else(default_plugin_dir);
        fs::create_dir_all(&dir).context("create plugin directory")?;
        Ok(Self { plugin_dir: dir })
    }

    pub fn plugin_dir(&self) -> PathBuf {
        self.plugin_dir.clone()
    }

    pub fn list_aliases(&self, cwd: &Path) -> Result<HashMap<String, ResolvedAlias>> {
        if !self.plugin_dir.exists() {
            return Ok(HashMap::new());
        }

        let mut entries: Vec<_> = fs::read_dir(&self.plugin_dir)
            .context("unable to read plugin directory")?
            .filter_map(Result::ok)
            .filter(|entry| {
                let ft = entry.file_type().ok();
                ft.map(|ty| ty.is_dir()).unwrap_or(false)
            })
            .collect();

        entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_ascii_lowercase());

        let mut result = HashMap::new();
        let start = Instant::now();
        let global_timeout = Duration::from_millis(GLOBAL_TIMEOUT_MS);

        for entry in entries {
            if start.elapsed() > global_timeout {
                break;
            }

            let plugin_name = entry.file_name().to_string_lossy().to_string();
            let plugin_path = entry.path();
            match load_plugin_aliases(&plugin_path, cwd) {
                Ok(aliases) => {
                    for (key, alias) in aliases {
                        // First plugin wins while preserving directory sort order.
                        result.entry(key).or_insert(alias);
                    }
                }
                Err(err) if std::env::var("AVM_DEBUG").ok().as_deref() == Some("1") => {
                    eprintln!("[avm] plugin {plugin_name} skipped: {err}");
                }
                Err(_) => {}
            }
        }

        Ok(result)
    }

    pub fn list_plugins(&self) -> Result<HashMap<String, Manifest>> {
        if !self.plugin_dir.exists() {
            return Ok(HashMap::new());
        }

        let mut plugins = HashMap::new();
        for entry in fs::read_dir(&self.plugin_dir).context("failed reading plugin dir")? {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let ty = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if !ty.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(manifest) = read_manifest_path(&entry.path()) {
                plugins.insert(name, manifest);
            } else if is_protocol_plugin_source(&entry.path()) {
                let tool_name = protocol_tool_name(&name);
                let bin = entry.path().join("bin").join(binary_name("avm-plugin"));
                let manifest = PluginProcess::new(tool_name.clone(), bin)
                    .manifest()
                    .unwrap_or_else(|_| Manifest {
                        name: tool_name,
                        version: "unknown".to_string(),
                        api_version: None,
                        description: Some("installed via marketplace".to_string()),
                        section_label: None,
                        homepage: None,
                    });
                plugins.insert(name, manifest);
            } else if is_asdf_plugin_source(&entry.path()) {
                plugins.insert(name, asdf_manifest(&entry.path()));
            }
        }

        Ok(plugins)
    }

    pub fn read_manifest(&self, name: &str) -> Result<Manifest> {
        let plugin_path = self.plugin_dir.join(name);
        read_manifest_path(&plugin_path)
    }

    pub fn install_plugin(&self, source: &str) -> Result<()> {
        let is_remote = is_git_url(source);
        let target = if is_remote {
            let plugin_name = derive_remote_plugin_name(source)?;
            self.plugin_dir.join(plugin_name)
        } else {
            let source_metadata =
                fs::symlink_metadata(source).context("invalid plugin source path")?;
            if source_metadata.file_type().is_symlink() {
                return Err(anyhow!("plugin source must not be a symlink"));
            }

            let source_dir = fs::canonicalize(source).context("invalid plugin source path")?;
            validate_plugin_source_permissions(&source_dir)?;
            let source_name = source_dir
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("invalid plugin source"))?;
            self.plugin_dir.join(source_name)
        };

        if target.exists() {
            return Err(anyhow!(
                "plugin already installed; use `avm plugin update <name>` first",
            ));
        }

        let install_result = if is_remote {
            let child = Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg(source)
                .arg(&target)
                .env_clear()
                .env("PATH", default_plugin_path_env())
                .spawn()
                .context("failed to start git clone")?;
            let timeout = timeout_from_env("AVM_GIT_CLONE_TIMEOUT", GIT_CLONE_TIMEOUT_MS);
            let status = status_with_timeout(child, timeout, "git clone", "AVM_GIT_CLONE_TIMEOUT")?;
            if !status.success() {
                Err(anyhow!("git clone failed with status {}", status))
            } else {
                Ok(())
            }
        } else {
            let source = fs::canonicalize(source).context("invalid plugin source path")?;
            copy_dir_recursive(&source, &target).context("plugin copy failed")
        };

        if let Err(err) = install_result {
            let _ = fs::remove_dir_all(&target);
            return Err(err);
        }

        if is_avm_plugin_source(&target) {
            return Ok(());
        }
        if is_asdf_plugin_source(&target) {
            return Ok(());
        }

        if !target.join("plugin.json").exists() {
            let _ = fs::remove_dir_all(&target);
            return Err(anyhow!("invalid plugin: missing plugin.json"));
        }

        let _ = fs::remove_dir_all(&target);
        Err(anyhow!(
            "invalid plugin: missing bin/export-aliases or asdf bin/list-all + bin/install"
        ))
    }

    /// Accepts either the literal plugin directory name or a tool name that
    /// resolves to one via the `avm-plugin-<name>` / `asdf-<name>`
    /// conventions (`avm plugin remove node`, not just `avm plugin remove
    /// avm-plugin-node`), matching how `provider_by_name` finds it.
    pub fn remove_plugin(&self, name: &str) -> Result<()> {
        let literal = self.plugin_dir.join(name);
        let target = if literal.exists() {
            literal
        } else if let Some(dir) = self.find_protocol_plugin(name)? {
            dir
        } else if let Some((dir_name, _)) = self.find_asdf_plugin(name)? {
            self.plugin_dir.join(dir_name)
        } else {
            literal
        };
        if target.exists() {
            fs::remove_dir_all(target).context("remove plugin")?;
        }
        Ok(())
    }

    /// Accepts either the literal plugin directory name or a tool name
    /// (same resolution as `remove_plugin`/`provider_by_name`), and updates
    /// it the way it was installed:
    ///   - a marketplace install (`avm-plugin-<name>/bin/avm-plugin`, no
    ///     `.git`) re-fetches the latest release from the marketplace —
    ///     this used to silently no-op here, which is exactly the gap that
    ///     left a real install stuck on an old release with no error.
    ///   - a git-cloned plugin (asdf-style or otherwise) does `git pull`.
    ///   - anything else (a local, non-git source) has nothing to update.
    pub fn update_plugin(&self, name: &str) -> Result<()> {
        let literal = self.plugin_dir.join(name);
        let (target, dir_name) = if literal.exists() {
            (literal, name.to_string())
        } else if let Some(dir) = self.find_protocol_plugin(name)? {
            let dir_name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(name)
                .to_string();
            (dir, dir_name)
        } else if let Some((dir_name, dir)) = self.find_asdf_plugin(name)? {
            (dir, dir_name)
        } else {
            return Err(anyhow!("plugin '{}' not found", name));
        };

        if target.join(".git").exists() {
            let child = Command::new("git")
                .arg("-C")
                .arg(&target)
                .arg("pull")
                .arg("--ff-only")
                .env_clear()
                .env("PATH", default_plugin_path_env())
                .spawn()
                .context("failed to start git pull")?;
            let timeout = timeout_from_env("AVM_GIT_PULL_TIMEOUT", GIT_PULL_TIMEOUT_MS);
            let status = status_with_timeout(child, timeout, "git pull", "AVM_GIT_PULL_TIMEOUT")?;
            if !status.success() {
                return Err(anyhow!("plugin update failed with status {}", status));
            }
            return Ok(());
        }

        if is_protocol_plugin_source(&target) {
            let tool_name = protocol_tool_name(&dir_name);
            let entry = marketplace_lookup(&tool_name)?.ok_or_else(|| {
                anyhow!(
                    "'{tool_name}' isn't in the marketplace (was it installed from a direct \
                     URL instead of `avm plugin add {tool_name}`?) — nothing to update against"
                )
            })?;
            install_from_marketplace(&tool_name, &entry.repo, &self.plugin_dir)?;
            return Ok(());
        }

        // Local, non-git source (e.g. `avm plugin add ./my-plugin-dir`) —
        // there's nowhere to pull an update from.
        Ok(())
    }

    pub fn asdf_provider(&self, name: &str) -> Result<Option<AsdfToolProvider>> {
        let Some((plugin_name, plugin_path)) = self.find_asdf_plugin(name)? else {
            return Ok(None);
        };

        Ok(Some(AsdfToolProvider {
            name: name.to_string(),
            plugin_name,
            plugin_path,
        }))
    }

    /// Tier 2: a user-installed third-party plugin speaking the native
    /// protocol (`docs/migration/PLUGIN_PROTOCOL.md`). Directory convention
    /// mirrors the asdf one (`asdf-<name>` → tool `<name>`): a plugin
    /// directory named `avm-plugin-<name>` with an executable `bin/avm-plugin`
    /// is surfaced as tool `<name>`.
    pub fn protocol_provider(&self, name: &str) -> Result<Option<PluginProcess>> {
        let Some(plugin_path) = self.find_protocol_plugin(name)? else {
            return Ok(None);
        };
        Ok(Some(PluginProcess::new(
            name,
            plugin_path.join("bin").join(binary_name("avm-plugin")),
        )))
    }

    fn find_protocol_plugin(&self, name: &str) -> Result<Option<PathBuf>> {
        if !self.plugin_dir.exists() {
            return Ok(None);
        }

        let mut entries: Vec<_> = fs::read_dir(&self.plugin_dir)
            .context("unable to read plugin directory")?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false))
            .collect();
        entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_ascii_lowercase());

        for entry in entries {
            let plugin_path = entry.path();
            if !is_protocol_plugin_source(&plugin_path) {
                continue;
            }
            let plugin_name = entry.file_name().to_string_lossy().to_string();
            if protocol_tool_name(&plugin_name) == name {
                return Ok(Some(plugin_path));
            }
        }

        Ok(None)
    }

    pub fn list_asdf_provider_names(&self) -> Result<Vec<String>> {
        if !self.plugin_dir.exists() {
            return Ok(Vec::new());
        }

        let mut providers = Vec::new();
        for entry in fs::read_dir(&self.plugin_dir).context("unable to read plugin directory")? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if !entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false) {
                continue;
            }
            let plugin_path = entry.path();
            if !is_asdf_plugin_source(&plugin_path) {
                continue;
            }
            let plugin_name = entry.file_name().to_string_lossy().to_string();
            providers.push(asdf_tool_name(&plugin_name));
        }
        providers.sort_unstable();
        providers.dedup();
        Ok(providers)
    }

    fn find_asdf_plugin(&self, name: &str) -> Result<Option<(String, PathBuf)>> {
        if !self.plugin_dir.exists() {
            return Ok(None);
        }

        let mut entries: Vec<_> = fs::read_dir(&self.plugin_dir)
            .context("unable to read plugin directory")?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false))
            .collect();
        entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_ascii_lowercase());

        for entry in entries {
            let plugin_path = entry.path();
            if !is_asdf_plugin_source(&plugin_path) {
                continue;
            }

            let plugin_name = entry.file_name().to_string_lossy().to_string();
            if asdf_tool_name(&plugin_name) == name {
                return Ok(Some((plugin_name, plugin_path)));
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct AsdfToolProvider {
    name: String,
    plugin_name: String,
    plugin_path: PathBuf,
}

impl AsdfToolProvider {
    fn install_path(&self, version: &str) -> Result<PathBuf> {
        let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("HOME not set"))?;
        Ok(PathBuf::from(home)
            .join(".avm")
            .join("tools")
            .join(&self.name)
            .join(version))
    }

    fn bin_path_for(&self, version: &str, binary: &str) -> Result<Option<PathBuf>> {
        let install_path = self.install_path(version)?;
        let candidate = install_path.join("bin").join(binary_name(binary));
        if candidate.exists() {
            return Ok(Some(candidate));
        }
        Ok(None)
    }

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
            Some(path) => format!(". {} >/dev/null 2>&1; env -0", shell_single_quote(path)),
            None => "env -0".to_string(),
        };
        let mut cmd = Command::new("bash");
        cmd.arg("-c").arg(&script);
        cmd.env("ASDF_INSTALL_VERSION", version);
        cmd.env("ASDF_INSTALL_PATH", install_path);
        cmd.env("ASDF_INSTALL_TYPE", "version");
        let output = run_with_timeout(cmd, PLUGIN_TIMEOUT_MS * 4)?;

        let mut map = HashMap::new();
        for entry in output.split('\0') {
            if let Some((key, value)) = entry.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            }
        }
        Ok(map)
    }
}

fn shell_single_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

impl ToolProvider for AsdfToolProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_installed(&self, version: &str) -> bool {
        self.bin_path_for(version, &self.name)
            .ok()
            .flatten()
            .is_some()
    }

    fn installed_versions(&self) -> Result<Vec<String>> {
        let home = match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home),
            None => return Ok(Vec::new()),
        };
        let root = home.join(".avm").join("tools").join(&self.name);
        let mut versions = Vec::new();
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };

        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    versions.push(name.to_string());
                }
            }
        }

        versions.sort_unstable();
        Ok(versions)
    }

    fn available_versions(&self, query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {
        let output = self.run_asdf_command("list-all", None, ASDF_LIST_TIMEOUT_MS)?;
        let mut versions = output
            .split_whitespace()
            .filter(|version| matches_tool_query(version, &query))
            .map(|version| ToolVersion {
                version: version.to_string(),
                label: version.to_string(),
                channel: Some(self.plugin_name.clone()),
                is_lts: false,
                is_security: false,
            })
            .collect::<Vec<_>>();

        if matches!(query, ToolVersionQuery::Recent) {
            versions.truncate(10);
        } else if matches!(query, ToolVersionQuery::Latest) {
            versions.truncate(1);
        }

        Ok(versions)
    }

    fn executable_path(&self, version: &str) -> Result<Option<PathBuf>> {
        self.bin_path_for(version, &self.name)
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
        let install_path = self.install_path(version)?;
        if install_path
            .join("bin")
            .join(binary_name(&self.name))
            .exists()
        {
            return Ok(());
        }
        fs::create_dir_all(&install_path).context("failed to create asdf install path")?;
        if let Err(err) = self.run_asdf_command("install", Some(version), ASDF_INSTALL_TIMEOUT_MS) {
            let _ = fs::remove_dir_all(&install_path);
            return Err(err);
        }
        Ok(())
    }

    fn uninstall(&self, version: &str) -> Result<()> {
        let uninstall = self.plugin_path.join("bin").join("uninstall");
        if uninstall.exists() {
            self.run_asdf_command("uninstall", Some(version), ASDF_INSTALL_TIMEOUT_MS)?;
        }

        let install_path = self.install_path(version)?;
        if install_path.exists() {
            fs::remove_dir_all(install_path).context("failed to remove asdf-managed version")?;
        }
        Ok(())
    }
}

/// Host-side runner for avm's native plugin protocol (see
/// `docs/migration/PLUGIN_PROTOCOL.md`): a plugin is a standalone executable
/// speaking a small JSON-on-stdout contract for "read" calls, and plain
/// exit-code + inherited stdio for `install`/`uninstall` so progress streams
/// live. This implements the same `ToolProvider` trait as `AsdfToolProvider`
/// and the in-process builtins, so every existing call site in `avm-cli`
/// works unchanged regardless of which tier resolved the provider.
#[derive(Debug, Clone)]
pub struct PluginProcess {
    tool_name: String,
    executable: PathBuf,
}

const PLUGIN_PROCESS_READ_TIMEOUT_MS: u64 = 30_000;
const PLUGIN_PROCESS_INSTALL_TIMEOUT_MS: u64 = 1_800_000;

impl PluginProcess {
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

    fn call_json<T: serde::de::DeserializeOwned>(&self, args: &[&str]) -> Result<T> {
        let mut cmd = Command::new(&self.executable);
        cmd.args(args);
        let timeout = timeout_from_env("AVM_PLUGIN_READ_TIMEOUT", PLUGIN_PROCESS_READ_TIMEOUT_MS);
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

    fn run_status(&self, args: &[&str]) -> Result<()> {
        let child = Command::new(&self.executable)
            .args(args)
            .spawn()
            .with_context(|| format!("failed to start plugin '{}'", self.tool_name))?;
        let timeout = timeout_from_env(
            "AVM_PLUGIN_INSTALL_TIMEOUT",
            PLUGIN_PROCESS_INSTALL_TIMEOUT_MS,
        );
        let status = status_with_timeout(
            child,
            timeout,
            &format!("plugin '{}' `{}`", self.tool_name, args.join(" ")),
            "AVM_PLUGIN_INSTALL_TIMEOUT",
        )?;
        if !status.success() {
            return Err(anyhow!(
                "plugin '{}' `{}` failed: {}",
                self.tool_name,
                args.join(" "),
                status
            ));
        }
        Ok(())
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
            .call_json::<avm_plugin_api::protocol::InstalledVersionsResponse>(&["installed-versions"])?
            .versions)
    }

    fn available_versions(&self, query: ToolVersionQuery) -> Result<Vec<ToolVersion>> {
        let query_arg = query.to_arg();
        Ok(self
            .call_json::<avm_plugin_api::protocol::VersionsResponse>(&["versions", "--query", &query_arg])?
            .versions)
    }

    fn executable_path(&self, version: &str) -> Result<Option<PathBuf>> {
        Ok(self
            .call_json::<avm_plugin_api::protocol::ExecutablePathResponse>(&["executable-path", version])?
            .path
            .map(PathBuf::from))
    }

    fn env_vars(&self, version: &str) -> Result<HashMap<String, String>> {
        Ok(self
            .call_json::<avm_plugin_api::protocol::EnvVarsResponse>(&["env-vars", version])?
            .env)
    }

    fn install(&self, version: &str) -> Result<()> {
        self.run_status(&["install", version])
    }

    fn uninstall(&self, version: &str) -> Result<()> {
        self.run_status(&["uninstall", version])
    }
}

const DEFAULT_MARKETPLACE_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/PrajaNova/avm-marketplace/main/registry.json";
const MARKETPLACE_TIMEOUT_MS: u64 = 20_000;
const MARKETPLACE_INSTALL_TIMEOUT_MS: u64 = 300_000;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MarketplaceEntry {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub section_label: Option<String>,
    /// `<owner>/<repo>` — its GitHub Releases are the actual install source.
    /// Never fetched or built from source; only compiled release assets.
    pub repo: String,
}

#[derive(Debug, serde::Deserialize)]
struct RegistryFile {
    plugins: Vec<MarketplaceEntry>,
}

fn marketplace_registry_url() -> String {
    std::env::var("AVM_MARKETPLACE_URL").unwrap_or_else(|_| DEFAULT_MARKETPLACE_REGISTRY_URL.to_string())
}

/// Fetch and parse `registry.json` from the avm marketplace
/// (github.com/PrajaNova/avm-marketplace) — the list of known plugin names,
/// their descriptions, and the GitHub repo each resolves to. Override with
/// `AVM_MARKETPLACE_URL` (a `raw.githubusercontent.com`-style URL, or a
/// local file path for tests).
pub fn marketplace_registry() -> Result<Vec<MarketplaceEntry>> {
    let url = marketplace_registry_url();

    let local = Path::new(&url);
    let raw = if local.exists() {
        fs::read_to_string(local)
            .with_context(|| format!("failed to read marketplace registry from {}", local.display()))?
    } else {
        let mut cmd = Command::new("curl");
        cmd.arg("-fsSL")
            .arg("--connect-timeout")
            .arg("10")
            .arg("--max-time")
            .arg((MARKETPLACE_TIMEOUT_MS / 1000).to_string())
            .arg(&url);
        run_with_timeout(cmd, MARKETPLACE_TIMEOUT_MS)
            .with_context(|| format!("failed to fetch marketplace registry from {url}"))?
    };

    let parsed: RegistryFile =
        serde_json::from_str(&raw).context("failed to parse marketplace registry.json")?;
    Ok(parsed.plugins)
}

pub fn marketplace_lookup(name: &str) -> Result<Option<MarketplaceEntry>> {
    Ok(marketplace_registry()?.into_iter().find(|e| e.name == name))
}

fn marketplace_platform() -> Result<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        other => return Err(anyhow!("unsupported platform for marketplace install: {other}")),
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => return Err(anyhow!("unsupported architecture for marketplace install: {other}")),
    };
    Ok((os, arch))
}

#[derive(Debug, serde::Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Install a marketplace plugin by fetching its **compiled** latest GitHub
/// Release for the current platform — never source, never built locally.
/// `avm-plugin-<name>_<os>_<arch>.tar.gz` on `repo`'s latest release,
/// containing exactly one file named `avm-plugin-<name>`, is the expected
/// contract (documented in the avm-marketplace repo's README).
pub fn install_from_marketplace(name: &str, repo: &str, plugin_dir: &Path) -> Result<String> {
    let (os, arch) = marketplace_platform()?;
    let release_api = format!("https://api.github.com/repos/{repo}/releases/latest");

    let mut cmd = Command::new("curl");
    cmd.arg("-fsSL")
        .arg("-H")
        .arg("Accept: application/vnd.github+json")
        .arg("--connect-timeout")
        .arg("10")
        .arg("--max-time")
        .arg((MARKETPLACE_TIMEOUT_MS / 1000).to_string())
        .arg(&release_api);
    let body = run_with_timeout(cmd, MARKETPLACE_TIMEOUT_MS)
        .with_context(|| format!("failed to query latest release for {repo}"))?;
    let release: GithubRelease =
        serde_json::from_str(&body).with_context(|| format!("malformed release info for {repo}"))?;

    let asset_name = format!("avm-plugin-{name}_{os}_{arch}.tar.gz");
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            anyhow!(
                "release {} of {repo} has no asset named '{asset_name}' (no build for {os}/{arch})",
                release.tag_name
            )
        })?;

    let target_dir = plugin_dir.join(format!("avm-plugin-{name}"));
    let bin_dir = target_dir.join("bin");
    fs::create_dir_all(&bin_dir).context("failed to create plugin bin dir")?;

    let tmp = std::env::temp_dir().join(format!("avm-plugin-{name}-{}.tar.gz", std::process::id()));
    let mut download = Command::new("curl");
    download
        .arg("-fL")
        .arg("--connect-timeout")
        .arg("10")
        .arg("--max-time")
        .arg((MARKETPLACE_INSTALL_TIMEOUT_MS / 1000).to_string())
        .arg(&asset.browser_download_url)
        .arg("-o")
        .arg(&tmp);
    let child = download.spawn().context("failed to start download")?;
    let status = status_with_timeout(
        child,
        MARKETPLACE_INSTALL_TIMEOUT_MS,
        &format!("downloading {asset_name}"),
        "AVM_MARKETPLACE_INSTALL_TIMEOUT",
    )?;
    if !status.success() {
        return Err(anyhow!("failed to download {}: {status}", asset.browser_download_url));
    }

    let extract_dir = std::env::temp_dir().join(format!("avm-plugin-{name}-extract-{}", std::process::id()));
    fs::create_dir_all(&extract_dir).context("failed to create extraction temp dir")?;
    let mut tar = Command::new("tar");
    tar.arg("-xzf").arg(&tmp).arg("-C").arg(&extract_dir);
    let status = tar.status().context("failed to run tar")?;
    let _ = fs::remove_file(&tmp);
    if !status.success() {
        return Err(anyhow!("failed to extract {asset_name}"));
    }

    let extracted_bin = extract_dir.join(binary_name(&format!("avm-plugin-{name}")));
    if !extracted_bin.exists() {
        return Err(anyhow!(
            "{asset_name} did not contain the expected file avm-plugin-{name}"
        ));
    }
    let dest = bin_dir.join(binary_name("avm-plugin"));
    // On macOS, overwriting an existing binary at `dest` in place (as
    // `fs::copy` alone does) and then executing it shortly after — exactly
    // what `avm plugin update` does — can race the OS's Gatekeeper
    // "provenance sandbox" tracking and get the process SIGKILLed (exit
    // 137) on its first run or two. Removing the old file first, so the new
    // one lands on a fresh inode instead of overwriting in place, avoids it.
    // (Not fs::rename for the copy itself: the extraction temp dir and
    // ~/.avm/plugins can be on different filesystems/mounts — observed in
    // Docker, /tmp is tmpfs and the home dir is on the container's overlay
    // fs — rename() fails EXDEV across devices, copy+remove works
    // unconditionally.)
    let _ = fs::remove_file(&dest);
    fs::copy(&extracted_bin, &dest).context("failed to move plugin binary into place")?;
    let _ = fs::remove_file(&extracted_bin);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms)?;
    }
    let _ = fs::remove_dir_all(&extract_dir);

    Ok(release.tag_name)
}

fn default_plugin_dir() -> PathBuf {
    if let Ok(home) = std::env::var("AVM_PLUGIN_DIR") {
        return PathBuf::from(home);
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".avm").join("plugins");
    }

    PathBuf::from(".").join(".avm").join("plugins")
}

fn read_manifest_path(path: &Path) -> Result<Manifest> {
    let raw =
        fs::read_to_string(path.join("plugin.json")).context("unable to read plugin manifest")?;
    let manifest: Manifest = serde_json::from_str(&raw).context("invalid plugin manifest")?;
    Ok(manifest)
}

fn validate_plugin_source_permissions(path: &Path) -> Result<()> {
    let meta = fs::metadata(path).context("failed to read plugin source metadata")?;
    if !meta.is_dir() {
        return Err(anyhow!("plugin source must be a directory"));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        use std::os::unix::fs::PermissionsExt;
        if meta.uid() == 0 && current_euid() != 0 {
            return Err(anyhow!("plugin sources owned by root are not allowed"));
        }
        let mode = meta.permissions().mode();
        if mode & 0o002 != 0 {
            return Err(anyhow!("plugin source must not be world-writable"));
        }
    }

    Ok(())
}

#[cfg(unix)]
fn current_euid() -> u32 {
    extern "C" {
        fn geteuid() -> u32;
    }

    // SAFETY: geteuid has no arguments, does not mutate Rust-managed memory, and is always available on Unix.
    unsafe { geteuid() }
}

fn is_avm_plugin_source(path: &Path) -> bool {
    path.join("plugin.json").exists() && path.join("bin").join("export-aliases").exists()
}

fn is_asdf_plugin_source(path: &Path) -> bool {
    path.join("bin").join("list-all").exists() && path.join("bin").join("install").exists()
}

fn is_protocol_plugin_source(path: &Path) -> bool {
    path.join("bin").join(binary_name("avm-plugin")).exists()
}

fn protocol_tool_name(plugin_name: &str) -> String {
    plugin_name
        .strip_prefix("avm-plugin-")
        .unwrap_or(plugin_name)
        .to_string()
}

fn asdf_manifest(path: &Path) -> Manifest {
    let plugin_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("asdf-plugin")
        .to_string();
    let tool_name = asdf_tool_name(&plugin_name);

    Manifest {
        name: tool_name,
        version: "asdf-compatible".to_string(),
        api_version: Some(1),
        description: Some(format!("asdf-compatible plugin from {plugin_name}")),
        section_label: Some("asdf plugins".to_string()),
        homepage: None,
    }
}

fn asdf_tool_name(plugin_name: &str) -> String {
    plugin_name
        .strip_prefix("asdf-")
        .unwrap_or(plugin_name)
        .to_string()
}

fn sandbox_asdf_command(cmd: &mut Command, plugin_path: &Path) {
    cmd.env_clear();
    cmd.current_dir(plugin_path);
    cmd.env("ASDF_DIR", plugin_path);
    cmd.env("PATH", default_plugin_path_env());
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }
    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        cmd.env("TMPDIR", tmpdir);
    }
}

fn matches_tool_query(version: &str, query: &ToolVersionQuery) -> bool {
    match query {
        ToolVersionQuery::Recent | ToolVersionQuery::Latest => true,
        ToolVersionQuery::Major(major) => version_major(version) == Some(*major),
    }
}

fn version_major(version: &str) -> Option<u64> {
    let version = version
        .rsplit_once('-')
        .map(|(_, version)| version)
        .unwrap_or(version);
    version
        .split(['.', '+', '-'])
        .find(|part| !part.is_empty())
        .and_then(|part| part.parse::<u64>().ok())
}

fn binary_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn derive_remote_plugin_name(source: &str) -> Result<String> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("plugin source is empty"));
    }

    let mut candidate = trimmed;
    if candidate.starts_with("git@") {
        candidate = candidate
            .split_once(':')
            .map(|(_, tail)| tail)
            .unwrap_or(candidate);
    }

    candidate = candidate
        .split(['#', '?'])
        .next()
        .unwrap_or(candidate)
        .trim_end_matches('/');
    let name = candidate
        .split('/')
        .filter(|part| !part.is_empty())
        .last()
        .ok_or_else(|| anyhow!("unable to derive plugin name"))?;

    let name = name.strip_suffix(".git").unwrap_or(name);
    if name.is_empty() || name.contains('\0') || name.contains('/') || name.contains('\\') {
        return Err(anyhow!("invalid plugin name derived from source"));
    }
    if name.chars().any(|ch| ch.is_control()) {
        return Err(anyhow!("invalid plugin name derived from source"));
    }

    Ok(name.to_string())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).context("failed to create plugin destination")?;
    for entry in fs::read_dir(source).context("failed to read plugin source")? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src = entry.path();
        let dst = destination.join(entry.file_name());

        if file_type.is_symlink() {
            return Err(anyhow!(
                "plugin source contains symlink entries; remove symlinks before installing"
            ));
        }

        if file_type.is_dir() {
            copy_dir_recursive(&src, &dst)?;
            continue;
        }

        if file_type.is_file() {
            fs::copy(&src, &dst).with_context(|| format!("failed to copy {:?}", src))?;
            continue;
        }

        return Err(anyhow!("unsupported plugin source entry type: {:?}", src));
    }
    Ok(())
}

fn sandbox_plugin_command(cmd: &mut Command, plugin_path: &Path) {
    cmd.env_clear();
    cmd.current_dir(plugin_path);
    cmd.env("AVM_PLUGIN_DIR", plugin_path);
    cmd.env("PATH", default_plugin_path_env());
}

#[cfg(unix)]
fn default_plugin_path_env() -> &'static str {
    "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
}

#[cfg(not(unix))]
fn default_plugin_path_env() -> &'static str {
    r#"C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;C:\Windows\System32\WindowsPowerShell\v1.0\"#
}

fn run_with_timeout(mut cmd: Command, timeout_ms: u64) -> Result<String> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start plugin command")?;

    // Pipes have a small OS buffer (~64KB). Waiting for the child to exit
    // before reading them deadlocks the moment output exceeds that: the
    // child blocks mid-write with nobody draining the pipe, so it never
    // exits, so wait_timeout never returns Some — it just times out. Drain
    // both pipes on their own threads *while* waiting, so arbitrarily
    // large output never blocks the child.
    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut pipe) = stdout_pipe {
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut pipe) = stderr_pipe {
            let _ = pipe.read_to_string(&mut buf);
        }
        buf
    });

    let timeout = Duration::from_millis(timeout_ms);
    let status = child
        .wait_timeout(timeout)
        .context("failed while waiting for plugin command")?;
    let status = match status {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(anyhow!("plugin command timed out"));
        }
    };

    let out = stdout_reader.join().unwrap_or_default();
    let err = stderr_reader.join().unwrap_or_default();

    if !status.success() {
        if !err.trim().is_empty() {
            return Err(anyhow!("plugin command failed: {err}"));
        }
        return Err(anyhow!("plugin command failed with exit code"));
    }

    Ok(out)
}

fn normalize_section(manifest: &Manifest) -> String {
    manifest
        .section_label
        .clone()
        .unwrap_or_else(|| manifest.name.clone())
}

fn load_plugin_aliases(plugin_path: &Path, cwd: &Path) -> Result<HashMap<String, ResolvedAlias>> {
    let manifest = read_manifest_path(plugin_path)?;

    let wasm_hook = plugin_path.join("bin").join("export-aliases.wasm");
    let bin_hook = plugin_path.join("bin").join("export-aliases");

    if !bin_hook.exists() && !wasm_hook.exists() {
        return Err(anyhow!("missing export-aliases"));
    }

    let hook_output = if bin_hook.exists() {
        let health = plugin_path.join("bin").join("health-check");
        if health.exists() {
            let mut cmd = Command::new(health);
            cmd.arg("--dir").arg(cwd);
            sandbox_plugin_command(&mut cmd, plugin_path);
            if run_with_timeout(cmd, PLUGIN_TIMEOUT_MS).is_err() {
                return Err(anyhow!("plugin health-check failed"));
            }
        }

        let mut cmd = Command::new(bin_hook);
        cmd.arg("--dir").arg(cwd);
        sandbox_plugin_command(&mut cmd, plugin_path);
        run_with_timeout(cmd, PLUGIN_TIMEOUT_MS)?
    } else {
        return Err(anyhow!(
            "wasm plugin execution is not enabled in this baseline; please keep node scripts in the merged provider"
        ));
    };

    if hook_output.trim().is_empty() {
        return Ok(HashMap::new());
    }

    let response: ExportResponse =
        serde_json::from_str(&hook_output).context("invalid plugin response")?;
    let mut aliases = HashMap::new();
    let section = normalize_section(&manifest);

    for (key, value) in response.aliases {
        let mapped = match value {
            AliasValue::Simple(command) => AliasDetail {
                command,
                description: None,
                source: Some("plugin".to_string()),
            },
            AliasValue::Detailed(detail) => detail,
        };

        if mapped.command.trim().is_empty() {
            continue;
        }

        aliases.insert(
            key,
            ResolvedAlias {
                command: mapped.command,
                description: mapped.description,
                plugin_name: manifest.name.clone(),
                section_name: section.clone(),
                source: mapped.source,
            },
        );
    }

    Ok(aliases)
}

fn is_git_url(source: &str) -> bool {
    source.starts_with("https://")
        || source.starts_with("http://")
        || source.starts_with("git@")
        || source.starts_with("git://")
        || source.starts_with("ssh://")
}

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

    pub fn remove_plugin(&self, name: &str) -> Result<()> {
        let target = self.plugin_dir.join(name);
        if target.exists() {
            fs::remove_dir_all(target).context("remove plugin")?;
        }
        Ok(())
    }

    pub fn update_plugin(&self, name: &str) -> Result<()> {
        let target = self.plugin_dir.join(name);
        if !target.exists() {
            return Err(anyhow!("plugin '{}' not found", name));
        }

        if !target.join(".git").exists() {
            return Ok(());
        }

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

    let timeout = Duration::from_millis(timeout_ms);
    let status = child
        .wait_timeout(timeout)
        .context("failed while waiting for plugin command")?;
    let status = match status {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(anyhow!("plugin command timed out"));
        }
    };

    let mut out = String::new();
    let mut err = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut out);
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr.read_to_string(&mut err);
    }

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

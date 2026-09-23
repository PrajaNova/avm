use anyhow::{anyhow, Context, Result};
use avm_plugin_api::{
    env_timeout_ms, fetch, list_installed, remove_version, run_timed, tool_dir, wait_deadline,
    Manifest, ToolProvider, ToolVersion, ToolVersionQuery,
};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const ASDF_LIST_TIMEOUT_MS: u64 = 20_000;
const ASDF_ENV_TIMEOUT_MS: u64 = 2_000;
const ASDF_INSTALL_TIMEOUT_MS: u64 = 120_000;
const GIT_CLONE_TIMEOUT_MS: u64 = 120_000;
const GIT_PULL_TIMEOUT_MS: u64 = 60_000;
const PROTOCOL_PREFIX: &str = "avm-plugin-";
const ASDF_PREFIX: &str = "asdf-";

#[derive(Debug)]
pub struct PluginManager {
    plugin_dir: PathBuf,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        let dir = default_plugin_dir();
        fs::create_dir_all(&dir).context("create plugin directory")?;
        Ok(Self { plugin_dir: dir })
    }

    pub fn plugin_dir(&self) -> PathBuf {
        self.plugin_dir.clone()
    }

    /// Plugin directories as `(dir name, path)`, sorted case-insensitively.
    fn plugin_dirs(&self) -> Vec<(String, PathBuf)> {
        let mut dirs: Vec<(String, PathBuf)> = fs::read_dir(&self.plugin_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false))
            .map(|entry| (entry.file_name().to_string_lossy().to_string(), entry.path()))
            .collect();
        dirs.sort_by_key(|(name, _)| name.to_ascii_lowercase());
        dirs
    }

    /// First plugin dir that passes `is_kind` and maps to tool `name` once
    /// `prefix` (`avm-plugin-` / `asdf-`) is stripped.
    fn find(&self, name: &str, prefix: &str, is_kind: fn(&Path) -> bool) -> Option<(String, PathBuf)> {
        self.plugin_dirs()
            .into_iter()
            .find(|(dir, path)| is_kind(path) && tool_name(dir, prefix) == name)
    }

    /// Accepts either the literal plugin directory name or a tool name that
    /// resolves to one via the `avm-plugin-<name>` / `asdf-<name>` conventions.
    fn find_any(&self, name: &str) -> Option<(String, PathBuf)> {
        let literal = self.plugin_dir.join(name);
        if literal.exists() {
            return Some((name.to_string(), literal));
        }
        self.find(name, PROTOCOL_PREFIX, is_protocol_plugin_source)
            .or_else(|| self.find(name, ASDF_PREFIX, is_asdf_plugin_source))
    }

    pub fn list_plugins(&self) -> HashMap<String, Manifest> {
        let mut plugins = HashMap::new();
        for (name, path) in self.plugin_dirs() {
            if is_protocol_plugin_source(&path) {
                let tool = tool_name(&name, PROTOCOL_PREFIX).to_string();
                let manifest = PluginProcess::new(tool.clone(), path.join("bin").join("avm-plugin"))
                    .manifest()
                    .unwrap_or_else(|_| Manifest {
                        name: tool,
                        version: "unknown".to_string(),
                        api_version: None,
                        description: Some("installed via marketplace".to_string()),
                        section_label: None,
                        homepage: None,
                    });
                plugins.insert(name, manifest);
            } else if is_asdf_plugin_source(&path) {
                plugins.insert(name, asdf_manifest(&path));
            }
        }
        plugins
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
            let mut git = Command::new("git");
            git.args(["clone", "--depth", "1", source])
                .arg(&target)
                .env_clear()
                .env("PATH", DEFAULT_PLUGIN_PATH_ENV);
            run_timed(git, GIT_CLONE_TIMEOUT_MS, "git clone", "AVM_GIT_CLONE_TIMEOUT")
        } else {
            let source = fs::canonicalize(source).context("invalid plugin source path")?;
            copy_dir_recursive(&source, &target).context("plugin copy failed")
        };

        if let Err(err) = install_result {
            let _ = fs::remove_dir_all(&target);
            return Err(err);
        }

        if is_asdf_plugin_source(&target) {
            return Ok(());
        }

        let _ = fs::remove_dir_all(&target);
        Err(anyhow!("invalid plugin: missing asdf bin/list-all + bin/install"))
    }

    pub fn remove_plugin(&self, name: &str) -> Result<()> {
        if let Some((_, target)) = self.find_any(name) {
            fs::remove_dir_all(target).context("remove plugin")?;
        }
        Ok(())
    }

    /// Accepts either the literal plugin directory name or a tool name
    /// (same resolution as `remove_plugin`/`provider_by_name`), and updates
    /// it the way it was installed:
    ///   - a marketplace install (`avm-plugin-<name>/bin/avm-plugin`, no
    ///     `.git`) re-fetches the latest release from the marketplace.
    ///   - a git-cloned plugin (asdf-style or otherwise) does `git pull`.
    ///   - anything else (a local, non-git source) has nothing to update.
    pub fn update_plugin(&self, name: &str) -> Result<()> {
        let (dir_name, target) = self
            .find_any(name)
            .ok_or_else(|| anyhow!("plugin '{}' not found", name))?;

        if target.join(".git").exists() {
            let mut git = Command::new("git");
            git.arg("-C")
                .arg(&target)
                .args(["pull", "--ff-only"])
                .env_clear()
                .env("PATH", DEFAULT_PLUGIN_PATH_ENV);
            return run_timed(git, GIT_PULL_TIMEOUT_MS, "git pull", "AVM_GIT_PULL_TIMEOUT");
        }

        if is_protocol_plugin_source(&target) {
            let tool = tool_name(&dir_name, PROTOCOL_PREFIX);
            let entry = marketplace_lookup(tool)?.ok_or_else(|| {
                anyhow!(
                    "'{tool}' isn't in the marketplace (was it installed from a direct \
                     URL instead of `avm plugin add {tool}`?) — nothing to update against"
                )
            })?;
            install_from_marketplace(tool, &entry.repo, &self.plugin_dir)?;
            return Ok(());
        }

        // Local, non-git source (e.g. `avm plugin add ./my-plugin-dir`) —
        // there's nowhere to pull an update from.
        Ok(())
    }

    pub fn asdf_provider(&self, name: &str) -> Option<AsdfToolProvider> {
        let (plugin_name, plugin_path) = self.find(name, ASDF_PREFIX, is_asdf_plugin_source)?;
        Some(AsdfToolProvider {
            name: name.to_string(),
            plugin_name,
            plugin_path,
        })
    }

    /// A user-installed plugin speaking the native protocol
    /// (`docs/migration/PLUGIN_PROTOCOL.md`): a plugin directory named
    /// `avm-plugin-<name>` with an executable `bin/avm-plugin` is surfaced
    /// as tool `<name>`.
    pub fn protocol_provider(&self, name: &str) -> Option<PluginProcess> {
        let (_, plugin_path) = self.find(name, PROTOCOL_PREFIX, is_protocol_plugin_source)?;
        Some(PluginProcess::new(name, plugin_path.join("bin").join("avm-plugin")))
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
        Ok(tool_dir(&self.name)?.join(version))
    }

    fn bin_path(&self, version: &str) -> Option<PathBuf> {
        let candidate = self.install_path(version).ok()?.join("bin").join(&self.name);
        candidate.exists().then_some(candidate)
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
            Some(path) => format!(
                ". {} >/dev/null 2>&1; env -0",
                crate::cli::sh_quote(&path.to_string_lossy())
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
        let all = output
            .split_whitespace()
            .map(|version| ToolVersion {
                version: version.to_string(),
                label: version.to_string(),
                channel: Some(self.plugin_name.clone()),
                is_lts: false,
                is_security: false,
            })
            .collect();
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
const MARKETPLACE_TIMEOUT_SECS: u32 = 20;
const MARKETPLACE_INSTALL_TIMEOUT_MS: u64 = 300_000;
const DEFAULT_PLUGIN_PATH_ENV: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MarketplaceEntry {
    pub name: String,
    pub description: String,
    /// `<owner>/<repo>` — its GitHub Releases are the actual install source.
    /// Never fetched or built from source; only compiled release assets.
    pub repo: String,
}

#[derive(Debug, serde::Deserialize)]
struct RegistryFile {
    plugins: Vec<MarketplaceEntry>,
}

/// Fetch and parse `registry.json` from the avm marketplace
/// (github.com/PrajaNova/avm-marketplace) — the list of known plugin names,
/// their descriptions, and the GitHub repo each resolves to. Override with
/// `AVM_MARKETPLACE_URL` (a `raw.githubusercontent.com`-style URL, or a
/// local file path for tests).
pub fn marketplace_registry() -> Result<Vec<MarketplaceEntry>> {
    let url = std::env::var("AVM_MARKETPLACE_URL")
        .unwrap_or_else(|_| DEFAULT_MARKETPLACE_REGISTRY_URL.to_string());
    let raw = fetch(&url, MARKETPLACE_TIMEOUT_SECS)
        .with_context(|| format!("failed to fetch marketplace registry from {url}"))?;
    let parsed: RegistryFile =
        serde_json::from_slice(&raw).context("failed to parse marketplace registry.json")?;
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
    let body = fetch(&release_api, MARKETPLACE_TIMEOUT_SECS)
        .with_context(|| format!("failed to query latest release for {repo}"))?;
    let release: GithubRelease =
        serde_json::from_slice(&body).with_context(|| format!("malformed release info for {repo}"))?;

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

    let target_dir = plugin_dir.join(format!("{PROTOCOL_PREFIX}{name}"));
    let bin_dir = target_dir.join("bin");
    fs::create_dir_all(&bin_dir).context("failed to create plugin bin dir")?;

    let tmp = std::env::temp_dir().join(format!("avm-plugin-{name}-{}.tar.gz", std::process::id()));
    let mut download = Command::new("curl");
    download
        .args(["-fL", "--connect-timeout", "10"])
        .arg(&asset.browser_download_url)
        .arg("-o")
        .arg(&tmp);
    run_timed(
        download,
        MARKETPLACE_INSTALL_TIMEOUT_MS,
        &format!("downloading {asset_name}"),
        "AVM_MARKETPLACE_INSTALL_TIMEOUT",
    )?;

    let extract_dir = std::env::temp_dir().join(format!("avm-plugin-{name}-extract-{}", std::process::id()));
    fs::create_dir_all(&extract_dir).context("failed to create extraction temp dir")?;
    let mut tar = Command::new("tar");
    tar.arg("-xzf").arg(&tmp).arg("-C").arg(&extract_dir);
    let status = tar.status().context("failed to run tar")?;
    let _ = fs::remove_file(&tmp);
    if !status.success() {
        return Err(anyhow!("failed to extract {asset_name}"));
    }

    let extracted_bin = extract_dir.join(format!("avm-plugin-{name}"));
    if !extracted_bin.exists() {
        return Err(anyhow!(
            "{asset_name} did not contain the expected file avm-plugin-{name}"
        ));
    }
    let dest = bin_dir.join("avm-plugin");
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
    if let Ok(dir) = std::env::var("AVM_PLUGIN_DIR") {
        return PathBuf::from(dir);
    }
    crate::shims::avm_home()
        .map(|home| home.join("plugins"))
        .unwrap_or_else(|_| PathBuf::from(".avm/plugins"))
}

fn is_asdf_plugin_source(path: &Path) -> bool {
    path.join("bin").join("list-all").exists() && path.join("bin").join("install").exists()
}

fn is_protocol_plugin_source(path: &Path) -> bool {
    path.join("bin").join("avm-plugin").exists()
}

/// Plugin dir name → tool name (`avm-plugin-node` → `node`, `asdf-kotlin` → `kotlin`).
fn tool_name<'a>(dir: &'a str, prefix: &str) -> &'a str {
    dir.strip_prefix(prefix).unwrap_or(dir)
}

fn validate_plugin_source_permissions(path: &Path) -> Result<()> {
    let meta = fs::metadata(path).context("failed to read plugin source metadata")?;
    if !meta.is_dir() {
        return Err(anyhow!("plugin source must be a directory"));
    }

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

fn current_euid() -> u32 {
    extern "C" {
        fn geteuid() -> u32;
    }

    // SAFETY: geteuid has no arguments, does not mutate Rust-managed memory, and is always available on Unix.
    unsafe { geteuid() }
}

fn asdf_manifest(path: &Path) -> Manifest {
    let plugin_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("asdf-plugin")
        .to_string();
    Manifest {
        name: tool_name(&plugin_name, ASDF_PREFIX).to_string(),
        version: "asdf-compatible".to_string(),
        api_version: Some(1),
        description: Some(format!("asdf-compatible plugin from {plugin_name}")),
        section_label: Some("asdf plugins".to_string()),
        homepage: None,
    }
}

fn sandbox_asdf_command(cmd: &mut Command, plugin_path: &Path) {
    cmd.env_clear();
    cmd.current_dir(plugin_path);
    cmd.env("ASDF_DIR", plugin_path);
    cmd.env("PATH", DEFAULT_PLUGIN_PATH_ENV);
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }
    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        cmd.env("TMPDIR", tmpdir);
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
        .rfind(|part| !part.is_empty())
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

fn is_git_url(source: &str) -> bool {
    source.starts_with("https://")
        || source.starts_with("http://")
        || source.starts_with("git@")
        || source.starts_with("git://")
        || source.starts_with("ssh://")
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
    // exits, so the wait never returns Some — it just times out. Drain
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

    let status = wait_deadline(&mut child, timeout_ms)
        .context("failed while waiting for plugin command")?;
    let status = match status {
        Some(status) => status,
        None => {
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

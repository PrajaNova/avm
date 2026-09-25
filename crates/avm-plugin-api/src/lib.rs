use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    #[serde(rename = "api_version")]
    pub api_version: Option<u32>,
    pub description: Option<String>,
    pub section_label: Option<String>,
    pub homepage: Option<String>,
}

impl Manifest {
    pub fn new(name: &str, version: &str, description: &str, section_label: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            api_version: Some(1),
            description: Some(description.to_string()),
            section_label: Some(section_label.to_string()),
            homepage: Some(format!("https://github.com/PrajaNova/avm-plugin-{name}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedAlias {
    pub command: String,
    pub description: Option<String>,
    pub plugin_name: String,
    pub section_name: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ToolVersionQuery {
    Recent,
    Latest,
    Major(u64),
}

impl ToolVersionQuery {
    /// Wire form used as a plugin process's `--query` argument (see
    /// `docs/migration/PLUGIN_PROTOCOL.md`). Shared by the host (building the
    /// argument) and plugin executables (parsing it), so both sides always
    /// agree on the format without duplicating it.
    pub fn to_arg(&self) -> String {
        match self {
            ToolVersionQuery::Recent => "recent".to_string(),
            ToolVersionQuery::Latest => "latest".to_string(),
            ToolVersionQuery::Major(major) => format!("major:{major}"),
        }
    }

    /// Apply the query to a newest-first list; `major` extracts each item's major version.
    pub fn filter<T>(&self, items: Vec<T>, major: impl Fn(&T) -> u64) -> Vec<T> {
        match self {
            ToolVersionQuery::Recent => items,
            ToolVersionQuery::Latest => items.into_iter().take(1).collect(),
            ToolVersionQuery::Major(m) => items.into_iter().filter(|i| major(i) == *m).collect(),
        }
    }

    pub fn parse_arg(arg: &str) -> anyhow::Result<Self> {
        match arg {
            "recent" => Ok(ToolVersionQuery::Recent),
            "latest" => Ok(ToolVersionQuery::Latest),
            _ => {
                let major = arg
                    .strip_prefix("major:")
                    .ok_or_else(|| anyhow::anyhow!("unknown --query value: {arg}"))?;
                Ok(ToolVersionQuery::Major(major.parse().map_err(|_| {
                    anyhow::anyhow!("invalid major version in --query: {arg}")
                })?))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolVersion {
    pub version: String,
    pub label: String,
    pub channel: Option<String>,
    pub is_lts: bool,
    pub is_security: bool,
}

/// `~/.avm/tools/<tool>` — where every provider keeps `<version>/` dirs.
pub fn tool_dir(tool: &str) -> anyhow::Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME not set"))?;
    Ok(PathBuf::from(home).join(".avm").join("tools").join(tool))
}

/// Sorted version dirs under `tool_dir(tool)` that pass `is_installed`.
pub fn list_installed(tool: &str, is_installed: impl Fn(&str) -> bool) -> anyhow::Result<Vec<String>> {
    let entries = match std::fs::read_dir(tool_dir(tool)?) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(anyhow::Error::new(err).context(format!("failed to read {tool} tools dir"))),
    };
    let mut versions: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|v| is_installed(v))
        .collect();
    versions.sort_unstable();
    Ok(versions)
}

pub fn remove_version(tool: &str, version: &str) -> anyhow::Result<()> {
    let target = tool_dir(tool)?.join(version);
    if target.exists() {
        std::fs::remove_dir_all(&target)
            .map_err(|e| anyhow::anyhow!("failed to remove managed {tool} {version}: {e}"))?;
    }
    Ok(())
}

/// Wait for `child` up to `ms`; kills it and returns `None` on timeout.
pub fn wait_deadline(child: &mut std::process::Child, ms: u64) -> std::io::Result<Option<std::process::ExitStatus>> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// `$env_var` (seconds) overrides `default_ms`.
pub fn env_timeout_ms(env_var: &str, default_ms: u64) -> u64 {
    std::env::var(env_var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|secs| secs.saturating_mul(1000))
        .unwrap_or(default_ms)
}

/// Spawn `cmd`, fail on non-zero exit or on timeout (`$env_var` seconds, else `default_ms`).
pub fn run_timed(mut cmd: std::process::Command, default_ms: u64, label: &str, env_var: &str) -> anyhow::Result<()> {
    let ms = env_timeout_ms(env_var, default_ms);
    let mut child = cmd.spawn().map_err(|e| anyhow::anyhow!("failed to spawn {label}: {e}"))?;
    match wait_deadline(&mut child, ms).map_err(|e| anyhow::anyhow!("failed while waiting for {label}: {e}"))? {
        None => Err(anyhow::anyhow!("{label} timed out after {}s — set {env_var}=<seconds> to extend", ms / 1000)),
        Some(status) if !status.success() => Err(anyhow::anyhow!("{label} failed: {status}")),
        Some(_) => Ok(()),
    }
}

/// Read `url_or_path` from disk if it exists (tests / offline mirrors), else `curl` it.
pub fn fetch(url_or_path: &str, max_time_secs: u32) -> anyhow::Result<Vec<u8>> {
    let local = std::path::Path::new(url_or_path);
    if local.exists() {
        return std::fs::read(local).map_err(|e| anyhow::anyhow!("failed to read {}: {e}", local.display()));
    }
    let output = std::process::Command::new("curl")
        .args(["-fsSL", "--connect-timeout", "10", "--max-time", &max_time_secs.to_string(), url_or_path])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to fetch {url_or_path}: {e}"))?;
    if !output.status.success() {
        anyhow::bail!("failed to fetch {url_or_path}: curl exited with {}", output.status);
    }
    Ok(output.stdout)
}

/// Lowercase hex sha256 of the file at `path`, streamed.
pub fn sha256_file(path: &std::path::Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| anyhow::anyhow!("failed to open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Check `path` against the entry for `name` in `checksums` (`sha256sum`
/// output: `<hex>  <name>` or `<hex> *<name>` per line). Returns the hash on
/// a match; errors if `name` is unlisted or the hash differs.
pub fn verify_sha256(path: &std::path::Path, checksums: &str, name: &str) -> anyhow::Result<String> {
    let expected = checksums
        .lines()
        .filter_map(|line| line.split_once(char::is_whitespace))
        .find(|(_, file)| file.trim_start().trim_start_matches('*') == name)
        .map(|(hash, _)| hash.to_ascii_lowercase())
        .ok_or_else(|| anyhow::anyhow!("checksums file has no entry for {name}"))?;
    let actual = sha256_file(path)?;
    if actual != expected {
        anyhow::bail!("checksum mismatch for {name}\n  expected {expected}\n  got      {actual}");
    }
    Ok(actual)
}

/// Wire responses a plugin process prints as one JSON document on stdout for
/// its "read" commands. See `docs/migration/PLUGIN_PROTOCOL.md` for the full
/// contract, including why `install`/`uninstall` are deliberately NOT part
/// of this JSON envelope (they stream human progress with stdio inherited
/// and signal success/failure via exit code instead).
pub mod protocol {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct VersionsResponse {
        pub versions: Vec<ToolVersion>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IsInstalledResponse {
        pub installed: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InstalledVersionsResponse {
        pub versions: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ExecutablePathResponse {
        pub path: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EnvVarsResponse {
        pub env: HashMap<String, String>,
    }
}

pub trait ToolProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_installed(&self, version: &str) -> bool;
    fn installed_versions(&self) -> anyhow::Result<Vec<String>>;
    fn available_versions(&self, query: ToolVersionQuery) -> anyhow::Result<Vec<ToolVersion>>;
    fn executable_path(&self, version: &str) -> anyhow::Result<Option<PathBuf>>;
    /// Environment variables this tool should export when selected (e.g.
    /// `ANDROID_HOME`, `JAVA_HOME`). Defaults to none; providers override.
    fn env_vars(&self, _version: &str) -> anyhow::Result<HashMap<String, String>> {
        Ok(HashMap::new())
    }
    fn install(&self, version: &str) -> anyhow::Result<()>;
    fn uninstall(&self, version: &str) -> anyhow::Result<()>;
}

/// Shared CLI dispatch for a plugin executable's `main.rs`. Every plugin
/// (builtin or third-party) that wraps a `ToolProvider` implementation calls
/// `runner::run(manifest, &provider)` from `main` — this is the plugin-side
/// half of the contract `avm_runtime::PluginProcess` speaks from the host.
/// See `docs/migration/PLUGIN_PROTOCOL.md` for the wire format.
pub mod runner {
    use super::*;
    use std::process::ExitCode;

    pub fn run(manifest: Manifest, provider: &dyn ToolProvider) -> ExitCode {
        match dispatch(&manifest, provider, &std::env::args().skip(1).collect::<Vec<_>>()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::FAILURE
            }
        }
    }

    fn dispatch(manifest: &Manifest, provider: &dyn ToolProvider, args: &[String]) -> anyhow::Result<()> {
        let (command, rest) = args
            .split_first()
            .ok_or_else(|| anyhow::anyhow!("missing plugin command"))?;

        match command.as_str() {
            "manifest" => print_json(manifest),
            "versions" => {
                let query = parse_query_flag(rest)?;
                print_json(&protocol::VersionsResponse {
                    versions: provider.available_versions(query)?,
                })
            }
            "is-installed" => {
                let version = require_version(rest)?;
                print_json(&protocol::IsInstalledResponse {
                    installed: provider.is_installed(version),
                })
            }
            "installed-versions" => print_json(&protocol::InstalledVersionsResponse {
                versions: provider.installed_versions()?,
            }),
            "executable-path" => {
                let version = require_version(rest)?;
                print_json(&protocol::ExecutablePathResponse {
                    path: provider
                        .executable_path(version)?
                        .map(|p| p.to_string_lossy().to_string()),
                })
            }
            "env-vars" => {
                let version = require_version(rest)?;
                print_json(&protocol::EnvVarsResponse {
                    env: provider.env_vars(version)?,
                })
            }
            "install" => provider.install(require_version(rest)?),
            "uninstall" => provider.uninstall(require_version(rest)?),
            other => Err(anyhow::anyhow!("unknown plugin command: {other}")),
        }
    }

    fn require_version(args: &[String]) -> anyhow::Result<&str> {
        args.first()
            .map(String::as_str)
            .ok_or_else(|| anyhow::anyhow!("version argument required"))
    }

    fn parse_query_flag(args: &[String]) -> anyhow::Result<ToolVersionQuery> {
        match args {
            [flag, value] if flag == "--query" => ToolVersionQuery::parse_arg(value),
            [] => Ok(ToolVersionQuery::Recent),
            _ => Err(anyhow::anyhow!("usage: versions [--query recent|latest|major:<N>]")),
        }
    }

    fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
        println!("{}", serde_json::to_string(value)?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_filter() {
        let v = vec![22u64, 21, 20, 22];
        assert_eq!(ToolVersionQuery::Recent.filter(v.clone(), |x| *x), v);
        assert_eq!(ToolVersionQuery::Latest.filter(v.clone(), |x| *x), vec![22]);
        assert_eq!(ToolVersionQuery::Major(22).filter(v, |x| *x), vec![22, 22]);
    }

    #[test]
    fn verify_sha256_matches_and_rejects() {
        let path = std::env::temp_dir().join(format!("avm-sha-test-{}", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(sha256_file(&path).unwrap(), abc);
        let sums = format!("{}  other.tar.gz\n{abc}  a.tar.gz\n", "0".repeat(64));
        assert_eq!(verify_sha256(&path, &sums, "a.tar.gz").unwrap(), abc);
        assert_eq!(verify_sha256(&path, &format!("{abc} *a.tar.gz"), "a.tar.gz").unwrap(), abc);
        assert!(verify_sha256(&path, &sums, "other.tar.gz").unwrap_err().to_string().contains("mismatch"));
        assert!(verify_sha256(&path, &sums, "missing.tar.gz").unwrap_err().to_string().contains("no entry"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn run_timed_reports_timeout_and_failure() {
        let mut sleep = std::process::Command::new("sleep");
        sleep.arg("5");
        assert!(run_timed(sleep, 100, "sleep", "AVM_TEST_UNSET").unwrap_err().to_string().contains("timed out"));
        assert!(run_timed(std::process::Command::new("false"), 5_000, "false", "AVM_TEST_UNSET").is_err());
        assert!(run_timed(std::process::Command::new("true"), 5_000, "true", "AVM_TEST_UNSET").is_ok());
    }
}

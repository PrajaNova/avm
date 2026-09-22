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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasDetail {
    pub command: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AliasValue {
    Simple(String),
    Detailed(AliasDetail),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResponse {
    pub api_version: Option<u32>,
    pub aliases: HashMap<String, AliasValue>,
}

#[derive(Debug, Clone)]
pub struct ResolvedAlias {
    pub command: String,
    pub description: Option<String>,
    pub plugin_name: String,
    pub section_name: String,
    pub source: Option<String>,
}

impl From<(&str, AliasValue, &Manifest)> for ResolvedAlias {
    fn from((plugin_name, value, manifest): (&str, AliasValue, &Manifest)) -> Self {
        match value {
            AliasValue::Simple(command) => Self {
                command,
                description: None,
                plugin_name: plugin_name.to_string(),
                section_name: manifest
                    .section_label
                    .clone()
                    .unwrap_or_else(|| plugin_name.to_string()),
                source: Some("script".to_string()),
            },
            AliasValue::Detailed(detail) => Self {
                command: detail.command,
                description: detail.description,
                plugin_name: plugin_name.to_string(),
                section_name: manifest
                    .section_label
                    .clone()
                    .unwrap_or_else(|| plugin_name.to_string()),
                source: detail.source,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolResolvedPath {
    pub path: PathBuf,
    pub version: String,
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

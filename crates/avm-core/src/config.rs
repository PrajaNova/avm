use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub tools: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ConfigLoadResult {
    pub aliases: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub tools: HashMap<String, String>,
    pub is_structured: bool,
}

fn validate_env_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }

    let mut chars = key.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }

    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            return false;
        }
    }

    true
}

fn parse_config(raw: &[u8]) -> Result<ConfigLoadResult> {
    let root: serde_json::Value = serde_json::from_slice(raw).context("invalid config json")?;

    match root {
        serde_json::Value::Object(object) => {
            let has_aliases = object.contains_key("aliases");
            let has_env = object.contains_key("env");
            let has_tools = object.contains_key("tools");

            if has_aliases || has_env || has_tools {
                let aliases = parse_string_map(object.get("aliases"), "aliases")?;
                let env = parse_string_map(object.get("env"), "env")?;
                let tools = parse_string_map(object.get("tools"), "tools")?;

                for key in env.keys() {
                    if !validate_env_key(key) {
                        return Err(anyhow::anyhow!("invalid env key: {key}"));
                    }
                }

                return Ok(ConfigLoadResult {
                    aliases,
                    env,
                    tools,
                    is_structured: true,
                });
            }

            let aliases = serde_json::from_value::<HashMap<String, String>>(
                serde_json::Value::Object(object),
            )
            .context("invalid legacy flat config")?;
            return Ok(ConfigLoadResult {
                aliases,
                env: HashMap::new(),
                tools: HashMap::new(),
                is_structured: false,
            });
        }
        serde_json::Value::Null => Ok(ConfigLoadResult {
            aliases: HashMap::new(),
            env: HashMap::new(),
            tools: HashMap::new(),
            is_structured: true,
        }),
        _ => Err(anyhow::anyhow!("invalid config format")),
    }
}

fn parse_string_map(
    value: Option<&serde_json::Value>,
    section: &str,
) -> Result<HashMap<String, String>> {
    match value {
        None | Some(serde_json::Value::Null) => Ok(HashMap::new()),
        Some(value) => serde_json::from_value(value.clone())
            .with_context(|| format!("invalid structured config section: {section}")),
    }
}

pub fn load_with_env(root: impl AsRef<Path>, local_file: &str) -> Result<ConfigLoadResult> {
    let root = root.as_ref();
    let file_path = root.join(local_file);

    match fs::read(&file_path) {
        Ok(raw) => match parse_config(&raw) {
            Ok(cfg) => Ok(cfg),
            Err(err) => {
                // Don't let a corrupt config block every avm command. Back the
                // file up with a timestamped suffix and continue with an empty
                // config. The user can copy values back from the .broken file.
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let backup = file_path.with_extension(format!("broken-{stamp}.json"));
                let _ = fs::rename(&file_path, &backup);
                eprintln!(
                    "warning: {} was malformed ({err}); backed up to {} and continuing with an empty config.",
                    file_path.display(),
                    backup.display()
                );
                Ok(ConfigLoadResult {
                    aliases: HashMap::new(),
                    env: HashMap::new(),
                    tools: HashMap::new(),
                    is_structured: true,
                })
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(ConfigLoadResult {
            aliases: HashMap::new(),
            env: HashMap::new(),
            tools: HashMap::new(),
            is_structured: true,
        }),
        Err(err) => Err(err.into()),
    }
}

pub fn load_config(root: impl AsRef<Path>, local_file: &str) -> Result<ConfigLoadResult> {
    load_with_env(root, local_file)
}

pub fn write_default_config(root: impl AsRef<Path>, local_file: &str) -> Result<()> {
    let path = root.as_ref().join(local_file);
    let cfg = ConfigFile {
        aliases: HashMap::new(),
        env: HashMap::new(),
        tools: HashMap::new(),
    };
    let raw = serde_json::to_vec_pretty(&cfg)?;
    fs::write(path, raw).context("unable to create default config")
}

pub fn save_config(
    root: impl AsRef<Path>,
    local_file: &str,
    aliases: &HashMap<String, String>,
    env: &HashMap<String, String>,
    tools: &HashMap<String, String>,
    structured: bool,
) -> Result<()> {
    let mut path = PathBuf::from(root.as_ref());
    path.push(local_file);

    let mut aliases = aliases.clone();
    if aliases.is_empty() {
        aliases = HashMap::new();
    }

    if !structured {
        let raw = serde_json::to_vec_pretty(&aliases)?;
        fs::write(path, raw).context("failed to save flat config")
    } else {
        let cfg = ConfigFile {
            aliases,
            env: env.clone(),
            tools: tools.clone(),
        };
        for key in cfg.env.keys() {
            if !validate_env_key(key) {
                return Err(anyhow::anyhow!("invalid env key: {key}"));
            }
        }
        let raw = serde_json::to_vec_pretty(&cfg)?;
        fs::write(path, raw).context("failed to save structured config")
    }
}

pub fn save_flat_legacy(
    root: impl AsRef<Path>,
    local_file: &str,
    aliases: &HashMap<String, String>,
) -> Result<()> {
    save_config(
        root,
        local_file,
        aliases,
        &HashMap::new(),
        &HashMap::new(),
        false,
    )
}

pub fn migrate_legacy_if_needed(root: impl AsRef<Path>, local_file: &str) -> Result<bool> {
    let parsed = load_with_env(&root, local_file)?;
    if parsed.is_structured {
        return Ok(false);
    }
    save_config(
        root,
        local_file,
        &parsed.aliases,
        &parsed.env,
        &parsed.tools,
        true,
    )?;
    Ok(true)
}

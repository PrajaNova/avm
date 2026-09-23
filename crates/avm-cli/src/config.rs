use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const CONFIG_FILE: &str = ".avm.json";

#[derive(Debug, Clone, Default, Serialize)]
pub struct ConfigLoadResult {
    pub aliases: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub tools: HashMap<String, String>,
}

fn validate_env_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .enumerate()
            .all(|(i, c)| c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
}

fn validate_env(env: &HashMap<String, String>) -> Result<()> {
    match env.keys().find(|key| !validate_env_key(key)) {
        Some(key) => Err(anyhow!("invalid env key: {key}")),
        None => Ok(()),
    }
}

fn parse_config(raw: &[u8]) -> Result<ConfigLoadResult> {
    let root: serde_json::Value = serde_json::from_slice(raw).context("invalid config json")?;

    match root {
        serde_json::Value::Object(object) => {
            if !["aliases", "env", "tools"].iter().any(|k| object.contains_key(*k)) {
                // Legacy flat `{ "alias": "command" }` file; the next save
                // rewrites it in the structured form.
                let aliases = serde_json::from_value(serde_json::Value::Object(object))
                    .context("invalid legacy flat config")?;
                return Ok(ConfigLoadResult {
                    aliases,
                    ..Default::default()
                });
            }

            let cfg = ConfigLoadResult {
                aliases: parse_string_map(object.get("aliases"), "aliases")?,
                env: parse_string_map(object.get("env"), "env")?,
                tools: parse_string_map(object.get("tools"), "tools")?,
            };
            validate_env(&cfg.env)?;
            Ok(cfg)
        }
        serde_json::Value::Null => Ok(ConfigLoadResult::default()),
        _ => Err(anyhow!("invalid config format")),
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

/// Load `<root>/.avm.json`; a missing file is an empty config.
pub fn load(root: &Path) -> Result<ConfigLoadResult> {
    let file_path = root.join(CONFIG_FILE);

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
                Ok(ConfigLoadResult::default())
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(ConfigLoadResult::default()),
        Err(err) => Err(err.into()),
    }
}

/// Write `<root>/.avm.json`, always in the structured form.
pub fn save(root: &Path, cfg: &ConfigLoadResult) -> Result<()> {
    validate_env(&cfg.env)?;
    let raw = serde_json::to_vec_pretty(cfg)?;
    fs::write(root.join(CONFIG_FILE), raw).context("failed to save config")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_keys_and_legacy_flat_config() {
        assert!(validate_env_key("_A1") && validate_env_key("PATH"));
        assert!(!validate_env_key("") && !validate_env_key("1A") && !validate_env_key("A-B"));
        let flat = parse_config(br#"{"dev":"npm run dev"}"#).unwrap();
        assert_eq!(flat.aliases["dev"], "npm run dev");
        assert!(parse_config(br#"{"env":{"1BAD":"x"}}"#).is_err());
    }
}

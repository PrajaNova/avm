use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const CONFIG_FILE: &str = ".avm.json";

#[derive(Debug, Clone, Default, Serialize)]
pub struct ConfigLoadResult {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub root: bool,
    pub aliases: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub tools: HashMap<String, String>,
    /// Global config only: project dirs trusted without `avm trust` (globs, `~` allowed).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trusted_paths: Vec<String>,
    /// Global config only: `false` stops reading `.nvmrc`, `.java-version`, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idiomatic_version_files: Option<bool>,
}

const SECTIONS: &[&str] = &["aliases", "env", "tools", "trusted_paths", "idiomatic_version_files", "root"];

fn validate_env_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .enumerate()
            .all(|(i, c)| c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
}

pub(crate) fn validate_env(env: &HashMap<String, String>) -> Result<()> {
    match env.keys().find(|key| !validate_env_key(key)) {
        Some(key) => Err(anyhow!("invalid env key: {key}")),
        None => Ok(()),
    }
}

fn parse_config(raw: &[u8]) -> Result<ConfigLoadResult> {
    let root: serde_json::Value = serde_json::from_slice(raw).context("invalid config json")?;

    match root {
        serde_json::Value::Object(object) => {
            if !SECTIONS.iter().any(|k| object.contains_key(*k)) {
                // Legacy flat `{ "alias": "command" }` file; the next save
                // rewrites it in the structured form.
                let aliases = serde_json::from_value(serde_json::Value::Object(object))
                    .context("invalid legacy flat config")?;
                return Ok(ConfigLoadResult {
                    aliases,
                    ..Default::default()
                });
            }

            let mut cfg = ConfigLoadResult {
                root: match object.get("root") {
                    None => false,
                    Some(v) => v.as_bool().ok_or_else(|| anyhow!("root must be a boolean"))?,
                },
                aliases: parse_string_map(object.get("aliases"), "aliases")?,
                env: parse_string_map(object.get("env"), "env")?,
                tools: parse_string_map(object.get("tools"), "tools")?,
                trusted_paths: match object.get("trusted_paths") {
                    None | Some(serde_json::Value::Null) => Vec::new(),
                    Some(v) => serde_json::from_value(v.clone()).context("trusted_paths must be a list of strings")?,
                },
                idiomatic_version_files: object.get("idiomatic_version_files").and_then(|v| v.as_bool()),
            };
            validate_env(&cfg.env)?;
            if cfg!(windows) {
                cfg.env = cfg.env.into_iter().map(|(key, value)| (key.to_ascii_uppercase(), value)).collect();
            }
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
    load_file(&root.join(CONFIG_FILE))
}

pub fn load_file(file_path: &Path) -> Result<ConfigLoadResult> {
    match fs::read(file_path) {
        Ok(raw) => parse_config(&raw).with_context(|| format!("invalid {}; repair the file before continuing", file_path.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(ConfigLoadResult::default()),
        Err(err) => Err(err.into()),
    }
}

/// Write `<root>/.avm.json`, always in the structured form.
pub fn save(root: &Path, cfg: &ConfigLoadResult) -> Result<()> {
    validate_env(&cfg.env)?;
    let raw = serde_json::to_vec_pretty(cfg)?;
    avm_plugin_api::storage::atomic_write(&root.join(CONFIG_FILE), &raw).context("failed to save config")
}

/// Low to high precedence within a directory. Profiles cannot name paths.
pub fn file_names(profiles: &str) -> Result<Vec<String>> {
    let mut names = vec![CONFIG_FILE.to_string()];
    if !profiles.is_empty() {
        for profile in profiles.split(',') {
            let profile = profile.trim();
            if profile.is_empty() || !profile.bytes().all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c)) {
                return Err(anyhow!("invalid environment profile: {profile}"));
            }
            if profile != "local" {
                names.push(format!(".avm.{profile}.json"));
            }
        }
    }
    names.push(".avm.local.json".to_string());
    Ok(names)
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
        let settings = parse_config(br#"{"trusted_paths":["~/work/**"],"idiomatic_version_files":false}"#).unwrap();
        assert!(settings.aliases.is_empty() && settings.trusted_paths == ["~/work/**"]);
        assert_eq!(settings.idiomatic_version_files, Some(false));
        assert!(parse_config(br#"{"root":true}"#).unwrap().root);
        assert!(parse_config(br#"{"root":"yes"}"#).is_err());
        assert!(!String::from_utf8(serde_json::to_vec(&ConfigLoadResult::default()).unwrap()).unwrap().contains("trusted"));
    }
}

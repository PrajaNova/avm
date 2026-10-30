//! The plugin registry, `marketplace/registry.json`:
//! `{ "schema_version": 1, "plugins": [{ "name", "description", "repo", ... }] }`.
//! It only names the repo; the binary always comes from the newest `avm-plugin-<name>-v*`
//! release of that repo, so a new plugin version never needs a registry change. Entries may
//! still carry `section_label` (the website) and `release_tag` (the pin older avm releases
//! install); both are ignored here.
use super::*;
use std::collections::BTreeMap;

/// The only registry layout this avm understands; a newer one asks the user to update avm.
const SCHEMA_VERSION: u32 = 1;

/// The file as published: a schema version and the list of plugins.
#[derive(Debug, serde::Deserialize)]
struct RegistryFile {
    schema_version: u32,
    plugins: Vec<RegistryEntry>,
}

/// One registry entry: the plugin name, the repo whose releases carry it, a description and
/// optional search tags.
#[derive(Debug, serde::Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub repo: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub type Registry = BTreeMap<String, RegistryEntry>;

/// Where the registry is read from: `AVM_REGISTRY_URL` (a URL or a local file), else the file on the main branch.
fn registry_url() -> String {
    std::env::var("AVM_REGISTRY_URL").unwrap_or_else(|_| REGISTRY_URL.to_string())
}

/// Fetches and validates the registry; errors say where it was read from.
pub(super) fn load_registry() -> Result<Registry> {
    let url = registry_url();
    let body = fetch(&url, MARKETPLACE_TIMEOUT_SECS).with_context(|| {
        format!("failed to load the plugin registry from {url} (override with AVM_REGISTRY_URL)")
    })?;
    parse_registry(&body).with_context(|| format!("invalid plugin registry at {url}"))
}

/// Parses registry JSON, rejecting an unsupported schema, duplicate names and entries whose
/// name or repo link is unusable.
fn parse_registry(body: &[u8]) -> Result<Registry> {
    let file: RegistryFile = serde_json::from_slice(body).context("malformed JSON")?;
    if file.schema_version != SCHEMA_VERSION {
        return Err(anyhow!(
            "unsupported registry schema_version {} (this avm reads {SCHEMA_VERSION}); update avm",
            file.schema_version
        ));
    }
    let mut registry = Registry::new();
    for entry in file.plugins {
        avm_plugin_sdk::validate_name(&entry.name)?;
        repo_slug(&entry.repo).with_context(|| format!("plugin '{}'", entry.name))?;
        let name = entry.name.clone();
        if registry.insert(name.clone(), entry).is_some() {
            return Err(anyhow!("plugin '{name}' is listed twice"));
        }
    }
    Ok(registry)
}

/// `owner/repo` from `https://github.com/owner/repo[.git]` or a bare `owner/repo`.
pub(super) fn repo_slug(link: &str) -> Result<String> {
    let slug = link
        .strip_prefix("https://github.com/")
        .unwrap_or(link)
        .trim_end_matches('/')
        .trim_end_matches(".git");
    let part_ok = |p: &str| {
        !p.is_empty()
            && p.bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
    };
    if slug.split('/').count() == 2 && slug.split('/').all(part_ok) {
        Ok(slug.to_string())
    } else {
        Err(anyhow!(
            "invalid repo link '{link}' (expected https://github.com/<owner>/<repo>)"
        ))
    }
}

/// The repo (`owner/repo`) that releases plugin `name`, or an error pointing at search.
pub(super) fn registry_repo(name: &str) -> Result<String> {
    let registry = load_registry()?;
    let entry = registry.get(name).ok_or_else(|| {
        anyhow!("no plugin named '{name}' in the registry — try `avm plugin search {name}`, or install a third-party one with `avm plugin install <owner>/<repo>`")
    })?;
    repo_slug(&entry.repo)
}

/// `(name, description)` rows of registry plugins whose name or description contains `keyword`, or that carry it as a tag (case-insensitive).
pub(super) fn search_registry(keyword: &str) -> Result<Vec<(String, String)>> {
    Ok(matching(&load_registry()?, keyword))
}

/// Pure matching behind `search_registry`.
fn matching(registry: &Registry, keyword: &str) -> Vec<(String, String)> {
    let key = keyword.to_lowercase();
    let hit = |s: &str| s.to_lowercase().contains(&key);
    registry
        .iter()
        .filter(|(name, e)| {
            hit(name) || hit(&e.description) || e.tags.iter().any(|t| t.to_lowercase() == key)
        })
        .map(|(name, e)| (name.clone(), e.description.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "schema_version": 1,
        "plugins": [
            {"name": "node", "description": "Node.js", "section_label": "Node",
             "repo": "PrajaNova/avm", "release_tag": "avm-plugin-node-v0.3.4", "tags": ["npm"]},
            {"name": "java", "description": "OpenJDK", "repo": "https://github.com/PrajaNova/avm"}
        ]
    }"#;

    #[test]
    fn parses_entries_ignoring_pins_and_defaulting_missing_fields() {
        let registry = parse_registry(SAMPLE.as_bytes()).unwrap();
        assert_eq!(registry["node"].tags, ["npm"]);
        assert!(registry["java"].tags.is_empty());
        assert_eq!(registry["java"].description, "OpenJDK");
    }

    #[test]
    fn rejects_bad_names_repos_schema_duplicates_and_json() {
        for bad in [
            r#"{"schema_version":1,"plugins":[{"name":"../x","repo":"o/r"}]}"#,
            r#"{"schema_version":1,"plugins":[{"name":"x","repo":"not a repo"}]}"#,
            r#"{"schema_version":1,"plugins":[{"name":"x","repo":"https://github.com/o"}]}"#,
            r#"{"schema_version":1,"plugins":[{"name":"x"}]}"#,
            r#"{"schema_version":2,"plugins":[]}"#,
            r#"{"plugins":[]}"#,
            r#"{"schema_version":1,"plugins":[{"name":"x","repo":"o/r"},{"name":"x","repo":"o/r"}]}"#,
            r#"{"node":{"repo":"o/r"}}"#,
            "[]",
        ] {
            assert!(parse_registry(bad.as_bytes()).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_checked_in_registry_parses() {
        let body = include_bytes!("../../../../marketplace/registry.json");
        let registry = parse_registry(body).unwrap();
        for name in ["node", "java", "python", "android"] {
            assert!(registry.contains_key(name), "{name}");
        }
    }

    #[test]
    fn slug_accepts_urls_and_bare_slugs() {
        for link in [
            "https://github.com/o/r",
            "https://github.com/o/r.git",
            "https://github.com/o/r/",
            "o/r",
        ] {
            assert_eq!(repo_slug(link).unwrap(), "o/r", "{link}");
        }
        assert!(repo_slug("https://gitlab.com/o/r").is_err());
    }

    #[test]
    fn search_matches_name_description_and_tags() {
        let registry = parse_registry(SAMPLE.as_bytes()).unwrap();
        let names =
            |k| -> Vec<String> { matching(&registry, k).into_iter().map(|r| r.0).collect() };
        assert_eq!(names("NPM"), ["node"]);
        assert_eq!(names("openjdk"), ["java"]);
        assert_eq!(names("zzz"), Vec::<String>::new());
        // A tag matches whole, so `np` does not hit the `npm` tag.
        assert_eq!(names("np"), Vec::<String>::new());
    }
}

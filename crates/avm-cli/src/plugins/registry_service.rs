//! The plugin registry: a JSON map of plugin name to where its releases live,
//! `{ "<name>": { "repo": "<github url>", "desc": "...", "tags": [...] } }`.
//! It only names the repo; the binary always comes from that repo's release.
use super::*;
use std::collections::BTreeMap;

/// One registry entry: the repo whose releases carry the plugin, a description and search tags.
#[derive(Debug, serde::Deserialize)]
pub struct RegistryEntry {
    pub repo: String,
    #[serde(default)]
    pub desc: String,
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

/// Parses registry JSON, rejecting entries whose name or repo link is unusable.
fn parse_registry(body: &[u8]) -> Result<Registry> {
    let registry: Registry = serde_json::from_slice(body).context("malformed JSON")?;
    for (name, entry) in &registry {
        avm_plugin_sdk::validate_name(name)?;
        repo_slug(&entry.repo).with_context(|| format!("plugin '{name}'"))?;
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
            hit(name) || hit(&e.desc) || e.tags.iter().any(|t| t.to_lowercase() == key)
        })
        .map(|(name, e)| (name.clone(), e.desc.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "node": {"repo": "https://github.com/PrajaNova/avm", "desc": "Node.js", "tags": ["npm"]},
        "java": {"repo": "PrajaNova/avm", "desc": "OpenJDK"}
    }"#;

    #[test]
    fn parses_entries_and_defaults_missing_fields() {
        let registry = parse_registry(SAMPLE.as_bytes()).unwrap();
        assert_eq!(registry["node"].tags, ["npm"]);
        assert!(registry["java"].tags.is_empty());
    }

    #[test]
    fn rejects_bad_names_repos_and_json() {
        for bad in [
            r#"{"../x": {"repo": "o/r"}}"#,
            r#"{"x": {"repo": "not a repo"}}"#,
            r#"{"x": {"repo": "https://github.com/o"}}"#,
            r#"{"x": {}}"#,
            "[]",
        ] {
            assert!(parse_registry(bad.as_bytes()).is_err(), "{bad}");
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

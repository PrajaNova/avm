use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use avm_plugin_api::ResolvedAlias;

#[derive(Debug, Clone)]
pub enum AliasSource {
    Local,
    Global,
    Plugin,
}

#[derive(Debug, Clone)]
pub struct ResolvedAliasLookup {
    pub command: String,
    pub source: AliasSource,
    pub plugin_name: Option<String>,
    pub argv: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedConfig {
    pub local_aliases: HashMap<String, String>,
    pub global_aliases: HashMap<String, String>,
    pub local_env: HashMap<String, String>,
    pub global_env: HashMap<String, String>,
    pub local_tools: HashMap<String, String>,
    pub global_tools: HashMap<String, String>,
    #[serde(skip)]
    pub plugin_aliases: HashMap<String, ResolvedAlias>,
    /// Untrusted config layers: aliases/env are dropped; tool pins still apply.
    pub untrusted: Vec<PathBuf>,
    pub alias_origins: HashMap<String, String>,
    pub env_origins: HashMap<String, String>,
    /// Config or version file each winning tool pin came from.
    pub tool_origins: HashMap<String, String>,
    /// Each local pin's spec as written (`20`, `lts/*`, `>=18`, `20.11.1`).
    pub tool_specs: HashMap<String, String>,
    pub global_tool_specs: HashMap<String, String>,
}

impl ResolvedConfig {
    /// Look up an alias by key, preferring local over global over plugin aliases; plugin aliases for node script runners also get an `argv`.
    pub fn resolve_alias(&self, key: &str) -> Option<ResolvedAliasLookup> {
        if let Some(value) = self.local_aliases.get(key) {
            return Some(ResolvedAliasLookup {
                command: value.clone(),
                source: AliasSource::Local,
                plugin_name: None,
                argv: None,
            });
        }

        if let Some(value) = self.global_aliases.get(key) {
            return Some(ResolvedAliasLookup {
                command: value.clone(),
                source: AliasSource::Global,
                plugin_name: None,
                argv: None,
            });
        }

        self.plugin_aliases.get(key).map(|a| ResolvedAliasLookup {
            command: a.command.clone(),
            source: AliasSource::Plugin,
            plugin_name: Some(a.plugin_name.clone()),
            argv: a
                .source
                .as_deref()
                .filter(|prefix| {
                    a.plugin_name == "node"
                        && matches!(*prefix, "npm run" | "pnpm run" | "bun run" | "yarn")
                })
                .map(|prefix| {
                    prefix
                        .split_whitespace()
                        .map(str::to_string)
                        .chain([key.to_string()])
                        .collect()
                }),
        })
    }

    /// Resolve a tool's version, local config first then global; `None` if unpinned.
    pub fn resolve_tool(&self, key: &str) -> Option<(String, AliasSource)> {
        if let Some(version) = self.local_tools.get(key) {
            return Some((version.clone(), AliasSource::Local));
        }
        if let Some(version) = self.global_tools.get(key) {
            return Some((version.clone(), AliasSource::Global));
        }
        None
    }

    /// All pinned tools with their version and origin; local pins override global ones.
    pub fn resolve_tools_with_source(&self) -> HashMap<String, (String, AliasSource)> {
        let mut merged: HashMap<String, (String, AliasSource)> = HashMap::new();
        for (tool, version) in &self.global_tools {
            merged.insert(tool.clone(), (version.clone(), AliasSource::Global));
        }
        for (tool, version) in &self.local_tools {
            merged.insert(tool.clone(), (version.clone(), AliasSource::Local));
        }
        merged
    }

    /// Every alias name from every source, sorted.
    pub fn alias_names(&self) -> BTreeSet<&String> {
        self.local_aliases
            .keys()
            .chain(self.global_aliases.keys())
            .chain(self.plugin_aliases.keys())
            .collect()
    }

    /// Alias names similar to `query` (score >= 0.80), best first, at most 8; ties stay alphabetical.
    pub fn suggest_aliases(&self, query: &str) -> Vec<String> {
        let candidates = self.alias_names();
        let mut scored: Vec<(&String, f64)> = candidates
            .into_iter()
            .map(|key| (key, alias_match_score(query, key)))
            .filter(|(_, score)| *score >= 0.80)
            .collect();
        // Stable sort: equal scores keep the BTreeSet's alphabetical order.
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .map(|(key, _)| key.clone())
            .take(8)
            .collect()
    }
}

/// Merge project config layers and global config with plugin aliases.
pub fn load(
    cwd: &Path,
    home: &Path,
    plugin_aliases: HashMap<String, ResolvedAlias>,
) -> anyhow::Result<ResolvedConfig> {
    let mut cfg = load_unlocked(cwd, home, plugin_aliases)?;
    let locked = match crate::lockfile::load(cwd)? {
        Some(lock) => match lock.apply(&mut cfg) {
            Ok(()) => true,
            // A stale lock must not break shims or the prompt; only `install --frozen` is strict.
            Err(err) => {
                eprintln!("avm: {err:#}; using .avm.json versions");
                false
            }
        },
        None => false,
    };
    if !locked {
        for (tool, version) in &mut cfg.local_tools {
            // Version-file pins were already resolved (their spec differs) in `load_unlocked`.
            if cfg.tool_specs.get(tool) == Some(version) {
                *version = crate::version_files::resolve(tool, version);
            }
        }
    }
    for (tool, version) in &mut cfg.global_tools {
        *version = crate::version_files::resolve(tool, version);
    }
    Ok(cfg)
}

/// Merge config layers and version-file pins without applying `.avm.lock` or resolving specs to installed versions.
/// Errors on unreadable configs or invalid tool names/version specs.
pub fn load_unlocked(
    cwd: &Path,
    home: &Path,
    plugin_aliases: HashMap<String, ResolvedAlias>,
) -> anyhow::Result<ResolvedConfig> {
    let profiles = std::env::var("AVM_ENV").unwrap_or_default();
    let names = crate::config::file_names(&profiles)?;
    let mut cfg = load_config_layers(cwd, home, &names)?;
    cfg.plugin_aliases = plugin_aliases;
    // Version-file pins stay below explicit JSON pins and are never cached:
    // installed runtimes and version files can change independently.
    let global = crate::config::load(home)?;
    for (tool, (version, origin, spec)) in
        crate::version_files::pins(cwd, home, global.idiomatic_version_files != Some(false))
    {
        if !cfg.local_tools.contains_key(&tool) {
            cfg.local_tools.insert(tool.clone(), version);
            cfg.tool_origins.insert(tool.clone(), origin);
            cfg.tool_specs.insert(tool, spec);
        }
    }
    for (tool, spec) in cfg.local_tools.iter().chain(cfg.global_tools.iter()) {
        avm_plugin_api::validate_name(tool)?;
        avm_plugin_api::validate_version_spec(spec)?;
    }
    Ok(cfg)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigCache {
    key: String,
    signature: String,
    config: ResolvedConfig,
}

/// Include missing candidates (new overrides), symlink targets, timestamps and
/// content hashes: restoring an mtime must never resurrect trusted commands.
fn signature(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| match std::fs::metadata(path) {
            Ok(meta) => format!(
                "{path:?}:{:?}:{:?}:{:?}:{:?}",
                path.canonicalize(),
                meta.len(),
                meta.modified(),
                avm_plugin_api::sha256_file(path)
            ),
            Err(err) => format!("{path:?}:{:?}", err.kind()),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every config file path whose state affects the merged config: global, per-directory candidates up to the root, and the trust store.
fn candidate_paths(cwd: &Path, home: &Path, names: &[String]) -> Vec<PathBuf> {
    let mut paths = vec![home.join(crate::config::CONFIG_FILE)];
    for dir in cwd.ancestors() {
        if dir != home {
            paths.extend(names.iter().map(|name| dir.join(name)));
        }
    }
    if let Ok(home) = crate::shims::avm_home() {
        paths.push(home.join("trusted.json"));
    }
    paths
}

/// Merge the global config and each ancestor directory's config files into one `ResolvedConfig`, using a signature-validated on-disk cache when possible.
fn load_config_layers(cwd: &Path, home: &Path, names: &[String]) -> anyhow::Result<ResolvedConfig> {
    let key = format!(
        "v1:{cwd:?}:{home:?}:{names:?}:{:?}",
        std::env::var("AVM_TRUST_ALL")
    );
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hash);
    let cache_path = crate::shims::avm_home()
        .ok()
        .map(|p| p.join(format!("cache/config-{:016x}.json", hash.finish())));
    let paths = candidate_paths(cwd, home, names);
    let input_signature = signature(&paths);
    if let Some(cache) = cache_path
        .as_ref()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|raw| serde_json::from_slice::<ConfigCache>(&raw).ok())
    {
        if cache.key == key && cache.signature == input_signature {
            return Ok(cache.config);
        }
    }
    let global = crate::config::load(home)?;
    let mut cfg = ResolvedConfig {
        local_aliases: HashMap::new(),
        global_aliases: global.aliases,
        local_env: HashMap::new(),
        global_env: global.env,
        local_tools: HashMap::new(),
        global_tool_specs: global.tools.clone(),
        global_tools: global.tools,
        plugin_aliases: HashMap::new(),
        untrusted: Vec::new(),
        alias_origins: HashMap::new(),
        env_origins: HashMap::new(),
        tool_origins: HashMap::new(),
        tool_specs: HashMap::new(),
    };
    let global_file = home.join(crate::config::CONFIG_FILE).display().to_string();
    for (values, origins) in [
        (&cfg.global_aliases, &mut cfg.alias_origins),
        (&cfg.global_env, &mut cfg.env_origins),
        (&cfg.global_tools, &mut cfg.tool_origins),
    ] {
        origins.extend(values.keys().map(|k| (k.clone(), global_file.clone())));
    }
    let mut layers = Vec::new();
    for dir in cwd.ancestors() {
        if dir == home {
            continue;
        }
        let mut stop = false;
        for name in names {
            let file = dir.join(name);
            let layer = crate::config::load_file(&file)?;
            // Only the base file controls traversal; profiles override values.
            stop |= name == crate::config::CONFIG_FILE && layer.root;
            layers.push((file, layer));
        }
        if stop {
            break;
        }
    }
    // Ancestors first, but base → profiles → personal within each directory.
    for chunk in layers.chunks(names.len()).rev() {
        for (file, layer) in chunk {
            let trusted = crate::trust::is_trusted(file, &global.trusted_paths);
            if file.exists() && !trusted {
                cfg.untrusted.push(file.clone());
            }
            for (values, merged, origins, tool_pins) in [
                (
                    &layer.aliases,
                    &mut cfg.local_aliases,
                    &mut cfg.alias_origins,
                    false,
                ),
                (&layer.env, &mut cfg.local_env, &mut cfg.env_origins, false),
                (
                    &layer.tools,
                    &mut cfg.local_tools,
                    &mut cfg.tool_origins,
                    true,
                ),
            ] {
                if !trusted && !tool_pins {
                    continue;
                }
                for (key, value) in values {
                    merged.insert(key.clone(), value.clone());
                    origins.insert(key.clone(), file.display().to_string());
                }
            }
        }
    }
    cfg.tool_specs = cfg.local_tools.clone();
    // The signature was taken before loading, so a file edited mid-load just misses next time.
    if let Some(path) = cache_path {
        let cache = ConfigCache {
            key,
            signature: input_signature,
            config: cfg.clone(),
        };
        // Cache failure cannot prevent config resolution (e.g. read-only home).
        if let Some(dir) = path.parent() {
            if std::fs::create_dir_all(dir).is_ok() {
                if let Ok(raw) = serde_json::to_vec(&cache) {
                    let _ = avm_plugin_api::storage::atomic_write(&path, &raw);
                }
            }
        }
    }
    Ok(cfg)
}

/// Similarity of two alias names in 0.0..=1.0: 1.0 if equal, 0.9 if one contains the other, else edit-distance based; separator-insensitive.
fn alias_match_score(query: &str, candidate: &str) -> f64 {
    let query = normalize_for_comparison(query);
    let candidate = normalize_for_comparison(candidate);
    if query.is_empty() || candidate.is_empty() {
        return 0.0;
    }
    if query == candidate {
        return 1.0;
    }
    if query.contains(&candidate) || candidate.contains(&query) {
        return 0.90;
    }

    let distance = levenshtein_distance(&query, &candidate);
    let max_len = query.len().max(candidate.len()) as f64;
    1.0 - (distance as f64 / max_len)
}

/// Split on `-`, `:`, `_`, `.`, sort the parts and join them, so word order and separators don't affect comparison.
fn normalize_for_comparison(s: &str) -> String {
    let mut parts: Vec<&str> = s
        .split(['-', ':', '_', '.'])
        .filter(|p| !p.is_empty())
        .collect();
    parts.sort_unstable();
    parts.join("")
}

/// Byte-wise Levenshtein edit distance between `s` and `t`.
fn levenshtein_distance(s: &str, t: &str) -> usize {
    let t = t.as_bytes();
    // One row of the edit-distance matrix at a time: `prev` is the row for the previous `s` byte.
    let mut prev: Vec<usize> = (0..=t.len()).collect();
    for (i, &a) in s.as_bytes().iter().enumerate() {
        let mut row = Vec::with_capacity(prev.len());
        row.push(i + 1);
        for (&b, pair) in t.iter().zip(prev.windows(2)) {
            let (Some(&diagonal), Some(&above), Some(&left)) =
                (pair.first(), pair.get(1), row.last())
            else {
                continue;
            };
            row.push(
                (above + 1)
                    .min(left + 1)
                    .min(diagonal + usize::from(a != b)),
            );
        }
        prev = row;
    }
    prev.last().copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_matches_known_values() {
        for (a, b, want) in [
            ("", "", 0),
            ("", "abc", 3),
            ("abc", "", 3),
            ("kitten", "sitting", 3),
            ("build", "biuld", 2),
            ("same", "same", 0),
        ] {
            assert_eq!(levenshtein_distance(a, b), want, "{a} vs {b}");
        }
    }
}

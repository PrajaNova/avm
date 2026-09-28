use std::collections::{BTreeSet, HashMap};
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
}

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub local_aliases: HashMap<String, String>,
    pub global_aliases: HashMap<String, String>,
    pub local_env: HashMap<String, String>,
    pub global_env: HashMap<String, String>,
    pub local_tools: HashMap<String, String>,
    pub global_tools: HashMap<String, String>,
    pub plugin_aliases: HashMap<String, ResolvedAlias>,
    /// The local `.avm.json` when it exists but isn't trusted (#20): its
    /// aliases and env are dropped; its tool pins still apply.
    pub untrusted: Option<PathBuf>,
    /// Version file each version-file pin came from (e.g. `./.nvmrc`).
    pub tool_origins: HashMap<String, String>,
    /// Each local pin's spec as written (`20`, `lts/*`, `>=18`, `20.11.1`).
    pub tool_specs: HashMap<String, String>,
}

impl ResolvedConfig {
    pub fn resolve_alias(&self, key: &str) -> Option<ResolvedAliasLookup> {
        if let Some(value) = self.local_aliases.get(key) {
            return Some(ResolvedAliasLookup {
                command: value.clone(),
                source: AliasSource::Local,
                plugin_name: None,
            });
        }

        if let Some(value) = self.global_aliases.get(key) {
            return Some(ResolvedAliasLookup {
                command: value.clone(),
                source: AliasSource::Global,
                plugin_name: None,
            });
        }

        self.plugin_aliases.get(key).map(|a| ResolvedAliasLookup {
            command: a.command.clone(),
            source: AliasSource::Plugin,
            plugin_name: Some(a.plugin_name.clone()),
        })
    }

    pub fn resolve_tool(&self, key: &str) -> Option<(String, AliasSource)> {
        if let Some(version) = self.local_tools.get(key) {
            return Some((version.clone(), AliasSource::Local));
        }
        if let Some(version) = self.global_tools.get(key) {
            return Some((version.clone(), AliasSource::Global));
        }
        None
    }

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

    pub fn suggest_aliases(&self, query: &str) -> Vec<String> {
        let candidates: BTreeSet<&String> = self
            .local_aliases
            .keys()
            .chain(self.global_aliases.keys())
            .chain(self.plugin_aliases.keys())
            .collect();
        let mut scored: Vec<(&String, f64)> = candidates
            .into_iter()
            .map(|key| (key, alias_match_score(query, key)))
            .filter(|(_, score)| *score >= 0.80)
            .collect();
        // Stable sort: equal scores keep the BTreeSet's alphabetical order.
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().map(|(key, _)| key.clone()).take(8).collect()
    }
}

/// Merge the local (`cwd`) and global (`home`) `.avm.json` with plugin aliases.
pub fn load(
    cwd: &Path,
    home: &Path,
    plugin_aliases: HashMap<String, ResolvedAlias>,
) -> anyhow::Result<ResolvedConfig> {
    let mut local = crate::config::load(cwd)?;
    let global = crate::config::load(home)?;
    let local_file = cwd.join(crate::config::CONFIG_FILE);
    let untrusted = (local_file.exists() && !crate::trust::is_trusted(&local_file, &global.trusted_paths))
        .then(|| {
            local.aliases.clear();
            local.env.clear();
            local_file
        });

    // Version files first, then `.avm.json` `tools` on top (#21).
    let idiomatic = global.idiomatic_version_files != Some(false);
    let mut local_tools = HashMap::new();
    let mut tool_origins = HashMap::new();
    let mut tool_specs = HashMap::new();
    for (tool, (version, origin, spec)) in crate::version_files::pins(cwd, home, idiomatic) {
        local_tools.insert(tool.clone(), version);
        tool_origins.insert(tool.clone(), origin);
        tool_specs.insert(tool, spec);
    }
    for (tool, version) in local.tools {
        tool_origins.remove(&tool);
        tool_specs.insert(tool.clone(), version.clone());
        local_tools.insert(tool, version);
    }

    Ok(ResolvedConfig {
        local_aliases: local.aliases,
        global_aliases: global.aliases,
        local_env: local.env,
        global_env: global.env,
        local_tools,
        global_tools: global.tools,
        plugin_aliases,
        untrusted,
        tool_origins,
        tool_specs,
    })
}

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

fn normalize_for_comparison(s: &str) -> String {
    let mut parts: Vec<&str> = s
        .split(['-', ':', '_', '.'])
        .filter(|p| !p.is_empty())
        .collect();
    parts.sort_unstable();
    parts.join("")
}

fn levenshtein_distance(s: &str, t: &str) -> usize {
    let m = s.len();
    let n = t.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }

    let s_bytes = s.as_bytes();
    let t_bytes = t.as_bytes();

    for i in 1..=m {
        for j in 1..=n {
            let cost = if s_bytes[i - 1] == t_bytes[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = std::cmp::min(
                std::cmp::min(dp[i - 1][j] + 1, dp[i][j - 1] + 1),
                dp[i - 1][j - 1] + cost,
            );
        }
    }

    dp[m][n]
}

//! Tool pins from files projects already have (#21): `.tool-versions`,
//! `.nvmrc`, `.node-version`, `package.json` (`volta.node`, `engines.node`),
//! `.java-version`, `.sdkmanrc`. Parsed in-process rather than by asking each
//! plugin, because this runs on every shimmed `node`/`java` call.
// ponytail: first-party tools only; add `version-files`/`parse-version-file`
// plugin verbs when a third-party plugin needs its own files.
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// `tool → (installed version or raw spec, origin file, spec as written)`. Nearest directory
/// wins; within a directory `.tool-versions` beats the idiomatic files. Walks
/// up from `cwd`, stopping before `home` (whose files would be global pins).
pub fn pins(cwd: &Path, home: &Path, idiomatic: bool) -> HashMap<String, (String, String, String)> {
    let mut found: HashMap<String, (String, String, String)> = HashMap::new();
    for dir in cwd.ancestors().take_while(|d| *d != home) {
        let mut here: Vec<(String, String, &str)> = Vec::new();
        if let Ok(raw) = fs::read_to_string(dir.join(".tool-versions")) {
            for (tool, spec) in parse_tool_versions(&raw) {
                here.push((tool, spec, ".tool-versions"));
            }
        }
        if idiomatic {
            for (file, tool, parse) in IDIOMATIC {
                if let Some(spec) = fs::read_to_string(dir.join(file)).ok().and_then(|raw| parse(&raw)) {
                    here.push((tool.to_string(), spec, file));
                }
            }
        }
        for (tool, spec, file) in here {
            found.entry(tool.clone()).or_insert_with(|| {
                let origin = if dir == cwd { format!("./{file}") } else { dir.join(file).display().to_string() };
                (resolve(&tool, &spec), origin, spec.clone())
            });
        }
    }
    found
}

type Parser = fn(&str) -> Option<String>;
const IDIOMATIC: &[(&str, &str, Parser)] = &[
    (".nvmrc", "node", first_line),
    (".node-version", "node", first_line),
    ("package.json", "node", parse_package_json),
    (".java-version", "java", first_line),
    (".sdkmanrc", "java", parse_sdkmanrc),
];

fn first_line(raw: &str) -> Option<String> {
    raw.lines().map(|l| l.split('#').next().unwrap_or("").trim()).find(|l| !l.is_empty()).map(str::to_string)
}

/// asdf format: `<tool> <version> [fallback...]`; asdf names mapped to avm's.
fn parse_tool_versions(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.split('#').next()?.split_whitespace();
            let tool = parts.next()?;
            let version = parts.next()?;
            if version == "system" || version.starts_with("ref:") || version.starts_with("path:") {
                return None;
            }
            let tool = match tool {
                "nodejs" => "node",
                other => other,
            };
            Some((tool.to_string(), version.to_string()))
        })
        .collect()
}

fn parse_package_json(raw: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(raw).ok()?;
    let at = |a: &str, b: &str| json.get(a)?.get(b)?.as_str().map(str::to_string);
    at("volta", "node").or_else(|| at("engines", "node"))
}

/// `java=17.0.9-tem` → `17.0.9` (the SDKMAN vendor suffix is dropped).
fn parse_sdkmanrc(raw: &str) -> Option<String> {
    let value = raw.lines().find_map(|l| l.trim().strip_prefix("java="))?.trim();
    Some(value.split('-').next().unwrap_or(value).to_string())
}

/// Map a spec (`20`, `v20.11.1`, `lts/*`, `>=18 <21`, `temurin-17`) to the
/// newest installed version that satisfies it; unmatched specs come back
/// as-is so the caller reports "not installed".
pub fn resolve(tool: &str, spec: &str) -> String {
    let installed = avm_plugin_api::tool_dir(tool)
        .ok()
        .and_then(|dir| fs::read_dir(dir).ok())
        .map(|entries| {
            entries.flatten().filter_map(|e| e.file_name().into_string().ok()).filter(|n| !n.starts_with('.')).collect()
        })
        .unwrap_or_default();
    pick(tool, spec, installed).unwrap_or_else(|| spec.trim_start_matches('v').to_string())
}

/// Newest of `installed` (any list of version names) matching `spec`.
pub(crate) fn pick(tool: &str, spec: &str, installed: Vec<String>) -> Option<String> {
    let spec = spec.trim();
    let mut candidates: Vec<(Vec<u64>, String)> =
        installed.into_iter().filter_map(|name| Some((numbers(&name)?, name))).collect();
    candidates.sort();
    let newest = |ok: &dyn Fn(&[u64]) -> bool| candidates.iter().rev().find(|(v, _)| ok(v)).map(|(_, n)| n.clone());

    if tool == "node" {
        let lts_major = |name: &str| match name {
            "*" => Some(None),
            "argon" => Some(Some(4)), "boron" => Some(Some(6)), "carbon" => Some(Some(8)),
            "dubnium" => Some(Some(10)), "erbium" => Some(Some(12)), "fermium" => Some(Some(14)),
            "gallium" => Some(Some(16)), "hydrogen" => Some(Some(18)), "iron" => Some(Some(20)),
            "jod" => Some(Some(22)), "krypton" => Some(Some(24)),
            _ => None,
        };
        match spec.to_ascii_lowercase().as_str() {
            "node" | "latest" | "current" | "stable" => return newest(&|_| true),
            // ponytail: even majors are LTS lines; ask the plugin's index if an odd one ever is.
            "lts" => return newest(&|v| v[0] % 2 == 0),
            s => {
                if let Some(which) = s.strip_prefix("lts/").and_then(lts_major) {
                    return newest(&|v| which.map_or(v[0] % 2 == 0, |m| v[0] == m));
                }
            }
        }
    }
    if spec.contains(['<', '>', '^', '~', '|', '*', ' ']) || spec.ends_with(".x") || spec == "x" {
        return newest(&|v| satisfies(v, spec));
    }
    // Exact, or a prefix at a component boundary (`20` → `20.11.1`,
    // `17` → `openjdk-17.0.9+9`), vendor prefixes ignored.
    let want = numbers(spec)?;
    newest(&|v| v.len() >= want.len() && v[..want.len()] == want[..])
}

/// Numeric components of a version name: `openjdk-17.0.9+9` → `[17, 0, 9, 9]`.
pub(crate) fn numbers(name: &str) -> Option<Vec<u64>> {
    let start = name.find(|c: char| c.is_ascii_digit())?;
    let nums: Vec<u64> = name[start..]
        .split(|c: char| !c.is_ascii_digit())
        .take_while(|p| !p.is_empty())
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    Some(nums)
}

/// npm semver range check (`>=18 <21`, `^20.1`, `~1.2`, `18 || 20`,
/// `20.x`, `1.2.3 - 2`), on `major.minor.patch`.
fn satisfies(version: &[u64], range: &str) -> bool {
    let v = [version.first().copied().unwrap_or(0), version.get(1).copied().unwrap_or(0), version.get(2).copied().unwrap_or(0)];
    range.split("||").any(|set| {
        let set = set.trim();
        if let Some((lo, hi)) = set.split_once(" - ") {
            return comparator(&v, &format!(">={}", lo.trim())) && comparator(&v, &format!("<={}", hi.trim()));
        }
        set.split_whitespace().all(|c| comparator(&v, c))
    })
}

fn comparator(v: &[u64; 3], c: &str) -> bool {
    let (op, rest) = [">=", "<=", ">", "<", "=", "^", "~"]
        .iter()
        .find_map(|op| c.strip_prefix(op).map(|r| (*op, r)))
        .unwrap_or(("", c));
    // Partial version: `None` for an omitted or wildcard component.
    let parts: Vec<Option<u64>> = rest.trim_start_matches('v').split('.').map(|p| p.parse().ok()).collect();
    let given = parts.iter().take_while(|p| p.is_some()).count().min(3);
    let base = [0, 1, 2].map(|i| if i < given { parts[i].unwrap_or(0) } else { 0 });
    // Upper bound one step past the last given component (`1.2` → `1.3.0`).
    let bump = |at: usize| {
        let mut b = base;
        b[at] += 1;
        for x in b.iter_mut().skip(at + 1) {
            *x = 0;
        }
        b
    };
    let lower_upper = |lo: [u64; 3], hi: Option<[u64; 3]>| *v >= lo && hi.is_none_or(|hi| *v < hi);
    match op {
        _ if given == 0 => true,
        "" | "=" if given < 3 => lower_upper(base, Some(bump(given - 1))),
        "" | "=" => *v == base,
        ">=" => *v >= base,
        "<" => *v < base,
        ">" if given < 3 => *v >= bump(given - 1),
        ">" => *v > base,
        "<=" if given < 3 => *v < bump(given - 1),
        "<=" => *v <= base,
        "~" => lower_upper(base, Some(bump(if given >= 2 { 1 } else { 0 }))),
        _ => {
            // `^`: bump the first non-zero given component.
            let at = (0..given).find(|&i| base[i] != 0).unwrap_or(given - 1);
            lower_upper(base, Some(bump(at)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn picks_newest_installed_match() {
        let node = installed(&["18.19.0", "20.11.1", "20.9.0", "21.1.0", "22.3.0"]);
        let p = |spec: &str| pick("node", spec, node.clone());
        assert_eq!(p("20").as_deref(), Some("20.11.1"));
        assert_eq!(p("v20.9.0").as_deref(), Some("20.9.0"));
        assert_eq!(p("lts/*").as_deref(), Some("22.3.0"));
        assert_eq!(p("lts/iron").as_deref(), Some("20.11.1"));
        assert_eq!(p("node").as_deref(), Some("22.3.0"));
        assert_eq!(p(">=18 <21").as_deref(), Some("20.11.1"));
        assert_eq!(p("^20.10").as_deref(), Some("20.11.1"));
        assert_eq!(p("18 || 21").as_deref(), Some("21.1.0"));
        assert_eq!(p("20.x").as_deref(), Some("20.11.1"));
        assert_eq!(p("~20.9").as_deref(), Some("20.9.0"));
        assert_eq!(p("19"), None);
        let java = installed(&["openjdk-17.0.9+9", "openjdk-21.0.2+13"]);
        assert_eq!(pick("java", "17", java.clone()).as_deref(), Some("openjdk-17.0.9+9"));
        assert_eq!(pick("java", "temurin-21.0.2", java).as_deref(), Some("openjdk-21.0.2+13"));
    }

    #[test]
    fn ranges() {
        let s = |v: [u64; 3], r: &str| satisfies(&v, r);
        assert!(s([1, 2, 3], "1.2.3 - 2") && s([2, 9, 0], "1.2.3 - 2") && !s([3, 0, 0], "1.2.3 - 2"));
        assert!(s([0, 2, 5], "^0.2.3") && !s([0, 3, 0], "^0.2.3"));
        assert!(s([2, 0, 0], ">1") && !s([1, 9, 9], ">1") && s([1, 2, 9], "<=1.2") && !s([1, 3, 0], "<=1.2"));
        assert!(s([5, 0, 0], "*") && s([5, 0, 0], ""));
    }

    #[test]
    fn parses_files() {
        assert_eq!(parse_tool_versions("nodejs 20.11.1 18\n# c\njava openjdk-17\nruby system\n"),
            vec![("node".into(), "20.11.1".into()), ("java".into(), "openjdk-17".into())]);
        assert_eq!(first_line("\n  v20.11.1 # pinned\n").as_deref(), Some("v20.11.1"));
        assert_eq!(parse_sdkmanrc("# x\njava=17.0.9-tem\n").as_deref(), Some("17.0.9"));
        assert_eq!(parse_package_json(r#"{"engines":{"node":">=18"},"volta":{"node":"20.1.0"}}"#).as_deref(), Some("20.1.0"));
        assert_eq!(parse_package_json(r#"{"engines":{"node":">=18"}}"#).as_deref(), Some(">=18"));
    }
}

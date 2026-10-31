//! Trust store for project config (#20). A project's `.avm.json` aliases/env
//! and its `.env` files run as the user, so they apply only once the user has
//! trusted that exact file content. `~/.avm/trusted.json` maps each file's
//! absolute path to the sha256 it had when trusted; any edit re-blocks it.
use super::store;
use crate::constants::msg::{self, fill};
use anyhow::{Context, Result};
use fs_err as fs;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Location of the trust store (`~/.avm/trusted.json`).
fn store_path() -> Result<PathBuf> {
    Ok(crate::shims::avm_home()?.join("trusted.json"))
}

/// All trusted files as path to sha256; an unreadable store is treated as empty.
pub fn list() -> BTreeMap<String, String> {
    read_store().unwrap_or_default()
}

/// Read the trust store, treating a missing file as empty and a malformed one as an error.
fn read_store() -> Result<BTreeMap<String, String>> {
    store::read_or_default(&store_path()?, "trust store")
}

/// Write the trust store, creating the avm home directory if needed.
fn save(store: &BTreeMap<String, String>) -> Result<()> {
    let path = store_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).context(msg::TRUST_STORE_HOME_FAILED)?;
    }
    store::write_pretty(&path, store).context(msg::TRUST_STORE_WRITE_FAILED)
}

/// Canonical path used as the store key; errors if `file` does not exist.
fn key(file: &Path) -> Result<PathBuf> {
    file.canonicalize().with_context(|| {
        fill(
            msg::TRUST_STORE_NOT_FOUND,
            &[("path", &file.display().to_string())],
        )
    })
}

/// Trusted if `AVM_TRUST_ALL=1`, it lives directly in `$HOME` (the global
/// config), its directory matches a `trusted_paths` glob from the global
/// config, or its current sha256 is the one recorded by `avm trust`.
pub fn is_trusted(file: &Path, trusted_paths: &[String]) -> bool {
    if std::env::var("AVM_TRUST_ALL").as_deref() == Ok("1") {
        return true;
    }
    let Ok(file) = file.canonicalize() else {
        return true; // nothing on disk, nothing to run
    };
    let Some(dir) = file.parent() else {
        return false;
    };
    let home = avm_plugin_sdk::home_dir().ok();
    if home
        .as_deref()
        .and_then(|h| h.canonicalize().ok())
        .as_deref()
        == Some(dir)
    {
        return true;
    }
    if trusted_paths
        .iter()
        .any(|pattern| glob_match(&resolve_prefix(&expand_home(pattern, home.as_deref())), dir))
    {
        return true;
    }
    match avm_plugin_sdk::sha256_file(&file) {
        Ok(hash) => list().get(&*file.to_string_lossy()) == Some(&hash),
        Err(_) => false,
    }
}

/// Record `file`'s current sha256 as trusted, under the store lock.
pub fn trust(file: &Path) -> Result<()> {
    let file = key(file)?;
    let hash = avm_plugin_sdk::sha256_file(&file)?;
    let _guard = avm_plugin_sdk::storage::lock(&store_path()?)?;
    let mut store = read_store()?;
    store.insert(file.to_string_lossy().into_owned(), hash);
    save(&store)
}

/// Returns whether an entry was removed.
pub fn revoke(file: &Path) -> Result<bool> {
    let file = key(file)?;
    let _guard = avm_plugin_sdk::storage::lock(&store_path()?)?;
    let mut store = read_store()?;
    let removed = store.remove(&*file.to_string_lossy()).is_some();
    save(&store)?;
    Ok(removed)
}

/// Replace a leading `~` in `pattern` with the home directory, when known.
fn expand_home(pattern: &str, home: Option<&Path>) -> String {
    match (pattern.strip_prefix("~"), home) {
        (Some(rest), Some(home)) => format!("{}{rest}", home.display()),
        _ => pattern.to_string(),
    }
}

/// Canonicalize the pattern's literal (glob-free) leading dirs, so a pattern
/// written through a symlink (macOS `/var` → `/private/var`) still matches.
fn resolve_prefix(pattern: &str) -> String {
    let cut = pattern.find('*').map_or(pattern.len(), |i| {
        pattern[..i].rfind(['/', '\\']).unwrap_or(0)
    });
    match Path::new(&pattern[..cut]).canonicalize() {
        Ok(real) if cut > 0 => format!("{}{}", real.display(), &pattern[cut..]),
        _ => pattern.to_string(),
    }
}

/// Path glob over `/`-separated segments: `*` matches within one segment,
/// `**` matches any number of segments (including none).
fn glob_match(pattern: &str, path: &Path) -> bool {
    /// Split a path or pattern into non-empty `/` or `\` separated segments.
    fn segments(s: &str) -> Vec<&str> {
        s.split(['/', '\\']).filter(|p| !p.is_empty()).collect()
    }
    /// Match one path segment against a pattern segment where `*` matches any run of characters.
    fn seg_match(p: &str, s: &str) -> bool {
        match p.split_once('*') {
            None => p == s,
            Some((head, tail)) => {
                s.starts_with(head)
                    && (0..=s.len() - head.len()).any(|i| {
                        s.is_char_boundary(head.len() + i) && seg_match(tail, &s[head.len() + i..])
                    })
            }
        }
    }
    /// Match pattern segments against path segments, with `**` spanning zero or more segments.
    fn go(p: &[&str], s: &[&str]) -> bool {
        match p.split_first() {
            None => s.is_empty(),
            Some((&"**", rest)) => (0..=s.len()).any(|i| go(rest, &s[i..])),
            Some((head, rest)) => !s.is_empty() && seg_match(head, s[0]) && go(rest, &s[1..]),
        }
    }
    let path = path.to_string_lossy();
    go(&segments(pattern), &segments(&path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs() {
        let m = |p: &str, s: &str| glob_match(p, Path::new(s));
        assert!(m("/home/u/work/**", "/home/u/work"));
        assert!(m("/home/u/work/**", "/home/u/work/a/b"));
        assert!(!m("/home/u/work/**", "/home/u/play/a"));
        assert!(m("/home/u/*/api", "/home/u/svc/api"));
        assert!(!m("/home/u/*/api", "/home/u/a/b/api"));
        assert!(m("/home/u/proj-*", "/home/u/proj-x"));
        assert!(!m("/home/u/proj-*", "/home/u/other"));
        assert_eq!(
            expand_home("~/work/**", Some(Path::new("/h"))),
            "/h/work/**"
        );
    }
}

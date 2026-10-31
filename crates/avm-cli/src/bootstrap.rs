use crate::common::state::{home_dir, restore_inherited_env, UNTRUSTED_DOTENV};
use crate::config;
use crate::constants::msg::{self, fill};
use anyhow::{anyhow, Context, Result};
use fs_err as fs;
use std::collections::HashSet;
use std::path::Path;

/// Prepares the process before any command runs: `HOME`, the inherited-env
/// snapshot and trusted `.env` files.
pub fn bootstrap() -> Result<()> {
    // Windows often has only USERPROFILE; plugins (including ones built on an
    // older avm-plugin-sdk) find ~/.avm through HOME, so pass ours down.
    if std::env::var_os("HOME").is_none() {
        if let Ok(home) = home_dir() {
            std::env::set_var("HOME", home);
        }
    }
    restore_inherited_env()?;
    load_dotenv_env()
}

/// Loads `.env` files from the filesystem root down to the cwd, nearer files overriding farther ones.
/// Pre-existing process variables are protected; untrusted `.env` files are skipped with a notice.
fn load_dotenv_env() -> Result<()> {
    let cwd = std::env::current_dir().context(msg::CWD_READ_FAILED)?;
    let protected: HashSet<String> = std::env::vars().map(|(key, _)| key).collect();
    let trusted_paths = home_dir()
        .and_then(|home| config::load(&home))
        .map(|c| c.trusted_paths)
        .unwrap_or_default();

    let dirs: Vec<&Path> = cwd.ancestors().collect();
    let mut skipped = Vec::new();
    for dir in dirs.into_iter().rev() {
        let env_file = dir.join(".env");
        if !env_file.exists() {
            continue;
        }
        // A project's .env reaches every shimmed tool's environment, so it
        // needs the same trust as its .avm.json (#20).
        if crate::trust::is_trusted(&env_file, &trusted_paths) {
            load_env_file(&env_file, &protected)?;
        } else {
            skipped.push(env_file);
        }
    }
    let _ = UNTRUSTED_DOTENV.set(skipped);

    Ok(())
}

/// Builds a `path:line` error from `template`; `line_no` is zero-based.
fn env_file_error(template: &str, path: &Path, line_no: usize) -> anyhow::Error {
    anyhow!(fill(
        template,
        &[
            ("path", &path.display().to_string()),
            ("line", &(line_no + 1).to_string()),
        ]
    ))
}

/// Parses one `.env` file (`KEY=VALUE`, optional `export `, `#` comments) and sets each variable not in `protected`.
/// Errors on an unreadable file or a malformed assignment.
fn load_env_file(path: &Path, protected: &HashSet<String>) -> Result<()> {
    let contents = fs::read_to_string(path).with_context(|| {
        fill(
            msg::ENV_FILE_READ_FAILED,
            &[("path", &path.display().to_string())],
        )
    })?;

    for (line_no, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            return Err(env_file_error(msg::ENV_FILE_BAD_ASSIGNMENT, path, line_no));
        };

        let key = key.trim();
        if key.is_empty() {
            return Err(env_file_error(msg::ENV_FILE_BAD_KEY, path, line_no));
        }
        if protected.contains(key) {
            continue;
        }

        std::env::set_var(key, parse_env_value(value.trim()));
    }

    Ok(())
}

/// Strips one pair of matching single or double quotes around a `.env` value; other values are returned unchanged.
fn parse_env_value(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0];
        let last = bytes[value.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }

    value.to_string()
}

//! `avm trust`.
use crate::common::ui;
use crate::config;
use crate::constants::msg::{self, fill};
use crate::project::TrustArgs;
use ::fs_err as fs;
use anyhow::{anyhow, Context, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// `avm trust`: lists, trusts or (with `revoke`) untrusts the active config files and `.env` in a directory or a single file.
/// Errors when a directory has none.
pub fn cmd_trust(args: TrustArgs) -> Result<()> {
    if args.list {
        for path in crate::trust::list().keys() {
            println!("{path}");
        }
        return Ok(());
    }
    let target = match args.path {
        Some(path) => path,
        None => std::env::current_dir().context(msg::PROJECT_CWD_FAILED)?,
    };
    let files: Vec<PathBuf> = if target.is_dir() {
        let names = config::file_names(&std::env::var("AVM_ENV").unwrap_or_default())?;
        names
            .iter()
            .map(|f| target.join(f))
            .chain([target.join(".env")])
            .filter(|f| f.exists())
            .collect()
    } else {
        vec![target.clone()]
    };
    if files.is_empty() {
        let path = target.display().to_string();
        return Err(anyhow!(fill(msg::TRUST_NONE, &[("path", &path)])));
    }
    for file in files {
        let path = file.display().to_string();
        if args.revoke {
            let template = if crate::trust::revoke(&file)? {
                msg::TRUST_REVOKED
            } else {
                msg::TRUST_WAS_NOT
            };
            ui::info(fill(template, &[("path", &path)]));
            continue;
        }
        // Show what's being trusted: the commands and env it will run with.
        if file
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(".avm.") && n.ends_with(".json"))
        {
            let cfg = config::load_file(&file)?;
            for (key, value) in cfg.aliases.iter().collect::<BTreeMap<_, _>>() {
                ui::info(fill(msg::TRUST_ALIAS, &[("key", key), ("value", value)]));
            }
            for (key, value) in cfg.env.iter().collect::<BTreeMap<_, _>>() {
                ui::info(fill(msg::TRUST_ENV, &[("key", key), ("value", value)]));
            }
        } else {
            for line in fs::read_to_string(&file)?
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            {
                ui::info(format!("  {line}"));
            }
        }
        crate::trust::trust(&file)?;
        ui::success(fill(msg::TRUST_DONE, &[("path", &path)]));
    }
    Ok(())
}

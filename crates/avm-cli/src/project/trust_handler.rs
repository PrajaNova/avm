//! `avm trust`.
use crate::config;
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
        None => std::env::current_dir().context("failed to read current directory")?,
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
        return Err(anyhow!(
            "no active avm config or .env in {}",
            target.display()
        ));
    }
    for file in files {
        if args.revoke {
            let removed = crate::trust::revoke(&file)?;
            println!(
                "{} {}",
                if removed {
                    "✓ Revoked trust for"
                } else {
                    "Was not trusted:"
                },
                file.display()
            );
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
                println!("  alias {key} → {value}");
            }
            for (key, value) in cfg.env.iter().collect::<BTreeMap<_, _>>() {
                println!("  env   {key}={value}");
            }
        } else {
            for line in fs::read_to_string(&file)?
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            {
                println!("  {line}");
            }
        }
        crate::trust::trust(&file)?;
        done!("Trusted {}", file.display());
    }
    Ok(())
}

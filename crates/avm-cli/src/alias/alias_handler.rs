//! Alias commands: add, remove and list.
use crate::alias::alias_utils::validate_alias_name;
use crate::alias::{AddArgs, AliasCommands, RemoveArgs};
use crate::common::config_utils::{edit_config, print_layered, scope};
use crate::common::state::load_state;
use crate::common::ui;
use crate::constants::msg::{self, fill};
use anyhow::{anyhow, Result};

/// `avm alias add`: stores an alias in the local or global config; rejects reserved names.
/// Asks before replacing any existing alias, even with the same command (`-y` skips the prompt).
pub fn cmd_add(args: AddArgs) -> Result<()> {
    validate_alias_name(&args.key)?;
    let value = args.value.join(" ");
    let mut cancelled = false;
    edit_config(args.global, args.global, |cfg| {
        if let Some(old) = cfg.aliases.get(&args.key) {
            let question = fill(
                msg::ALIAS_OVERRIDE_PROMPT,
                &[("key", &args.key), ("old", old), ("value", &value)],
            );
            if !ui::confirm(&question, args.yes)? {
                cancelled = true;
                return Ok(());
            }
        }
        cfg.aliases.insert(args.key.clone(), value);
        Ok(())
    })?;
    if cancelled {
        ui::cancelled();
    } else {
        ui::success(fill(
            msg::ALIAS_ADDED,
            &[("scope", scope(args.global)), ("key", &args.key)],
        ));
    }
    Ok(())
}

/// `avm alias remove`: deletes an alias from the chosen config; errors when the alias is not defined there.
pub fn cmd_remove(args: RemoveArgs) -> Result<()> {
    edit_config(args.global, false, |cfg| {
        match cfg.aliases.remove(&args.key) {
            Some(_) => Ok(()),
            None => Err(anyhow!(fill(
                msg::ALIAS_NOT_FOUND,
                &[("scope", scope(args.global)), ("key", &args.key)]
            ))),
        }
    })?;
    ui::success(fill(
        msg::ALIAS_REMOVED,
        &[("scope", scope(args.global)), ("key", &args.key)],
    ));
    Ok(())
}

/// `avm alias add/remove/list` (or `avm -al ...`), same shape as `avm plugin`.
pub fn cmd_alias(command: AliasCommands) -> Result<()> {
    match command {
        AliasCommands::Add(args) => cmd_add(args),
        AliasCommands::Remove(args) => cmd_remove(args),
        AliasCommands::List => cmd_alias_list(),
    }
}

/// Lists local and global aliases merged, with local winning, or a hint when none exist.
fn cmd_alias_list() -> Result<()> {
    let cfg = load_state()?;
    if cfg.local_aliases.is_empty() && cfg.global_aliases.is_empty() {
        ui::info(msg::ALIAS_NONE);
        return Ok(());
    }
    print_layered(
        &cfg.local_aliases,
        &cfg.global_aliases,
        "",
        " → ",
        &cfg.alias_origins,
    );
    Ok(())
}

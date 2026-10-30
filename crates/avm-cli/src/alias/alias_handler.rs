//! Alias commands: add, remove and list.
use crate::alias::alias_utils::validate_alias_name;
use crate::alias::{AddArgs, AliasCommands, RemoveArgs};
use crate::common::config_utils::{edit_config, print_layered, scope};
use crate::common::state::load_state;
use anyhow::{anyhow, Result};

/// `avm alias add`: stores an alias in the local or global config; rejects reserved names.
pub fn cmd_add(args: AddArgs) -> Result<()> {
    validate_alias_name(&args.key)?;
    edit_config(args.global, args.global, |cfg| {
        cfg.aliases.insert(args.key.clone(), args.value.join(" "));
        Ok(())
    })?;
    done!("Added {} alias '{}'", scope(args.global), args.key);
    Ok(())
}

/// `avm alias remove`: deletes an alias from the chosen config; errors when the alias is not defined there.
pub fn cmd_remove(args: RemoveArgs) -> Result<()> {
    edit_config(args.global, false, |cfg| {
        match cfg.aliases.remove(&args.key) {
            Some(_) => Ok(()),
            None => Err(anyhow!("alias '{}' not found", args.key)),
        }
    })?;
    done!("Removed {} alias '{}'", scope(args.global), args.key);
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
        println!("No aliases configured.");
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

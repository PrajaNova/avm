use super::*;

pub fn cmd_which(key: &str) -> Result<()> {
    let cfg = load_state()?;
    if let Some(alias) = cfg.resolve_alias(key) {
        match alias.source {
            AliasSource::Local => println!("local alias '{key}': {}", alias.command),
            AliasSource::Global => println!("global alias '{key}': {}", alias.command),
            AliasSource::Plugin => {
                let plugin = alias.plugin_name.unwrap_or_else(|| "plugin".to_string());
                println!("plugin alias '{key}' from {plugin}: {}", alias.command);
            }
        }
        return Ok(());
    }

    if let Some((version, source)) = cfg.resolve_tool(key) {
        println!("tool '{key}': {version} ({})", alias_source_label(&source));
        return Ok(());
    }

    println!("No mapping found for '{key}'.");
    Ok(())
}

pub fn cmd_env(command: Option<EnvCommands>) -> Result<()> {
    match command {
        None => cmd_env_print(),
        Some(EnvCommands::Add { key, value, global }) => cmd_env_add(key, value, global),
        Some(EnvCommands::Remove { key, global }) => cmd_env_remove(key, global),
        Some(EnvCommands::List) => cmd_env_list(),
    }
}

fn cmd_env_print() -> Result<()> {
    let cfg = load_state()?;
    let mut env = resolved_tool_env(&cfg)?;
    env.extend(merge_env(&cfg));
    // No PATH here: this output is `eval`'d straight into the interactive
    // shell on every `avm` invocation (see shell-init's `_avm_apply_env`).
    // Exporting a resolved-tool PATH prefix here would re-clobber the
    // shims-first ordering shell-init just set up, on every single call —
    // shims already resolve the right managed version per directory, so
    // there's nothing this needs to add for shell use. The equivalent
    // PATH prefix is still applied for real, scoped only to that one
    // subprocess, in `exec-shim`.
    let mut keys: Vec<_> = env.keys().collect();
    keys.sort();
    for key in keys {
        println!("export {key}={}", sh_quote(&env[key]));
    }
    Ok(())
}

pub fn cmd_resolve(args: ResolveArgs) -> Result<()> {
    let cfg = load_state()?;
    let alias = cfg
        .resolve_alias(&args.key)
        .ok_or_else(|| alias_not_found_error(&args.key, &cfg))?;
    println!("{}", build_shell_alias_string(&alias.command, &args.args)?);
    Ok(())
}

pub fn cmd_run(args: RunArgs) -> Result<()> {
    let cfg = load_state()?;
    let alias_key = &args.args[0];
    let alias = match cfg.resolve_alias(alias_key) {
        Some(alias) => alias,
        None if ui::can_select() => {
            let suggestions = cfg.suggest_aliases(alias_key);
            let Some(selected) = select_alias_suggestion(alias_key, &suggestions)? else {
                return Ok(());
            };
            cfg.resolve_alias(&selected)
                .ok_or_else(|| alias_not_found_error(alias_key, &cfg))?
        }
        None => return Err(alias_not_found_error(alias_key, &cfg)),
    };

    let script = build_shell_alias_string(&alias.command, &args.args[1..])?;
    let status = Command::new("sh")
        .arg("-c")
        .arg(script)
        .envs(child_env(&cfg)?)
        .status()
        .context("failed to run alias via sh")?;
    std::process::exit(status.code().unwrap_or(1));
}

/// Build the `sh -c` script for an alias: expand `$1..$N` with sh-quoted
/// args; if no positional placeholders were used, append extra args
/// sh-quoted at the end. `$VAR`, `${...}`, `$(...)` are left for the shell.
fn build_shell_alias_string(template: &str, args: &[String]) -> Result<String> {
    let mut output = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    let mut used_placeholder = false;

    while let Some(ch) = chars.next() {
        if ch != '$' || !chars.peek().is_some_and(|c| c.is_ascii_digit()) {
            output.push(ch);
            continue;
        }
        let mut digits = String::new();
        while let Some(n) = chars.next_if(|c| c.is_ascii_digit()) {
            digits.push(n);
        }
        let index: usize = digits
            .parse()
            .with_context(|| format!("invalid placeholder ${digits}"))?;
        if index == 0 || index > args.len() {
            return Err(anyhow!("placeholder ${index} out of bounds"));
        }
        output.push_str(&sh_quote(&args[index - 1]));
        used_placeholder = true;
    }

    if !used_placeholder {
        for arg in args {
            output.push(' ');
            output.push_str(&sh_quote(arg));
        }
    }
    Ok(output)
}

fn select_alias_suggestion(query: &str, suggestions: &[String]) -> Result<Option<String>> {
    if suggestions.is_empty() {
        return Err(anyhow!("alias '{query}' not found"));
    }

    let labels: Vec<String> = suggestions.iter().map(|s| format!("avm {s}")).collect();
    match ui::select(&format!("Alias '{query}' not found"), &labels)? {
        Some(index) => Ok(Some(suggestions[index].clone())),
        None => {
            println!("Cancelled.");
            Ok(None)
        }
    }
}

fn alias_not_found_error(key: &str, cfg: &ResolvedConfig) -> anyhow::Error {
    let suggestions = cfg.suggest_aliases(key);
    if suggestions.is_empty() {
        return anyhow!("alias '{key}' not found");
    }

    anyhow!(
        "alias '{key}' not found\n\nDid you mean?\n{}",
        suggestions
            .iter()
            .map(|suggestion| format!("  avm {suggestion}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

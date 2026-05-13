fn cmd_which(key: &str) -> Result<()> {
    let cfg = load_state()?;
    if let Some(alias) = cfg.resolve_alias(key, &cfg) {
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

    if let Some((version, source)) = cfg.resolve_tool(key, &cfg) {
        println!("tool '{key}': {version} ({})", alias_source_label(&source));
        return Ok(());
    }

    println!("No mapping found for '{key}'.");
    Ok(())
}
fn cmd_env(args: EnvArgs) -> Result<()> {
    if args.format != "export" {
        return Err(anyhow!("unknown env format: {}", args.format));
    }

    let cfg = load_state()?;
    let mut env = merge_env(&cfg);
    if let Some(path_prefix) = resolved_tool_path_prefix(&cfg)? {
        env.insert("PATH".to_string(), path_prefix);
    }

    let mut keys: Vec<_> = env.keys().collect();
    keys.sort();
    for key in keys {
        println!("export {key}={}", shell_quote(&env[key]));
    }
    Ok(())
}

fn cmd_resolve(args: ResolveArgs) -> Result<()> {
    let cfg = load_state()?;
    let alias = cfg.resolve_alias(&args.key, &cfg).ok_or_else(|| {
        alias_not_found_error(&args.key, &cfg)
    })?;
    let command = build_alias_command(&alias.command, &args.args)?;
    println!("{}", shell_quote_command(&command));
    Ok(())
}

fn cmd_run(args: RunArgs) -> Result<()> {
    if args.args.is_empty() {
        return Err(anyhow!("run requires a command name"));
    }

    let cfg = load_state()?;
    let alias_key = args.args[0].clone();
    let alias = match cfg.resolve_alias(&alias_key, &cfg) {
        Some(alias) => alias,
        None => {
            if ui::can_select() {
                let suggestions = cfg.suggest_aliases(&alias_key);
                if let Some(selected) = select_alias_suggestion(&alias_key, &suggestions)? {
                    cfg.resolve_alias(&selected, &cfg)
                        .ok_or_else(|| alias_not_found_error(&alias_key, &cfg))?
                } else {
                    return Ok(());
                }
            } else {
                return Err(alias_not_found_error(&alias_key, &cfg));
            }
        }
    };
    let command = build_alias_command(&alias.command, &args.args[1..])?;
    if command.is_empty() {
        return Err(anyhow!("alias '{}' resolved to empty command", args.args[0]));
    }

    let mut env = std::env::vars().collect::<HashMap<String, String>>();
    if let Some(path_prefix) = resolved_tool_path_prefix(&cfg)? {
        env.insert("PATH".to_string(), path_prefix);
    }
    for (key, value) in merge_env(&cfg) {
        env.insert(key, value);
    }

    let status = Command::new(&command[0])
        .args(&command[1..])
        .envs(env)
        .status()
        .context("failed to run alias")?;
    std::process::exit(status.code().unwrap_or(1));
}

fn select_alias_suggestion(query: &str, suggestions: &[String]) -> Result<Option<String>> {
    if suggestions.is_empty() {
        return Err(anyhow!("alias '{query}' not found"));
    }

    let items = suggestions
        .iter()
        .map(|suggestion| ui::SelectItem {
            label: format!("avm {suggestion}"),
        })
        .collect::<Vec<_>>();

    match ui::select(
        &format!("Alias '{query}' not found"),
        "Use Up/Down to choose a suggestion, Enter to run, q to cancel.",
        &items,
        8,
    )? {
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

fn shell_quote_command(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| shell_quote(part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_alias_command(template: &str, args: &[String]) -> Result<Vec<String>> {
    let mut tokens = split_command_template(template)?;
    if tokens.is_empty() {
        return Err(anyhow!("invalid empty command"));
    }

    let mut contains_placeholder = false;
    let mut expanded = Vec::with_capacity(tokens.len());
    for token in tokens.drain(..) {
        let replaced = expand_template_placeholders(&token, args)?;
        if replaced != token {
            contains_placeholder = true;
        }
        expanded.push(replaced);
    }

    if !contains_placeholder && template.contains('$') {
        // Preserve literal $ when no placeholder syntax was actually used.
        expanded = split_command_template(template)?;
    }

    if !contains_placeholder {
        expanded.extend_from_slice(args);
    }

    Ok(expanded)
}

fn expand_template_placeholders(value: &str, args: &[String]) -> Result<String> {
    let mut output = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            output.push(ch);
            continue;
        }

        let mut digits = String::new();
        while let Some(next) = chars.peek() {
            if next.is_ascii_digit() {
                if let Some(digit) = chars.next() {
                    digits.push(digit);
                }
            } else {
                break;
            }
        }

        if digits.is_empty() {
            output.push('$');
            continue;
        }

        let index: usize = digits
            .parse()
            .with_context(|| format!("invalid placeholder ${digits}"))?;
        if index == 0 || index > args.len() {
            return Err(anyhow!("placeholder ${index} out of bounds"));
        }
        output.push_str(&args[index - 1]);
    }

    Ok(output)
}

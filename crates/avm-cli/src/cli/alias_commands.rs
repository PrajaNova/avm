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
    let mut env = std::env::vars().collect::<HashMap<String, String>>();
    if let Some(path_prefix) = resolved_tool_path_prefix(&cfg)? {
        env.insert("PATH".to_string(), path_prefix);
    }
    for (key, value) in merge_env(&cfg) {
        env.insert(key, value);
    }

    let extra_args = &args.args[1..];
    let status = if needs_shell(&alias.command) {
        let script = build_shell_alias_string(&alias.command, extra_args)?;
        let shell = pick_shell();
        Command::new(&shell.0)
            .args(&shell.1)
            .arg(script)
            .envs(env)
            .status()
            .with_context(|| format!("failed to run alias via {}", shell.0))?
    } else {
        let command = build_alias_command(&alias.command, extra_args)?;
        if command.is_empty() {
            return Err(anyhow!("alias '{}' resolved to empty command", args.args[0]));
        }
        Command::new(&command[0])
            .args(&command[1..])
            .envs(env)
            .status()
            .with_context(|| format!("failed to run alias '{}'", args.args[0]))?
    };
    std::process::exit(status.code().unwrap_or(1));
}

/// Returns true when the alias body contains shell metacharacters that require
/// a real shell to execute correctly (pipes, redirects, chained commands,
/// command substitution, globs, env expansion, etc.).
fn needs_shell(template: &str) -> bool {
    // Inspect outside of quoted substrings to avoid false positives like
    // `echo "a;b"`. We track single/double quote state and look at unquoted
    // characters only.
    let mut in_single = false;
    let mut in_double = false;
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_single {
            if c == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if c == '\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }
        match c {
            '\'' => in_single = true,
            '"' => in_double = true,
            '|' | ';' | '&' | '>' | '<' | '`' | '*' | '?' => return true,
            '$' => {
                let next = bytes.get(i + 1).map(|b| *b as char);
                // $VAR, ${...}, $(...) all require the shell. Numeric
                // placeholders ($1, $2, ...) are handled internally.
                if let Some(n) = next {
                    if n == '(' || n == '{' || n.is_ascii_alphabetic() || n == '_' {
                        return true;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

#[cfg(unix)]
fn pick_shell() -> (String, Vec<String>) {
    (
        std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string()),
        vec!["-c".to_string()],
    )
}

#[cfg(windows)]
fn pick_shell() -> (String, Vec<String>) {
    ("cmd".to_string(), vec!["/C".to_string()])
}

/// POSIX single-quote escaping: safe for any byte string.
fn sh_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/' | b':' | b'='))
    {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Build a shell-mode alias string: expand `$1..$N` with sh-quoted args; if no
/// positional placeholders were used, append extra args sh-quoted at the end.
fn build_shell_alias_string(template: &str, args: &[String]) -> Result<String> {
    let mut output = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    let mut used_placeholder = false;

    while let Some(ch) = chars.next() {
        if ch != '$' {
            output.push(ch);
            continue;
        }
        // Only consume *numeric* placeholders here; leave $VAR / ${...} / $(...)
        // for the shell to interpret.
        let Some(&peek) = chars.peek() else {
            output.push('$');
            continue;
        };
        if !peek.is_ascii_digit() {
            output.push('$');
            continue;
        }
        let mut digits = String::new();
        while let Some(&n) = chars.peek() {
            if n.is_ascii_digit() {
                digits.push(n);
                chars.next();
            } else {
                break;
            }
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

//! Cross-layer lookups: which, resolve, run and the bare `avm` home picker.
use crate::args::Cli;
use crate::common::config_utils::alias_source_label;
use crate::common::env_utils::child_env;
use crate::common::state::load_state_with_aliases;
use crate::common::ui;
use crate::constants::msg::{self, fill};
use crate::global::{cmd_plugin_command, cmd_provider_tool, ResolveArgs, RunArgs};
use crate::plugins;
use crate::resolver::{AliasSource, ResolvedConfig};
use crate::tool::temporary_tool_env;
use anyhow::{anyhow, Context, Result};
use avm_plugin_sdk::sh_quote;
use std::process::Command;

/// `avm which`: prints where an alias comes from (local, global or plugin) and its command.
/// Errors when the alias does not exist.
pub fn cmd_which(key: &str) -> Result<()> {
    let cfg = load_state_with_aliases()?;
    if let Some(alias) = cfg.resolve_alias(key) {
        match alias.source {
            AliasSource::Local | AliasSource::Global => {
                let template = match alias.source {
                    AliasSource::Local => msg::GLOBAL_WHICH_LOCAL,
                    _ => msg::GLOBAL_WHICH_GLOBAL,
                };
                let origin = cfg
                    .alias_origins
                    .get(key)
                    .map(String::as_str)
                    .unwrap_or("unknown");
                ui::info(fill(
                    template,
                    &[
                        ("key", key),
                        ("command", &alias.command),
                        ("origin", origin),
                    ],
                ));
            }
            AliasSource::Plugin => {
                let plugin = alias.plugin_name.unwrap_or_else(|| "plugin".to_string());
                ui::info(fill(
                    msg::GLOBAL_WHICH_PLUGIN,
                    &[
                        ("key", key),
                        ("plugin", &plugin),
                        ("command", &alias.command),
                    ],
                ));
            }
        }
        return Ok(());
    }

    if let Some((version, source)) = cfg.resolve_tool(key) {
        let origin = match (&source, cfg.tool_origins.get(key)) {
            (_, Some(file)) => fill(msg::GLOBAL_WHICH_ORIGIN_FILE, &[("file", file)]),
            _ => alias_source_label(&source).to_string(),
        };
        ui::info(fill(
            msg::GLOBAL_WHICH_TOOL,
            &[("key", key), ("version", &version), ("origin", &origin)],
        ));
        return Ok(());
    }

    ui::info(fill(msg::GLOBAL_WHICH_NONE, &[("key", key)]));
    Ok(())
}

/// `avm resolve`: prints the shell command an alias expands to, with extra args appended and quoted.
/// Errors when the alias is unknown.
pub fn cmd_resolve(args: ResolveArgs) -> Result<()> {
    let cfg = load_state_with_aliases()?;
    let alias = cfg
        .resolve_alias(&args.key)
        .ok_or_else(|| alias_not_found_error(&args.key, &cfg))?;
    let script = match alias.argv {
        Some(argv) => argv
            .iter()
            .chain(&args.args)
            .map(|arg| sh_quote(arg))
            .collect::<Vec<_>>()
            .join(" "),
        None => build_shell_alias_string(&alias.command, &args.args)?,
    };
    println!("{script}");
    Ok(())
}

/// `avm run`: runs an alias, or forwards to a plugin command when the key names one.
/// A missing alias offers an interactive suggestion picker on a terminal and errors otherwise.
pub fn cmd_run(args: RunArgs) -> Result<()> {
    let cfg = load_state_with_aliases()?;
    let alias_key = &args.key;
    let alias = match plugins::route(&cfg, alias_key) {
        plugins::Route::Alias(alias) => alias,
        // `avm android versions`: a plugin command, not an alias.
        plugins::Route::Plugin => {
            let mut plugin_args = vec![args.key.clone()];
            plugin_args.extend(args.args);
            return cmd_provider_tool(plugin_args, &cfg);
        }
        plugins::Route::Missing if ui::can_select() => {
            let suggestions = cfg.suggest_aliases(alias_key);
            let Some(selected) = select_alias_suggestion(alias_key, &cfg, &suggestions)? else {
                return Ok(());
            };
            cfg.resolve_alias(&selected)
                .ok_or_else(|| alias_not_found_error(alias_key, &cfg))?
        }
        plugins::Route::Missing => return Err(alias_not_found_error(alias_key, &cfg)),
    };

    let env = if args.tools.is_empty() {
        child_env(&cfg)?
    } else {
        temporary_tool_env(&cfg, &args.tools, args.no_install)?
    };
    // ponytail: Windows runs aliases through `cmd /C` with sh-style quoting of
    // extra args; a cmd-aware quoter if args with spaces or quotes misbehave.
    let (shell, flag) = crate::platform::ALIAS_SHELL;
    let mut command = match alias.argv {
        Some(argv) => {
            let program = crate::platform::resolve_program(
                &argv[0],
                env.get("PATH").map(String::as_str).unwrap_or(""),
                true,
            )?;
            let mut command = Command::new(program);
            command.args(&argv[1..]).args(&args.args);
            command
        }
        None => {
            let mut command = Command::new(shell);
            command
                .arg(flag)
                .arg(build_shell_alias_string(&alias.command, &args.args)?);
            command
        }
    };
    let status = command
        .envs(env)
        .status()
        .with_context(|| fill(msg::GLOBAL_RUN_FAILED, &[("shell", shell)]))?;
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
            .with_context(|| fill(msg::GLOBAL_PLACEHOLDER_INVALID, &[("digits", &digits)]))?;
        if index == 0 || index > args.len() {
            return Err(anyhow!(fill(
                msg::GLOBAL_PLACEHOLDER_OUT_OF_BOUNDS,
                &[("index", &index.to_string())]
            )));
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

/// `name  command  [source]`, so the picker shows what each alias will run.
fn alias_label(cfg: &ResolvedConfig, name: &str) -> String {
    match cfg.resolve_alias(name) {
        Some(alias) => format!(
            "{name}  {}  [{}]",
            alias.command,
            alias_source_label(&alias.source)
        ),
        None => name.to_string(),
    }
}

/// Asks the user to pick one of the suggested aliases; returns `None` when cancelled.
/// Errors immediately when there are no suggestions.
fn select_alias_suggestion(
    query: &str,
    cfg: &ResolvedConfig,
    suggestions: &[String],
) -> Result<Option<String>> {
    if suggestions.is_empty() {
        return Err(anyhow!(fill(
            msg::GLOBAL_ALIAS_NOT_FOUND,
            &[("key", query)]
        )));
    }

    let labels: Vec<String> = suggestions.iter().map(|s| alias_label(cfg, s)).collect();
    match ui::select(
        &fill(msg::GLOBAL_ALIAS_PICK_TITLE, &[("key", query)]),
        &labels,
    )? {
        Some(index) => Ok(Some(suggestions[index].clone())),
        None => {
            ui::cancelled();
            Ok(None)
        }
    }
}

/// Builds the "alias not found" error, listing close alias names and any untrusted config files that may hide it.
fn alias_not_found_error(key: &str, cfg: &ResolvedConfig) -> anyhow::Error {
    let mut message = fill(msg::GLOBAL_ALIAS_NOT_FOUND, &[("key", key)]);
    let suggestions = cfg.suggest_aliases(key);
    if !suggestions.is_empty() {
        message.push_str(msg::GLOBAL_DID_YOU_MEAN);
        message.push_str(
            &suggestions
                .iter()
                .map(|s| format!("  avm {s}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    for file in &cfg.untrusted {
        message.push_str(&fill(
            msg::GLOBAL_UNTRUSTED_FILE,
            &[("file", &file.display().to_string())],
        ));
    }
    anyhow!(message)
}

enum HomeChoice {
    Alias(String),
    Tool(String),
    Help,
}

/// Bare `avm` on a terminal: one picker over this project's aliases and tools, so
/// running a script or changing a version needs no remembered command.
pub fn cmd_home() -> Result<()> {
    use clap::CommandFactory;
    let print_help = || Cli::command().print_help().map_err(Into::into);
    if !ui::can_select() {
        return print_help();
    }
    let cfg = load_state_with_aliases()?;
    let mut tools: Vec<_> = cfg.resolve_tools_with_source().into_iter().collect();
    tools.sort_by(|a, b| a.0.cmp(&b.0));
    let mut choices = Vec::new();
    let mut labels = Vec::new();
    for name in cfg.alias_names() {
        labels.push(alias_label(&cfg, name));
        choices.push(HomeChoice::Alias(name.clone()));
    }
    for (tool, (version, _)) in tools {
        labels.push(fill(
            msg::GLOBAL_HOME_TOOL_LABEL,
            &[("tool", &tool), ("version", &version)],
        ));
        choices.push(HomeChoice::Tool(tool));
    }
    if choices.is_empty() {
        return print_help();
    }
    labels.push(msg::GLOBAL_HOME_HELP_LABEL.to_string());
    choices.push(HomeChoice::Help);
    match ui::select(msg::GLOBAL_HOME_TITLE, &labels)? {
        Some(index) => match choices.swap_remove(index) {
            HomeChoice::Alias(key) => cmd_run(RunArgs {
                key,
                tools: Vec::new(),
                no_install: false,
                args: Vec::new(),
            }),
            HomeChoice::Tool(tool) => cmd_plugin_command(vec![tool]),
            HomeChoice::Help => print_help(),
        },
        None => Ok(()),
    }
}

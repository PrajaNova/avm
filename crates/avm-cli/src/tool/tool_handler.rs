use crate::common::env_utils::{child_env, extend_env, merge_env};
use crate::common::state::load_state;
use crate::global::provider_by_name;
use crate::resolver::ResolvedConfig;
use crate::tool::ExecArgs;
use anyhow::{anyhow, Context, Result};
use std::collections::{HashMap, HashSet};
use std::process::Command;

/// `avm exec`: runs a command with temporary `tool@version` environments on PATH, installing missing versions unless `--no-install`.
/// Errors when no command follows `--` or the command cannot be found.
pub fn cmd_exec(args: ExecArgs) -> Result<()> {
    let cfg = load_state()?;
    let env = temporary_tool_env(&cfg, &args.tools, args.no_install)?;
    let (program, arguments) = args
        .command
        .split_first()
        .ok_or_else(|| anyhow!("command required after --"))?;
    let program = crate::platform::resolve_program(
        program,
        env.get("PATH").map(String::as_str).unwrap_or(""),
        false,
    )?;
    let mut command = Command::new(program);
    command.args(arguments).envs(env);
    crate::platform::run_replacing_process(command)
}

/// Splits a `tool@version` argument into its parts and validates both.
/// Errors on a missing `@` or on characters unsafe for a tool name or version path.
fn tool_spec(value: &str) -> Result<(&str, &str)> {
    let (tool, version) = value
        .split_once('@')
        .ok_or_else(|| anyhow!("expected tool@version, got '{value}'"))?;
    let valid_tool = !tool.is_empty()
        && tool
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_');
    let version_name = version.strip_prefix("lts/").unwrap_or(version);
    if !valid_tool
        || version_name.is_empty()
        || version_name == "."
        || version_name == ".."
        || !version_name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-_+*".contains(&c))
    {
        return Err(anyhow!("invalid tool specification '{value}'"));
    }
    Ok((tool, version))
}

/// Overrides live only in this process; provider installs never write pins.
pub fn temporary_tool_env(
    cfg: &ResolvedConfig,
    specs: &[String],
    no_install: bool,
) -> Result<HashMap<String, String>> {
    let specs = specs
        .iter()
        .map(|s| tool_spec(s))
        .collect::<Result<Vec<_>>>()?;
    let mut seen = HashSet::new();
    for (tool, _) in &specs {
        if !seen.insert(*tool) {
            return Err(anyhow!("tool '{tool}' specified more than once"));
        }
    }
    let mut cfg = cfg.clone();
    let mut paths = Vec::new();
    let mut provider_env = HashMap::new();
    for (tool, spec) in specs {
        let provider = provider_by_name(tool)?;
        let version = if provider.is_installed(spec) {
            spec.to_string()
        } else if no_install {
            let range = if spec == "latest" { "*" } else { spec };
            crate::version_files::pick(tool, range, provider.installed_versions()?).ok_or_else(
                || anyhow!("{tool}@{spec} is not installed; omit --no-install to install it"),
            )?
        } else {
            crate::lockfile::resolve_spec(provider.as_ref(), spec.trim())?
        };
        if !provider.is_installed(&version) {
            if no_install {
                return Err(anyhow!(
                    "{tool}@{spec} is not installed; omit --no-install to install it"
                ));
            }
            eprintln!("Installing {tool} {version}...");
            provider.install(&version)?;
        }
        let executable = provider
            .executable_path(&version)?
            .ok_or_else(|| anyhow!("{tool} {version} has no installed executable"))?;
        let bin = executable
            .parent()
            .ok_or_else(|| anyhow!("{tool} executable has no parent directory"))?;
        if !paths.contains(&bin.to_path_buf()) {
            paths.push(bin.to_path_buf());
        }
        extend_env(&mut provider_env, provider.env_vars(&version)?);
        cfg.local_tools.insert(tool.to_string(), version);
    }
    let mut env = child_env(&cfg)?;
    env.extend(provider_env);
    env.extend(merge_env(&cfg));
    // Explicit selections must precede even a configured PATH or AVM shims.
    if let Some(path) = env.get("PATH") {
        for entry in std::env::split_paths(path) {
            if !paths.contains(&entry) {
                paths.push(entry);
            }
        }
    }
    env.insert(
        "PATH".to_string(),
        std::env::join_paths(paths)
            .context("failed to build temporary tool PATH")?
            .to_string_lossy()
            .into_owned(),
    );
    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{Cli, Commands};
    use crate::global::GlobalManager;
    use crate::tool::ToolManager;
    use clap::Parser;

    #[test]
    fn validates_tool_specs() {
        for spec in [
            "node@18",
            "node@lts",
            "node@lts/iron",
            "java@openjdk-21.0.1+12",
        ] {
            assert!(tool_spec(spec).is_ok(), "{spec}");
        }
        for spec in [
            "node",
            "@18",
            "node@",
            "../node@18",
            "node@../18",
            "node@..",
            "node@18@20",
        ] {
            assert!(tool_spec(spec).is_err(), "{spec}");
        }
    }

    #[test]
    fn parses_exec_and_run_without_consuming_target_flags() {
        for name in ["exec", "x"] {
            let cli = Cli::try_parse_from([
                "avm",
                name,
                "--no-install",
                "node@18",
                "java@21",
                "--",
                "node",
                "--help",
            ])
            .unwrap();
            let Some(Commands::Tool(ToolManager::Exec(args))) = cli.command else {
                panic!("expected exec")
            };
            assert!(args.no_install);
            assert_eq!(args.tools, ["node@18", "java@21"]);
            assert_eq!(args.command, ["node", "--help"]);
        }
        assert!(Cli::try_parse_from(["avm", "exec", "node@18"]).is_err());
        let cli = Cli::try_parse_from([
            "avm", "run", "dev", "--with", "node@18", "--with", "java@21", "--", "--help",
        ])
        .unwrap();
        let Some(Commands::Global(GlobalManager::Run(args))) = cli.command else {
            panic!("expected run")
        };
        assert_eq!(args.key, "dev");
        assert_eq!(args.tools, ["node@18", "java@21"]);
        assert_eq!(args.args, ["--help"]);
        let cli = Cli::try_parse_from(["avm", "run", "dev", "--port", "3000"]).unwrap();
        let Some(Commands::Global(GlobalManager::Run(args))) = cli.command else {
            panic!("expected run")
        };
        assert_eq!(args.args, ["--port", "3000"]);
        let cli =
            Cli::try_parse_from(["avm", "--env", "ci", "run", "dev", "--", "--env", "target"])
                .unwrap();
        assert_eq!(cli.profiles.as_deref(), Some("ci"));
        let Some(Commands::Global(GlobalManager::Run(args))) = cli.command else {
            panic!("expected run")
        };
        assert_eq!(args.args, ["--env", "target"]);
    }
}

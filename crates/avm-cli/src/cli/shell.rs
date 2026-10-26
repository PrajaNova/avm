use super::*;
use clap::CommandFactory;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Sh,
    Pwsh,
    Cmd,
}

impl Default for Shell {
    fn default() -> Self {
        if cfg!(windows) {
            Shell::Pwsh
        } else {
            Shell::Sh
        }
    }
}

impl Shell {
    pub fn unset(self, key: &str) -> String {
        match self {
            Shell::Sh => format!("unset {key}"),
            Shell::Pwsh => format!("Remove-Item Env:{key} -ErrorAction SilentlyContinue"),
            Shell::Cmd => format!("set \"{key}=\""),
        }
    }
    pub fn export(self, key: &str, value: &str) -> String {
        match self {
            Shell::Sh => format!("export {key}={}", sh_quote(value)),
            Shell::Pwsh => format!("$env:{key} = {}", pwsh_quote(value)),
            Shell::Cmd => format!("set \"{key}={value}\""),
        }
    }

    pub fn warn(self, message: &str) -> String {
        match self {
            Shell::Sh => format!("echo {} >&2", sh_quote(message)),
            Shell::Pwsh => format!("[Console]::Error.WriteLine({})", pwsh_quote(message)),
            Shell::Cmd => format!("echo {message} 1>&2"),
        }
    }
}

/// PowerShell single-quoted string: only `'` needs escaping (as `''`).
pub fn pwsh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// PowerShell setup: shims first on PATH, an `avm` function that routes
/// aliases like the sh one, and env re-applied after every `avm` call.
/// Works in Windows PowerShell 5.1 and pwsh 7.
pub fn pwsh_init_script() -> String {
    r#"$env:AVM_SHIM_DIR = if ($env:AVM_HOME) { Join-Path $env:AVM_HOME 'shims' } else { Join-Path $HOME '.avm\shims' }
& avm-bin shims install *> $null
$_avmSep = [IO.Path]::PathSeparator
$env:PATH = (@($env:AVM_SHIM_DIR) + @($env:PATH -split [regex]::Escape($_avmSep) | Where-Object { $_ -and $_ -ne $env:AVM_SHIM_DIR })) -join $_avmSep

function global:_avm_apply_env {
  $global:_avm_last_pwd = $PWD.Path
  $_avmEnv = (& avm-bin env --shell pwsh 2>$null) -join "`n"
  if ($_avmEnv) { try { Invoke-Expression $_avmEnv } catch {} }
}
_avm_apply_env

if (-not $global:_avm_original_prompt) { $global:_avm_original_prompt = (Get-Command prompt).ScriptBlock }
# Like zsh's chpwd: refresh at the prompt only when the directory changed.
function global:prompt {
  if ($PWD.Path -ne $global:_avm_last_pwd) { _avm_apply_env }
  & $global:_avm_original_prompt
}

function global:avm {
  if ($PWD.Path -ne $global:_avm_last_pwd) { _avm_apply_env }
  $builtins = __AVM_BUILTINS_PWSH__
  if ($args.Count -eq 0 -or $builtins -contains $args[0] -or $args[0] -like '-*') {
    & avm-bin @args
  } else {
    & avm-bin run @args
  }
  $rc = $LASTEXITCODE
  _avm_apply_env
  $global:LASTEXITCODE = $rc
}"#
    .replace("__AVM_BUILTINS_PWSH__", &builtin_commands().iter().map(|name| pwsh_quote(name)).collect::<Vec<_>>().join(","))
}

/// Every built-in subcommand name and alias, taken from the clap definition so the
/// shell wrappers can never drift from it. Anything else is an alias or a plugin command.
fn builtin_commands() -> Vec<String> {
    let mut cli = Cli::command();
    cli.build(); // adds the implicit `help` subcommand
    let mut names: Vec<String> = cli
        .get_subcommands()
        .flat_map(|cmd| std::iter::once(cmd.get_name()).chain(cmd.get_all_aliases()))
        .map(str::to_string)
        .collect();
    names.sort();
    names.dedup();
    names
}

pub fn shell_init_script() -> String {
    r#"unfunction avm 2>/dev/null || unset -f avm 2>/dev/null || true

AVM_SHIM_DIR="${AVM_HOME:-$HOME/.avm}/shims"
command avm-bin shims install >/dev/null 2>&1 || mkdir -p "$AVM_SHIM_DIR" 2>/dev/null || true

if [ -n "${ZSH_VERSION:-}" ]; then
  path=("${(@)path:#$AVM_SHIM_DIR}")
  path=("$AVM_SHIM_DIR" "${path[@]}")
  export PATH
elif [[ ":$PATH:" == *":$AVM_SHIM_DIR:"* ]]; then
  _avm_next_path=""
  _avm_old_ifs="$IFS"
  IFS=":"
  for _avm_path_entry in $PATH; do
    if [ "$_avm_path_entry" != "$AVM_SHIM_DIR" ] && [ -n "$_avm_path_entry" ]; then
      if [ -z "$_avm_next_path" ]; then
        _avm_next_path="$_avm_path_entry"
      else
        _avm_next_path="$_avm_next_path:$_avm_path_entry"
      fi
    fi
  done
  IFS="$_avm_old_ifs"
  export PATH="$AVM_SHIM_DIR${_avm_next_path:+:$_avm_next_path}"
  unset _avm_next_path _avm_old_ifs _avm_path_entry
else
  export PATH="$AVM_SHIM_DIR:$PATH"
fi
rehash 2>/dev/null || hash -r 2>/dev/null || true

# Apply provider/config env vars (ANDROID_HOME, JAVA_HOME, ...) to the live
# shell, restoring the original values of variables no longer selected.
_avm_apply_env() {
  _AVM_LAST_PWD="$PWD"
  eval "$(command avm-bin env 2>/dev/null)" 2>/dev/null || true
}
# Bash has no chpwd hook, so it refreshes at the prompt, but only after the
# directory changed (as zsh does) instead of starting avm-bin on every Enter.
_avm_prompt_env() {
  [ "$PWD" = "${_AVM_LAST_PWD:-}" ] || _avm_apply_env
}
_avm_apply_env

if [ -n "${ZSH_VERSION:-}" ]; then
  typeset -ga chpwd_functions
  chpwd_functions=("${(@)chpwd_functions:#_avm_apply_env}" _avm_apply_env)
elif [ -n "${BASH_VERSION:-}" ]; then
  if [[ $(declare -p PROMPT_COMMAND 2>/dev/null) == "declare -a"* ]]; then
    case " ${PROMPT_COMMAND[*]} " in
      *"_avm_prompt_env"*|*"_avm_apply_env"*) ;;
      *) PROMPT_COMMAND=(_avm_prompt_env "${PROMPT_COMMAND[@]}") ;;
    esac
  else
    case "${PROMPT_COMMAND:-}" in
      *"_avm_prompt_env"*|*"_avm_apply_env"*) ;;
      *) PROMPT_COMMAND="_avm_prompt_env${PROMPT_COMMAND:+; $PROMPT_COMMAND}" ;;
    esac
  fi
fi

avm() {
  _avm_prompt_env
  local _avm_key="${1:-}"
  local _avm_rc
  # Built-ins and flags go straight to the CLI; everything else is an alias or
  # plugin command, which `avm-bin run` resolves in this one process.
  case "$_avm_key" in
    ""|-*|__AVM_BUILTINS_SH__)
      command avm-bin "$@"
      ;;
    *)
      command avm-bin run "$@"
      ;;
  esac
  _avm_rc=$?
  _avm_apply_env
  return $_avm_rc
}
"#
    .replace("__AVM_BUILTINS_SH__", &builtin_commands().join("|"))
}

pub use avm_plugin_api::sh_quote;

#[cfg(test)]
mod tests {
    use super::*;

    // Regression: piping empty `avm-bin env` output straight into
    // Invoke-Expression throws (it rejects an empty -Command), and that
    // error isn't caught by try/catch since it's a pipeline binding error,
    // not a thrown exception — breaks shell-init on every fresh install.
    #[test]
    fn wrappers_list_every_builtin_from_clap() {
        for script in [shell_init_script(), pwsh_init_script()] {
            assert!(!script.contains("__AVM_BUILTINS"));
            for name in ["alias", "help", "run", "ls", "prune"] {
                assert!(script.contains(name), "{name}");
            }
        }
        assert!(
            builtin_commands().iter().any(|n| n == "alias")
                && builtin_commands().iter().any(|n| n == "help")
        );
    }

    #[test]
    fn pwsh_init_guards_against_empty_env_output() {
        let script = pwsh_init_script();
        assert!(
            !script.contains("Invoke-Expression }"),
            "must not pipe `avm-bin env` output directly into Invoke-Expression"
        );
        assert!(script.contains("if ($_avmEnv)"));
    }
}

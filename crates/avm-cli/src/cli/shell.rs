use super::*;

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
  try { (& avm-bin env --shell pwsh 2>$null) -join "`n" | Invoke-Expression } catch {}
}
_avm_apply_env

function global:avm {
  $builtins = 'init','add','list','ls','remove','rm','which','env','trust','self-update','help','shell-init','plugin','resolve','run','shims','exec-shim','node','java','--help','-h'
  if ($args.Count -gt 0 -and @('-v','--version') -contains $args[0]) {
    & avm-bin --version
  } elseif ($args.Count -eq 0 -or $builtins -contains $args[0]) {
    & avm-bin @args
  } else {
    & avm-bin resolve @args *> $null
    if ($LASTEXITCODE -eq 0) { & avm-bin run @args } else { & avm-bin @args }
  }
  $rc = $LASTEXITCODE
  _avm_apply_env
  $global:LASTEXITCODE = $rc
}"#
    .to_string()
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
# shell. Safe to call repeatedly; only exports what `avm env` prints.
_avm_apply_env() {
  eval "$(command avm-bin env 2>/dev/null)" 2>/dev/null || true
}
_avm_apply_env

avm() {
  if [ $# -eq 0 ]; then
    command avm-bin "$@"
    return $?
  fi

  local _avm_key="$1"
  local _avm_rc
  case "$_avm_key" in
    -v|--version)
      command avm-bin --version
      return $?
      ;;
    init|add|list|ls|remove|rm|which|env|trust|self-update|help|shell-init|plugin|completion|--help|-h|resolve|run|shims|exec-shim|node|java)
      command avm-bin "$@"
      _avm_rc=$?
      _avm_apply_env
      return $_avm_rc
      ;;
  esac

  if command avm-bin resolve "$@" >/dev/null 2>&1; then
    command avm-bin run "$@"
    _avm_rc=$?
    _avm_apply_env
    return $_avm_rc
  fi

  if command avm-bin "$_avm_key" --help >/dev/null 2>&1; then
    command avm-bin "$@"
    _avm_rc=$?
    _avm_apply_env
    return $_avm_rc
  fi
  command avm-bin run "$@"
  _avm_rc=$?
  _avm_apply_env
  return $_avm_rc
}
"#
    .to_string()
}

/// POSIX single-quote escaping: safe for any byte string.
pub fn sh_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/' | b':' | b'='))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

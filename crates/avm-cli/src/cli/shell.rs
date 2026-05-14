fn shell_init_script() -> String {
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

avm() {
  if [ $# -eq 0 ]; then
    command avm-bin "$@"
    return $?
  fi

  local _avm_key="$1"
    case "$_avm_key" in
    init|add|list|ls|remove|rm|which|env|tool|tools|version|help|shell-init|plugin|completion|--help|-h|--version|-v|resolve|run|shims|exec-shim|node|java)
      command avm-bin "$@"
      return $?
      ;;
  esac

  if command avm-bin resolve "$@" >/dev/null 2>&1; then
    command avm-bin run "$@"
    return $?
  fi

  if command avm-bin "$_avm_key" --help >/dev/null 2>&1; then
    command avm-bin "$@"
    return $?
  fi
  command avm-bin run "$@"
}
"#
    .to_string()
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

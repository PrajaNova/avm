fn shell_init_script() -> String {
    r#"unfunction avm 2>/dev/null || unset -f avm 2>/dev/null || true

AVM_SHIM_DIR="$HOME/.avm/shims"
if [[ -d "$AVM_SHIM_DIR" && ":$PATH:" != *":$AVM_SHIM_DIR:"* ]]; then
  export PATH="$AVM_SHIM_DIR:$PATH"
fi

avm() {
  if [ $# -eq 0 ]; then
    command avm-bin "$@"
    return $?
  fi

  local _avm_key="$1"
  case "$_avm_key" in
    init|add|list|ls|remove|rm|which|env|tool|tools|version|help|shell-init|plugin|completion|--help|-h|--version|-v|resolve|run|shims|exec-shim|node)
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
  command "$@"
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

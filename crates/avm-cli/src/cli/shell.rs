fn shell_init_script() -> String {
    let shim_dir = "$HOME/.avm/shims";
    format!(
        r#"
if [ -n "{shim_dir}" ] && [ -d "{shim_dir}" ]; then
  case ":$PATH:" in
    *":{shim_dir}:"*) ;;
    *) export PATH="{shim_dir}:$PATH" ;;
  esac
fi

avm() {{
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
  command "$@"
}}
"#
    )
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

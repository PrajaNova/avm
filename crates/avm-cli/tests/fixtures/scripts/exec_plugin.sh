#!/bin/sh
tool=TOOL
dir="$HOME/.avm/tools/$tool/$2"
case "$1" in
  is-installed) if [ -f "$dir/bin/$tool" ]; then echo '{"installed":true}'; else echo '{"installed":false}'; fi ;;
  installed-versions) echo '{"versions":["18.1.0","21.1.0"]}' ;;
  versions)
    case "$3" in
      major:18) version=18.1.0 ;; major:21) version=21.1.0 ;; *) version=22.1.0 ;;
    esac
    echo "{\"versions\":[{\"version\":\"$version\",\"label\":\"$version\",\"is_lts\":true,\"channel\":\"Iron\",\"is_security\":false}]}" ;;
  install)
    mkdir -p "$dir/bin"
    printf '#!/bin/sh\nprintf "%%s\\n" "%s-%s" "$@"\n' "$tool" "$2" > "$dir/bin/$tool"
    chmod +x "$dir/bin/$tool" ;;
  executable-path) echo "{\"path\":\"$dir/bin/$tool\"}" ;;
  env-vars)
    case "$tool" in java) key=JAVA_HOME ;; *) key=NODE_HOME ;; esac
    echo "{\"env\":{\"$key\":\"$dir\"}}" ;;
  *) exit 1 ;;
esac

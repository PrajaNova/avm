SUITE=java
source /e2e/lib.sh
avm_shell
W="$HOME/java"
P="$W/project"
check_aliases_and_env "$W" "$P"

log "java: plugin"
expect_ok "avm plugin add java (sha256-verified)" avm plugin add java

log "java: global 17 (install -g), local 21"
expect_ok "avm java install 17 -g" avm java install 17 -g
expect_ok "avm java install 21 (in project)" in_dir "$P" avm java install 21
hash -r
expect_contains "java outside the project is 17" "$(java -version 2>&1)" 'version "17'
expect_contains "java in the project is 21" "$(cd "$P" && java -version 2>&1)" 'version "21'
expect_contains "javac in the project is 21" "$(cd "$P" && javac -version 2>&1)" "javac 21"

log "java: JAVA_HOME"
expect_contains "JAVA_HOME outside → a 17 JDK" "$(avm env | grep JAVA_HOME)" "openjdk-17"
expect_contains "JAVA_HOME in project → a 21 JDK" "$(cd "$P" && avm env | grep JAVA_HOME)" "openjdk-21"

log "java: version files"
mkdir -p "$W/jv" && echo "17" > "$W/jv/.java-version"
expect_contains ".java-version 17" "$(cd "$W/jv" && avm which java)" "from ./.java-version"
expect_contains "java follows .java-version" "$(cd "$W/jv" && java -version 2>&1)" 'version "17'

finish

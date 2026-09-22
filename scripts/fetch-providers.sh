#!/usr/bin/env bash
# Build the node/java/android providers (now in their own repos — see
# github.com/PrajaNova/avm-plugin-{node,java,android}) and place the
# resulting binaries next to avm-bin, so `avm_runtime::builtin_plugin_process`
# (same directory as the running avm-bin) finds them for local dev.
#
# Release packaging does the equivalent as part of the CI build; this script
# is the local-dev-loop version of that step.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${1:-debug}"
TARGET_DIR="$ROOT_DIR/target/$PROFILE"
# Must live outside this repo's directory tree — a nested Cargo.toml under
# target/ would otherwise be picked up by this repo's own workspace root.
CACHE_DIR="${AVM_PROVIDERS_CACHE:-$HOME/.cache/avm-dev-providers}"

PROVIDERS=(
  "avm-plugin-node|https://github.com/PrajaNova/avm-plugin-node.git"
  "avm-plugin-java|https://github.com/PrajaNova/avm-plugin-java.git"
  "avm-plugin-android|https://github.com/PrajaNova/avm-plugin-android.git"
)

mkdir -p "$TARGET_DIR" "$CACHE_DIR"

for entry in "${PROVIDERS[@]}"; do
  name="${entry%%|*}"
  url="${entry##*|}"
  repo_dir="$CACHE_DIR/$name"

  if [ -d "$repo_dir/.git" ]; then
    echo "Updating $name..."
    git -C "$repo_dir" pull --ff-only
  else
    echo "Cloning $name..."
    git clone --depth 1 "$url" "$repo_dir"
  fi

  build_args=(build --manifest-path "$repo_dir/Cargo.toml")
  if [ "$PROFILE" = "release" ]; then
    build_args+=(--release)
  fi
  cargo "${build_args[@]}"

  cp "$repo_dir/target/$PROFILE/$name" "$TARGET_DIR/$name"
  echo "-> $TARGET_DIR/$name"
done

echo "Done. avm-bin will discover these next to itself in target/$PROFILE."

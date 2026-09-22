#!/bin/bash
set -e

REPO="prajanova/avm"
VERSION="${AVM_VERSION:-latest}"

echo "Installing avm..."

# Detect OS and architecture
uname_out="$(uname -s)"
case "${uname_out}" in
    Linux*)     os=linux;;
    Darwin*)    os=darwin;;
    *)          os=unknown
esac

case "$(uname -m)" in
    x86_64)     arch=amd64;;
    arm64)      arch=arm64;;
    aarch64)    arch=arm64;;
    *)          arch=unknown
esac

if [ "$os" = "unknown" ] || [ "$arch" = "unknown" ]; then
    echo "Error: Unsupported platform: ${uname_out} $(uname -m)"
    exit 1
fi

echo "Detected: $os/$arch"

# Download and install
binary_name="avm_${os}_${arch}"
if [ "$VERSION" = "latest" ]; then
    url="https://github.com/${REPO}/releases/latest/download/${binary_name}.tar.gz"
else
    url="https://github.com/${REPO}/releases/download/${VERSION}/${binary_name}.tar.gz"
fi

echo "Downloading $binary_name..."

# Create temp directory
tmpdir=$(mktemp -d)
trap "rm -rf $tmpdir" EXIT

cd "$tmpdir"

if command -v curl &> /dev/null; then
    curl -fsSL "$url" -o "${binary_name}.tar.gz"
else
    wget -q "$url" -O "${binary_name}.tar.gz"
fi

tar -xzf "${binary_name}.tar.gz"

# Create ~/.avm.json if it doesn't exist
if [ ! -f "$HOME/.avm.json" ]; then
    echo "{}" > "$HOME/.avm.json"
    echo "✓ Created $HOME/.avm.json"
fi

# Install binary as avm-bin (shell function will be named 'avm')
INSTALL_PATH=""
extracted_binary="avm-bin"
if [ ! -f "$extracted_binary" ] && [ -f "avm" ]; then
    extracted_binary="avm"
fi
if [ ! -f "$extracted_binary" ]; then
    echo "Error: archive did not contain avm-bin"
    exit 1
fi

if [ -w "/usr/local/bin" ]; then
    chmod +x "$extracted_binary"
    mv "$extracted_binary" /usr/local/bin/avm-bin
    INSTALL_PATH="/usr/local/bin/avm-bin"
elif sudo -n true 2>/dev/null; then
    chmod +x "$extracted_binary"
    sudo mv "$extracted_binary" /usr/local/bin/avm-bin
    INSTALL_PATH="/usr/local/bin/avm-bin"
else
    mkdir -p "$HOME/.local/bin"
    chmod +x "$extracted_binary"
    mv "$extracted_binary" "$HOME/.local/bin/avm-bin"
    INSTALL_PATH="$HOME/.local/bin/avm-bin"
    NEED_PATH_UPDATE=1
fi

echo "✓ Installed avm-bin to $INSTALL_PATH"

# Detect shell and add setup to config
CURRENT_SHELL=$(basename "${SHELL:-bash}")
SHELL_CONFIGS=()

case "$CURRENT_SHELL" in
    zsh)
        SHELL_CONFIGS+=("$HOME/.zshrc")
        ;;
    bash)
        # bash loads different files depending on OS/context
        if [ -f "$HOME/.bashrc" ]; then
            SHELL_CONFIGS+=("$HOME/.bashrc")
        fi
        if [ "$(uname -s)" = "Darwin" ] && [ -f "$HOME/.bash_profile" ]; then
            SHELL_CONFIGS+=("$HOME/.bash_profile")
        fi
        if [ ${#SHELL_CONFIGS[@]} -eq 0 ]; then
            SHELL_CONFIGS+=("$HOME/.bashrc")
        fi
        ;;
    *)
        [ -f "$HOME/.zshrc" ] && SHELL_CONFIGS+=("$HOME/.zshrc")
        [ -f "$HOME/.bashrc" ] && SHELL_CONFIGS+=("$HOME/.bashrc")
        ;;
esac

SHELL_SETUP_BLOCK='
# avm (Any Version Manager) - BEGIN (do not edit this block)
export PATH="$HOME/.local/bin:$PATH"
if command -v avm-bin >/dev/null 2>&1; then
  eval "$(avm-bin shell-init)"
fi
# avm - END
'

for cfg in "${SHELL_CONFIGS[@]}"; do
    [ -z "$cfg" ] && continue
    
    if [ ! -f "$cfg" ]; then
        touch "$cfg"
    fi

    if ! grep -q "# avm (Any Version Manager) - BEGIN" "$cfg" 2>/dev/null; then
        echo "$SHELL_SETUP_BLOCK" >> "$cfg"
        echo "✓ Added avm setup to $cfg"
    else
        echo "✓ avm setup already present in $cfg"
    fi
done

echo ""
echo "Welcome to avm."
echo ""
echo "To activate avm in your current shell, run:"
for cfg in "${SHELL_CONFIGS[@]}"; do
    [ -n "$cfg" ] && echo "  source $cfg"
done
echo ""
echo "Or simply restart your terminal."
echo ""
echo "Next steps:"
echo "  1. Initialize local config:  avm init"
echo "  2. Add a local alias:        avm add start 'npm run dev'"
echo "  3. Add a global alias:       avm add -g deploy 'sh ./deploy.sh'"
echo "  4. List aliases:             avm list"
echo "  5. Use an alias:             avm start"
echo ""

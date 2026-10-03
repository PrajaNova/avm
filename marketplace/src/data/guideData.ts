import { ComparisonItem } from '../types';

// Checked against each project's docs, September 2026 (sources in the avm
// README). "Not documented" = we couldn't find it, not confirmed absent.
export const COMPARISONS: ComparisonItem[] = [
  {
    feature: 'Implementation',
    avm: 'Rust',
    asdf: 'Go (rewritten from Bash in v0.16)',
    vfox: 'Go',
    mise: 'Rust',
    proto: 'Rust'
  },
  {
    feature: 'Plugin model',
    avm: 'Compiled executable per tool, separate process, typed JSON-over-stdio; asdf plugins via an adapter',
    asdf: 'Bash scripts',
    vfox: 'Lua',
    mise: 'asdf and vfox plugins plus backends (aqua, npm, cargo, …)',
    proto: 'WASM plugins, or TOML/JSON/YAML definitions'
  },
  {
    feature: 'Download verification',
    avm: 'sha256 on by default for plugins, avm itself and first-party runtimes (fails closed); build provenance attestations',
    asdf: 'Left to each plugin',
    vfox: 'Plugin-supplied checksum',
    mise: 'aqua backend: checksums, cosign/minisign/SLSA/attestations',
    proto: 'Checksums, minisign, GPG'
  },
  {
    feature: 'Existing version files',
    avm: '.tool-versions, .nvmrc, .node-version, package.json, .java-version, .sdkmanrc, on by default',
    asdf: '.tool-versions; others opt-in',
    vfox: '.tool-versions, .nvmrc, .node-version, .sdkmanrc',
    mise: 'Opt-in per tool',
    proto: '.nvmrc etc. on by default'
  },
  {
    feature: 'Config trust',
    avm: 'avm trust (hash-pinned; editing re-blocks)',
    asdf: 'Not documented',
    vfox: 'Not documented',
    mise: 'mise trust',
    proto: 'Not documented'
  },
  {
    feature: 'Windows',
    avm: 'PowerShell (phase 1); Windows plugins coming',
    asdf: 'WSL only',
    vfox: 'Native',
    mise: 'Native',
    proto: 'Native'
  },
  {
    feature: 'Lockfile',
    avm: 'Not yet (roadmap #25)',
    asdf: 'No',
    vfox: 'Not documented',
    mise: 'mise.lock (opt-in)',
    proto: '.protolock (unstable)'
  },
  {
    feature: 'Tasks',
    avm: 'Aliases + package.json scripts (no dependency graph yet, #33)',
    asdf: 'No',
    vfox: 'No',
    mise: 'Yes',
    proto: 'No (moon is separate)'
  },
  {
    feature: 'Pinned version missing',
    avm: 'Falls back to the system binary with a warning',
    asdf: 'Error',
    vfox: 'Not documented',
    mise: 'Auto-install',
    proto: 'Error (auto-install opt-in)'
  },
  {
    feature: 'Plugin subcommands',
    avm: 'Yes (e.g. avm android avd)',
    asdf: 'Yes',
    vfox: 'No',
    mise: 'Not documented',
    proto: 'Not documented'
  },
  {
    feature: 'Global npm packages across switches',
    avm: 'Built in',
    asdf: 'Per plugin (default-packages file)',
    vfox: 'Not documented',
    mise: 'Default-packages file (deprecated)',
    proto: 'Shared globals dir'
  }
];

export const INSTALL_METHODS = [
  {
    id: 'curl',
    name: 'Shell Script',
    tag: 'Recommended',
    command: 'curl -fsSL https://raw.githubusercontent.com/prajanova/avm/main/install.sh | bash',
    description: 'Detects OS and CPU (Apple Silicon / Intel Mac / Linux amd64 & arm64), verifies the sha256 against the release, and installs avm-bin.'
  },
  {
    id: 'brew',
    name: 'Homebrew',
    tag: 'macOS / Linux',
    command: 'brew install prajanova/tap/avm',
    description: 'Installs from the PrajaNova Homebrew tap (Apple Silicon, Intel, and Linux).'
  },
  {
    id: 'npm',
    name: 'npm / npx',
    tag: 'Node ecosystem',
    command: 'npm install -g @prajanova/avm@beta',
    description: 'Downloads the prebuilt, checksum-verified binary for your platform (macOS, Linux, Windows).'
  },
  {
    id: 'powershell',
    name: 'PowerShell',
    tag: 'Windows',
    command: 'irm https://raw.githubusercontent.com/PrajaNova/avm/main/install.ps1 | iex',
    description: 'Installs avm-bin to %LOCALAPPDATA%\\avm\\bin after verifying its sha256, and adds the shell hook to your PowerShell profile.'
  },
  {
    id: 'cargo',
    name: 'Cargo',
    tag: 'Rust developers',
    command: 'cargo install --git https://github.com/PrajaNova/avm.git avm-cli',
    description: 'Builds and installs directly from source using the latest Rust compiler.'
  }
];

export const SHELL_SETUP_GUIDE = [
  {
    shell: 'zsh',
    file: '~/.zshrc',
    snippet: `echo 'eval "$(avm shell-init)"' >> ~/.zshrc\nsource ~/.zshrc`
  },
  {
    shell: 'bash',
    file: '~/.bashrc',
    snippet: `echo 'eval "$(avm shell-init)"' >> ~/.bashrc\nsource ~/.bashrc`
  },
  {
    shell: 'pwsh',
    file: '$PROFILE',
    snippet: `Add-Content $PROFILE 'Invoke-Expression ((& avm-bin shell-init pwsh) -join "\`n")'`
  }
];

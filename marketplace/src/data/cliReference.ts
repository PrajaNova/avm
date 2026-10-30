import { CliCommand } from '../types';

export const CLI_COMMANDS: CliCommand[] = [
  // Core & Aliases
  {
    command: 'avm init',
    category: 'Core',
    description: 'Initializes an .avm.json configuration file and adds .avm.local.json to .gitignore in the current working directory.',
    example: 'avm init',
    outputExample: '✓ Created .avm.json in current directory'
  },
  {
    command: 'avm which',
    args: '<alias-or-tool>',
    category: 'Core',
    description: 'Shows the exact file an alias or tool version comes from, including parent configs, profiles, personal overrides, and version files (.nvmrc, .tool-versions, …).',
    example: 'avm which node',
    outputExample: `tool 'node': 22.14.0 (from ./.nvmrc)`
  },
  {
    command: 'avm trust',
    args: '[path] [--revoke | --list]',
    category: 'Core',
    description: "Trusts this project's .avm.json and .env so their aliases and env apply. It shows what it enables; editing the file outside avm blocks it again until re-trusted.",
    example: 'avm trust',
    outputExample: `  alias dev → pnpm run dev
  env   NODE_ENV=development
✓ Trusted ~/code/my-app/.avm.json`
  },
  {
    command: 'avm <alias>',
    args: '[args...]',
    category: 'Aliases',
    description: 'Executes a configured project alias or package.json script directly with injected environment and tools.',
    example: 'avm dev --port 3000'
  },
  {
    command: 'avm resolve',
    args: '<alias> [args...]',
    category: 'Aliases',
    description: 'Expands and prints the exact shell command that would be executed.',
    example: 'avm resolve dev'
  },
  {
    command: 'avm exec',
    args: '[--no-install] <tool@version>... -- <command> [args...]',
    category: 'Core',
    description: 'Runs a command with temporary tool versions (also: avm x), provider env and trusted project env. Installs missing versions without changing pins or lockfiles; --no-install selects installed versions only. Plugins must already be installed.',
    example: 'avm exec node@18 java@21 -- ./gradlew build'
  },
  {
    command: 'avm run',
    args: '<alias> [--with <tool@version>]... [--no-install] [-- args...]',
    category: 'Aliases',
    description: 'Runs an alias, optionally with temporary tool versions. Repeat --with for multiple tools and place options before alias arguments. Missing versions install automatically unless --no-install is set.',
    example: 'avm run --with node@18 dev --port 3000'
  },
  {
    command: 'avm -p install',
    args: '[--frozen]',
    category: 'Core',
    description: 'Installs project tools without changing pins, including global selections only when unlocked. Uses exact versions and verified downloads when .avm.lock exists. --frozen validates project selections and requires an asset for this platform.',
    example: 'avm -p install --frozen'
  },
  {
    command: 'avm lock',
    args: '[--platform linux_amd64,darwin_arm64,windows_amd64]',
    category: 'Core',
    description: 'Locks project selections, excluding personal global pins, with exact versions, plugin identities and per-platform SHA-256 checksums. Requires updated native Node or Java plugins; Android and asdf support unlocked installs only.',
    example: 'avm lock --platform linux_amd64,darwin_arm64'
  },
  {
    command: 'avm env',
    args: '[--shell sh|pwsh|cmd] [--env <profiles>]',
    category: 'Env',
    description: 'Prints shell statements for merged env and provider vars (JAVA_HOME, ANDROID_HOME, …). Loads parent configs, then nearest-directory base, profiles (AVM_ENV=ci,staging or --env ci,staging; later names win), and .avm.local.json overrides. Restores original values when leaving a project. Shell setup refreshes at prompts, on zsh directory changes, and around avm commands.',
    example: 'avm env --shell pwsh',
    outputExample: `$env:NODE_ENV = 'development'
$env:JAVA_HOME = 'C:\\Users\\dev\\.avm\\tools\\java\\openjdk-17.0.13+11'`
  },
  {
    command: 'avm env add',
    args: '[-g] <KEY> <value>',
    category: 'Env',
    description: 'Adds an env var to local (or global with -g) .avm.json. Also avm ea. See env remove and env list for other operations.',
    example: 'avm env add NODE_ENV development',
    outputExample: "✓ Added local env var 'NODE_ENV=development'"
  },

  // Plugins
  {
    command: 'avm plugin install',
    args: '<name | owner/repo | git-url | path>',
    category: 'Plugins',
    description: "Installs a plugin's compiled release for your platform from GitHub releases (a first-party name, or owner/repo), after verifying its sha256 against the release's checksums.txt. A mismatch aborts with nothing installed. Run it again to update. Also `avm -p install`. Legacy asdf plugins use the compatibility adapter.",
    example: 'avm plugin install node',
    outputExample: `Fetching 'node' from PrajaNova/avm-plugin-node...
✓ Installed node v0.2.0`
  },
  {
    command: 'avm plugin list',
    category: 'Plugins',
    description: 'Lists every tool with its active version, where it is pinned, and its installed versions (the active one starred), plus plugin-provided aliases. Also `avm -p list` and `avm plugin ls`. Aliases are under `avm alias list`, env vars under `avm env list`.',
    example: 'avm plugin list',
    outputExample: `TOOL  ACTIVE   SCOPE  SET BY       INSTALLED
node  22.14.0  local  ./.avm.json  20.11.1  22.14.0*`
  },
  {
    command: 'avm plugin search',
    args: '<keyword>',
    category: 'Plugins',
    description: 'Lists first-party plugins and GitHub repos tagged with the avm-plugin topic that match the keyword.',
    example: 'avm plugin search java'
  },
  {
    command: 'avm plugin uninstall',
    args: '<name>',
    category: 'Plugins',
    description: 'Uninstalls an installed plugin binary from ~/.avm/plugins.',
    example: 'avm plugin uninstall java'
  },

  // Version Management
  {
    command: 'avm <plugin> versions',
    category: 'Versions',
    description: 'Lists installable upstream versions for a plugin (e.g. node, java, android).',
    example: 'avm node versions'
  },
  {
    command: 'avm <plugin> <filter> versions',
    category: 'Versions',
    description: 'Filters available versions by major line or query.',
    example: 'avm node 20 versions'
  },
  {
    command: 'avm <plugin> latest versions',
    category: 'Versions',
    description: 'Shows the latest upstream version available for installation.',
    example: 'avm java latest versions'
  },
  {
    command: 'avm <plugin> use',
    args: '<version> [--global]',
    category: 'Versions',
    description: 'Sets the active version in local .avm.json (or global ~/.avm.json with --global).',
    example: 'avm node use 22.14.0'
  },
  {
    command: 'avm <plugin> install',
    args: '<version> [--global | --no-pin]',
    category: 'Versions',
    description: 'Installs a runtime and pins it locally, also setting a global pin if none exists. --global pins globally only; --no-pin installs without changing pins. Fuzzy versions resolve before installation.',
    example: 'avm java install 17'
  },
  {
    command: 'avm <plugin> uninstall',
    args: '<version>',
    category: 'Versions',
    description: 'Removes an installed runtime version from ~/.avm/tools/.',
    example: 'avm node uninstall 18.20.0'
  },

  // Shims & Shell
  {
    command: 'avm shell-init',
    category: 'Shims',
    args: '[sh | pwsh]',
    description: 'Prints the shell hook that puts shims on PATH and applies env after each avm call (bash/zsh by default, or PowerShell).',
    example: 'eval "$(avm shell-init)"'
  },
  {
    command: 'avm shims install',
    category: 'Shims',
    description: 'Regenerates shims for all managed tools in ~/.avm/shims.',
    example: 'avm shims install'
  },
  {
    command: 'avm shims path',
    category: 'Shims',
    description: 'Prints the absolute path to the avm shims directory.',
    example: 'avm shims path',
    outputExample: '~/.avm/shims'
  },
  {
    "command": "avm --version",
    "args": "",
    "category": "Core",
    "description": "Print the installed CLI version; -v is the short form.",
    "example": "avm --version"
  },
  {
    "command": "avm alias add",
    "args": "[-g] <key> <command...>",
    "category": "Aliases",
    "description": "Add a local or global alias. Also avm -al add.",
    "example": "avm alias add test \"npm test\""
  },
  {
    "command": "avm alias remove",
    "args": "[-g] <key>",
    "category": "Aliases",
    "description": "Remove an alias. Also avm alias rm.",
    "example": "avm alias remove test"
  },
  {
    "command": "avm alias list",
    "args": "",
    "category": "Aliases",
    "description": "List merged configured aliases. Also avm alias ls and avm -al list.",
    "example": "avm alias list"
  },

  {
    "command": "avm env remove",
    "args": "[-g] <key>",
    "category": "Env",
    "description": "Remove a configured variable from the local or global base config. Also avm env rm.",
    "example": "avm env remove NODE_ENV"
  },
  {
    "command": "avm env list",
    "args": "",
    "category": "Env",
    "description": "List merged configured environment values; use bare avm env for provider variables and shell exports. Also avm env ls.",
    "example": "avm env list"
  },
  {
    "command": "avm outdated",
    "args": "[--json]",
    "category": "Versions",
    "description": "Show current runtime selections, newest releases within the selected range and overall, and pin sources.",
    "example": "avm outdated --json"
  },
  {
    "command": "avm upgrade",
    "args": "[tools...] [--bump] [--dry-run] [-y]",
    "category": "Versions",
    "description": "Install newer versions within each pin range and update editable base pins. --bump allows major changes and asks for confirmation; -y confirms. Inherited/profile/version-file pins are kept at their source. Regenerate locks after pin changes.",
    "example": "avm upgrade node --dry-run"
  },
  {
    "command": "avm prune",
    "args": "[--older-than 90d] [--include-unrecorded] [--dry-run] [-y]",
    "category": "Versions",
    "description": "Remove versions unused by global pins or tracked projects. Preview first. --include-unrecorded considers versions without usage records; -y confirms deletion.",
    "example": "avm prune --older-than 90d --dry-run"
  },
  {
    "command": "avm self-update",
    "args": "[--version <version>]",
    "category": "Core",
    "description": "Update direct installations with checksum verification. For package-manager installations, print the matching upgrade command.",
    "example": "avm self-update"
  },
  {
    "command": "avm create",
    "args": "<name>",
    "category": "Plugins",
    "description": "Print instructions to create a plugin from the official template. Does not create or publish the repository automatically.",
    "example": "avm create kotlin"
  },
  {
    "command": "avm <plugin>",
    "args": "",
    "category": "Versions",
    "description": "Open the interactive provider menu in a terminal; show status when interactive input is unavailable.",
    "example": "avm node"
  },
  {
    "command": "avm <plugin> list",
    "args": "",
    "category": "Versions",
    "description": "Show selected and installed versions. Also avm <plugin> ls. versions has the alias available, and use has the alias set.",
    "example": "avm java list"
  },
  {
    "command": "avm shims activate",
    "args": "",
    "category": "Shims",
    "description": "Add the shim directory to Unix shell startup files (.zshenv, .bashrc, .profile). On Windows, add the output of shims path to user PATH separately.",
    "example": "avm shims activate"
  },
  {
    "command": "avm shims remove",
    "args": "<tool>",
    "category": "Shims",
    "description": "Remove one shim without uninstalling its runtime. Regenerate with shims install (also shims reshim).",
    "example": "avm shims remove node"
  },
  {
    "command": "avm android avd list",
    "args": "",
    "category": "Plugins",
    "description": "List Android Virtual Devices using the selected installed Android SDK. Also avd ls.",
    "example": "avm android avd list"
  },
  {
    "command": "avm android avd create",
    "args": "[name] [--api <level>] [--device <id>]",
    "category": "Plugins",
    "description": "Create an AVD interactively or supply name, API and device for scripting. Automatically install its matching system image in the selected SDK; device type selects the appropriate phone/TV/Wear/Automotive image tag. Also avd new.",
    "example": "avm android avd create pixel-test --api 34 --device pixel_6"
  },
  {
    "command": "avm android avd start",
    "args": "<name> [emulator args...]",
    "category": "Plugins",
    "description": "Launch an AVD with the selected SDK emulator. Additional arguments are forwarded. Also avd run.",
    "example": "avm android avd start pixel-test -no-snapshot"
  },
  {
    "command": "avm android avd delete",
    "args": "<name>",
    "category": "Plugins",
    "description": "Delete an AVD using the selected SDK. Also avd remove or avd rm.",
    "example": "avm android avd delete pixel-test"
  }
];

import { CliCommand } from '../types';

export const CLI_COMMANDS: CliCommand[] = [
  // Core & Aliases
  {
    command: 'avm init',
    category: 'Core',
    description: 'Initializes an .avm.json configuration file in the current working directory.',
    example: 'avm init',
    outputExample: '✓ Created .avm.json in current directory'
  },
  {
    command: 'avm add',
    args: '[--global] <alias> <command>',
    category: 'Aliases',
    description: 'Adds an executable alias to local or global .avm.json configuration.',
    example: 'avm add dev "pnpm run dev --filter web"',
    outputExample: "✓ Added local alias 'dev'"
  },
  {
    command: 'avm remove',
    args: '[--global] <alias>',
    category: 'Aliases',
    description: 'Removes an alias from the configuration file.',
    example: 'avm remove dev',
    outputExample: "✓ Removed local alias 'dev'"
  },
  {
    command: 'avm list',
    category: 'Core',
    description: 'Displays all merged aliases, environment variables, tools, and plugin commands in effect.',
    example: 'avm list',
    outputExample: `Aliases:
  dev       pnpm run dev (local)
  release   npm run release $1 (global)
Tools:
  node      22.14.0 (local)
  java      17.0.13+11 (global)
Env:
  NODE_ENV  development`
  },
  {
    command: 'avm which',
    args: '<alias-or-tool>',
    category: 'Core',
    description: 'Shows where an alias or tool version comes from: local or global .avm.json, or the version file that pinned it (.nvmrc, .tool-versions, …).',
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
    command: 'avm env',
    args: '[--shell sh|pwsh|cmd]',
    category: 'Env',
    description: 'Prints export statements for merged env and provider vars (JAVA_HOME, ANDROID_HOME, …) in sh, PowerShell, or cmd syntax.',
    example: 'avm env --shell pwsh',
    outputExample: `$env:NODE_ENV = 'development'
$env:JAVA_HOME = 'C:\\Users\\dev\\.avm\\tools\\java\\openjdk-17.0.13+11'`
  },
  {
    command: 'avm env add',
    args: '[-g] <KEY> <value>',
    category: 'Env',
    description: 'Adds an env var to local (or global with -g) .avm.json. Also: avm env remove, avm env list.',
    example: 'avm env add NODE_ENV development',
    outputExample: "✓ Added local env var 'NODE_ENV=development'"
  },

  // Plugins
  {
    command: 'avm plugin add',
    args: '<name | org/repo | git-url>',
    category: 'Plugins',
    description: "Installs a plugin's compiled release for your platform from the marketplace, after verifying its sha256 against the release's checksums.txt. A mismatch aborts with nothing installed.",
    example: 'avm plugin add node',
    outputExample: `Fetching 'node' from PrajaNova/avm-plugin-node...
✓ Installed node v0.2.0`
  },
  {
    command: 'avm plugin list',
    category: 'Plugins',
    description: 'Lists all currently installed plugins and their discovery paths.',
    example: 'avm plugin list',
    outputExample: `Installed plugins:
  - node    (~/.avm/plugins/avm-plugin-node/bin/avm-plugin)
  - java    (~/.avm/plugins/avm-plugin-java/bin/avm-plugin)
  - android (~/.avm/plugins/avm-plugin-android/bin/avm-plugin)`
  },
  {
    command: 'avm plugin available',
    category: 'Plugins',
    description: 'Queries the marketplace registry and lists all installable plugins.',
    example: 'avm plugin available'
  },
  {
    command: 'avm plugin remove',
    args: '<name>',
    category: 'Plugins',
    description: 'Uninstalls an installed plugin binary from ~/.avm/plugins.',
    example: 'avm plugin remove java'
  },
  {
    command: 'avm plugin update',
    args: '<name>',
    category: 'Plugins',
    description: 'Checks for newer releases of the plugin and updates the binary.',
    example: 'avm plugin update node'
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
    args: '<version>',
    category: 'Versions',
    description: 'Downloads, extracts, and manages a runtime version.',
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
    args: '[pwsh]',
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
  }
];

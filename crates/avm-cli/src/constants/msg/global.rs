//! Global provider and tool commands messages.

// which / resolve / run / home
pub const GLOBAL_WHICH_LOCAL: &str = "local alias '{key}': {command} (from {origin})";
pub const GLOBAL_WHICH_GLOBAL: &str = "global alias '{key}': {command} (from {origin})";
pub const GLOBAL_WHICH_PLUGIN: &str = "plugin alias '{key}' from {plugin}: {command}";
pub const GLOBAL_WHICH_TOOL: &str = "tool '{key}': {version} ({origin})";
pub const GLOBAL_WHICH_ORIGIN_FILE: &str = "from {file}";
pub const GLOBAL_WHICH_NONE: &str = "No mapping found for '{key}'.";
pub const GLOBAL_RUN_FAILED: &str = "failed to run alias via {shell}";
pub const GLOBAL_PLACEHOLDER_INVALID: &str = "invalid placeholder ${digits}";
pub const GLOBAL_PLACEHOLDER_OUT_OF_BOUNDS: &str = "placeholder ${index} out of bounds";
pub const GLOBAL_ALIAS_NOT_FOUND: &str = "alias '{key}' not found";
pub const GLOBAL_ALIAS_PICK_TITLE: &str = "Alias '{key}' not found";
pub const GLOBAL_DID_YOU_MEAN: &str = "\n\nDid you mean?\n";
pub const GLOBAL_UNTRUSTED_FILE: &str =
    "\n\n{file} is not trusted, so its aliases are disabled. Review it, then run 'avm trust {file}'.";
pub const GLOBAL_HOME_TOOL_LABEL: &str = "{tool} {version}  [versions, install, pin]";
pub const GLOBAL_HOME_HELP_LABEL: &str = "help  [all commands]";
pub const GLOBAL_HOME_TITLE: &str = "avm: what do you want to run?";

// provider commands
pub const PROVIDER_NAME_REQUIRED: &str = "plugin name required";
pub const PROVIDER_UNINSTALLED: &str = "Removed {name} {version}";
pub const PROVIDER_PASSTHROUGH_FAILED: &str = "failed to run `avm {name} {args}`";
pub const PROVIDER_UNKNOWN_COMMAND: &str =
    "unknown {name} command. Run `avm {name}` for an interactive menu, or `avm {name} --help`";
pub const PROVIDER_FILTER_UNKNOWN: &str = "unknown version filter: {value}";
pub const PROVIDER_INSTALLED: &str = "Installed {name} {version}";
pub const PROVIDER_INSTALLED_GLOBAL: &str = "Installed {name} {version} (global pin set)";
pub const PROVIDER_INSTALLED_BOTH: &str = "Installed {name} {version} (local + global pin set)";
pub const PROVIDER_INSTALLED_LOCAL: &str = "Installed {name} {version} (local pin set)";
pub const PROVIDER_SET_GLOBAL: &str = "Set global {tool} version to {version}";
pub const PROVIDER_SET_LOCAL: &str = "Set local {tool} version to {version}";
pub const PROVIDER_INSTALLING: &str = "Installing {name} {version}...";
pub const PROVIDER_HELP: &str = concat!(
    "Usage: avm {name} [COMMAND]\n",
    "\n",
    "Commands:\n",
    "  list                         Show selected and installed versions\n",
    "  versions                     Pick from available versions\n",
    "  <major> versions             Pick from one major line\n",
    "  latest versions              Pick the latest available version\n",
    "  use <version> [-g|--global]  Set version locally or globally\n",
    "  set <version> [-g|--global]  Alias for use\n",
    "  install <version|latest|N>   Install + auto-pin (local; global if unpinned)\n",
    "  install <version> --global   Install + pin globally only\n",
    "  install <version> --no-pin   Install without pinning\n",
    "  uninstall <version>          Remove managed version"
);
pub const PROVIDER_SHIM_SHADOWED: &str =
    "warning: plain `{tool}` currently resolves to {path} before AVM shims";
pub const PROVIDER_SHIM_HINT: &str =
    "run `eval \"$(avm shell-init)\"` and then `rehash` or `hash -r`";
pub const PROVIDER_STATUS_PLUGIN: &str = "Plugin: {name}";
pub const PROVIDER_STATUS_SELECTED: &str = "Selected version: {version} ({source})";
pub const PROVIDER_STATUS_SELECTED_NONE: &str = "Selected version: none";
pub const PROVIDER_COMMANDS_HEADING: &str = "Commands:";
pub const PROVIDER_STATUS_COMMANDS: [&str; 4] = [
    "avm {name} versions",
    "avm {name} use <version>",
    "avm {name} install <version>",
    "avm {name} uninstall <version>",
];
pub const PROVIDER_INSTALLED_NONE: &str = "Installed {name} versions: none";
pub const PROVIDER_INSTALLED_LIST: &str = "Installed {name} versions: {versions}";
pub const PROVIDER_AVAILABLE_NONE: &str = "Available {name} versions: none";
pub const PROVIDER_AVAILABLE_HEADING: &str = "Available {name} versions:";
pub const PROVIDER_USE_HEADING: &str = "Use:";
pub const PROVIDER_USE_COMMANDS: [&str; 2] =
    ["avm {name} install <version>", "avm {name} use <version>"];

// provider menus
pub const PROVIDER_MENU_TITLE: &str = "avm {name} — what next?";
pub const PROVIDER_MENU_ACTIONS: [&str; 5] = [
    "Show selected & installed versions",
    "Browse & install a version",
    "Install the latest version",
    "Uninstall an installed version",
    "Show all commands",
];
pub const PROVIDER_UNINSTALL_NONE: &str = "No installed {name} versions to remove.";
pub const PROVIDER_UNINSTALL_TITLE: &str = "Uninstall which {name} version?";
pub const PROVIDER_VERSIONS_TITLE: &str = "Available {name} versions (✓ installed, ● selected)";
pub const PROVIDER_SCOPE_LOCAL: &str = "Local: this project (writes .avm.json)";
pub const PROVIDER_SCOPE_GLOBAL: &str = "Global: every project";
pub const PROVIDER_SCOPE_TITLE: &str = "Use {name} {version} where?";

// outdated / upgrade
pub const OUTDATED_LIST_FAILED: &str = "couldn't list {tool} versions";
pub const OUTDATED_NONE_PINNED: &str = "No tools pinned here.";
pub const OUTDATED_PINNED_BY_FILE: &str = "{tool}: pinned by {source}; edit it to move to {target}";
pub const OUTDATED_UP_TO_DATE: &str = "{tool}: {current} is up to date";
pub const OUTDATED_PIN_FILE: &str = "installs; the version file already covers it";
pub const OUTDATED_PIN_SCOPE: &str = "{source} pin";
pub const OUTDATED_PLAN: &str = "{tool}: {current} → {target} ({pin})";
pub const OUTDATED_BUMP_CONFIRM: &str =
    "Move these pins to the latest release (may cross a major version)?";

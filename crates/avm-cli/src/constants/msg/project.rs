//! Project init, install, lock, prune and trust messages.

pub const PROJECT_CWD_FAILED: &str = "failed to read current directory";

pub const INIT_EXISTS: &str = "{file} already exists";
pub const INIT_READ_GITIGNORE: &str = "read .gitignore";
pub const INIT_UPDATE_GITIGNORE: &str = "update .gitignore";
pub const INIT_CREATED: &str = "Created {file} in current directory";

pub const LOCK_NOTHING: &str = "Nothing to lock. Add a tool first: avm node use 20";
pub const LOCK_LEGACY_PLUGIN: &str = "{tool} requires a native plugin with lockfile support; legacy asdf plugins can use `avm -p install` without a lockfile";
pub const LOCK_CANNOT_DOWNLOADS: &str =
    "{tool} cannot lock downloads; its plugin is too old, run `avm plugin install {tool}`";
/// `returned` and `resolved` are the version the plugin answered with and the one requested.
pub const LOCK_RESOLVE_VERSION: &str = "{tool} resolve returned {returned} instead of {resolved}";
pub const LOCK_RESOLVE_PLATFORMS: &str =
    "{tool} resolve returned downloads for a different set of platforms";
pub const LOCK_NO_DOWNLOAD: &str = "{tool} did not return a download for {platform}";
pub const LOCK_BAD_MANIFEST: &str = "invalid plugin manifest for {tool}";
/// `count` is the number of locked tools; `platforms` is comma separated.
pub const LOCK_DONE: &str = "✓ Locked {count} tools in {path} ({platforms})";
pub const LOCK_NOT_A_FILE: &str = "{path} is not a file";
pub const LOCK_FIND_DIR: &str = "find project lock directory";

pub const INSTALL_NOTHING_FROZEN: &str = "Nothing to install: no tools in .avm.json.";
pub const INSTALL_FROZEN_NO_LOCK: &str =
    "--frozen requires .avm.lock; run `avm plugin lock` and commit it first";
pub const INSTALL_STALE_LOCK: &str =
    "avm: {err}; installing from .avm.json and refreshing the lock";
pub const INSTALL_NO_TOOLS: &str = "No tools in .avm.json. Add one: avm node use 20";
pub const INSTALL_MISSING_FROM_LOCK: &str = "{tool} missing from .avm.lock";
/// `label` is empty or ` (global)`.
pub const INSTALL_ALREADY: &str = "{tool} {version} already installed{label}";
pub const INSTALL_DONE: &str = "Installed {tool} {version}{label}";
pub const INSTALL_NOT_INSTALLED: &str =
    "{tool} {version} installer succeeded but the tool is not installed";
pub const INSTALL_LOCK_WRITE_FAILED: &str = "avm: installed, but could not write .avm.lock: {err}";
pub const INSTALL_REFRESH_FAILED: &str =
    "avm: could not refresh .avm.lock ({err}); run `avm plugin lock`";
pub const INSTALL_NO_NATIVE_PLUGIN: &str = "{tool} requires a native plugin for locked installs";
/// `version` is the plugin version the lockfile expects.
pub const INSTALL_PLUGIN_DIFFERS: &str = "{tool} plugin differs from .avm.lock; install plugin {version} or regenerate with `avm plugin lock`";
pub const INSTALL_CHECKSUM_DIFFERS: &str = "{tool} plugin checksum differs from .avm.lock; restore the locked plugin or run `avm plugin lock`";
pub const INSTALL_PLUGIN_UNPINNED: &str =
    "{tool} plugin binary not pinned for {platform}; only plugin name and version are checked";
pub const INSTALL_NO_LOCKED_DOWNLOAD: &str =
    ".avm.lock has no {tool} download for {platform}; run `avm plugin lock --platform {platform}`";

pub const PRUNE_DAYS_FORMAT: &str = "--older-than takes days, like 90d";
pub const PRUNE_RESOLVE_FAILED: &str =
    "cannot safely prune: failed to resolve tracked project {path}";
pub const PRUNE_SKIPPED: &str = "Skipped {count} version(s) avm hasn't seen used yet; add --include-unrecorded to consider them (check the list with --dry-run first).";
pub const PRUNE_NOTHING: &str = "Nothing to prune.";
pub const PRUNE_LAST_USED: &str = "last used {days}d ago";
pub const PRUNE_NO_USE: &str = "no recorded use";
pub const PRUNE_ENTRY: &str = "  {tool} {version}  {size}  ({used})";
pub const PRUNE_GLOBAL_PACKAGES: &str =
    "    warning: holds global packages ({packages}); other versions fall back to them";
pub const PRUNE_SUMMARY: &str = "{count} version(s), {size} to reclaim.";
pub const PRUNE_KEPT: &str =
    "Kept: global pins and pins of {count} project(s) avm has seen (it records them as you use them).";
pub const PRUNE_CONFIRM: &str = "Remove them?";
pub const PRUNE_RACE: &str =
    "a project was tracked while pruning; nothing was removed, re-run `avm plugin prune`";
pub const PRUNE_REMOVE_FAILED: &str = "failed to remove {tool} {version}";
pub const PRUNE_REMOVED: &str = "Removed {tool} {version}";

pub const TRUST_NONE: &str = "no active avm config or .env in {path}";
pub const TRUST_REVOKED: &str = "✓ Revoked trust for {path}";
pub const TRUST_WAS_NOT: &str = "Was not trusted: {path}";
pub const TRUST_ALIAS: &str = "  alias {key} → {value}";
pub const TRUST_ENV: &str = "  env   {key}={value}";
pub const TRUST_DONE: &str = "Trusted {path}";

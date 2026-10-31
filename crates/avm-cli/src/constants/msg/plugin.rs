//! Plugin discovery, marketplace, registry and install messages.

pub const PLUGIN_FETCHING: &str = "Fetching '{name}' from {repo}...";
pub const PLUGIN_INSTALLED: &str = "✓ Installed {name} {version}";
pub const PLUGIN_INSTALLING_FROM: &str = "Installing plugin from {source}...";
pub const PLUGIN_INSTALLED_PLAIN: &str = "✓ Installed plugin";
pub const PLUGIN_REMOVED: &str = "Plugin '{name}' removed.";
pub const PLUGIN_SEARCH_EMPTY: &str = "No plugins match '{keyword}'.";
pub const PLUGIN_SEARCH_HINT: &str = "Install with: avm plugin install <name>";
pub const PLUGIN_NOT_INSTALLED: &str =
    "the '{name}' plugin is not installed\n\n  Install it with:\n    avm -p install {name}\n\n  Or find similar plugins:\n    avm -p search {name}";

// Plugin source discovery and install.
pub const PLUGIN_ASDF_MISSING_BIN: &str = "asdf plugin missing bin/{command}";
pub const PLUGIN_ASDF_INSTALL_PATH_FAILED: &str = "failed to create asdf install path";
pub const PLUGIN_DIR_CREATE_FAILED: &str = "create plugin directory";
pub const PLUGIN_SOURCE_PATH_INVALID: &str = "invalid plugin source path";
pub const PLUGIN_SOURCE_SYMLINK: &str = "plugin source must not be a symlink";
pub const PLUGIN_SOURCE_INVALID: &str = "invalid plugin source";
pub const PLUGIN_ALREADY_INSTALLED: &str =
    "plugin already installed; run `avm uninstall <name>` first";
pub const PLUGIN_COPY_FAILED: &str = "plugin copy failed";
pub const PLUGIN_INVALID_ASDF: &str = "invalid plugin: missing asdf bin/list-all + bin/install";
pub const PLUGIN_REMOVE_FAILED: &str = "remove plugin";
pub const PLUGIN_SOURCE_META_FAILED: &str = "failed to read plugin source metadata";
pub const PLUGIN_SOURCE_NOT_DIR: &str = "plugin source must be a directory";
pub const PLUGIN_SOURCE_EMPTY: &str = "plugin source is empty";
pub const PLUGIN_NAME_UNDERIVABLE: &str = "unable to derive plugin name";
pub const PLUGIN_NAME_INVALID: &str = "invalid plugin name derived from source";
pub const PLUGIN_DEST_CREATE_FAILED: &str = "failed to create plugin destination";
pub const PLUGIN_SOURCE_READ_FAILED: &str = "failed to read plugin source";
pub const PLUGIN_SOURCE_HAS_SYMLINKS: &str =
    "plugin source contains symlink entries; remove symlinks before installing";
pub const PLUGIN_COPY_FILE_FAILED: &str = "failed to copy {path}";
pub const PLUGIN_ENTRY_UNSUPPORTED: &str = "unsupported plugin source entry type: {path}";

// Plugin process and timeout.
pub const PLUGIN_CMD_START_FAILED: &str = "failed to start plugin command";
pub const PLUGIN_CMD_WAIT_FAILED: &str = "failed while waiting for plugin command";
pub const PLUGIN_CMD_TIMEOUT: &str = "plugin command timed out";
pub const PLUGIN_CMD_FAILED: &str = "plugin command failed: {stderr}";
pub const PLUGIN_CMD_FAILED_EXIT: &str = "plugin command failed with exit code";
pub const PLUGIN_CALL_FAILED: &str = "plugin '{name}' ({exe}) failed on `{args}`";
pub const PLUGIN_CALL_MALFORMED: &str =
    "plugin '{name}' ({exe}) returned malformed output for `{args}`: {output}";
pub const PLUGIN_RUN_LABEL: &str = "plugin '{name}' `{args}`";

// Marketplace.
pub const MARKETPLACE_QUERY_FAILED: &str = "failed to query {url}";
pub const MARKETPLACE_MALFORMED_RESPONSE: &str = "malformed response from {url}";
pub const MARKETPLACE_KEYWORD_INVALID: &str =
    "search keyword may only contain letters, digits, '-' and '_'";
pub const MARKETPLACE_UNSUPPORTED_OS: &str = "unsupported platform for marketplace install: {os}";
pub const MARKETPLACE_UNSUPPORTED_ARCH: &str =
    "unsupported architecture for marketplace install: {arch}";
pub const MARKETPLACE_RELEASE_QUERY_FAILED: &str = "failed to query release {which} of {repo}";
pub const MARKETPLACE_RELEASE_MALFORMED: &str = "malformed release info for {repo}";
pub const MARKETPLACE_ASSET_MISSING: &str = "release {tag} of {repo} has no {asset}";
pub const MARKETPLACE_NO_CHECKSUMS: &str = "release {tag} of {repo} publishes no checksums.txt, so {asset} can't be verified.\n  refusing to install. Set AVM_ALLOW_UNVERIFIED=1 to install it anyway.";
pub const MARKETPLACE_EXTRACT_DIR_FAILED: &str = "failed to create extraction dir";
pub const MARKETPLACE_MIRROR_COPY_FAILED: &str = "failed to copy {local}";
pub const MARKETPLACE_DOWNLOADING: &str = "downloading {asset}";
pub const MARKETPLACE_EXTRACTING: &str = "extracting {asset}";
pub const MARKETPLACE_CHECKSUMS_FAILED: &str = "failed to fetch checksums.txt";
pub const MARKETPLACE_REFUSING: &str =
    "{err}\n  refusing to install. Report at https://github.com/{repo}/issues";
pub const MARKETPLACE_UNVERIFIED: &str = "installing UNVERIFIED {asset} (AVM_ALLOW_UNVERIFIED=1)";
pub const MARKETPLACE_EXTRACT_FAILED: &str = "failed to extract {asset}";
pub const MARKETPLACE_NO_PLATFORM_RELEASE: &str =
    "plugin '{name}' has no release for {os}_{arch} (release {tag} of {repo})\n  available: {available}";
pub const MARKETPLACE_NONE_AVAILABLE: &str = "none";
pub const MARKETPLACE_BIN_MISSING: &str = "{asset} did not contain the expected file {bin}";
pub const MARKETPLACE_BIN_DIR_FAILED: &str = "failed to create plugin bin dir";
pub const MARKETPLACE_BIN_MOVE_FAILED: &str = "failed to move plugin binary into place";
pub const MARKETPLACE_META_FAILED: &str = "failed to write plugin meta.json";

// Registry.
pub const REGISTRY_LOAD_FAILED: &str =
    "failed to load the plugin registry from {url} (override with AVM_REGISTRY_URL)";
pub const REGISTRY_INVALID: &str = "invalid plugin registry at {url}";
pub const REGISTRY_MALFORMED_JSON: &str = "malformed JSON";
pub const REGISTRY_SCHEMA_UNSUPPORTED: &str =
    "unsupported registry schema_version {found} (this avm reads {supported}); update avm";
pub const REGISTRY_ENTRY_CONTEXT: &str = "plugin '{name}'";
pub const REGISTRY_DUPLICATE: &str = "plugin '{name}' is listed twice";
pub const REGISTRY_REPO_INVALID: &str =
    "invalid repo link '{link}' (expected https://github.com/<owner>/<repo>)";
pub const REGISTRY_PLUGIN_UNKNOWN: &str = "no plugin named '{name}' in the registry\n\n  Search for it:\n    avm -p search {name}\n\n  Or install a third-party plugin:\n    avm -p install <owner>/<repo>";

// Plugin list.
pub const PLUGIN_LIST_NO_PROJECT: &str = "Project: none here (no .avm.json). Showing global only.";
pub const PLUGIN_LIST_PROJECT: &str = "Project: {files}";
pub const PLUGIN_LIST_ALIASES: &str = "Plugin aliases:";
pub const PLUGIN_LIST_EMPTY: &str = "No plugins or tools yet.";
pub const PLUGIN_LIST_GET_STARTED: &str = "Get started:";
pub const PLUGIN_LIST_START_INSTALL: &str = "avm plugin install <plugin>";
pub const PLUGIN_LIST_START_USE: &str = "avm node use 20.11.1";

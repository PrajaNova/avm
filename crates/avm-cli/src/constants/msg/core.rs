//! Config, lockfile, trust and resolver messages.

pub const LOCK_READ_FAILED: &str = "read .avm.lock";
pub const LOCK_INVALID: &str = "invalid {path}";
pub const LOCK_UNSUPPORTED_VERSION: &str =
    "unsupported .avm.lock version {version}; update avm or regenerate with `avm plugin lock`";
/// `resolved` is the locked version, `requested` the spec it must satisfy.
pub const LOCK_UNSATISFIED: &str =
    "locked {tool} {resolved} does not satisfy '{requested}'; run `avm plugin lock`";
pub const LOCK_INVALID_ENTRY: &str = "invalid lock entry for {tool}; run `avm plugin lock`";
pub const LOCK_INVALID_HASH: &str = "invalid plugin hash for {tool}";
pub const LOCK_OUT_OF_DATE: &str =
    ".avm.lock is out of date with the selected tools; run `avm plugin lock`";
pub const LOCK_VERSION_REQUIRED: &str = "version required for {tool}";
pub const LOCK_NO_MATCH: &str = "no {tool} version matches '{spec}'";

pub const CONFIG_INVALID_ENV_KEY: &str = "invalid env key: {key}";
pub const CONFIG_INVALID_JSON: &str = "invalid config json";
pub const CONFIG_INVALID_LEGACY: &str = "invalid legacy flat config";
pub const CONFIG_ROOT_BOOL: &str = "root must be a boolean";
pub const CONFIG_TRUSTED_PATHS: &str = "trusted_paths must be a list of strings";
pub const CONFIG_INVALID_FORMAT: &str = "invalid config format";
pub const CONFIG_INVALID_SECTION: &str = "invalid structured config section: {section}";
pub const CONFIG_INVALID_FILE: &str = "invalid {path}; repair the file before continuing";
pub const CONFIG_SAVE_FAILED: &str = "failed to save config";
pub const CONFIG_INVALID_PROFILE: &str = "invalid environment profile: {profile}";

pub const TRUST_STORE_HOME_FAILED: &str = "failed to create avm home";
pub const TRUST_STORE_WRITE_FAILED: &str = "failed to write trusted.json";
pub const TRUST_STORE_NOT_FOUND: &str = "{path} not found";
pub const STORE_INVALID: &str = "invalid {what}";
pub const RESOLVER_LOCK_FALLBACK: &str = "avm: {err}; using .avm.json versions";
pub const TRACKED_FAILED: &str = "could not track project for prune: {err}";
pub const UNTRUSTED_NOTICE: &str = "avm: {files} not trusted, so its aliases and env are ignored. Review each file, then run 'avm trust <file>' to enable it.";

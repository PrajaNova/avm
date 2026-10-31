//! Shared: cancel, update, run, platform, config helpers messages.

pub const CANCELLED: &str = "Cancelled.";
pub const COMMAND_NOT_FOUND: &str = "command '{program}' not found in PATH";
pub const COMMAND_EXEC_FAILED: &str = "failed to execute command";
pub const CWD_READ_FAILED: &str = "failed to read current directory";
pub const CONFIG_MISSING_AT: &str = "no {file} found in {dir}";
pub const CONFIG_MISSING_LOCAL: &str = "no {file} found in current directory. Run `avm init` first";
pub const STATE_SCRIPTS_IGNORED: &str = "avm: ignoring package.json scripts: {err}";
pub const STATE_BASELINE_INVALID: &str = "invalid AVM environment baseline";
pub const PACKAGE_JSON_READ_FAILED: &str = "failed to read package.json";
pub const PACKAGE_JSON_PARSE_FAILED: &str = "failed to parse package.json";
pub const CONFIRM_NO_TERMINAL: &str =
    "{question}\n\n  No terminal to ask on. Re-run with -y to confirm.";
pub const BINARY_MOVE_ASIDE_FAILED: &str = "failed to move the running avm-bin aside";
pub const BINARY_SWAP_FAILED: &str = "failed to swap in the new binary";
pub const PLUGIN_SOURCE_ROOT_OWNED: &str = "plugin sources owned by root are not allowed";
pub const PLUGIN_SOURCE_WORLD_WRITABLE: &str = "plugin source must not be world-writable";
pub const ENV_FILE_READ_FAILED: &str = "failed to read env file {path}";
pub const ENV_FILE_BAD_ASSIGNMENT: &str = "invalid env assignment at {path}:{line}";
pub const ENV_FILE_BAD_KEY: &str = "invalid env key at {path}:{line}";
pub const DOTENV_LOAD_FAILED: &str = "avm: failed to load .env: {err}";
pub const COMMAND_FAILED: &str = "avm: {err}";
pub const UPDATE_PACKAGE_MANAGED: &str =
    "avm was installed with {method}, so update it the same way:\n  {cmd}";
pub const UPDATE_UP_TO_DATE: &str = "avm {current} is up to date";
pub const UPDATE_DONE: &str = "Updated avm {current} → {target}";
pub const UPDATE_AVAILABLE: &str =
    "avm {latest} is available (you have {current}). Update with: {how}";
pub const UPDATE_NO_PARENT_DIR: &str = "{exe} has no parent directory";
pub const UPDATE_FAILED: &str = "couldn't update {exe}";
pub const UPDATE_LOCATE_EXE_FAILED: &str = "failed to locate avm-bin";

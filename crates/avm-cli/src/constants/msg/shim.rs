//! Shims messages.

pub const SHIM_INSTALLED: &str = "avm shims installed.";
pub const SHIM_ALREADY_ACTIVE: &str = "avm shims already active in your shell startup files.";
pub const SHIM_ACTIVATED: &str = "Added avm shims to {path}";
pub const SHIM_OPEN_NEW_SHELL: &str = "Open a new shell (or `source` the file) to apply.";
pub const SHIM_REMOVED: &str = "Removed shim for {tool}";
pub const SHIM_NOT_INSTALLED: &str =
    "warning: managed {tool} {version} is not installed; falling back to system {command}";
pub const SHIM_RUN_FAILED: &str = "failed to run shim target";
pub const SHIM_RESHIM_FAILED: &str =
    "avm: couldn't refresh shims after the install ({err}); run `avm shims install`";
pub const SHIM_CREATE_DIR_FAILED: &str = "create shims dir";
pub const SHIM_PROFILE_UPDATE_FAILED: &str = "failed to update {path}";
pub const SHIM_REMOVE_FAILED: &str = "remove shim";
pub const SHIM_WRITE_FAILED: &str = "write shim for {tool}";
pub const SHIM_INSTALL_FAILED: &str = "install shim for {tool}";
pub const SHIM_METADATA_FAILED: &str = "shim metadata";
pub const SHIM_CHMOD_FAILED: &str = "chmod shim";
pub const SHIM_LOCATE_BIN_FAILED: &str = "locate avm-bin";
pub const SHIM_BIN_NOT_ON_PATH: &str = "avm-bin not found on PATH";

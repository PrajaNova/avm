//! Tool commands messages.

pub const TOOL_COMMAND_REQUIRED: &str = "command required after --";
pub const TOOL_EXPECTED_SPEC: &str = "expected tool@version, got '{value}'";
pub const TOOL_INVALID_SPEC: &str = "invalid tool specification '{value}'";
pub const TOOL_INVALID_NAME: &str = "invalid tool name '{tool}'";
pub const TOOL_DUPLICATE: &str = "tool '{tool}' specified more than once";
pub const TOOL_NOT_INSTALLED: &str =
    "{tool}@{spec} is not installed; omit --no-install to install it";
pub const TOOL_INSTALLING: &str = "Installing {tool} {version}...";
pub const TOOL_NO_EXECUTABLE: &str = "{tool} {version} has no installed executable";
pub const TOOL_NO_PARENT: &str = "{tool} executable has no parent directory";
pub const TOOL_PATH_FAILED: &str = "failed to build temporary tool PATH";

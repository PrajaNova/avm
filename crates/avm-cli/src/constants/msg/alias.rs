//! Alias messages.

pub const ALIAS_OVERRIDE_PROMPT: &str =
    "alias '{key}' already exists\n\n  current: {old}\n  new:     {value}\n\nOverride it?";
pub const ALIAS_ADDED: &str = "Added {scope} alias '{key}'";
pub const ALIAS_REMOVED: &str = "Removed {scope} alias '{key}'";
pub const ALIAS_NOT_FOUND: &str = "alias '{key}' not found in the {scope} config";
pub const ALIAS_NONE: &str = "No aliases configured.";
pub const ALIAS_INVALID_NAME: &str = "invalid alias name '{name}'";
pub const ALIAS_RESERVED_NAME: &str =
    "'{name}' is reserved for a built-in command, so it cannot be an alias";
pub const ALIAS_INVALID_INVOCATION: &str = "invalid alias invocation";

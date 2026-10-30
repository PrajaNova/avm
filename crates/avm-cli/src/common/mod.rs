//! Code shared by every feature: config helpers, env merging, shell scripts, state loading, the terminal UI and the `Manager` trait.
pub mod config_utils;
pub mod env_utils;
pub mod manager;
pub(crate) mod shell;
pub mod state;
pub mod ui;

#[macro_use]
mod say;

pub mod cli;
pub mod constants;
pub mod core;
pub mod platform;
pub mod plugins;
pub mod update;

// Domain modules stay addressable as `crate::config`, `crate::resolver`, ...
pub use self::core::{config, lockfile, resolver, shims, trust, version_files};

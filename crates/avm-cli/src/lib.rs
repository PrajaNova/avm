#[macro_use]
mod say;

mod alias;
mod app;
mod args;
mod bootstrap;
mod common;
pub mod constants;
pub mod core;
mod env;
mod global;
pub mod platform;
pub mod plugins;
mod project;
mod run;
pub mod shims;
mod tool;
pub mod update;

pub use common::shell::sh_quote;
pub use run::main;

// Domain modules stay addressable as `crate::config`, `crate::resolver`, ...
pub use self::core::{config, lockfile, resolver, trust, version_files};

//! User-facing text, one file per feature, all re-exported so callers write `msg::NAME`.
//! Placeholders are `{key}`, filled with `fill`.
mod alias;
mod app;
mod common;
mod core;
mod env;
mod global;
mod plugin;
mod project;
mod shim;
mod tool;

pub use self::core::*;
pub use alias::*;
pub use app::*;
pub use common::*;
pub use env::*;
pub use global::*;
pub use plugin::*;
pub use project::*;
pub use shim::*;
pub use tool::*;

/// Fills `{key}` placeholders: `fill(msg::PLUGIN_REMOVED, &[("name", "node")])`.
pub fn fill(template: &str, vars: &[(&str, &str)]) -> String {
    vars.iter().fold(template.to_string(), |out, (key, value)| {
        out.replace(&format!("{{{key}}}"), value)
    })
}

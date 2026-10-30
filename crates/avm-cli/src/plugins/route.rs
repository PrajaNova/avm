//! Decides who handles `avm <key> ...`.
use super::PluginRegistry;
use crate::resolver::{ResolvedAliasLookup, ResolvedConfig};

/// Who runs `avm <key> ...`: an alias first, then an installed plugin's own command.
pub enum Route {
    Alias(ResolvedAliasLookup),
    Plugin,
    Missing,
}

/// Route `key` to an alias if one resolves, else to an installed plugin, else `Missing`.
pub fn route(cfg: &ResolvedConfig, key: &str) -> Route {
    match cfg.resolve_alias(key) {
        Some(alias) => Route::Alias(alias),
        None if is_installed_plugin(key) => Route::Plugin,
        None => Route::Missing,
    }
}

/// Whether a plugin for `name` is installed locally (no network).
pub fn is_installed_plugin(name: &str) -> bool {
    PluginRegistry::new().is_ok_and(|plugins| plugins.provider(name).is_some())
}

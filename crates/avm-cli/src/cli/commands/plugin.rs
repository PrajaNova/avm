use crate::cli::args::PluginCommands;
use crate::cli::commands::cmd_install;
use crate::cli::run::task::{Cmd, Runnable};
use crate::plugins as runtime;
use crate::plugins::PluginManager;
use anyhow::Result;

use crate::constants::msg::*;

pub struct PluginInstall {
    sources: Vec<String>,
    frozen: bool,
}
pub struct PluginUninstall {
    names: Vec<String>,
}
pub struct PluginSearch {
    keyword: String,
}

impl Runnable for PluginInstall {
    fn run(self: Box<Self>) -> Result<()> {
        // No names: install the project's tools instead of plugins.
        if self.sources.is_empty() {
            cmd_install(self.frozen)
        } else {
            cmd_plugin_install(self.sources)
        }
    }
}

impl Runnable for PluginUninstall {
    fn run(self: Box<Self>) -> Result<()> {
        cmd_plugin_uninstall(self.names)
    }
}

impl Runnable for PluginSearch {
    fn run(self: Box<Self>) -> Result<()> {
        cmd_plugin_search(&self.keyword)
    }
}

impl PluginCommands {
    /// Converts the parsed subcommand into a runnable command.
    pub fn into_command(self) -> Cmd {
        match self {
            Self::Install { sources, frozen } => Box::new(PluginInstall { sources, frozen }),
            Self::Uninstall { names } => Box::new(PluginUninstall { names }),
            Self::Search { keyword } => Box::new(PluginSearch { keyword }),
        }
    }
}

/// Installs each plugin spec from the marketplace (or a direct source) into the plugin directory.
/// Stops with an error on the first spec that fails.
fn cmd_plugin_install(specs: Vec<String>) -> Result<()> {
    let plugin_manager = PluginManager::new()?;
    for spec in specs {
        match runtime::resolve_plugin_source(&spec)? {
            Some(source) => {
                println!(
                    "{}",
                    fill(
                        PLUGIN_FETCHING,
                        &[("name", &source.name), ("repo", &source.repo)]
                    )
                );
                let version =
                    runtime::install_from_marketplace(&source, &plugin_manager.plugin_dir())?;
                println!(
                    "{}",
                    fill(
                        PLUGIN_INSTALLED,
                        &[("name", &source.name), ("version", &version)]
                    )
                );
            }
            // Not a release source: an asdf-style git URL or a local path.
            None => {
                println!("{}", fill(PLUGIN_INSTALLING_FROM, &[("source", &spec)]));
                plugin_manager.install_plugin(&spec)?;
                println!("{PLUGIN_INSTALLED_PLAIN}");
            }
        }
    }
    Ok(())
}

/// Removes each named plugin, erroring on the first that cannot be removed.
fn cmd_plugin_uninstall(names: Vec<String>) -> Result<()> {
    let plugin_manager = PluginManager::new()?;
    for name in names {
        plugin_manager.remove_plugin(&name)?;
        println!("{}", fill(PLUGIN_REMOVED, &[("name", &name)]));
    }
    Ok(())
}

/// Prints marketplace plugins matching `keyword`, or a notice when none match.
fn cmd_plugin_search(keyword: &str) -> Result<()> {
    let rows = runtime::search_plugins(keyword)?;
    if rows.is_empty() {
        println!("{}", fill(PLUGIN_SEARCH_EMPTY, &[("keyword", keyword)]));
        return Ok(());
    }
    for (name, description) in rows {
        println!("  {name} - {description}");
    }
    println!();
    println!("{PLUGIN_SEARCH_HINT}");
    Ok(())
}

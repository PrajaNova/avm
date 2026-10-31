use crate::common::ui;
use crate::constants::msg::*;
use crate::global::{cmd_outdated, cmd_upgrade};
use crate::plugins as runtime;
use crate::plugins::PluginRegistry;
use crate::plugins::{cmd_plugin_list, PluginCommands};
use crate::project::{cmd_install, cmd_lock, cmd_prune};
use crate::run::task::{Cmd, Runnable, Task};
use anyhow::Result;

pub struct PluginInstall {
    sources: Vec<String>,
    frozen: bool,
}
pub struct PluginUninstall {
    names: Vec<String>,
}
pub struct PluginList;
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

impl Runnable for PluginList {
    fn run(self: Box<Self>) -> Result<()> {
        cmd_plugin_list()
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
            Self::List => Box::new(PluginList),
            Self::Search { keyword } => Box::new(PluginSearch { keyword }),
            Self::Outdated { json } => Task::cmd(move || cmd_outdated(json)),
            Self::Upgrade {
                tools,
                bump,
                dry_run,
                yes,
            } => Task::cmd(move || cmd_upgrade(tools, bump, dry_run, yes)),
            Self::Lock { platform } => Task::cmd(move || cmd_lock(platform)),
            Self::Prune {
                older_than,
                include_unrecorded,
                dry_run,
                yes,
            } => Task::cmd(move || cmd_prune(older_than, include_unrecorded, dry_run, yes)),
        }
    }
}

/// Installs each plugin spec from the marketplace (or a direct source) into the plugin directory.
/// Stops with an error on the first spec that fails.
fn cmd_plugin_install(specs: Vec<String>) -> Result<()> {
    let plugin_manager = PluginRegistry::new()?;
    for spec in specs {
        match runtime::resolve_plugin_source(&spec)? {
            Some(source) => {
                ui::info(fill(
                    PLUGIN_FETCHING,
                    &[("name", &source.name), ("repo", &source.repo)],
                ));
                let version =
                    runtime::install_from_marketplace(&source, &plugin_manager.plugin_dir())?;
                ui::info(fill(
                    PLUGIN_INSTALLED,
                    &[("name", &source.name), ("version", &version)],
                ));
            }
            // Not a release source: an asdf-style git URL or a local path.
            None => {
                ui::info(fill(PLUGIN_INSTALLING_FROM, &[("source", &spec)]));
                plugin_manager.install_plugin(&spec)?;
                ui::info(PLUGIN_INSTALLED_PLAIN);
            }
        }
    }
    Ok(())
}

/// Removes each named plugin, erroring on the first that cannot be removed.
fn cmd_plugin_uninstall(names: Vec<String>) -> Result<()> {
    let plugin_manager = PluginRegistry::new()?;
    for name in names {
        plugin_manager.remove_plugin(&name)?;
        ui::info(fill(PLUGIN_REMOVED, &[("name", &name)]));
    }
    Ok(())
}

/// Prints marketplace plugins matching `keyword`, or a notice when none match.
fn cmd_plugin_search(keyword: &str) -> Result<()> {
    let rows = runtime::search_plugins(keyword)?;
    if rows.is_empty() {
        ui::info(fill(PLUGIN_SEARCH_EMPTY, &[("keyword", keyword)]));
        return Ok(());
    }
    for (name, description) in rows {
        ui::info(format!("  {name} - {description}"));
    }
    ui::blank();
    ui::info(PLUGIN_SEARCH_HINT);
    Ok(())
}

//! The code behind each command; `dispatch` decides which one runs.

mod alias;
mod edit;
mod exec;
mod init;
mod install;
mod list;
mod outdated;
pub(super) mod plugin;
mod provider;
mod provider_menu;
mod prune;
mod shim;
mod trust;

pub(super) use alias::*;
pub(super) use edit::*;
pub(super) use exec::*;
pub(super) use init::*;
pub(super) use install::*;
pub(super) use list::*;
pub(super) use outdated::*;
pub(super) use provider::*;
pub(super) use prune::*;
pub(super) use shim::*;
pub(super) use trust::*;

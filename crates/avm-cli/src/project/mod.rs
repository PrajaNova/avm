//! The project feature: its manager (clap commands and dispatch) and handlers.
mod init_handler;
mod install_handler;
mod project_manager;
mod prune_handler;
mod trust_handler;

#[allow(unused_imports)]
pub(crate) use init_handler::*;
#[allow(unused_imports)]
pub(crate) use install_handler::*;
pub use project_manager::*;
#[allow(unused_imports)]
pub(crate) use prune_handler::*;
#[allow(unused_imports)]
pub(crate) use trust_handler::*;

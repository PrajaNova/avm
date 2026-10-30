//! The global feature: its manager (clap commands and dispatch) and handlers.
mod global_handler;
mod global_manager;
mod outdated_handler;
mod provider_handler;
mod provider_menu_handler;

#[allow(unused_imports)]
pub(crate) use global_handler::*;
pub use global_manager::*;
#[allow(unused_imports)]
pub(crate) use outdated_handler::*;
#[allow(unused_imports)]
pub(crate) use provider_handler::*;
#[allow(unused_imports)]
pub(crate) use provider_menu_handler::*;

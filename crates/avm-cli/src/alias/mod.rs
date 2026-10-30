//! The alias feature: its manager (clap commands and dispatch) and handlers.
mod alias_handler;
mod alias_manager;
pub(crate) mod alias_utils;

#[allow(unused_imports)]
pub(crate) use alias_handler::*;
pub use alias_manager::*;

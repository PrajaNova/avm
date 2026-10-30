//! End-to-end tests: drive the real `avm-bin` against throwaway sandboxes.
//!
//! One module per source feature (`crates/avm-cli/src/<feature>/`), plus:
//! - `common/`   harness: `Sandbox`, running `avm-bin`, fluent assertions.
//! - `fixtures/` test data: fake plugins, prebuilt projects, a mock download server.
//!
//! See `tests/README.md` for how to add a test.

mod common;
mod fixtures;

mod alias;
mod app;
mod bench;
mod core;
mod env;
mod global;
mod plugins;
mod project;
mod shims;
#[cfg(unix)]
mod tool;

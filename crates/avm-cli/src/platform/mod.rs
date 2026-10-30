//! OS-specific code: exactly one of `unix.rs` / `windows.rs` is compiled, and
//! callers use `platform::item` without any `cfg`.
#[cfg_attr(windows, path = "windows.rs")]
#[cfg_attr(not(windows), path = "unix.rs")]
mod imp;

pub use imp::*;

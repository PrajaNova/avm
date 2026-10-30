//! Test data for the end-to-end suites: fake plugins, prebuilt projects and a mock download
//! server. Static plugin scripts live in `scripts/`; the helpers here install them.

pub mod native;
#[cfg(unix)]
pub mod plugins;
#[cfg(unix)]
pub mod projects;

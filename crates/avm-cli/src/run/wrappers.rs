use crate::config;
use crate::run::task::{Cmd, Runnable};
use anyhow::Result;

// Wrappers add behavior around any command and are themselves commands, so
// they stack in any order. Each decides from its own condition whether to act.

/// Prints the update notice after a successful command that asks for one.
pub struct WithUpdateNotice {
    inner: Cmd,
}

impl WithUpdateNotice {
    /// Wraps `inner` so the update notice is shown after it succeeds.
    pub fn new(inner: Cmd) -> Self {
        Self { inner }
    }
}

impl Runnable for WithUpdateNotice {
    fn run(self: Box<Self>) -> Result<()> {
        let notify = self.inner.wants_update_notice();
        self.inner.run()?;
        if notify {
            crate::update::maybe_notify();
        }
        Ok(())
    }

    fn wants_update_notice(&self) -> bool {
        false
    }
}

/// Applies `--env <profiles>` (validated) before the command runs; a no-op without the flag.
pub struct WithProfiles {
    inner: Cmd,
    profiles: Option<String>,
}

impl WithProfiles {
    /// Wraps `inner` so the optional `profiles` list is validated and exported as `AVM_ENV` before it runs.
    pub fn new(inner: Cmd, profiles: Option<String>) -> Self {
        Self { inner, profiles }
    }
}

impl Runnable for WithProfiles {
    fn run(self: Box<Self>) -> Result<()> {
        if let Some(profiles) = &self.profiles {
            config::file_names(profiles)?;
            std::env::set_var("AVM_ENV", profiles);
        }
        self.inner.run()
    }

    fn wants_update_notice(&self) -> bool {
        self.inner.wants_update_notice()
    }
}

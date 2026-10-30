//! RAII guard: raw mode and the alternate screen are undone on drop, even on early return or panic.
use anyhow::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io;

pub(super) struct RawTerminal {
    active: bool,
}

impl RawTerminal {
    /// Switches the terminal to raw mode on the alternate screen with the cursor hidden.
    pub(super) fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let terminal = Self { active: true };
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(terminal)
    }

    /// Undoes `enter`; safe to call twice. Reports the first failure after attempting both steps.
    pub(super) fn restore(&mut self) -> Result<()> {
        if self.active {
            let raw_mode = disable_raw_mode();
            let cursor = execute!(io::stdout(), Show, LeaveAlternateScreen);
            raw_mode?;
            cursor?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

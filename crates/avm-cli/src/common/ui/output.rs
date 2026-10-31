//! Console output every command shares. Message text lives in `constants::msg`; how it
//! looks (symbols, prefixes, streams, indentation) is decided only here, so a restyle
//! or a TUI backend later means editing this file, not the call sites.
use super::can_select;
use crate::constants::msg::{self, fill};
use anyhow::{anyhow, Result};
use std::fmt::Display;
use std::io::{self, Write};

/// `✓ <message>` on stdout.
pub fn success(message: impl Display) {
    println!("✓ {message}");
}

/// A plain status line on stdout.
pub fn info(message: impl Display) {
    println!("{message}");
}

/// An empty line on stdout, to separate blocks.
pub fn blank() {
    println!();
}

/// `warning: <message>` on stderr.
pub fn warn(message: impl Display) {
    eprintln!("warning: {message}");
}

/// A plain notice on stderr that is neither a warning nor an error.
pub fn notice(message: impl Display) {
    eprintln!("{message}");
}

/// `avm: <message>` on stderr; multiline messages keep their own layout.
pub fn error(message: impl Display) {
    eprintln!("avm: {message}");
}

/// Prints the shared "Cancelled." line.
pub fn cancelled() {
    info(msg::CANCELLED);
}

/// A heading followed by its lines indented two spaces, e.g. a "Commands:" block.
pub fn section(heading: impl Display, lines: &[String]) {
    info(heading);
    for line in lines {
        info(format_args!("  {line}"));
    }
}

/// Asks `question [y/N]` and returns whether the answer was yes. `yes` answers without asking.
/// Errors when there is no terminal to ask on, telling the user to pass `-y`.
pub fn confirm(question: &str, yes: bool) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    if !can_select() {
        return Err(anyhow!(fill(
            msg::CONFIRM_NO_TERMINAL,
            &[("question", question)]
        )));
    }
    print!("{question} [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

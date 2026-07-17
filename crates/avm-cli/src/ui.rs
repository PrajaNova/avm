use anyhow::{anyhow, Context, Result};
use std::io::{self, IsTerminal, Read, Write};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub label: String,
}

pub fn can_select() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub fn select(
    title: &str,
    help: &str,
    items: &[SelectItem],
    page_size: usize,
) -> Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }

    let mut terminal = RawTerminal::enter()?;
    let mut selected = 0usize;
    let mut offset = 0usize;
    let page_size = page_size.max(1);

    loop {
        render(title, help, items, selected, offset, page_size)?;

        let mut byte = [0u8; 1];
        io::stdin().read_exact(&mut byte)?;
        match byte[0] {
            b'\n' | b'\r' => {
                terminal.restore()?;
                return Ok(Some(selected));
            }
            b'q' | 3 => {
                terminal.restore()?;
                return Ok(None);
            }
            27 => {
                let mut seq = [0u8; 2];
                if io::stdin().read_exact(&mut seq).is_ok() && seq[0] == b'[' {
                    match seq[1] {
                        b'A' => selected = selected.saturating_sub(1),
                        b'B' => {
                            if selected + 1 < items.len() {
                                selected += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        if selected < offset {
            offset = selected;
        } else if selected >= offset + page_size {
            offset = selected + 1 - page_size;
        }
    }
}

fn render(
    title: &str,
    help: &str,
    items: &[SelectItem],
    selected: usize,
    offset: usize,
    page_size: usize,
) -> Result<()> {
    let mut stdout = io::stdout();
    write!(stdout, "\x1b[?25l\x1b[2J\x1b[H")?;
    write!(stdout, "{title}\r\n")?;
    write!(stdout, "{help}\r\n\r\n")?;

    for (index, item) in items.iter().enumerate().skip(offset).take(page_size) {
        if index == selected {
            write!(stdout, "> {}\r\n", item.label)?;
        } else {
            write!(stdout, "  {}\r\n", item.label)?;
        }
    }

    write!(
        stdout,
        "\r\nShowing {}-{} of {}\r\n",
        offset + 1,
        usize::min(offset + page_size, items.len()),
        items.len()
    )?;
    stdout.flush()?;
    Ok(())
}

struct RawTerminal {
    active: bool,
}

impl RawTerminal {
    fn enter() -> Result<Self> {
        write!(io::stdout(), "\x1b[?25l")?;
        io::stdout().flush()?;
        let status = Command::new("stty")
            .arg("raw")
            .arg("-echo")
            .status()
            .context("failed to enter raw terminal mode")?;
        if !status.success() {
            return Err(anyhow!("failed to enter raw terminal mode"));
        }
        Ok(Self { active: true })
    }

    fn restore(&mut self) -> Result<()> {
        if self.active {
            let _ = Command::new("stty").arg("sane").status();
            let _ = write!(io::stdout(), "\x1b[?25h\r\n");
            let _ = io::stdout().flush();
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

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

/// Interactive picker: type to filter (substring, case-insensitive),
/// Up/Down to move within the filtered results, Enter to select. Search
/// eats every printable key (including "q"), so cancel is Ctrl+C — the one
/// key that can't collide with something a user might want to type into
/// the search box.
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
    let mut query = String::new();
    let mut filtered = filter_items(items, &query);
    let mut selected = 0usize;
    let mut offset = 0usize;
    let page_size = page_size.max(1);

    loop {
        render(title, help, items, &filtered, selected, offset, page_size, &query)?;

        let mut byte = [0u8; 1];
        io::stdin().read_exact(&mut byte)?;
        match byte[0] {
            b'\n' | b'\r' => {
                terminal.restore()?;
                return Ok(filtered.get(selected).copied());
            }
            3 => {
                // Ctrl+C
                terminal.restore()?;
                return Ok(None);
            }
            21 => {
                // Ctrl+U: clear the whole search query
                if !query.is_empty() {
                    query.clear();
                    filtered = filter_items(items, &query);
                    selected = 0;
                    offset = 0;
                }
            }
            127 | 8 => {
                // Backspace
                if query.pop().is_some() {
                    filtered = filter_items(items, &query);
                    selected = 0;
                    offset = 0;
                }
            }
            27 => {
                let mut seq = [0u8; 2];
                if io::stdin().read_exact(&mut seq).is_ok() && seq[0] == b'[' {
                    match seq[1] {
                        b'A' => selected = selected.saturating_sub(1),
                        b'B' => {
                            if selected + 1 < filtered.len() {
                                selected += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
            b if (0x20..0x7f).contains(&b) => {
                query.push(b as char);
                filtered = filter_items(items, &query);
                selected = 0;
                offset = 0;
            }
            _ => {}
        }

        if !filtered.is_empty() && selected >= filtered.len() {
            selected = filtered.len() - 1;
        }
        if selected < offset {
            offset = selected;
        } else if selected >= offset + page_size {
            offset = selected + 1 - page_size;
        }
    }
}

fn filter_items(items: &[SelectItem], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..items.len()).collect();
    }
    let query = query.to_ascii_lowercase();
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.label.to_ascii_lowercase().contains(&query))
        .map(|(index, _)| index)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn render(
    title: &str,
    help: &str,
    items: &[SelectItem],
    filtered: &[usize],
    selected: usize,
    offset: usize,
    page_size: usize,
    query: &str,
) -> Result<()> {
    let mut stdout = io::stdout();
    write!(stdout, "\x1b[?25l\x1b[2J\x1b[H")?;
    write!(stdout, "{title}\r\n")?;
    write!(stdout, "{help}\r\n")?;
    write!(stdout, "Search: {query}\u{2588}\r\n\r\n")?;

    if filtered.is_empty() {
        write!(stdout, "  (no matches)\r\n")?;
    } else {
        for (row, &item_index) in filtered.iter().enumerate().skip(offset).take(page_size) {
            let marker = if row == selected { ">" } else { " " };
            write!(stdout, "{marker} {}\r\n", items[item_index].label)?;
        }
    }

    if query.is_empty() {
        write!(
            stdout,
            "\r\nShowing {}-{} of {}\r\n",
            if filtered.is_empty() { 0 } else { offset + 1 },
            usize::min(offset + page_size, filtered.len()),
            items.len()
        )?;
    } else {
        write!(
            stdout,
            "\r\nShowing {}-{} of {} ({} total)\r\n",
            if filtered.is_empty() { 0 } else { offset + 1 },
            usize::min(offset + page_size, filtered.len()),
            filtered.len(),
            items.len()
        )?;
    }
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

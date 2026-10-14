use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io::{self, IsTerminal, Write};

const HELP: &str = "Type to search, Up/Down/PgUp/PgDn to move, Enter to select, Esc to cancel.";
const PAGE_SIZE: usize = 10;

pub fn can_select() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Interactive picker: type to filter (substring, case-insensitive),
/// Up/Down/PageUp/PageDown/Home/End to move within the filtered results
/// (Up/Down wrap), Enter to select. Search eats every printable key (including
/// "q"), so cancel is Esc or Ctrl+C — neither can be typed into the search box.
/// Drawn on the alternate screen, so the terminal comes back exactly as it was.
pub fn select(title: &str, items: &[String]) -> Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }

    let mut terminal = RawTerminal::enter()?;
    let mut query = String::new();
    let mut filtered = filter_items(items, &query);
    let mut selected = 0usize;
    let mut offset = 0usize;
    let page_size = PAGE_SIZE;

    loop {
        render(title, items, &filtered, selected, offset, &query)?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        match key.code {
            KeyCode::Enter => {
                terminal.restore()?;
                return Ok(filtered.get(selected).copied());
            }
            KeyCode::Esc => {
                terminal.restore()?;
                return Ok(None);
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                terminal.restore()?;
                return Ok(None);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !query.is_empty() {
                    query.clear();
                    filtered = filter_items(items, &query);
                    selected = 0;
                    offset = 0;
                }
            }
            KeyCode::Backspace => {
                if query.pop().is_some() {
                    filtered = filter_items(items, &query);
                    selected = 0;
                    offset = 0;
                }
            }
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End => {
                selected = move_selection(key.code, selected, filtered.len(), page_size);
            }
            KeyCode::Char(c) => {
                query.push(c);
                filtered = filter_items(items, &query);
                selected = 0;
                offset = 0;
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

/// New selected row after a navigation key over `len` rows. Up/Down wrap; the rest clamp.
fn move_selection(key: KeyCode, selected: usize, len: usize, page: usize) -> usize {
    if len == 0 {
        return 0;
    }
    match key {
        KeyCode::Up => (selected + len - 1) % len,
        KeyCode::Down => (selected + 1) % len,
        KeyCode::PageUp => selected.saturating_sub(page),
        KeyCode::PageDown => (selected + page).min(len - 1),
        KeyCode::Home => 0,
        KeyCode::End => len - 1,
        _ => selected,
    }
}

fn filter_items(items: &[String], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..items.len()).collect();
    }
    let query = query.to_ascii_lowercase();
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.to_ascii_lowercase().contains(&query))
        .map(|(index, _)| index)
        .collect()
}

fn render(
    title: &str,
    items: &[String],
    filtered: &[usize],
    selected: usize,
    offset: usize,
    query: &str,
) -> Result<()> {
    let page_size = PAGE_SIZE;
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    write!(stdout, "{title}\r\n")?;
    write!(stdout, "{HELP}\r\n")?;
    write!(stdout, "Search: {query}\u{2588}\r\n\r\n")?;

    if filtered.is_empty() {
        write!(stdout, "  (no matches)\r\n")?;
    } else {
        for (row, &item_index) in filtered.iter().enumerate().skip(offset).take(page_size) {
            let marker = if row == selected { ">" } else { " " };
            write!(stdout, "{marker} {}\r\n", items[item_index])?;
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
        enable_raw_mode()?;
        let terminal = Self { active: true };
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(terminal)
    }

    fn restore(&mut self) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_wraps_and_clamps() {
        use KeyCode::*;
        assert_eq!(move_selection(Up, 0, 3, 10), 2);
        assert_eq!(move_selection(Down, 2, 3, 10), 0);
        assert_eq!(move_selection(PageDown, 1, 25, 10), 11);
        assert_eq!(move_selection(PageDown, 20, 25, 10), 24);
        assert_eq!(move_selection(PageUp, 3, 25, 10), 0);
        assert_eq!(
            (
                move_selection(Home, 7, 25, 10),
                move_selection(End, 7, 25, 10)
            ),
            (0, 24)
        );
        assert_eq!(move_selection(End, 0, 0, 10), 0, "no rows: no panic");
    }

    #[test]
    #[ignore = "requires a console; follow each prompt with --ignored --nocapture"]
    fn console_picker_selects_filters_cancels_and_restores_terminal() {
        assert!(can_select(), "run with --ignored --nocapture in a terminal");
        let items = vec!["node 20".into(), "node 22".into(), "java 17".into()];
        for (title, expected) in [
            ("Navigation: Down then Enter", Some(1)),
            ("Filter: type 22x, Backspace, Enter", Some(1)),
            ("Clear: type 17, Ctrl+U, type 22, Enter", Some(1)),
            ("Cancel: Ctrl+C", None),
            ("Cancel: Esc", None),
        ] {
            assert_eq!(select(title, &items).unwrap(), expected);
            assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
        }
        {
            let _terminal = RawTerminal::enter().unwrap();
            assert!(crossterm::terminal::is_raw_mode_enabled().unwrap());
        }
        assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
        assert_eq!(select("Empty", &[]).unwrap(), None);
    }
}

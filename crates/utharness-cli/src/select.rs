//! Interactive single-choice picker for the chat CLI.
//!
//! Full-terminal path: arrow keys (or j/k), type-to-filter, Enter to confirm,
//! Esc to cancel. Piped stdin, dumb terminals, or plain-text modes fall back
//! to a numbered list read from stdin, which is also how integration tests
//! drive the picker deterministically.

use std::io::{self, BufRead, IsTerminal, Write};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::QueueableCommand;

pub struct PickItem {
    pub title: String,
    pub hint: String,
}

impl PickItem {
    pub fn new(title: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            hint: hint.into(),
        }
    }
}

/// Case-insensitive substring filter over title + hint. Pure for unit tests.
pub fn filter_indices(query: &str, items: &[PickItem]) -> Vec<usize> {
    let needle = query.to_lowercase();
    if needle.trim().is_empty() {
        return (0..items.len()).collect();
    }
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            item.title.to_lowercase().contains(&needle)
                || item.hint.to_lowercase().contains(&needle)
        })
        .map(|(index, _)| index)
        .collect()
}

fn plain_mode() -> bool {
    crate::icons::ascii_mode() || !io::stdin().is_terminal() || !io::stdout().is_terminal()
}

/// Picker sharing the caller's stdin handle. Required when the caller (the
/// chat REPL) already owns a lock: re-locking stdin on the same thread
/// deadlocks, and separate per-line locks would drop buffered piped input.
pub fn pick_with_reader(
    prompt: &str,
    items: &[PickItem],
    reader: &mut impl BufRead,
) -> Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }
    if plain_mode() {
        return pick_numbered(prompt, items, reader);
    }
    pick_interactive(prompt, items)
}

fn pick_numbered(
    prompt: &str,
    items: &[PickItem],
    reader: &mut impl BufRead,
) -> Result<Option<usize>> {
    println!("{prompt}");
    for (position, item) in items.iter().enumerate() {
        if item.hint.is_empty() {
            println!("  {}. {}", position + 1, item.title);
        } else {
            println!("  {}. {} — {}", position + 1, item.title, item.hint);
        }
    }
    print!("Select [1-{}] (Enter cancels): ", items.len());
    io::stdout().flush()?;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    match trimmed.parse::<usize>() {
        Ok(number) if number >= 1 && number <= items.len() => Ok(Some(number - 1)),
        _ => {
            println!("No selection made.");
            Ok(None)
        }
    }
}

fn pick_interactive(prompt: &str, items: &[PickItem]) -> Result<Option<usize>> {
    crossterm::terminal::enable_raw_mode()?;
    let outcome = run_picker(prompt, items);
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = io::stdout().queue(crossterm::cursor::Show);
    println!();
    outcome
}

fn run_picker(prompt: &str, items: &[PickItem]) -> Result<Option<usize>> {
    let mut query = String::new();
    let mut cursor: usize = 0;
    let mut visible = filter_indices("", items);
    loop {
        draw(prompt, &query, items, &visible, cursor)?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => return Ok(None),
            (KeyCode::Enter, _) => return Ok(visible.get(cursor).copied()),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(None),
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) if query.is_empty() => {
                cursor = cursor
                    .saturating_sub(1)
                    .min(visible.len().saturating_sub(1));
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) if query.is_empty() => {
                cursor = (cursor + 1).min(visible.len().saturating_sub(1));
            }
            (KeyCode::Backspace, _) => {
                query.pop();
                visible = filter_indices(&query, items);
                cursor = 0;
            }
            (KeyCode::Char(ch), KeyModifiers::NONE) | (KeyCode::Char(ch), KeyModifiers::SHIFT) => {
                query.push(ch);
                visible = filter_indices(&query, items);
                cursor = 0;
            }
            _ => {}
        }
        if visible.is_empty() {
            cursor = 0;
        } else {
            cursor = cursor.min(visible.len() - 1);
        }
    }
}

fn draw(
    prompt: &str,
    query: &str,
    items: &[PickItem],
    visible: &[usize],
    cursor: usize,
) -> Result<()> {
    use crossterm::{cursor as term_cursor, style::Print, terminal, QueueableCommand};
    let mut out = io::stdout();
    out.queue(term_cursor::Hide)?
        .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?
        .queue(Print(format!(
            "{prompt} (↑↓ navigate · type to filter · Enter select · Esc cancel)\n"
        )))?;
    if !query.is_empty() {
        out.queue(Print(format!("filter: {query}\n")))?;
    }
    let rows = 12usize;
    let start = cursor
        .saturating_sub(rows - 1)
        .min(visible.len().saturating_sub(rows));
    for (row, &index) in visible.iter().skip(start).take(rows).enumerate() {
        let item = &items[index];
        let marker = if start + row == cursor { "❯" } else { " " };
        let line = if item.hint.is_empty() {
            format!("{marker} {}\n", item.title)
        } else {
            format!("{marker} {} — {}\n", item.title, item.hint)
        };
        out.queue(Print(line))?;
    }
    if visible.is_empty() {
        out.queue(Print("  (no matches)\n"))?;
    }
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<PickItem> {
        vec![
            PickItem::new("groq", "low-latency hosted inference"),
            PickItem::new("cerebras", "ultra-fast hosted inference"),
            PickItem::new("cohere", "command models"),
        ]
    }

    #[test]
    fn empty_query_returns_everything_in_order() {
        assert_eq!(filter_indices("", &sample()), vec![0, 1, 2]);
    }

    #[test]
    fn filter_matches_title_or_hint_case_insensitively() {
        assert_eq!(filter_indices("GROQ", &sample()), vec![0]);
        assert_eq!(filter_indices("hosted inference", &sample()), vec![0, 1]);
        assert_eq!(filter_indices("command", &sample()), vec![2]);
        assert!(filter_indices("zzz-no-match", &sample()).is_empty());
    }
}

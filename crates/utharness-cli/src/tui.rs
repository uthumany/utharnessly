use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{io, process::Command, time::Duration};

const CYAN: Color = Color::Rgb(91, 214, 224);
const GREEN: Color = Color::Rgb(115, 217, 176);
const YELLOW: Color = Color::Rgb(239, 190, 93);
const PURPLE: Color = Color::Rgb(185, 141, 255);
const BLUE: Color = Color::Rgb(120, 173, 255);
const MUTED: Color = Color::Rgb(126, 143, 157);
const SURFACE: Color = Color::Rgb(14, 21, 29);
const SOFT: Color = Color::Rgb(210, 222, 229);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Chat,
    Composer,
}

#[derive(Debug)]
struct TuiState {
    focus: Focus,
    draft: String,
    sent_count: usize,
    status: String,
    palette_open: bool,
    attention_dismissed: bool,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            focus: Focus::Composer,
            draft: String::new(),
            sent_count: 0,
            status: "Ready · offline planner".into(),
            palette_open: false,
            attention_dismissed: false,
        }
    }
}

impl TuiState {
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => return true,
                KeyCode::Char('k') => {
                    self.palette_open = !self.palette_open;
                    self.status = if self.palette_open {
                        "Command palette open".into()
                    } else {
                        "Command palette closed".into()
                    };
                }
                KeyCode::Char('b') => {
                    self.status = "Navigation is available from Ctrl+K".into();
                    self.palette_open = true;
                }
                KeyCode::Char('l') => {
                    self.draft.clear();
                    self.status = "Composer cleared".into();
                }
                _ => {}
            }
            return false;
        }

        if self.palette_open {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.palette_open = false,
                KeyCode::Char('1') => self.status = "Chat selected".into(),
                KeyCode::Char('2') => self.status = "Tasks selected".into(),
                KeyCode::Char('3') => self.status = "Files selected".into(),
                KeyCode::Char('4') => self.status = "Agents selected".into(),
                KeyCode::Char('5') => self.status = "Memory selected".into(),
                _ => {}
            }
            return false;
        }

        match key.code {
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Chat => Focus::Composer,
                    Focus::Composer => Focus::Chat,
                }
            }
            KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Chat => Focus::Composer,
                    Focus::Composer => Focus::Chat,
                }
            }
            KeyCode::Char(c) if self.focus == Focus::Composer => self.draft.push(c),
            KeyCode::Backspace if self.focus == Focus::Composer => {
                self.draft.pop();
            }
            KeyCode::Enter if self.focus == Focus::Composer => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.draft.push('\n');
                } else if !self.draft.trim().is_empty() {
                    self.sent_count += 1;
                    self.status = format!("Message {} sent · planning", self.sent_count);
                    self.draft.clear();
                }
            }
            KeyCode::Esc => {
                self.draft.clear();
                self.status = "Draft cleared".into();
            }
            _ => {}
        }
        false
    }

    fn reference_prefix(&self) -> Option<&str> {
        let line = self.draft.rsplit('\n').next().unwrap_or_default();
        let start = line.rfind('@')?;
        Some(&line[start + 1..])
    }
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = draw_loop(&mut terminal);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn draw_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut state = TuiState::default();
    loop {
        terminal.draw(|frame| render(frame, &state))?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if state.handle_key(key) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn render(frame: &mut Frame<'_>, state: &TuiState) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(SURFACE)), area);
    if area.width < 64 || area.height < 16 {
        render_narrow(frame, area, state);
        if state.palette_open {
            render_palette(frame, area);
        }
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(if state.reference_prefix().is_some() {
                8
            } else {
                5
            }),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, sections[0], area.width);
    render_chat(frame, sections[1], state);
    render_composer(frame, sections[2], state);
    render_telemetry(frame, sections[3]);
    render_footer(frame, sections[4], state);
    if state.palette_open {
        render_palette(frame, area);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, width: u16) {
    let model = std::env::var("UTHARNESS_MODEL").unwrap_or_else(|_| "Offline Planner".into());
    let provider = if model == "Offline Planner" {
        "local"
    } else {
        "OpenRouter"
    };
    let branch = git_branch();
    let path = current_workspace();
    let mut spans = vec![
        Span::styled(
            "UTHY",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  ● ONLINE",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(MUTED)),
        Span::styled(provider, Style::default().fg(BLUE)),
        Span::styled(" / ", Style::default().fg(MUTED)),
        Span::styled(model, Style::default().fg(Color::White)),
        Span::styled("  │  ", Style::default().fg(MUTED)),
        Span::styled(branch, Style::default().fg(PURPLE)),
    ];
    if width >= 96 {
        spans.extend([
            Span::styled("  │  ", Style::default().fg(MUTED)),
            Span::styled(path, Style::default().fg(BLUE)),
            Span::styled("  │  ", Style::default().fg(MUTED)),
            Span::styled("112.6K remaining", Style::default().fg(SOFT)),
        ]);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_chat(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let mut lines = vec![
        Line::from(Span::styled("UTHARNESS", Style::default().fg(GREEN).add_modifier(Modifier::BOLD))),
        Line::from("Welcome back. I can inspect this workspace, form a scoped plan, and ask before making changes."),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tip  ", Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)),
            Span::styled("Start with a clear task. Use @file, @folder, @agent, @skill, or @memory to add context.", Style::default().fg(SOFT)),
        ]),
    ];
    if !state.status.starts_with("Ready") {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            state.status.as_str(),
            Style::default().fg(YELLOW),
        )));
    }

    let attention = provider_needs_setup() && !state.attention_dismissed;
    if attention {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "╭─ PROJECT SETUP ─────────────────────────────────────────────────────────╮",
            Style::default().fg(YELLOW),
        )));
        lines.push(Line::from(Span::styled("│ Offline Planner is active. Configure an API provider to enable model-backed execution. │", Style::default().fg(SOFT))));
        lines.push(Line::from(Span::styled(
            "╰─────────────────────────────────────────────────────────────────────────╯",
            Style::default().fg(YELLOW),
        )));
    }

    let title = if state.focus == Focus::Chat {
        "CHAT · FOCUSED"
    } else {
        "CHAT"
    };
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::default()
                .title(title)
                .title_style(Style::default().fg(if state.focus == Focus::Chat {
                    CYAN
                } else {
                    MUTED
                })),
        ),
        area,
    );
}

fn render_composer(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let has_references = state.reference_prefix().is_some();
    let constraints = if has_references {
        vec![
            Constraint::Min(3),
            Constraint::Length(2),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Min(3), Constraint::Length(1)]
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    let input = if state.draft.is_empty() {
        "Type your message or @path/to/file".to_string()
    } else {
        state.draft.clone()
    };
    let style = if state.focus == Focus::Composer {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(MUTED)
    };
    frame.render_widget(
        Paragraph::new(input)
            .style(style)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(" MESSAGE ")
                    .title_style(Style::default().fg(CYAN))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(CYAN)),
            ),
        sections[0],
    );
    if let Some(prefix) = state.reference_prefix() {
        let suggestions = reference_suggestions(prefix);
        frame.render_widget(
            Paragraph::new(suggestions)
                .style(Style::default().fg(BLUE))
                .block(
                    Block::default()
                        .title(" REFERENCES ")
                        .title_style(Style::default().fg(BLUE))
                        .borders(Borders::TOP),
                ),
            sections[1],
        );
        frame.render_widget(
            Paragraph::new(
                " Enter Send   Shift+Enter New line   @ References   Ctrl+K Commands   Ctrl+L Clear",
            )
            .style(Style::default().fg(MUTED)),
            sections[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new(
                " Enter Send   Shift+Enter New line   @ References   Ctrl+K Commands   Ctrl+L Clear",
            )
            .style(Style::default().fg(MUTED)),
            sections[1],
        );
    }
}

fn render_telemetry(frame: &mut Frame<'_>, area: Rect) {
    let model = std::env::var("UTHARNESS_MODEL").unwrap_or_else(|_| "Offline Planner".into());
    let provider = if model == "Offline Planner" {
        "local"
    } else {
        "OpenRouter"
    };
    let telemetry = format!("workspace {}  │  permission SAFE  │  provider {}  │  model {}  │  branch {}  │  context 112.6K remaining", current_workspace(), provider, model, git_branch());
    frame.render_widget(
        Paragraph::new(telemetry).style(Style::default().fg(MUTED)),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let text = if state.focus == Focus::Composer {
        "Tab Focus chat   Ctrl+K Command palette   Ctrl+L Clear   F1 Help   Ctrl+C Exit"
    } else {
        "Tab Focus composer   Ctrl+K Command palette   Ctrl+C Exit"
    };
    frame.render_widget(Paragraph::new(text).style(Style::default().fg(MUTED)), area);
}

fn render_narrow(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "UTHY",
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ● ONLINE", Style::default().fg(GREEN)),
        ])),
        sections[0],
    );
    frame.render_widget(Paragraph::new("UTHARNESS\n\nWelcome back. Start with a task or add @context.\n\nOffline Planner ready.").wrap(Wrap { trim: true }), sections[1]);
    let draft = if state.draft.is_empty() {
        "Type your message or @path/to/file"
    } else {
        state.draft.as_str()
    };
    frame.render_widget(
        Paragraph::new(draft).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(" MESSAGE ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(CYAN)),
        ),
        sections[2],
    );
    frame.render_widget(
        Paragraph::new("Ctrl+K Commands   Ctrl+C Exit").style(Style::default().fg(MUTED)),
        sections[3],
    );
}

fn render_palette(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered_rect(64, 58, area);
    let items = "COMMAND PALETTE\n\n1  Chat                 Open conversation\n2  Tasks                Inspect execution\n3  Files                Browse workspace\n4  Agents               Manage agents\n5  Memory               Search project memory\n\nCtrl+P  Model   Ctrl+O  Files   Ctrl+G  Agents\nEsc     Close";
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(items)
            .style(Style::default().fg(SOFT))
            .block(
                Block::default()
                    .title(" COMMANDS ")
                    .title_style(Style::default().fg(CYAN))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(CYAN)),
            ),
        popup,
    );
}

fn reference_suggestions(prefix: &str) -> String {
    let options = [
        "file   src/main.rs",
        "folder src/",
        "agent @tester",
        "skill @debugger",
        "memory project notes",
    ];
    options
        .iter()
        .filter(|item| prefix.is_empty() || item.contains(prefix))
        .take(4)
        .copied()
        .collect::<Vec<_>>()
        .join("   ·   ")
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn provider_needs_setup() -> bool {
    std::env::var_os("OPENROUTER_API_KEY").is_none()
        && std::env::var_os("OPENAI_API_KEY").is_none()
        && std::env::var_os("UTHARNESS_PROVIDER_URL").is_none()
}

fn current_workspace() -> String {
    std::env::current_dir()
        .ok()
        .map(|path| shorten_path(&path.to_string_lossy()))
        .unwrap_or_else(|| "~/workspace".into())
}

fn git_branch() -> String {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "no-branch".into())
}

fn shorten_path(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        path.to_string()
    } else {
        path.replacen(&home, "~", 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composer_supports_multiline_input_and_send() {
        let mut state = TuiState::default();
        state.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        state.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert_eq!(state.draft, "h\ni");
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(state.draft.is_empty());
        assert_eq!(state.sent_count, 1);
    }

    #[test]
    fn ctrl_k_opens_command_palette_without_stealing_input() {
        let mut state = TuiState::default();
        state.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert!(state.palette_open);
        state.handle_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert!(state.palette_open);
        state.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!state.palette_open);
    }

    #[test]
    fn references_are_detected_from_the_current_line() {
        let state = TuiState {
            draft: "Inspect @src/".into(),
            ..Default::default()
        };
        assert_eq!(state.reference_prefix(), Some("src/"));
        assert!(reference_suggestions("src").contains("src/main.rs"));
    }
}

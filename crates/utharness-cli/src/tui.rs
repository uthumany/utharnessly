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
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::{io, process::Command, time::Duration};

const CYAN: Color = Color::Rgb(91, 214, 224);
const GREEN: Color = Color::Rgb(115, 217, 176);
const YELLOW: Color = Color::Rgb(239, 190, 93);
const PURPLE: Color = Color::Rgb(185, 141, 255);
const BLUE: Color = Color::Rgb(120, 173, 255);
const MUTED: Color = Color::Rgb(126, 143, 157);
const PANEL: Color = Color::Rgb(31, 42, 54);
const SURFACE: Color = Color::Rgb(14, 21, 29);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Navigation,
    Chat,
    Inspector,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorTab {
    Task,
    Context,
    Agents,
    Tools,
    Git,
}

impl InspectorTab {
    const ALL: [Self; 5] = [
        Self::Task,
        Self::Context,
        Self::Agents,
        Self::Tools,
        Self::Git,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Task => "TASK",
            Self::Context => "CONTEXT",
            Self::Agents => "AGENTS",
            Self::Tools => "TOOLS",
            Self::Git => "GIT",
        }
    }

    fn next(self) -> Self {
        let index = Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let index = Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug)]
struct TuiState {
    focus: Focus,
    inspector: InspectorTab,
    nav_collapsed: bool,
    draft: String,
    sent_count: usize,
    status: String,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            focus: Focus::Chat,
            inspector: InspectorTab::Task,
            nav_collapsed: false,
            draft: String::new(),
            sent_count: 0,
            status: "Ready · offline planner".into(),
        }
    }
}

impl TuiState {
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => return true,
                KeyCode::Char('b') => {
                    self.nav_collapsed = !self.nav_collapsed;
                    self.status = if self.nav_collapsed {
                        "Navigation collapsed"
                    } else {
                        "Navigation expanded"
                    }
                    .into();
                }
                KeyCode::Char('1') => self.inspector = InspectorTab::Task,
                KeyCode::Char('2') => self.inspector = InspectorTab::Context,
                KeyCode::Char('3') => self.inspector = InspectorTab::Agents,
                KeyCode::Char('4') => self.inspector = InspectorTab::Tools,
                KeyCode::Char('5') => self.inspector = InspectorTab::Git,
                _ => {}
            }
            return false;
        }
        match key.code {
            KeyCode::Char('q') if self.draft.is_empty() => return true,
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Navigation => Focus::Chat,
                    Focus::Chat => Focus::Inspector,
                    Focus::Inspector => Focus::Navigation,
                }
            }
            KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Navigation => Focus::Inspector,
                    Focus::Chat => Focus::Navigation,
                    Focus::Inspector => Focus::Chat,
                }
            }
            KeyCode::Left => self.inspector = self.inspector.previous(),
            KeyCode::Right => self.inspector = self.inspector.next(),
            KeyCode::Char('h') if self.focus == Focus::Inspector => {
                self.inspector = self.inspector.previous()
            }
            KeyCode::Char('l') if self.focus == Focus::Inspector => {
                self.inspector = self.inspector.next()
            }
            KeyCode::Char(c) => self.draft.push(c),
            KeyCode::Backspace => {
                self.draft.pop();
            }
            KeyCode::Enter => {
                if !self.draft.trim().is_empty() {
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
    let outer = Block::default().style(Style::default().bg(SURFACE));
    frame.render_widget(outer, area);
    if area.width < 52 || area.height < 14 {
        render_tiny(frame, area, state);
        return;
    }
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
            Constraint::Length(2),
        ])
        .split(area);
    render_header(frame, sections[0], area.width);
    render_body(frame, sections[1], state);
    render_composer(frame, sections[2], state);
    render_footer(frame, sections[3], state);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, width: u16) {
    let branch = git_branch();
    let path = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(shorten_path))
        .unwrap_or_else(|| "~/workspace".into());
    let mut line = vec![
        Span::styled(
            " UTHY ",
            Style::default()
                .fg(Color::Black)
                .bg(CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  ● ONLINE",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(MUTED)),
        Span::styled("Offline Planner", Style::default().fg(Color::White)),
        Span::styled(" │ ", Style::default().fg(MUTED)),
        Span::styled(branch, Style::default().fg(PURPLE)),
        Span::styled(" │ ", Style::default().fg(MUTED)),
        Span::styled(path, Style::default().fg(BLUE)),
    ];
    if width >= 100 {
        line.extend([
            Span::styled(" │ ", Style::default().fg(MUTED)),
            Span::styled("Context ", Style::default().fg(MUTED)),
            Span::styled("15.4K/128K", Style::default().fg(Color::White)),
            Span::styled(" │ ", Style::default().fg(MUTED)),
            Span::styled("3 agents", Style::default().fg(PURPLE)),
        ]);
    }
    frame.render_widget(
        Paragraph::new(Line::from(line)).block(panel("", false)),
        area,
    );
}

fn render_body(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let wide = area.width >= 100;
    let medium = area.width >= 78;
    let constraints = if wide && !state.nav_collapsed {
        vec![
            Constraint::Percentage(16),
            Constraint::Percentage(60),
            Constraint::Percentage(24),
        ]
    } else if medium {
        vec![Constraint::Percentage(66), Constraint::Percentage(34)]
    } else {
        vec![Constraint::Percentage(100)]
    };
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);
    let mut index = 0;
    if wide && !state.nav_collapsed {
        render_navigation(frame, panes[index], state);
        index += 1;
    }
    render_chat(frame, panes[index], state);
    index += 1;
    if medium {
        render_inspector(frame, panes[index], state);
    }
}

fn render_navigation(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let items = [
        ("◆", "Chat", ""),
        ("✓", "Tasks", "3"),
        ("▤", "Files", ""),
        ("◉", "Agents", "2"),
        ("✦", "Skills", "34"),
        ("◫", "Memory", ""),
        ("◷", "Jobs", "1"),
        ("◇", "Models", ""),
        ("⚙", "Tools", ""),
        ("≡", "Logs", ""),
        ("⚙", "Settings", ""),
    ];
    let list = items
        .iter()
        .enumerate()
        .map(|(index, (icon, name, count))| {
            let selected = index == 0;
            let marker = if selected { "▸" } else { " " };
            let suffix = if count.is_empty() {
                String::new()
            } else {
                format!("  {count}")
            };
            let style = if selected {
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    marker,
                    Style::default().fg(if selected { CYAN } else { MUTED }),
                ),
                Span::raw(" "),
                Span::styled(*icon, style),
                Span::raw(" "),
                Span::styled(*name, style),
                Span::styled(suffix, Style::default().fg(MUTED)),
            ]))
        })
        .collect::<Vec<_>>();
    let title = if state.focus == Focus::Navigation {
        " NAVIGATION · FOCUSED "
    } else {
        " NAVIGATION "
    };
    frame.render_widget(
        List::new(list).block(panel(title, state.focus == Focus::Navigation)),
        area,
    );
}

fn render_chat(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let mut lines = vec![
        Line::from(Span::styled(
            "YOU",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from("Fix the failing authentication tests and run the suite."),
        Line::from(""),
        Line::from(Span::styled(
            "UTHARNESS",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        )),
        Line::from("I found two failures related to session expiration."),
        Line::from("The execution plan is staged below."),
        Line::from(""),
        Line::from(Span::styled(
            "◆ READ  ",
            Style::default().fg(BLUE).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "src/auth/session.rs",
            Style::default().fg(BLUE),
        )),
        Line::from(Span::styled(
            "◆ READ  ",
            Style::default().fg(BLUE).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "tests/auth_test.rs",
            Style::default().fg(BLUE),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "╭─ EDIT  src/auth/session.rs ───────────────────────────╮",
            Style::default().fg(BLUE),
        )),
        Line::from(Span::styled(
            "│ +12  -4   View diff · Open file · Revert              │",
            Style::default().fg(BLUE),
        )),
        Line::from(Span::styled(
            "╰──────────────────────────────────────────────────────╯",
            Style::default().fg(BLUE),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "TESTER",
            Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
        )),
        Line::from("Running authentication suite..."),
        Line::from(Span::styled("✓ 14 / 14 passed", Style::default().fg(GREEN))),
    ];
    if !state.status.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            state.status.as_str(),
            Style::default().fg(YELLOW),
        )));
    }
    let title = if state.focus == Focus::Chat {
        " CHAT · FOCUSED "
    } else {
        " CHAT "
    };
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(title, state.focus == Focus::Chat)),
        area,
    );
}

fn render_inspector(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let tabs = InspectorTab::ALL
        .iter()
        .map(|tab| tab.title())
        .collect::<Vec<_>>();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);
    let selected = InspectorTab::ALL
        .iter()
        .position(|tab| *tab == state.inspector)
        .unwrap_or(0);
    let tabs_widget = Tabs::new(tabs)
        .select(selected)
        .highlight_style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .divider("│")
        .block(panel(" INSPECTOR ", state.focus == Focus::Inspector));
    frame.render_widget(tabs_widget, vertical[0]);
    let content = match state.inspector {
        InspectorTab::Task => task_view(),
        InspectorTab::Context => context_view(),
        InspectorTab::Agents => agents_view(),
        InspectorTab::Tools => tools_view(),
        InspectorTab::Git => git_view(),
    };
    frame.render_widget(
        Paragraph::new(content)
            .wrap(Wrap { trim: true })
            .block(panel(
                state.inspector.title(),
                state.focus == Focus::Inspector,
            )),
        vertical[1],
    );
}

fn task_view() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "TASK  Fix authentication tests",
            Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("✓ 01", Style::default().fg(GREEN))),
        Line::from("   Inspect repository       0.8s"),
        Line::from(Span::styled("✓ 02", Style::default().fg(GREEN))),
        Line::from("   Load project memory      0.1s"),
        Line::from(Span::styled("● 03", Style::default().fg(YELLOW))),
        Line::from("   Analyze auth module      4.2s"),
        Line::from(Span::styled("○ 04", Style::default().fg(MUTED))),
        Line::from("   Edit implementation"),
        Line::from(Span::styled("○ 05", Style::default().fg(MUTED))),
        Line::from("   Run tests"),
        Line::from(Span::styled("○ 06", Style::default().fg(MUTED))),
        Line::from("   Review diff"),
        Line::from(Span::styled("○ 07", Style::default().fg(MUTED))),
        Line::from("   Checkpoint"),
        Line::from(""),
        Line::from(Span::styled(
            "Progress  ████████░░░░░░░░ 43%",
            Style::default().fg(CYAN),
        )),
    ]
}

fn context_view() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "CONTEXT",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "██████░░░░░░░░░░░░░░ 12%",
            Style::default().fg(CYAN),
        )),
        Line::from("15.4K / 128K"),
        Line::from(""),
        Line::from("Messages                 6.2K"),
        Line::from("Files                    4.1K"),
        Line::from("Memory                   2.3K"),
        Line::from("Tools                    1.7K"),
        Line::from("System                   1.1K"),
        Line::from(""),
        Line::from(Span::styled(
            "Ctrl+L  Compact context",
            Style::default().fg(MUTED),
        )),
    ]
}

fn agents_view() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "AGENTS  3 active",
            Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("● Lead", Style::default().fg(PURPLE))),
        Line::from("  Planning implementation"),
        Line::from(Span::styled("● Coder", Style::default().fg(PURPLE))),
        Line::from("  Editing src/runtime.rs"),
        Line::from(Span::styled("◐ Tester", Style::default().fg(YELLOW))),
        Line::from("  cargo test"),
        Line::from(Span::styled("○ Reviewer", Style::default().fg(MUTED))),
        Line::from("  Waiting"),
        Line::from(""),
        Line::from(Span::styled(
            "Ctrl+G  Spawn agent",
            Style::default().fg(MUTED),
        )),
    ]
}

fn tools_view() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "TOOLS  1 running",
            Style::default().fg(BLUE).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("◆ SHELL", Style::default().fg(BLUE))),
        Line::from("  $ cargo test"),
        Line::from("  218 passed · 2 failed"),
        Line::from("  Exit 101 · 4.8s"),
        Line::from(""),
        Line::from(Span::styled("○ EDIT", Style::default().fg(YELLOW))),
        Line::from("  Awaiting approval"),
        Line::from(""),
        Line::from(Span::styled(
            "Ctrl+T  Focus tool",
            Style::default().fg(MUTED),
        )),
    ]
}

fn git_view() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "GIT  main ↑2",
            Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("+12", Style::default().fg(GREEN)),
            Span::raw(" insertions    "),
            Span::styled("-4", Style::default().fg(Color::Red)),
            Span::raw(" deletions"),
        ]),
        Line::from("M3 modified · ?1 untracked"),
        Line::from(""),
        Line::from(Span::styled(
            "No commit created",
            Style::default().fg(MUTED),
        )),
    ]
}

fn render_composer(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(1)])
        .split(area);
    let input = if state.draft.is_empty() {
        "> Ask Utharness...".to_string()
    } else {
        format!("> {}", state.draft)
    };
    let input_style = if state.focus == Focus::Chat {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(MUTED)
    };
    frame.render_widget(
        Paragraph::new(input)
            .style(input_style)
            .block(panel(" MESSAGE ", state.focus == Focus::Chat)),
        vertical[0],
    );
    frame.render_widget(
        Paragraph::new(
            " Enter Send │ Shift+Enter New Line │ @ Context │ / Commands │ Ctrl+K Palette",
        )
        .style(Style::default().fg(MUTED)),
        vertical[1],
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let hints = match state.focus {
        Focus::Chat => " Ctrl+B Navigation   Ctrl+1–5 Inspector   Ctrl+P Model   Ctrl+O Files   F1 Help   Ctrl+C Cancel ",
        Focus::Navigation => " Tab Focus Chat   Ctrl+B Collapse   Enter Select   Ctrl+G Agents   F1 Help ",
        Focus::Inspector => " ←/→ Tabs   Ctrl+1–5 Jump   Esc Clear   Ctrl+K Commands   Ctrl+C Cancel ",
    };
    frame.render_widget(
        Paragraph::new(hints)
            .style(Style::default().fg(MUTED))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(PANEL)),
            ),
        area,
    );
}

fn render_tiny(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let text = vec![
        Line::from(Span::styled(
            "UTHY  ● ONLINE",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from("────────────────────"),
        Line::from(Span::styled("Chat", Style::default().fg(Color::White))),
        Line::from("Offline Planner ready"),
        Line::from("────────────────────"),
        Line::from(if state.draft.is_empty() {
            "> prompt"
        } else {
            "> draft"
        }),
        Line::from("Enter send · q quit"),
    ];
    frame.render_widget(Paragraph::new(text).block(panel(" UTHY ", true)), area);
}

fn panel(title: &str, focused: bool) -> Block<'static> {
    Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_type(if focused {
            BorderType::Rounded
        } else {
            BorderType::Plain
        })
        .border_style(Style::default().fg(if focused { CYAN } else { PANEL }))
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
    if !home.is_empty() {
        path.replacen(&home, "~", 1)
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_shortcuts_update_layout_state() {
        let mut state = TuiState::default();
        assert!(!state.nav_collapsed);
        assert!(!state.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)));
        assert!(state.nav_collapsed);
        assert_eq!(state.inspector, InspectorTab::Task);
        state.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::CONTROL));
        assert_eq!(state.inspector, InspectorTab::Agents);
    }

    #[test]
    fn composer_accepts_and_sends_message() {
        let mut state = TuiState::default();
        state.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        state.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert_eq!(state.draft, "hi");
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(state.draft.is_empty());
        assert_eq!(state.sent_count, 1);
    }
}

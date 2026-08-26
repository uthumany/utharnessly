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
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Overlay {
    Commands,
    Model,
    Files,
    Agents,
    Tasks,
    Memory,
    Logs,
    Help,
}

impl Overlay {
    fn title(self) -> &'static str {
        match self {
            Self::Commands => "COMMAND PALETTE",
            Self::Model => "MODEL PICKER",
            Self::Files => "FILE PICKER",
            Self::Agents => "AGENT MANAGER",
            Self::Tasks => "TASK INSPECTOR",
            Self::Memory => "PROJECT MEMORY",
            Self::Logs => "RUNTIME LOGS",
            Self::Help => "KEYBOARD HELP",
        }
    }
}

#[derive(Debug)]
struct TuiState {
    focus: Focus,
    draft: String,
    sent_count: usize,
    status: String,
    overlay: Option<Overlay>,
    workspace_mode: bool,
    history_scroll: u16,
    attention_dismissed: bool,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            focus: Focus::Composer,
            draft: String::new(),
            sent_count: 0,
            status: "Ready · Focus Mode".into(),
            overlay: None,
            workspace_mode: false,
            history_scroll: 0,
            attention_dismissed: false,
        }
    }
}

impl TuiState {
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => return true,
                KeyCode::Char('k') => self.toggle_overlay(Overlay::Commands),
                KeyCode::Char('p') => self.toggle_overlay(Overlay::Model),
                KeyCode::Char('o') => self.toggle_overlay(Overlay::Files),
                KeyCode::Char('g') => self.toggle_overlay(Overlay::Agents),
                KeyCode::Char('t') => self.toggle_overlay(Overlay::Tasks),
                KeyCode::Char('m') => self.toggle_overlay(Overlay::Memory),
                KeyCode::Char('l') => self.toggle_overlay(Overlay::Logs),
                KeyCode::Char('b') => {
                    self.workspace_mode = !self.workspace_mode;
                    self.overlay = None;
                    self.status = if self.workspace_mode {
                        "Workspace Mode enabled · Ctrl+B to return to Focus Mode".into()
                    } else {
                        "Focus Mode enabled · conversation is primary".into()
                    };
                }
                _ => {}
            }
            return false;
        }

        if key.code == KeyCode::F(1) {
            self.toggle_overlay(Overlay::Help);
            return false;
        }

        if let Some(overlay) = self.overlay {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.overlay = None,
                KeyCode::Char('1') => self.toggle_overlay(Overlay::Commands),
                KeyCode::Char('2') => self.toggle_overlay(Overlay::Model),
                KeyCode::Char('3') => self.toggle_overlay(Overlay::Files),
                KeyCode::Char('4') => self.toggle_overlay(Overlay::Agents),
                KeyCode::Char('5') => self.toggle_overlay(Overlay::Tasks),
                KeyCode::Char('6') => self.toggle_overlay(Overlay::Memory),
                _ => self.status = format!("{} open · Esc closes", overlay.title()),
            }
            return false;
        }

        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Chat => Focus::Composer,
                    Focus::Composer => Focus::Chat,
                }
            }
            KeyCode::Up if self.focus == Focus::Chat => {
                self.history_scroll = self.history_scroll.saturating_add(1);
            }
            KeyCode::Down if self.focus == Focus::Chat => {
                self.history_scroll = self.history_scroll.saturating_sub(1);
            }
            KeyCode::PageUp => self.history_scroll = self.history_scroll.saturating_add(6),
            KeyCode::PageDown => self.history_scroll = self.history_scroll.saturating_sub(6),
            KeyCode::Char(c) if self.focus == Focus::Composer => self.draft.push(c),
            KeyCode::Backspace if self.focus == Focus::Composer => {
                self.draft.pop();
            }
            KeyCode::Enter if self.focus == Focus::Composer => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.draft.push('\n');
                } else if !self.draft.trim().is_empty() {
                    self.sent_count += 1;
                    self.status = format!("Message {} sent · plan queued", self.sent_count);
                    self.draft.clear();
                    self.history_scroll = 0;
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

    fn toggle_overlay(&mut self, overlay: Overlay) {
        self.overlay = if self.overlay == Some(overlay) {
            None
        } else {
            Some(overlay)
        };
        self.status = self
            .overlay
            .map(|item| format!("{} open", item.title()))
            .unwrap_or_else(|| "Overlay closed".into());
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
    } else if state.workspace_mode {
        render_workspace(frame, area, state);
    } else {
        render_focus(frame, area, state);
    }
    if let Some(overlay) = state.overlay {
        render_overlay(frame, area, overlay);
    }
}

fn render_focus(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
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
}

fn render_workspace(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, sections[0], area.width);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(18),
            Constraint::Percentage(57),
            Constraint::Percentage(25),
        ])
        .split(sections[1]);
    let navigation = [
        "◆ Chat",
        "✓ Tasks   3",
        "▤ Files",
        "◉ Agents",
        "✦ Skills",
        "◫ Memory",
        "◷ Jobs",
        "◇ Models",
        "≡ Logs",
    ];
    frame.render_widget(
        List::new(
            navigation
                .iter()
                .map(|item| ListItem::new(*item))
                .collect::<Vec<_>>(),
        )
        .block(
            Block::default()
                .title(" WORKSPACE ")
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(MUTED)),
        ),
        panes[0],
    );
    render_chat(frame, panes[1], state);
    let inspector = "TASK\n\n✓ Inspect repository\n✓ Load memory\n● Analyze auth module\n○ Edit files\n○ Run tests\n\nTOOLS\n\n◆ git_status   completed\n○ shell         waiting\n\nCtrl+T Inspector";
    frame.render_widget(
        Paragraph::new(inspector).wrap(Wrap { trim: true }).block(
            Block::default()
                .title(" INSPECTOR ")
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(MUTED)),
        ),
        panes[2],
    );
    render_telemetry(frame, sections[2]);
    render_footer(frame, sections[3], state);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, width: u16) {
    let model = std::env::var("UTHARNESS_MODEL").unwrap_or_else(|_| "Offline Planner".into());
    let provider = if model == "Offline Planner" {
        "local"
    } else {
        "OpenRouter"
    };
    let branch = git_branch();
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
            Span::styled(current_workspace(), Style::default().fg(BLUE)),
            Span::styled("  │  ", Style::default().fg(MUTED)),
            Span::styled("112.6K remaining", Style::default().fg(SOFT)),
        ]);
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_chat(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let mut lines = conversation_lines(state);
    if !state.status.starts_with("Ready") {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            state.status.as_str(),
            Style::default().fg(YELLOW),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((state.history_scroll, 0))
            .block(
                Block::default()
                    .title(if state.focus == Focus::Chat {
                        " CHAT · FOCUSED "
                    } else {
                        " CHAT "
                    })
                    .title_style(Style::default().fg(if state.focus == Focus::Chat {
                        CYAN
                    } else {
                        MUTED
                    })),
            ),
        area,
    );
}

fn conversation_lines(state: &TuiState) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            "YOU",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from("Fix the authentication tests and run the complete suite."),
        Line::from(""),
        Line::from(Span::styled(
            "UTHARNESS",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        )),
        Line::from("I will inspect the repository, read the auth module, and ask before edits."),
        Line::from(""),
    ];
    lines.extend(inline_card(
        "PLAN",
        "Fix authentication tests",
        "Inspect → read → edit → test",
        YELLOW,
        false,
    ));
    lines.extend(inline_card(
        "FILE",
        "src/auth/session.rs",
        "128 lines · read-only",
        BLUE,
        false,
    ));
    lines.extend(inline_card(
        "EDIT",
        "src/auth/session.rs",
        "+12  -4   View diff",
        YELLOW,
        true,
    ));
    lines.extend(inline_card(
        "TEST",
        "Running complete suite",
        "████████████████░░░░ 80%",
        GREEN,
        true,
    ));
    lines.push(Line::from(Span::styled(
        "218 passed   2 running   0 failed",
        Style::default().fg(GREEN),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "INLINE ACTIVITY",
        Style::default().fg(PURPLE).add_modifier(Modifier::BOLD),
    )));
    for (kind, title, detail, color, active) in [
        ("SHELL", "cargo test", "permission required", BLUE, false),
        (
            "DIFF",
            "auth/session.rs",
            "+12  -4 · View diff",
            YELLOW,
            false,
        ),
        ("GIT", "main", "2 files changed", PURPLE, false),
        ("BROWSER", "docs.rs", "waiting", BLUE, false),
        ("AGENT", "Tester", "running suite", PURPLE, true),
        ("MEMORY", "project notes", "2 matches", CYAN, false),
        ("SKILL", "debugger", "loaded", PURPLE, false),
        ("MCP", "filesystem", "connected", BLUE, false),
        ("ERROR", "none", "0 failed", GREEN, false),
    ] {
        lines.extend(inline_card(kind, title, detail, color, active));
    }
    if state.sent_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Message {} queued for the agent runtime.", state.sent_count),
            Style::default().fg(CYAN),
        )));
    }
    if !is_project_workspace() && !state.attention_dismissed {
        lines.push(Line::from(""));
        lines.extend(inline_card(
            "WARNING",
            "Project workspace required",
            "Open a Git project for repository-aware features",
            YELLOW,
            true,
        ));
    }
    lines
}

fn inline_card(
    kind: &str,
    title: &str,
    detail: &str,
    color: Color,
    active: bool,
) -> Vec<Line<'static>> {
    let marker = if active { "◆" } else { "✓" };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!("{marker} {kind} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(title.to_string(), Style::default().fg(SOFT)),
    ])];
    if active {
        lines.push(Line::from(Span::styled(
            format!("   {detail}"),
            Style::default().fg(color),
        )));
    } else {
        lines[0].spans.push(Span::styled(
            format!("  · {detail}"),
            Style::default().fg(MUTED),
        ));
    }
    lines
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
    frame.render_widget(
        Paragraph::new(input)
            .style(Style::default().fg(if state.focus == Focus::Composer {
                Color::White
            } else {
                MUTED
            }))
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
        frame.render_widget(
            Paragraph::new(reference_suggestions(prefix))
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
            Paragraph::new(" Enter Send   Shift+Enter New line   @ Context   Ctrl+K Commands")
                .style(Style::default().fg(MUTED)),
            sections[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new(" Enter Send   Shift+Enter New line   @ Context   Ctrl+K Commands")
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
    let text = format!(
        "workspace {}  │  SAFE MODE  │  {}/{}  │  context 112.6K remaining",
        current_workspace(),
        provider,
        model
    );
    frame.render_widget(Paragraph::new(text).style(Style::default().fg(MUTED)), area);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let text = if state.workspace_mode {
        "Ctrl+B Focus Mode   Ctrl+K Commands   Ctrl+T Tasks   F1 Help   Ctrl+C Exit"
    } else {
        "Ctrl+K Commands   Ctrl+P Model   Ctrl+O Files   Ctrl+G Agents   Ctrl+T Tasks   Ctrl+M Memory   Ctrl+L Logs   Ctrl+B Workspace   F1 Help"
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
    frame.render_widget(
        Paragraph::new(
            "CHAT\n\nUTHARNESS ready. Use Ctrl+K for commands.\n\n◆ TEST  80% · 218 passed",
        )
        .wrap(Wrap { trim: true }),
        sections[1],
    );
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

fn render_overlay(frame: &mut Frame<'_>, area: Rect, overlay: Overlay) {
    let popup = centered_rect(68, 62, area);
    let body = match overlay {
        Overlay::Commands => "1  Chat       2  Model       3  Files\n4  Agents     5  Tasks       6  Memory\n\nCtrl+L  Logs        Ctrl+B  Workspace Mode\nF1      Help        Esc      Close",
        Overlay::Model => "MODEL PICKER\n\n● Offline Planner       active · local\n○ OpenRouter / Qwen3-Coder\n○ OpenAI-compatible provider\n\nEnter select · Esc close",
        Overlay::Files => "FILE PICKER\n\n▸ src/\n  ├─ auth/session.rs\n  ├─ auth/token.rs\n  └─ runtime.rs\n▸ tests/\n  └─ auth_test.rs\n\nType to filter · Enter attach",
        Overlay::Agents => "AGENT MANAGER\n\n● Lead       planning\n● Coder      waiting for approval\n◐ Tester     running suite\n○ Reviewer   idle\n\nEnter inspect · Esc close",
        Overlay::Tasks => "TASK INSPECTOR\n\n✓ Inspect repository\n✓ Load project memory\n● Analyze auth module\n○ Edit implementation\n○ Run tests\n○ Review diff\n\nCtrl+T toggles this view",
        Overlay::Memory => "PROJECT MEMORY\n\n2 indexed records\n· SAFE shell policy\n· Authentication test conventions\n\nType to search · Enter attach",
        Overlay::Logs => "RUNTIME LOGS\n\n[info] session ready\n[info] tool policy SAFE\n[ok] SQLite event journal healthy\n[wait] no active provider stream",
        Overlay::Help => "KEYBOARD HELP\n\nEnter       Send message\nShift+Enter New line\nCtrl+K      Command palette\nCtrl+P/O/G/T/M/L  Context overlays\nCtrl+B      Workspace Mode\nTab         Focus chat/composer\nCtrl+C      Exit",
    };
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(body)
            .style(Style::default().fg(SOFT))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(format!(" {} ", overlay.title()))
                    .title_style(Style::default().fg(CYAN))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(CYAN)),
            ),
        popup,
    );
}

fn reference_suggestions(prefix: &str) -> String {
    [
        "file   src/main.rs",
        "folder src/",
        "agent @tester",
        "skill @debugger",
        "memory project notes",
    ]
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

fn is_project_workspace() -> bool {
    let Ok(path) = std::env::current_dir() else {
        return false;
    };
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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
    fn focus_shortcuts_open_context_overlays() {
        let mut state = TuiState::default();
        for (key, expected) in [
            ('p', Overlay::Model),
            ('o', Overlay::Files),
            ('g', Overlay::Agents),
            ('t', Overlay::Tasks),
            ('m', Overlay::Memory),
            ('l', Overlay::Logs),
        ] {
            state.handle_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::CONTROL));
            assert_eq!(state.overlay, Some(expected));
        }
    }

    #[test]
    fn ctrl_b_toggles_workspace_mode() {
        let mut state = TuiState::default();
        state.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert!(state.workspace_mode);
        state.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert!(!state.workspace_mode);
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

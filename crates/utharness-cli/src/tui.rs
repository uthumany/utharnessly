use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEventKind,
    },
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
use serde_json::json;
use std::{
    fs, io,
    process::Command,
    time::{Duration, Instant},
};
use utharness_core::{new_id, MessageRole};
use utharness_storage::Storage;

const CYAN: Color = Color::Rgb(91, 214, 224);
const GREEN: Color = Color::Rgb(115, 217, 176);
const YELLOW: Color = Color::Rgb(239, 190, 93);
const PURPLE: Color = Color::Rgb(185, 141, 255);
const BLUE: Color = Color::Rgb(120, 173, 255);
const MUTED: Color = Color::Rgb(126, 143, 157);
const SURFACE: Color = Color::Rgb(14, 21, 29);
const SOFT: Color = Color::Rgb(221, 226, 231);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorMode {
    TrueColor,
    Ansi256,
    Ansi16,
    Mono,
}

fn color_mode() -> ColorMode {
    if std::env::var_os("NO_COLOR").is_some()
        || std::env::var("UTHARNESS_ASCII").is_ok_and(|v| v == "1" || v == "true")
    {
        return ColorMode::Mono;
    }
    match std::env::var("UTHARNESS_COLOR").ok().as_deref() {
        Some("truecolor") | Some("24bit") => ColorMode::TrueColor,
        Some("ansi256") | Some("256") => ColorMode::Ansi256,
        Some("ansi16") | Some("16") => ColorMode::Ansi16,
        Some("none") | Some("mono") => ColorMode::Mono,
        _ if std::env::var("COLORTERM")
            .map(|v| v.contains("truecolor") || v.contains("24bit"))
            .unwrap_or(false) =>
        {
            ColorMode::TrueColor
        }
        _ if std::env::var("TERM")
            .map(|v| v.contains("256color"))
            .unwrap_or(false) =>
        {
            ColorMode::Ansi256
        }
        _ => ColorMode::Ansi16,
    }
}

fn adaptive(color: Color) -> Color {
    adapt_color(color, color_mode())
}

fn adapt_color(color: Color, mode: ColorMode) -> Color {
    match mode {
        ColorMode::TrueColor => color,
        ColorMode::Ansi256 => match color {
            c if c == CYAN => Color::Indexed(45),
            c if c == GREEN => Color::Indexed(78),
            c if c == YELLOW => Color::Indexed(221),
            c if c == PURPLE => Color::Indexed(141),
            c if c == BLUE => Color::Indexed(75),
            c if c == MUTED => Color::Indexed(102),
            c if c == SOFT => Color::Indexed(255),
            c if c == SURFACE => Color::Indexed(234),
            _ => Color::Indexed(255),
        },
        ColorMode::Ansi16 => match color {
            c if c == CYAN => Color::Cyan,
            c if c == GREEN => Color::Green,
            c if c == YELLOW => Color::Yellow,
            c if c == PURPLE => Color::Magenta,
            c if c == BLUE => Color::Blue,
            c if c == MUTED => Color::DarkGray,
            c if c == SURFACE => Color::Black,
            _ => Color::White,
        },
        ColorMode::Mono => Color::Reset,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Breakpoint {
    Wide,
    Full,
    Compact,
    Minimal,
}

fn breakpoint(width: u16) -> Breakpoint {
    match width {
        120..=u16::MAX => Breakpoint::Wide,
        80..=119 => Breakpoint::Full,
        60..=79 => Breakpoint::Compact,
        _ => Breakpoint::Minimal,
    }
}

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
    Provider,
    Skills,
    Settings,
    Permission,
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
            Self::Provider => "PROVIDER PICKER",
            Self::Skills => "SKILL REGISTRY",
            Self::Settings => "SETTINGS",
            Self::Permission => "PERMISSION REQUEST",
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
    overlay_body: String,
    workspace_mode: bool,
    history_scroll: u16,
    attention_dismissed: bool,
    started_at: Instant,
    pending_message: Option<String>,
    selected_model: String,
    selected_provider: String,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            focus: Focus::Composer,
            draft: String::new(),
            sent_count: 0,
            status: "Ready · Focus Mode".into(),
            overlay: None,
            overlay_body: String::new(),
            workspace_mode: false,
            history_scroll: 0,
            attention_dismissed: false,
            started_at: Instant::now(),
            pending_message: None,
            selected_model: std::env::var("UTHARNESS_MODEL")
                .unwrap_or_else(|_| "Offline Planner".into()),
            selected_provider: std::env::var("UTHARNESS_PROVIDER")
                .unwrap_or_else(|_| "local".into()),
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
                KeyCode::Char('y') => self.toggle_overlay(Overlay::Permission),
                KeyCode::Char('b') => {
                    self.workspace_mode = !self.workspace_mode;
                    self.overlay = None;
                    self.overlay_body.clear();
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
            if overlay == Overlay::Permission {
                match key.code {
                    KeyCode::Char('a') => {
                        self.status = "SAFE policy blocked shell · no command executed".into();
                        persist_permission_decision("blocked_by_safe_policy");
                    }
                    KeyCode::Char('d') => {
                        self.status = "Permission denied · no command executed".into();
                        persist_permission_decision("denied_by_user");
                    }
                    _ => {}
                }
            } else if overlay == Overlay::Model {
                match key.code {
                    KeyCode::Char('1') => {
                        self.selected_model = "Offline Planner".into();
                        self.selected_provider = "local".into();
                        self.status = "Offline Planner selected for this session".into();
                    }
                    KeyCode::Char('2') => {
                        self.selected_model = "Qwen3-Coder".into();
                        self.selected_provider = "OpenRouter".into();
                        self.status = "Qwen3-Coder selected for this session".into();
                    }
                    _ => {}
                }
            } else if overlay == Overlay::Provider {
                match key.code {
                    KeyCode::Char('1') => {
                        self.selected_provider = "local".into();
                        self.status = "Local provider selected for this session".into();
                    }
                    KeyCode::Char('2') => {
                        self.selected_provider = "OpenRouter".into();
                        self.status = "OpenRouter selected for this session".into();
                    }
                    _ => {}
                }
            }
            if let Some(active) = self.overlay {
                self.overlay_body = overlay_body(active, self);
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.overlay = None;
                    self.overlay_body.clear();
                }
                KeyCode::Char('1') if overlay != Overlay::Model && overlay != Overlay::Provider => {
                    self.toggle_overlay(Overlay::Commands)
                }
                KeyCode::Char('2') if overlay != Overlay::Model && overlay != Overlay::Provider => {
                    self.toggle_overlay(Overlay::Model)
                }
                KeyCode::Char('3') => self.toggle_overlay(Overlay::Files),
                KeyCode::Char('4') => self.toggle_overlay(Overlay::Agents),
                KeyCode::Char('5') => self.toggle_overlay(Overlay::Tasks),
                KeyCode::Char('6') => self.toggle_overlay(Overlay::Memory),
                KeyCode::Char('7') => self.toggle_overlay(Overlay::Provider),
                KeyCode::Char('8') => self.toggle_overlay(Overlay::Skills),
                KeyCode::Char('9') => self.toggle_overlay(Overlay::Settings),
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
                    self.pending_message = Some(self.draft.clone());
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
        self.overlay_body = self
            .overlay
            .map(|item| overlay_body(item, self))
            .unwrap_or_default();
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

    fn slash_prefix(&self) -> Option<&str> {
        let line = self.draft.rsplit('\n').next().unwrap_or_default();
        let start = line.rfind('/')?;
        if start == 0 || line[..start].ends_with(char::is_whitespace) {
            Some(&line[start + 1..])
        } else {
            None
        }
    }
}

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = draw_loop(&mut terminal);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

fn draw_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut state = TuiState::default();
    loop {
        terminal.draw(|frame| render(frame, &state))?;
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    if state.handle_key(key) {
                        break;
                    }
                    if let Some(message) = state.pending_message.take() {
                        persist_composer_message(&message);
                    }
                }
                Event::Resize(_, _) => {
                    state.status = "Terminal resized · layout recalculated".into()
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        state.history_scroll = state.history_scroll.saturating_add(3)
                    }
                    MouseEventKind::ScrollDown => {
                        state.history_scroll = state.history_scroll.saturating_sub(3)
                    }
                    MouseEventKind::Down(_) => state.focus = Focus::Composer,
                    _ => {}
                },
                _ => {}
            }
        }
    }
    Ok(())
}

fn render(frame: &mut Frame<'_>, state: &TuiState) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(adaptive(SURFACE))),
        area,
    );
    match breakpoint(area.width) {
        Breakpoint::Minimal => render_narrow(frame, area, state),
        Breakpoint::Compact => render_compact(frame, area, state),
        Breakpoint::Full => render_focus(frame, area, state),
        Breakpoint::Wide if state.workspace_mode && area.height >= 16 => {
            render_workspace(frame, area, state)
        }
        Breakpoint::Wide => render_focus(frame, area, state),
    }
    if let Some(overlay) = state.overlay {
        render_overlay(frame, area, overlay, &state.overlay_body);
    }
}

fn render_focus(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(
                if state.reference_prefix().is_some() || state.slash_prefix().is_some() {
                    8
                } else {
                    5
                },
            ),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, sections[0], area.width, state);
    render_chat(frame, sections[1], state);
    render_composer(frame, sections[2], state);
    render_telemetry(frame, sections[3], state);
    render_footer(frame, sections[4], state);
}

fn render_compact(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, sections[0], area.width, state);
    render_chat(frame, sections[1], state);
    let draft = if state.draft.is_empty() {
        "Type your message or @path/to/file"
    } else {
        state.draft.as_str()
    };
    frame.render_widget(
        Paragraph::new(draft).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(" > MESSAGE ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(adaptive(CYAN))),
        ),
        sections[2],
    );
    frame.render_widget(
        Paragraph::new("Ctrl+K commands  Ctrl+B workspace  Ctrl+C exit")
            .style(Style::default().fg(adaptive(MUTED))),
        sections[3],
    );
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
    render_header(frame, sections[0], area.width, state);
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
                .border_style(Style::default().fg(adaptive(MUTED))),
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
                .border_style(Style::default().fg(adaptive(MUTED))),
        ),
        panes[2],
    );
    render_telemetry(frame, sections[2], state);
    render_footer(frame, sections[3], state);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, width: u16, state: &TuiState) {
    let model = state.selected_model.as_str();
    let provider = state.selected_provider.as_str();
    let branch = git_branch();
    let mut spans = vec![
        Span::styled(
            "UTHY",
            Style::default()
                .fg(adaptive(CYAN))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  ● ONLINE",
            Style::default()
                .fg(adaptive(GREEN))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(adaptive(MUTED))),
        Span::styled(provider, Style::default().fg(adaptive(BLUE))),
        Span::styled(" / ", Style::default().fg(adaptive(MUTED))),
        Span::styled(model, Style::default().fg(Color::White)),
        Span::styled("  │  ", Style::default().fg(adaptive(MUTED))),
        Span::styled(branch, Style::default().fg(adaptive(PURPLE))),
    ];
    if width >= 96 {
        spans.extend([
            Span::styled("  │  ", Style::default().fg(adaptive(MUTED))),
            Span::styled(current_workspace(), Style::default().fg(adaptive(BLUE))),
            Span::styled("  │  ", Style::default().fg(adaptive(MUTED))),
            Span::styled("112.6K remaining", Style::default().fg(adaptive(SOFT))),
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
            Style::default().fg(adaptive(YELLOW)),
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
    let persisted = persisted_history_lines();
    let mut lines = if persisted.is_empty() {
        vec![
            Line::from(Span::styled(
                "YOU",
                Style::default()
                    .fg(adaptive(CYAN))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("Fix the authentication tests and run the complete suite."),
            Line::from(""),
            Line::from(Span::styled(
                "UTHARNESS",
                Style::default()
                    .fg(adaptive(GREEN))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(
                "I will inspect the repository, read the auth module, and ask before edits.",
            ),
            Line::from(""),
        ]
    } else {
        let mut history = persisted;
        history.push(Line::from(Span::styled(
            "UTHARNESS",
            Style::default()
                .fg(adaptive(GREEN))
                .add_modifier(Modifier::BOLD),
        )));
        history.push(Line::from("Runtime context restored from SQLite."));
        history.push(Line::from(""));
        history
    };
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
        format!(
            "   ⏱ elapsed {:.1}s",
            state.started_at.elapsed().as_secs_f64()
        ),
        Style::default().fg(adaptive(MUTED)),
    )));
    lines.push(Line::from(Span::styled(
        "218 passed   2 running   0 failed",
        Style::default().fg(adaptive(GREEN)),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "INLINE ACTIVITY",
        Style::default()
            .fg(adaptive(PURPLE))
            .add_modifier(Modifier::BOLD),
    )));
    for (kind, title, detail, color, active) in [
        ("SHELL", "cargo test", "permission required", BLUE, false),
        ("PERMISSION", "shell", "ASK before execution", YELLOW, true),
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
            Style::default().fg(adaptive(CYAN)),
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

fn persisted_history_lines() -> Vec<Line<'static>> {
    let Some((storage, workspace)) = runtime_storage() else {
        return Vec::new();
    };
    let Some(session) = storage
        .list_sessions(workspace.id)
        .ok()
        .and_then(|mut sessions| sessions.drain(..).next())
    else {
        return Vec::new();
    };
    let Ok(messages) = storage.messages(session.id) else {
        return Vec::new();
    };
    messages
        .into_iter()
        .flat_map(|message| {
            let (label, color) = match message.role {
                MessageRole::User => ("YOU", CYAN),
                MessageRole::Assistant => ("UTHARNESS", GREEN),
                MessageRole::System => ("SYSTEM", YELLOW),
                MessageRole::Tool => ("TOOL", BLUE),
            };
            vec![
                Line::from(Span::styled(
                    label,
                    Style::default()
                        .fg(adaptive(color))
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(message.content),
                Line::from(""),
            ]
        })
        .collect()
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
            Style::default()
                .fg(adaptive(color))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(title.to_string(), Style::default().fg(adaptive(SOFT))),
    ])];
    if active {
        lines.push(Line::from(Span::styled(
            format!("   {detail}"),
            Style::default().fg(adaptive(color)),
        )));
    } else {
        lines[0].spans.push(Span::styled(
            format!("  · {detail}"),
            Style::default().fg(adaptive(MUTED)),
        ));
    }
    lines
}

fn render_composer(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let has_references = state.reference_prefix().is_some();
    let has_slash = state.slash_prefix().is_some();
    let constraints = if has_references || has_slash {
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
                    .title_style(Style::default().fg(adaptive(CYAN)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(adaptive(CYAN))),
            ),
        sections[0],
    );
    if let Some(prefix) = state.reference_prefix() {
        frame.render_widget(
            Paragraph::new(reference_suggestions(prefix))
                .style(Style::default().fg(adaptive(BLUE)))
                .block(
                    Block::default()
                        .title(" REFERENCES ")
                        .title_style(Style::default().fg(adaptive(BLUE)))
                        .borders(Borders::TOP),
                ),
            sections[1],
        );
        frame.render_widget(
            Paragraph::new(
                " Enter Send   Shift+Enter New line   @ Context   / Commands   Ctrl+K Palette",
            )
            .style(Style::default().fg(adaptive(MUTED))),
            sections[2],
        );
    } else if let Some(prefix) = state.slash_prefix() {
        frame.render_widget(
            Paragraph::new(slash_suggestions(prefix))
                .style(Style::default().fg(adaptive(YELLOW)))
                .block(
                    Block::default()
                        .title(" COMMANDS ")
                        .title_style(Style::default().fg(adaptive(YELLOW)))
                        .borders(Borders::TOP),
                ),
            sections[1],
        );
        frame.render_widget(
            Paragraph::new(
                " Enter Send   Shift+Enter New line   @ Context   / Commands   Ctrl+K Palette",
            )
            .style(Style::default().fg(adaptive(MUTED))),
            sections[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new(
                " Enter Send   Shift+Enter New line   @ Context   / Commands   Ctrl+K Palette",
            )
            .style(Style::default().fg(adaptive(MUTED))),
            sections[1],
        );
    }
}

fn render_telemetry(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let model = state.selected_model.as_str();
    let provider = state.selected_provider.as_str();
    let text = format!(
        "workspace {}  │  SAFE MODE  │  {}/{}  │  context 112.6K remaining",
        current_workspace(),
        provider,
        model
    );
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(adaptive(MUTED))),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let text = if state.workspace_mode {
        "Ctrl+B Focus Mode   Ctrl+K Commands   Ctrl+T Tasks   F1 Help   Ctrl+C Exit"
    } else {
        "Ctrl+K Commands   Ctrl+P Model   Ctrl+O Files   Ctrl+G Agents   Ctrl+T Tasks   Ctrl+M Memory   Ctrl+L Logs   Ctrl+Y Permission   Ctrl+B Workspace   F1 Help"
    };
    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(adaptive(MUTED))),
        area,
    );
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
                Style::default()
                    .fg(adaptive(CYAN))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ● ONLINE", Style::default().fg(adaptive(GREEN))),
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
                .border_style(Style::default().fg(adaptive(CYAN))),
        ),
        sections[2],
    );
    frame.render_widget(
        Paragraph::new("Ctrl+K Commands   Ctrl+C Exit").style(Style::default().fg(adaptive(MUTED))),
        sections[3],
    );
}

fn render_overlay(frame: &mut Frame<'_>, area: Rect, overlay: Overlay, body: &str) {
    let popup = centered_rect(68, 62, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(body)
            .style(Style::default().fg(adaptive(SOFT)))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(format!(" {} ", overlay.title()))
                    .title_style(Style::default().fg(adaptive(CYAN)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(adaptive(CYAN))),
            ),
        popup,
    );
}

fn overlay_body(overlay: Overlay, state: &TuiState) -> String {
    match overlay {
        Overlay::Commands => "1  Chat       2  Model       3  Files\n4  Agents     5  Tasks       6  Memory\n7  Provider   8  Skills      9  Settings\n\nCtrl+L  Logs        Ctrl+B  Workspace Mode\nF1      Help        Esc      Close".into(),
        Overlay::Model => format!(
            "MODEL PICKER\n\n1  Offline Planner       {}\n2  Qwen3-Coder           {}\n\nEnter select · Esc close",
            if state.selected_model == "Offline Planner" { "active" } else { "available" },
            if state.selected_model == "Qwen3-Coder" { "active" } else { "available" }
        ),
        Overlay::Files => {
            let mut entries = fs::read_dir(".").ok().into_iter().flatten().filter_map(|entry| entry.ok()).map(|entry| entry.file_name().to_string_lossy().into_owned()).collect::<Vec<_>>();
            entries.sort();
            let listing = entries.into_iter().take(10).map(|entry| format!("  {entry}")).collect::<Vec<_>>().join("\n");
            format!("FILE PICKER\n\n{listing}\n\nType to filter · Enter attach")
        }
        Overlay::Agents => "AGENT MANAGER\n\n● Lead       planning\n● Coder      waiting for permission\n◐ Tester     running suite\n○ Reviewer   idle\n\nEnter inspect · Esc close".into(),
        Overlay::Tasks => runtime_storage()
            .and_then(|(storage, _)| storage.list_tasks().ok())
            .map(|tasks| {
                let body = tasks
                    .into_iter()
                    .take(8)
                    .map(|task| format!("{}  {}", task.status, task.title))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("TASK INSPECTOR\n\n{}\n\nSource: SQLite tasks", if body.is_empty() { "No persisted tasks".into() } else { body })
            })
            .unwrap_or_else(|| "TASK INSPECTOR\n\nSQLite runtime unavailable\n\nStart Utharness from a readable workspace.".into()),
        Overlay::Memory => runtime_storage()
            .and_then(|(storage, workspace)| storage.recent_memories(Some(workspace.id), 8).ok())
            .map(|memories| {
                let body = memories
                    .into_iter()
                    .map(|memory| format!("· {}", memory.content.replace('\n', " ")))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("PROJECT MEMORY\n\n{}\n\nSource: SQLite memory index", if body.is_empty() { "No persisted memories".into() } else { body })
            })
            .unwrap_or_else(|| "PROJECT MEMORY\n\nSQLite runtime unavailable\n\nMemory actions are disabled until the workspace can be opened.".into()),
        Overlay::Logs => runtime_storage()
            .and_then(|(storage, _)| storage.recent_events(8).ok())
            .map(|events| {
                let body = events.join("\n");
                format!("RUNTIME LOGS\n\n{}\n\nSource: SQLite event journal", if body.is_empty() { "No runtime events".into() } else { body })
            })
            .unwrap_or_else(|| "RUNTIME LOGS\n\nSQLite runtime unavailable\n\nNo persisted events can be read.".into()),
        Overlay::Help => "KEYBOARD HELP\n\nEnter       Send message\nShift+Enter New line\nCtrl+K      Command palette\nCtrl+P/O/G/T/M/L  Context overlays\nCtrl+Y      Permission dialog\nCtrl+B      Workspace Mode\nTab         Focus chat/composer\nCtrl+C      Exit".into(),
        Overlay::Provider => format!(
            "PROVIDER PICKER\n\n1  local        {}\n2  OpenRouter   {}\n\nProvider selection is process-local. Credentials remain environment-backed.",
            if state.selected_provider == "local" { "active" } else { "available" },
            if state.selected_provider == "OpenRouter" { "active" } else { "available" }
        ),
        Overlay::Skills => "SKILL REGISTRY\n\n✓ repo-explorer\n✓ code-search\n✓ build-fixer\n✓ test-runner\n✓ debugger\n✓ code-review\n\nSkills are loaded from the native registry.".into(),
        Overlay::Settings => "SETTINGS\n\npermission   SAFE\nanimations   reduced-motion aware\ncolors       TrueColor → ANSI → ASCII\nworkspace    local SQLite\n\nUse environment variables for provider and theme.".into(),
        Overlay::Permission => "PERMISSION REQUEST\n\nThe agent requested: shell → cargo test\nPolicy: SAFE (read-only tools only)\n\n[A] acknowledge blocked request\n[D] deny request\nEsc close\n\nNo shell command will execute in SAFE mode.".into(),
    }
}

fn runtime_storage() -> Option<(Storage, utharness_core::Workspace)> {
    let cwd = std::env::current_dir().ok()?;
    let home = std::env::var_os("UTHARNESS_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|path| std::path::PathBuf::from(path).join(".local/share/utharness"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from(".utharness"));
    let _ = fs::create_dir_all(&home);
    let db_path = std::env::var_os("UTHARNESS_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join("utharness.db"));
    let storage = Storage::open(db_path).ok()?;
    let workspace = storage.ensure_workspace(&cwd).ok()?;
    Some((storage, workspace))
}

fn persist_permission_decision(decision: &str) {
    let Some((storage, workspace)) = runtime_storage() else {
        return;
    };
    let Some(session) = storage
        .list_sessions(workspace.id)
        .ok()
        .and_then(|mut sessions| sessions.drain(..).next())
    else {
        return;
    };
    let _ = storage.record_event(
        "session",
        session.id,
        "permission_decision",
        &json!({"tool": "shell", "decision": decision, "policy": "safe"}),
        new_id(),
    );
}

fn persist_composer_message(message: &str) {
    let Some((storage, workspace)) = runtime_storage() else {
        return;
    };
    let cwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(_) => return,
    };
    let session = match storage
        .list_sessions(workspace.id)
        .ok()
        .and_then(|mut sessions| sessions.drain(..).next())
    {
        Some(session) => session,
        None => match storage.create_session(&workspace, "Focus Mode session", &cwd) {
            Ok(session) => session,
            Err(_) => return,
        },
    };
    let _ = storage.append_message(session.id, MessageRole::User, message);
    let _ = storage.record_event(
        "session",
        session.id,
        "focus_message_submitted",
        &json!({"mode": "focus", "character_count": message.chars().count()}),
        new_id(),
    );
}

fn slash_suggestions(prefix: &str) -> String {
    [
        "/model",
        "/provider",
        "/agents",
        "/files",
        "/git",
        "/tasks",
        "/memory",
        "/skills",
        "/theme",
        "/settings",
        "/doctor",
    ]
    .iter()
    .filter(|command| prefix.is_empty() || command.contains(prefix))
    .take(5)
    .copied()
    .collect::<Vec<_>>()
    .join("   ·   ")
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
    fn model_picker_updates_process_local_selection() {
        let mut state = TuiState::default();
        state.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
        state.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        assert_eq!(state.overlay, Some(Overlay::Model));
        assert_eq!(state.selected_model, "Qwen3-Coder");
        assert_eq!(state.selected_provider, "OpenRouter");
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
    fn responsive_breakpoints_match_the_requested_width_rules() {
        assert_eq!(breakpoint(120), Breakpoint::Wide);
        assert_eq!(breakpoint(240), Breakpoint::Wide);
        assert_eq!(breakpoint(119), Breakpoint::Full);
        assert_eq!(breakpoint(80), Breakpoint::Full);
        assert_eq!(breakpoint(79), Breakpoint::Compact);
        assert_eq!(breakpoint(60), Breakpoint::Compact);
        assert_eq!(breakpoint(59), Breakpoint::Minimal);
    }

    #[test]
    fn slash_commands_are_filtered_and_capped() {
        let suggestions = slash_suggestions("model");
        assert_eq!(suggestions, "/model");
        assert!(slash_suggestions("").split("   ·   ").count() <= 5);
        assert!(slash_suggestions("doctor").contains("/doctor"));
    }

    #[test]
    fn palette_falls_back_to_terminal_friendly_colors() {
        assert_eq!(adapt_color(CYAN, ColorMode::TrueColor), CYAN);
        assert_eq!(adapt_color(CYAN, ColorMode::Ansi256), Color::Indexed(45));
        assert_eq!(adapt_color(CYAN, ColorMode::Ansi16), Color::Cyan);
        assert_eq!(adapt_color(CYAN, ColorMode::Mono), Color::Reset);
        assert_eq!(adapt_color(SURFACE, ColorMode::Ansi16), Color::Black);
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

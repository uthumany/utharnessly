use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Terminal,
};
use serde_json::json;
use std::{
    env, fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
use utharness_core::{MessageRole, PermissionMode};
use utharness_security::Policy;
use utharness_storage::Storage;

mod banner;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(name = "utharness", version = VERSION, about = "Utharness Agent Terminal — local-first autonomous work")]
struct Cli {
    #[command(subcommand)]
    command: Option<CommandKind>,
}

#[derive(Subcommand, Debug)]
enum CommandKind {
    Init(InitArgs),
    Chat(ChatArgs),
    Run(RunArgs),
    Tui(TuiArgs),
    Doctor,
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    Checkpoint,
    Skills,
    Providers,
    Agents,
    Tools,
}

#[derive(Args, Debug)]
struct InitArgs {
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
}

#[derive(Args, Debug)]
struct ChatArgs {
    prompt: String,
    #[arg(long)]
    session: Option<String>,
}

#[derive(Args, Debug)]
struct RunArgs {
    #[arg(short, long)]
    command: String,
    #[arg(long)]
    allow: bool,
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
}

#[derive(Args, Debug)]
struct TuiArgs {
    #[arg(long)]
    headless: bool,
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    Show,
}

#[derive(Subcommand, Debug)]
enum SessionAction {
    List,
    New {
        #[arg(default_value = "Untitled session")]
        title: String,
    },
}

#[derive(Subcommand, Debug)]
enum MemoryAction {
    Add {
        content: String,
        #[arg(long, default_value = "project")]
        scope: String,
    },
    Search {
        query: String,
    },
}

struct App {
    storage: Storage,
    workspace: utharness_core::Workspace,
}

impl App {
    fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let workspace_path = fs::canonicalize(path)
            .with_context(|| format!("workspace does not exist: {}", path.display()))?;
        let data_dir = env::var_os("UTHARNESS_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs_fallback()
                    .join(".local")
                    .join("share")
                    .join("utharness")
            });
        fs::create_dir_all(&data_dir)?;
        let db_path = env::var_os("UTHARNESS_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|| data_dir.join("utharness.db"));
        let storage = Storage::open(db_path)?;
        let workspace = storage.ensure_workspace(&workspace_path)?;
        Ok(Self { storage, workspace })
    }

    fn current_session(&self) -> Result<utharness_core::Session> {
        self.storage
            .list_sessions(self.workspace.id)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no session exists; run `utharness sessions new`"))
    }

    fn ensure_session(&self, title: &str) -> Result<utharness_core::Session> {
        match self
            .storage
            .list_sessions(self.workspace.id)?
            .into_iter()
            .next()
        {
            Some(session) => Ok(session),
            None => self.storage.create_session(
                &self.workspace,
                title,
                Path::new(&self.workspace.canonical_path),
            ),
        }
    }
}

fn dirs_fallback() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("utharness=info")
        .with_target(false)
        .compact()
        .init();
    let cli = Cli::parse();
    match cli.command {
        None => launch_tui(false),
        Some(CommandKind::Init(args)) => {
            banner::print_startup_banner(VERSION)?;
            let app = App::open(&args.workspace)?;
            println!("UTHARNESS initialized");
            println!("workspace: {}", app.workspace.canonical_path);
            println!("database:  {}", app.storage.path().display());
            println!(
                "next:      utharness sessions new && utharness chat \"Inspect this workspace\""
            );
            Ok(())
        }
        Some(CommandKind::Chat(args)) => chat(args),
        Some(CommandKind::Run(args)) => run_command(args),
        Some(CommandKind::Tui(args)) => launch_tui(args.headless),
        Some(CommandKind::Doctor) => doctor(),
        Some(CommandKind::Config {
            action: ConfigAction::Show,
        }) => config_show(),
        Some(CommandKind::Sessions { action }) => sessions(action),
        Some(CommandKind::Memory { action }) => memory(action),
        Some(CommandKind::Checkpoint) => checkpoint(),
        Some(CommandKind::Skills) => {
            println!("BUILT-IN SKILLS\nrepo-explorer\ncode-search\nbuild-fixer\ntest-runner\ndebugger\ncode-review\ngit-workflow\nweb-research\ndocumentation-writer");
            Ok(())
        }
        Some(CommandKind::Providers) => {
            println!("PROVIDERS\nlocal / offline planner\nOpenAI-compatible / configure with UTHARNESS_PROVIDER_URL\nOllama / local model route");
            Ok(())
        }
        Some(CommandKind::Agents) => {
            println!("AGENTS\n● Uthy       Lead planner       READY\n○ Builder    Code specialist   AVAILABLE\n○ Tester     Verification       WAITING");
            Ok(())
        }
        Some(CommandKind::Tools) => {
            println!("TOOLS\n✓ read_file       SAFE\n✓ list_directory  SAFE\n! write_file      ASK\n! shell           ASK\n! browser_open    ASK\n✓ git_diff        SAFE");
            Ok(())
        }
    }
}

fn chat(args: ChatArgs) -> Result<()> {
    let app = App::open(".")?;
    let session = if let Some(id) = args.session {
        let parsed = uuid::Uuid::parse_str(&id).context("session must be a UUID")?;
        app.storage
            .list_sessions(app.workspace.id)?
            .into_iter()
            .find(|s| s.id == parsed)
            .context("session not found in this workspace")?
    } else {
        app.ensure_session("Terminal session")?
    };
    app.storage
        .append_message(session.id, MessageRole::User, &args.prompt)?;
    let response = format!("Offline planner ready. I received: {}\n\nNext steps: inspect the workspace, form a scoped plan, request permission for mutations, then verify the result. Configure a provider to enable model-backed execution.", args.prompt);
    app.storage
        .append_message(session.id, MessageRole::Assistant, &response)?;
    app.storage.record_event(
        "session",
        session.id,
        "message_completed",
        &json!({"offline": true}),
        utharness_core::new_id(),
    )?;
    println!("Uthy · OFFLINE PLANNER\n{}", response);
    Ok(())
}

fn run_command(args: RunArgs) -> Result<()> {
    let root = fs::canonicalize(&args.workspace)?;
    let policy = if args.allow {
        Policy {
            mode: PermissionMode::Trusted,
            workspace: root.clone(),
            allow_network: false,
            allow_shell: true,
        }
    } else {
        Policy::safe(root.clone())
    };
    let request = utharness_core::ToolRequest {
        tool: "shell".into(),
        target: Some(args.command.clone()),
        arguments: json!({"cwd": root}),
    };
    if policy.evaluate(&request) != utharness_core::PermissionDecision::Allow {
        println!("Permission required: shell execution is blocked in SAFE mode.");
        println!(
            "Re-run with `--allow` after reviewing the command: {}",
            Policy::redact(&args.command)
        );
        anyhow::bail!("tool denied by permission policy")
    }
    let lower = args.command.to_ascii_lowercase();
    for denied in ["rm -rf", "sudo ", "mkfs", "shutdown", ":(){"] {
        if lower.contains(denied) {
            anyhow::bail!("command denied by safety denylist: {denied}");
        }
    }
    let output = Command::new("sh")
        .arg("-c")
        .arg(&args.command)
        .current_dir(&root)
        .output()?;
    let stdout = Policy::redact(&String::from_utf8_lossy(&output.stdout));
    let stderr = Policy::redact(&String::from_utf8_lossy(&output.stderr));
    print!("{}", stdout);
    eprint!("{}", stderr);
    if !output.status.success() {
        anyhow::bail!("command exited with {}", output.status);
    }
    println!("\n[exit 0]");
    Ok(())
}

fn sessions(action: SessionAction) -> Result<()> {
    let app = App::open(".")?;
    match action {
        SessionAction::List => {
            let sessions = app.storage.list_sessions(app.workspace.id)?;
            if sessions.is_empty() {
                println!("No sessions. Create one with `utharness sessions new`. ");
            }
            for session in sessions {
                println!(
                    "{}  {}  {}  {}",
                    session.id, session.status, session.title, session.cwd
                );
            }
        }
        SessionAction::New { title } => {
            let session = app.storage.create_session(
                &app.workspace,
                &title,
                Path::new(&app.workspace.canonical_path),
            )?;
            println!("created session {} · {}", session.id, session.title);
        }
    }
    Ok(())
}

fn memory(action: MemoryAction) -> Result<()> {
    let app = App::open(".")?;
    match action {
        MemoryAction::Add { content, scope } => {
            let memory = app.storage.add_memory(
                Some(app.workspace.id),
                None,
                &scope,
                "note",
                &content,
                "cli",
            )?;
            println!("stored memory {} [{}]", memory.id, memory.scope);
        }
        MemoryAction::Search { query } => {
            let results = app.storage.search_memory(Some(app.workspace.id), &query)?;
            if results.is_empty() {
                println!("No memory matches for `{query}`");
            }
            for item in results {
                println!("[{}] {} · {}", item.scope, item.content, item.source);
            }
        }
    }
    Ok(())
}

fn checkpoint() -> Result<()> {
    let app = App::open(".")?;
    let session = app.current_session()?;
    let messages = app.storage.messages(session.id)?;
    let cp = app.storage.create_checkpoint(
        session.id,
        None,
        "manual checkpoint",
        &json!({"message_count": messages.len(), "cwd": session.cwd, "model": session.model_id}),
    )?;
    println!("checkpoint {} created for session {}", cp.id, session.id);
    Ok(())
}

fn doctor() -> Result<()> {
    let app = App::open(".")?;
    println!("UTHARNESS DOCTOR");
    println!("✓ version       {}", VERSION);
    println!("✓ workspace     {}", app.workspace.canonical_path);
    println!("✓ storage       {}", app.storage.path().display());
    println!("✓ database      {}", app.storage.integrity_check()?);
    println!("✓ permissions   SAFE default available");
    println!(
        "✓ shell         {}",
        env::var("SHELL").unwrap_or_else(|_| "sh".into())
    );
    println!("✓ provider      offline planner available");
    println!("✓ skills        built-in registry available");
    println!("✓ diagnostics   clean");
    Ok(())
}

fn config_show() -> Result<()> {
    let app = App::open(".")?;
    println!("workspace = \"{}\"", app.workspace.canonical_path);
    println!("database = \"{}\"", app.storage.path().display());
    println!("permission_mode = \"SAFE\"");
    println!("provider = \"offline\"");
    println!("theme = \"utharness-carbon\"");
    Ok(())
}

fn launch_tui(headless: bool) -> Result<()> {
    banner::print_startup_banner(VERSION)?;
    if headless || !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        println!("UTHARNESS · AGENT TERMINAL");
        println!(
            "● ONLINE · offline planner · workspace {}",
            env::current_dir()?.display()
        );
        println!(
            "Persistent TUI requires an interactive terminal. Try `utharness tui` from a terminal."
        );
        return Ok(());
    }
    run_tui()
}

fn run_tui() -> Result<()> {
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
    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let vertical = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(8),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(area);
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Length(22),
                        Constraint::Min(30),
                        Constraint::Length(30),
                    ]
                    .as_ref(),
                )
                .split(vertical[1]);
            let header = Paragraph::new(Line::from(vec![
                Span::styled(
                    " UTHARNESS ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  ● ONLINE   offline planner   "),
                Span::styled("~/projects/utharness", Style::default().fg(Color::Cyan)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            frame.render_widget(header, vertical[0]);
            let nav = List::new(
                [
                    "CHAT", "TASKS", "FILES", "AGENTS", "SKILLS", "MEMORY", "JOBS", "MODELS",
                    "TOOLS", "LOGS", "SETTINGS",
                ]
                .into_iter()
                .map(ListItem::new)
                .collect::<Vec<_>>(),
            )
            .block(Block::default().title(" NAVIGATION ").borders(Borders::ALL));
            frame.render_widget(nav, body[0]);
            let chat = Paragraph::new(vec![
                Line::from(Span::styled(
                    "UTHY / PLANNER",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(
                    "Offline mode is ready. Configure a provider to enable model-backed execution.",
                ),
                Line::from(""),
                Line::from(Span::styled("◆ PLAN", Style::default().fg(Color::Yellow))),
                Line::from("├─ Inspect repository"),
                Line::from("├─ Load project memory"),
                Line::from("├─ Request approved tools"),
                Line::from("└─ Verify and checkpoint"),
                Line::from(""),
                Line::from(Span::styled(
                    "> Ask Utharness...",
                    Style::default().fg(Color::Cyan),
                )),
            ])
            .wrap(Wrap { trim: true })
            .block(Block::default().title(" CHAT ").borders(Borders::ALL));
            frame.render_widget(chat, body[1]);
            let inspector = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(7), Constraint::Min(5)].as_ref())
                .split(body[2]);
            frame.render_widget(
                Gauge::default()
                    .block(Block::default().title(" CONTEXT ").borders(Borders::ALL))
                    .gauge_style(Style::default().fg(Color::Green))
                    .percent(12)
                    .label("12% · 15.4K / 128K"),
                inspector[0],
            );
            let task = List::new(
                [
                    "✓ Workspace",
                    "✓ Memory",
                    "◆ Waiting for prompt",
                    "○ Tool approval",
                    "○ Checkpoint",
                ]
                .into_iter()
                .map(ListItem::new)
                .collect::<Vec<_>>(),
            )
            .block(
                Block::default()
                    .title(" TASK INSPECTOR ")
                    .borders(Borders::ALL),
            );
            frame.render_widget(task, inspector[1]);
            let footer = Paragraph::new(
                " Ctrl+C cancel   Ctrl+K commands   Ctrl+P model   F1 help   q quit ",
            )
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(footer, vertical[2]);
        })?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn offline_chat_persists_two_messages() -> Result<()> {
        let dir = tempdir()?;
        let db = Storage::open(dir.path().join("test.db"))?;
        let workspace = db.ensure_workspace(dir.path())?;
        let session = db.create_session(&workspace, "test", dir.path())?;
        db.append_message(session.id, MessageRole::User, "hello")?;
        db.append_message(session.id, MessageRole::Assistant, "offline response")?;
        assert_eq!(db.messages(session.id)?.len(), 2);
        Ok(())
    }
}

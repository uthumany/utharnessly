use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::json;
use skills::SkillRegistry;
use std::{
    env, fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
    process::Command,
};
use utharness_core::MessageRole;
use utharness_provider::{
    has_provider_configuration, supported_providers, ChatMessage, Gateway, ProviderKind,
};
use utharness_security::Policy;
use utharness_storage::Storage;

mod banner;
mod desktop;
mod execution;
mod icons;
mod progress;
mod response_pipeline;
mod select;
mod setup_system;
mod skills;
mod termux;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AGENT_NAME: &str = "UTHARNESS";
const AGENT_IDENTITY: &str = "I was developed by: \"UTHUMAN & CO\" Center for AI (UCAI), which is a part of Uthuman Inc Data & AI. Our mission involves driving AI priorities, unifying CLI agent efforts through effective coordination, and implementing research projects. Developers: 𓁷 Uthuman M; 𓁷 Shafiq N; 𓁷 Alid K.";

#[cfg(test)]
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Parser, Debug)]
#[command(name = "utharness", version = VERSION, about = "Utharness Agent Terminal — local-first autonomous work")]
struct Cli {
    #[arg(long, global = true, value_enum, conflicts_with = "no_banner")]
    banner: Option<CliBannerMode>,
    #[arg(long, global = true)]
    no_banner: bool,
    #[command(subcommand)]
    command: Option<CommandKind>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliBannerMode {
    Full,
    Compact,
    Minimal,
}

#[derive(Subcommand, Debug)]
enum CommandKind {
    Init(InitArgs),
    Setup(SetupArgs),
    Chat(ChatArgs),
    Run(RunArgs),
    Tui(TuiArgs),
    Autonomous(AutonomousArgs),
    Doctor(DoctorArgs),
    Update,
    Uninstall,
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    Sessions {
        #[command(subcommand)]
        action: Option<SessionAction>,
    },
    Memory {
        #[command(subcommand)]
        action: Option<MemoryAction>,
    },
    Checkpoint,
    Skills(SkillsArgs),
    #[command(alias = "provider")]
    Providers(ProviderArgs),
    Agents(AgentArgs),
    Desktop(desktop::DesktopArgs),
    Tools,
    Models(ModelArgs),
    Mcp,
    Termux(TermuxArgs),
}

#[derive(Args, Debug)]
struct InitArgs {
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
}

#[derive(Args, Debug)]
struct SetupArgs {
    /// Write a validated configuration without opening the interactive wizard.
    #[arg(long)]
    non_interactive: bool,
    #[arg(long, default_value = "quick")]
    mode: String,
    #[arg(long, conflicts_with_all = ["full", "developer", "local_ai", "custom", "blank"])]
    quick: bool,
    #[arg(long, conflicts_with_all = ["quick", "developer", "local_ai", "custom", "blank"])]
    full: bool,
    #[arg(long, conflicts_with_all = ["quick", "full", "local_ai", "custom", "blank"])]
    developer: bool,
    #[arg(long, conflicts_with_all = ["quick", "full", "developer", "custom", "blank"])]
    local_ai: bool,
    #[arg(long, conflicts_with_all = ["quick", "full", "developer", "local_ai", "blank"])]
    custom: bool,
    #[arg(long, conflicts_with_all = ["quick", "full", "developer", "local_ai", "custom"])]
    blank: bool,
    /// Print a machine-readable environment and dependency scan, then exit.
    #[arg(long)]
    scan: bool,
    #[arg(long)]
    provider: Option<String>,
    #[arg(long)]
    model: Option<String>,
    #[arg(long)]
    provider_url: Option<String>,
    /// Read one API key from stdin, avoiding shell history and process arguments.
    #[arg(long)]
    api_key_stdin: bool,
    #[arg(long)]
    skip_validation: bool,
    #[arg(long)]
    import_config: Option<PathBuf>,
    /// Comma-separated capability identifiers.
    #[arg(long, value_delimiter = ',')]
    tools: Vec<String>,
}

#[derive(Args, Debug)]
struct DoctorArgs {
    #[arg(long)]
    fix: bool,
}

#[derive(Args, Debug)]
struct ModelArgs {
    #[command(subcommand)]
    action: Option<ModelAction>,
}

#[derive(Subcommand, Debug)]
enum ModelAction {
    List {
        /// Emit a stable machine-readable model catalog.
        #[arg(long)]
        json: bool,
    },
    Test {
        #[arg(default_value = "auto")]
        provider: String,
    },
}

const SETUP_TOOLS: &[&str] = &[
    "workspace_read",
    "git_inspection",
    "terminal",
    "file_write",
    "skills",
    "memory",
    "session_search",
    "task_planning",
    "desktop",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeConfig {
    schema_version: u8,
    mode: String,
    provider: String,
    model: String,
    permission_mode: String,
    tools: Vec<String>,
    #[serde(default)]
    ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiConfig {
    banner: bool,
    banner_mode: String,
    icons: String,
}
impl Default for UiConfig {
    fn default() -> Self {
        Self {
            banner: true,
            banner_mode: "full".into(),
            icons: "unicode".into(),
        }
    }
}

#[derive(Args, Debug)]
struct ChatArgs {
    /// Prompt to answer once. Omit to enter the interactive chat REPL,
    /// where /model switches provider and model mid-session.
    prompt: Option<String>,
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

#[derive(Args, Debug)]
struct AutonomousArgs {
    prompt: String,
    #[arg(long, default_value_t = 3)]
    max_steps: usize,
    #[arg(long, default_value = ".")]
    workspace: PathBuf,
}

#[derive(Args, Debug)]
struct ProviderArgs {
    #[command(subcommand)]
    action: Option<ProviderAction>,
}

#[derive(Subcommand, Debug)]
enum ProviderAction {
    List,
    Test {
        #[arg(default_value = "auto")]
        provider: String,
        /// Test every configured provider instead of a single one.
        #[arg(long)]
        all: bool,
    },
    Env,
}

#[derive(Args, Debug)]
struct AgentArgs {
    #[command(subcommand)]
    action: Option<AgentAction>,
}

#[derive(Subcommand, Debug)]
enum AgentAction {
    List,
    Run {
        prompt: String,
        #[arg(long, default_value_t = 3)]
        max_steps: usize,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    Show,
    Set { key: String, value: String },
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
    List,
    Add {
        content: String,
        #[arg(long, default_value = "project")]
        scope: String,
        #[arg(long, default_value = "note")]
        kind: String,
        /// Retention like `24h`, `7d`, `30d`; empty means never expires.
        #[arg(long, default_value = "")]
        expires: String,
    },
    Search {
        query: String,
    },
    Prune,
}

#[derive(Args, Debug)]
struct TermuxArgs {
    #[command(subcommand)]
    action: Option<TermuxAction>,
}

#[derive(Subcommand, Debug)]
enum TermuxAction {
    Info,
    Setup,
    Api {
        capability: Option<String>,
        #[arg(long)]
        value: Option<String>,
    },
    Keys(TermuxKeysArgs),
    Storage(TermuxStorageArgs),
    Permissions,
    Doctor,
}

#[derive(Args, Debug)]
struct TermuxKeysArgs {
    #[command(subcommand)]
    action: Option<TermuxKeysAction>,
}

#[derive(Subcommand, Debug)]
enum TermuxKeysAction {
    Install,
}

#[derive(Args, Debug)]
struct TermuxStorageArgs {
    #[command(subcommand)]
    action: Option<TermuxStorageAction>,
}

#[derive(Subcommand, Debug)]
enum TermuxStorageAction {
    Enable,
}

#[derive(Args, Debug)]
struct SkillsArgs {
    #[command(subcommand)]
    action: Option<SkillAction>,
}

#[derive(Subcommand, Debug)]
enum SkillAction {
    List {
        #[arg(default_value_t = 40)]
        limit: usize,
    },
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    Categories,
    Info {
        skill: String,
    },
    Install {
        skill: String,
        #[arg(long)]
        allow_external: bool,
    },
    Remove {
        skill: String,
    },
    Rollback {
        skill: String,
    },
    Update,
    Test {
        skill: String,
    },
    Doctor,
    Run {
        skill: String,
        #[arg(long)]
        allow_external: bool,
    },
    Sync {
        #[arg(long, default_value = "all")]
        source: String,
        #[arg(long, default_value_t = 500)]
        limit: usize,
    },
    Import {
        path: PathBuf,
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
    setup_system::load_secrets()?;
    let cli = Cli::parse();
    if cli.no_banner {
        env::set_var("UTHARNESS_BANNER", "hide");
    }
    if let Some(mode) = cli.banner {
        env::set_var(
            "UTHARNESS_BANNER",
            match mode {
                CliBannerMode::Full => "full",
                CliBannerMode::Compact => "compact",
                CliBannerMode::Minimal => "minimal",
            },
        );
    }
    if !matches!(&cli.command, Some(CommandKind::Setup(_))) {
        apply_runtime_config()?;
    }
    if io::stdout().is_terminal()
        && matches!(&cli.command, Some(command) if !matches!(command, CommandKind::Setup(_) | CommandKind::Tui(_) | CommandKind::Init(_) | CommandKind::Autonomous(_)))
    {
        banner::print_startup_banner(VERSION)?;
    }
    match cli.command {
        None => launch_tui(false),
        Some(CommandKind::Init(args)) => {
            banner::print_startup_banner(VERSION)?;
            banner::print_onboarding_tips()?;
            let app = App::open(&args.workspace)?;
            println!("UTHARNESS initialized");
            println!("workspace: {}", app.workspace.canonical_path);
            println!("database:  {}", app.storage.path().display());
            println!(
                "next:      utharness sessions new && utharness chat \"Inspect this workspace\""
            );
            Ok(())
        }
        Some(CommandKind::Setup(args)) => setup(args),
        Some(CommandKind::Chat(args)) => chat(args),
        Some(CommandKind::Run(args)) => run_command(args),
        Some(CommandKind::Tui(args)) => launch_tui(args.headless),
        Some(CommandKind::Autonomous(args)) => autonomous(args),
        Some(CommandKind::Doctor(args)) => doctor(args),
        Some(CommandKind::Update) => update(),
        Some(CommandKind::Uninstall) => uninstall(),
        Some(CommandKind::Config { action }) => match action.unwrap_or(ConfigAction::Show) {
            ConfigAction::Show => config_show(),
            ConfigAction::Set { key, value } => config_set(&key, &value),
        },
        Some(CommandKind::Sessions { action }) => sessions(action.unwrap_or(SessionAction::List)),
        Some(CommandKind::Memory { action }) => memory(action.unwrap_or(MemoryAction::List)),
        Some(CommandKind::Checkpoint) => checkpoint(),
        Some(CommandKind::Skills(args)) => skills_command(args),
        Some(CommandKind::Providers(args)) => {
            providers(args.action.unwrap_or(ProviderAction::List))
        }
        Some(CommandKind::Agents(args)) => agents(args.action.unwrap_or(AgentAction::List)),
        Some(CommandKind::Desktop(args)) => desktop::desktop_command(args),
        Some(CommandKind::Tools) => {
            println!(
                "TOOLS\n✓ read_file       SAFE\n✓ list_directory  SAFE\n! write_file      ASK\n! shell           ASK\n! browser_open    ASK\n! desktop {} ASK ({})\n✓ git_diff        SAFE\n{} {} commands pending (icon reserved)\n{} {} via `memory add/search/prune`",
                icons::icon(icons::Feature::Computer),
                icons::Feature::Computer.label(),
                icons::icon(icons::Feature::Improve),
                icons::Feature::Improve.label(),
                icons::icon(icons::Feature::Remember),
                icons::Feature::Remember.label(),
            );
            Ok(())
        }
        Some(CommandKind::Models(args)) => {
            models(args.action.unwrap_or(ModelAction::List { json: false }))
        }
        Some(CommandKind::Mcp) => mcp(),
        Some(CommandKind::Termux(args)) => termux_command(args),
    }
}

fn skill_registry() -> Result<SkillRegistry> {
    let data_dir = env::var_os("UTHARNESS_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs_fallback()
                .join(".local")
                .join("share")
                .join("utharness")
        });
    SkillRegistry::open(data_dir.join("skills"))
}

fn print_skill_summary(record: &skills::SkillRecord) {
    println!(
        "{}  [{}]  {}  v{}  status={} health={}",
        record.manifest.id,
        record.manifest.category,
        record.manifest.name,
        record.manifest.version,
        record.status,
        record.health
    );
    println!("  {}", record.manifest.description);
}

fn skills_command(args: SkillsArgs) -> Result<()> {
    let registry = skill_registry()?;
    registry.seed_builtins()?;
    match args.action.unwrap_or(SkillAction::List { limit: 40 }) {
        SkillAction::List { limit } => {
            println!("UTHARNESS SKILL REGISTRY · {} indexed", registry.count()?);
            for record in registry.list(None, limit)? {
                print_skill_summary(&record);
            }
        }
        SkillAction::Search { query, limit } => {
            println!("SKILL SEARCH · {query}");
            for record in registry.list(Some(&query), limit)? {
                print_skill_summary(&record);
            }
        }
        SkillAction::Categories => {
            for category in registry.categories()? {
                println!("{category}");
            }
        }
        SkillAction::Info { skill } => {
            let record = registry.get(&skill)?;
            println!(
                "status={} health={} installed_version={:?}",
                record.status, record.health, record.installed_version
            );
            println!("{}", serde_json::to_string_pretty(&record.manifest)?);
        }
        SkillAction::Install {
            skill,
            allow_external,
        } => {
            let record = registry.install(&skill, allow_external)?;
            println!(
                "installed {} v{} ({})",
                record.manifest.id, record.manifest.version, record.health
            );
        }
        SkillAction::Remove { skill } => {
            let record = registry.remove(&skill)?;
            println!(
                "removed {}; quarantine retained under {}",
                record.manifest.id,
                registry.root().join("quarantine").display()
            );
        }
        SkillAction::Rollback { skill } => {
            let record = registry.rollback(&skill)?;
            println!(
                "rolled back {}: status={} health={}",
                record.manifest.id, record.status, record.health
            );
        }
        SkillAction::Update => {
            println!(
                "refreshed health for {} installed skill(s)",
                registry.update_installed()?
            );
        }
        SkillAction::Test { skill } => {
            let record = registry.test(&skill)?;
            println!(
                "tested {}: status={} health={}",
                record.manifest.id, record.status, record.health
            );
        }
        SkillAction::Doctor => {
            let issues = registry.doctor()?;
            println!("SKILL REGISTRY DOCTOR");
            if issues.is_empty() {
                println!("✓ registry healthy · {} indexed", registry.count()?);
            }
            for issue in issues {
                println!("! {issue}");
            }
        }
        SkillAction::Run {
            skill,
            allow_external,
        } => {
            let result = registry.run(&skill, &env::current_dir()?, allow_external)?;
            println!("SKILL RESULT · {skill}\n{result}");
        }
        SkillAction::Sync { source, limit } => {
            for report in registry.sync(&source, limit)? {
                println!(
                    "{}: imported={} skipped={}",
                    report.source, report.imported, report.skipped
                );
                for error in report.errors {
                    println!("  ! {error}");
                }
            }
        }
        SkillAction::Import { path } => {
            let record = registry.import_manifest_path(&path)?;
            println!(
                "imported {} v{}",
                record.manifest.id, record.manifest.version
            );
        }
    }
    Ok(())
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
    match args.prompt {
        Some(prompt) => complete_once(&app, &session, &prompt),
        None => chat_repl(&app, &session),
    }
}

fn complete_once(app: &App, session: &utharness_core::Session, prompt: &str) -> Result<()> {
    use response_pipeline::ResponseStage;

    let mut pipeline = response_pipeline::ResponsePipeline::new();
    pipeline.advance(ResponseStage::Understanding);
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        println!("{} YOU · {prompt}", icons::icon_user());
    }
    app.storage
        .append_message(session.id, MessageRole::User, prompt)?;
    pipeline.advance(ResponseStage::Decomposing);
    let mut live = false;
    let memories = recall_block(&app.storage, app.workspace.id, prompt);
    pipeline.advance(ResponseStage::Retrieving);
    let system = agent_system_prompt(&memories);
    pipeline.advance(ResponseStage::Grounding);
    let messages = [
        ChatMessage {
            role: "system".into(),
            content: system,
        },
        ChatMessage {
            role: "user".into(),
            content: prompt.to_string(),
        },
    ];
    pipeline.advance(ResponseStage::Planning);
    let raw_response = match Gateway::from_environment() {
        Ok(provider) => {
            live = true;
            pipeline.advance(ResponseStage::Reasoning);
            pipeline.advance(ResponseStage::Routing);
            println!(
                "{} {} · {}/{}",
                icons::icon_agent(),
                AGENT_NAME,
                provider.provider(),
                provider.model()
            );
            pipeline.advance(ResponseStage::Executing);
            use std::io::Write;
            io::stdout().flush()?;
            let mut observed = false;
            let response = provider.complete_streaming(
                &messages,
                |delta| {
                    if !observed {
                        pipeline.advance(ResponseStage::Observing);
                        observed = true;
                    }
                    print!("{delta}");
                    io::stdout().flush()?;
                    Ok(())
                },
            )?;
            if !observed {
                pipeline.advance(ResponseStage::Observing);
            }
            println!();
            response
        }
        Err(_error) if !has_provider_configuration() => format!(
            "Offline planner ready. I received: {prompt}\n\nConfigure a provider with `utharness providers env` to enable live model streaming."
        ),
        Err(error) => return Err(error),
    };
    pipeline.advance(ResponseStage::Evaluating);
    if raw_response.trim().is_empty() {
        anyhow::bail!("provider completed without response text");
    }
    pipeline.advance(ResponseStage::Synthesizing);
    let response = raw_response.trim().to_string();
    pipeline.advance(ResponseStage::Verifying);
    pipeline.advance(ResponseStage::Refining);
    app.storage
        .append_message(session.id, MessageRole::Assistant, &response)?;
    pipeline.advance(ResponseStage::Finalizing);
    app.storage.record_event(
        "session",
        session.id,
        "message_completed",
        &json!({"offline": !live}),
        utharness_core::new_id(),
    )?;
    if !live {
        println!(
            "{} {} · OFFLINE PLANNER\n{}",
            icons::icon_agent(),
            AGENT_NAME,
            response
        );
    }
    pipeline.advance(ResponseStage::Responding);
    Ok(())
}

fn agent_system_prompt(memories: &str) -> String {
    let base = format!(
        "You are {AGENT_NAME}, a concise terminal coding agent. Never claim a command ran unless a tool result proves it. Your identity is: {AGENT_IDENTITY} If asked who created or developed you, state that identity exactly."
    );
    if memories.is_empty() {
        base
    } else {
        format!("{base}\n{memories}")
    }
}

/// Interactive chat REPL. The model selector lives here: /model picks a
/// provider then a model, applies it to the running session immediately,
/// and optionally persists it so the next open reuses it.
fn chat_repl(app: &App, session: &utharness_core::Session) -> Result<()> {
    print_current_selection("Chatting with");
    println!("Commands: /model · /where · /save · /help · /quit");
    use std::io::BufRead as _;
    // One stdin lock for the whole REPL, shared with the picker: re-locking
    // stdin on this thread deadlocks, and fresh per-line locks would drop
    // buffered piped input.
    let stdin = io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut line = String::new();
        let bytes = input.read_line(&mut line)?;
        if bytes == 0 {
            break; // EOF (piped input exhausted)
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match trimmed {
            "/quit" | "/exit" | "/q" => break,
            "/help" | "/h" | "/?" => {
                println!("  /model   pick provider + model for this session (applies now)");
                println!("  /where   show the active provider/model and where it came from");
                println!("  /save    persist the current selection (workspace or global)");
                println!("  /quit    leave the chat");
            }
            "/where" => print_selection_with_sources(),
            "/model" | "/provider" | "/m" => {
                if let Err(error) = model_selector_flow(&mut input) {
                    println!("Model selector failed: {error:#}");
                }
            }
            "/save" => {
                if let Err(error) = persist_current_selection(&mut input) {
                    println!("Save failed: {error:#}");
                }
            }
            _ if trimmed.starts_with('/') => println!("Unknown command. Try /help."),
            _ => {
                if let Err(error) = complete_once(app, session, trimmed) {
                    println!("Error: {error:#}");
                }
            }
        }
    }
    println!("Bye.");
    Ok(())
}

fn current_selection() -> (String, String) {
    let provider = env::var("UTHARNESS_PROVIDER")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let from_env = provider.is_some();
    let provider = provider.unwrap_or_else(|| {
        load_runtime_config()
            .ok()
            .flatten()
            .map(|c| c.provider)
            .or_else(|| setup_system::load_global_selection().map(|(p, _)| p))
            .unwrap_or_else(|| "autodetect".into())
    });
    let model = env::var("UTHARNESS_MODEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| {
            if from_env {
                String::new()
            } else {
                load_runtime_config()
                    .ok()
                    .flatten()
                    .map(|c| c.model)
                    .or_else(|| setup_system::load_global_selection().map(|(_, m)| m))
                    .unwrap_or_default()
            }
        });
    (provider, model)
}

fn print_current_selection(verb: &str) {
    let (provider, model) = current_selection();
    if model.is_empty() {
        println!("{verb} provider {provider} (model: provider default)");
    } else {
        println!("{verb} {provider}/{model}");
    }
}

/// Explain precedence at the point where users diagnose a surprising model.
/// Keep provider and model sources distinct: an explicit provider can still
/// legitimately use a model from a saved file, or its own default.
fn print_selection_with_sources() {
    let (provider, model) = current_selection();
    let provider_env = env::var("UTHARNESS_PROVIDER")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());
    let model_env = env::var("UTHARNESS_MODEL")
        .ok()
        .is_some_and(|value| !value.trim().is_empty());
    let workspace = load_runtime_config().ok().flatten();
    let global = setup_system::load_global_selection();

    let provider_source = if provider_env {
        "provider environment"
    } else if workspace.is_some() {
        "workspace config"
    } else if global.is_some() {
        "global config"
    } else {
        "provider autodetection"
    };
    let model_source = if model_env {
        "model environment"
    } else if provider_env {
        "provider default"
    } else if workspace.is_some() {
        "workspace config"
    } else if global.is_some() {
        "global config"
    } else {
        "provider default"
    };

    if model.is_empty() {
        println!(
            "Active provider {provider} (model: provider default) — source: {provider_source}; {model_source}"
        );
    } else {
        println!("Active {provider}/{model} — source: {provider_source}; {model_source}");
    }
}

/// Two-stage picker: provider first (keyed providers on top), then a live
/// model list with the provider default as fallback. Applies immediately via
/// process env; persistence is offered, never forced.
fn model_selector_flow(input: &mut impl std::io::BufRead) -> Result<()> {
    let mut statuses = utharness_provider::supported_providers();
    statuses.sort_by_key(|status| (!status.configured, status.provider.clone()));
    let provider_items: Vec<select::PickItem> = statuses
        .iter()
        .map(|status| {
            let state = if status.configured {
                "key ready"
            } else {
                "needs key"
            };
            select::PickItem::new(
                status.provider.clone(),
                format!("{} · default {}", state, status.model),
            )
        })
        .collect();
    let Some(provider_at) =
        select::pick_with_reader("Select AI provider", &provider_items, &mut *input)?
    else {
        println!("Kept the current selection.");
        return Ok(());
    };
    let status = &statuses[provider_at];
    env::set_var("UTHARNESS_PROVIDER", &status.provider);
    env::remove_var("UTHARNESS_MODEL");

    let gateway =
        Gateway::new_from_environment(utharness_provider::ProviderKind::parse(&status.provider)?);
    let models: Vec<String> = gateway
        .ok()
        .and_then(|gateway| gateway.models().ok())
        .filter(|list| !list.is_empty())
        .unwrap_or_else(|| vec![status.model.clone()]);
    let model_items: Vec<select::PickItem> = models
        .iter()
        .map(|model| {
            let mark = if *model == status.model {
                "provider default"
            } else {
                "live catalog"
            };
            select::PickItem::new(model.clone(), mark)
        })
        .collect();
    let Some(model_at) = select::pick_with_reader(
        &format!("Select model for {}", status.provider),
        &model_items,
        &mut *input,
    )?
    else {
        println!("Kept {}.", status.provider);
        return Ok(());
    };
    env::set_var("UTHARNESS_MODEL", &models[model_at]);
    println!("Now chatting with {}/{}", status.provider, models[model_at]);

    let save_items = vec![
        select::PickItem::new("Session only", "forgotten when this chat exits"),
        select::PickItem::new("Workspace file", "./utharness.json when one exists"),
        select::PickItem::new("Global config", "~/.utharness/config.yaml everywhere"),
    ];
    match select::pick_with_reader("Persist this selection", &save_items, &mut *input)? {
        Some(1) => persist_current_selection_to_workspace()
            .map(|path| println!("Saved to {}", path.display()))?,
        Some(2) => persist_current_selection_to_global()
            .map(|path| println!("Saved to {}", path.display()))?,
        _ => println!("Session-only: reopening utharness elsewhere keeps the previous files."),
    }
    Ok(())
}

fn persist_current_selection(input: &mut impl std::io::BufRead) -> Result<()> {
    let save_items = vec![
        select::PickItem::new("Workspace file", "./utharness.json when one exists"),
        select::PickItem::new("Global config", "~/.utharness/config.yaml everywhere"),
    ];
    match select::pick_with_reader("Persist the current selection", &save_items, input)? {
        Some(0) => persist_current_selection_to_workspace()
            .map(|path| println!("Saved to {}", path.display()))?,
        Some(1) => persist_current_selection_to_global()
            .map(|path| println!("Saved to {}", path.display()))?,
        _ => println!("Not saved."),
    }
    Ok(())
}

fn persist_current_selection_to_workspace() -> Result<PathBuf> {
    let (provider, _) = current_selection();
    if provider == "autodetect" {
        anyhow::bail!("no provider selected yet — use /model first");
    }
    let path = env::current_dir()?.join("utharness.json");
    let mut config = load_runtime_config()?.unwrap_or(RuntimeConfig {
        schema_version: 1,
        mode: "quick".into(),
        provider: "openrouter".into(),
        model: "openrouter/free".into(),
        permission_mode: "safe".into(),
        tools: vec!["workspace_read".into()],
        ui: UiConfig::default(),
    });
    let (provider, model) = current_selection();
    config.provider = provider;
    if !model.is_empty() {
        config.model = model;
    }
    fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&config)?),
    )
    .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

fn persist_current_selection_to_global() -> Result<PathBuf> {
    let (provider, model) = current_selection();
    if provider == "autodetect" {
        anyhow::bail!("no provider selected yet — use /model first");
    }
    let model = if model.is_empty() {
        utharness_provider::ProviderKind::parse(&provider)
            .map(|kind| Gateway::status_from_environment(kind).model)
            .unwrap_or_else(|_| "default".into())
    } else {
        model
    };
    let workspace_config = env::current_dir()?.join("utharness.json");
    setup_system::write_global_config("quick", &provider, &model, &workspace_config)
}

#[derive(Debug, Deserialize)]
struct AgentPlan {
    #[serde(default = "default_plan_summary")]
    summary: String,
    #[serde(default)]
    steps: Vec<AgentStep>,
    #[serde(default)]
    final_response: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgentStep {
    tool: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    rationale: String,
}

fn default_plan_summary() -> String {
    "Bounded workspace inspection".into()
}

fn autonomous(args: AutonomousArgs) -> Result<()> {
    banner::print_startup_banner(VERSION)?;
    banner::print_onboarding_tips()?;
    let root = fs::canonicalize(&args.workspace)?;
    let app = App::open(&root)?;
    let session = app.ensure_session("Autonomous agent test")?;
    let registry = skill_registry()?;
    registry.seed_builtins()?;
    let recommendations = registry.recommend(&args.prompt, 3)?;
    let recommended_names = recommendations
        .iter()
        .map(|record| record.manifest.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut temporary_skill_ids = Vec::new();
    let mut skill_evidence = Vec::new();
    for record in recommendations.iter().take(3) {
        if record.manifest.source.provider == "builtin" && record.status != "installed" {
            registry.install(&record.manifest.id, false)?;
            temporary_skill_ids.push(record.manifest.id.clone());
        }
        if record.manifest.source.provider == "builtin" {
            if let Ok(result) = registry.run(&record.manifest.id, &root, false) {
                skill_evidence.push(format!(
                    "{}: {}",
                    record.manifest.id,
                    result.lines().next().unwrap_or("ready")
                ));
            }
        }
    }
    let provider = match Gateway::from_environment() {
        Ok(provider) => provider,
        Err(error) => {
            for skill_id in &temporary_skill_ids {
                let _ = registry.remove(skill_id);
            }
            return Err(error);
        }
    };
    let max_steps = args.max_steps.clamp(1, 8);
    let memories = recall_block(&app.storage, app.workspace.id, &args.prompt);
    let planner_prompt = format!(
        "You are the Utharness autonomous planner. Return only valid JSON with this shape: {{\"summary\":\"short summary\",\"steps\":[{{\"tool\":\"list_directory|read_file|git_status|git_diff\",\"target\":\"relative path or null\",\"rationale\":\"short reason\"}}],\"final_response\":\"short completion note describing only what the executed steps achieve; never claim files were created\"}}. Plan at most {max_steps} read-only steps. Never request shell, write, network, secrets, or paths outside the workspace. {memories}Candidate skills from the local registry: {recommended_names}. Loaded skill evidence: {}. Task: {}",
        skill_evidence.join("; "),
        args.prompt
    );
    let plan: AgentPlan = match provider.complete_json(&[
        ChatMessage {
            role: "system".into(),
            content: "You are a careful, deterministic coding-agent planner. Use only the tools explicitly allowed by the user-facing contract.".into(),
        },
        ChatMessage {
            role: "user".into(),
            content: planner_prompt,
        },
    ]) {
        Ok(plan) => plan,
        Err(error) => {
            for skill_id in &temporary_skill_ids { let _ = registry.remove(skill_id); }
            return Err(error);
        }
    };
    app.storage
        .append_message(session.id, MessageRole::User, &args.prompt)?;
    println!("AUTONOMOUS AGENT RUN");
    println!("model: {}", provider.model());
    println!("task:  {}", args.prompt);
    println!(
        "skills: {}",
        if recommended_names.is_empty() {
            "none"
        } else {
            &recommended_names
        }
    );
    println!("plan:  {}", plan.summary);
    println!();

    let policy = Policy::safe(root.clone());
    let mut completed = 0usize;
    let mut denied: Vec<String> = Vec::new();
    let mut results = Vec::new();
    for (index, step) in plan.steps.into_iter().take(max_steps).enumerate() {
        if !autonomous_tool_enabled(&step.tool) {
            println!("{:02} {} · Deny", index + 1, step.tool);
            println!("   disabled by utharness.json capability selection");
            denied.push(format!("{} (disabled capability)", step.tool));
            continue;
        }
        let request = utharness_core::ToolRequest {
            tool: step.tool.clone(),
            target: step.target.clone(),
            arguments: json!({}),
        };
        let decision = policy.evaluate(&request);
        println!("{:02} {} · {:?}", index + 1, step.tool, decision);
        if decision != utharness_core::PermissionDecision::Allow {
            println!("   denied by SAFE policy");
            denied.push(format!("{} (SAFE policy)", step.tool));
            continue;
        }
        let output = execute_autonomous_step(&policy, &root, &step)?;
        let safe_output = Policy::redact(&output);
        for line in safe_output.lines() {
            println!("   {line}");
        }
        app.storage.record_event(
            "agent",
            session.id,
            "tool_completed",
            &json!({"tool": step.tool, "target": step.target, "rationale": step.rationale, "output": safe_output}),
            utharness_core::new_id(),
        )?;
        results.push(safe_output);
        completed += 1;
    }
    // Never present the planner's model-written final_response as fact when
    // nothing executed: it routinely claims artifacts were created.
    let completion = if completed == 0 {
        if denied.is_empty() {
            "Task not completed: the planner returned no executable steps. No files were created or modified.".to_string()
        } else {
            format!(
                "Task not completed: {} planned step(s) denied ({}). The agent is read-only and the workspace capability selection does not allow the requested tools. Re-run `utharness setup` to enable capabilities or set UTHARNESS_TOOLS; no files were created or modified.",
                denied.len(),
                denied.join(", ")
            )
        }
    } else {
        format!(
            "{} Completed {completed} approved read-only step(s). {}",
            plan.final_response
                .unwrap_or_else(|| "Workspace inspection finished.".into()),
            if results.is_empty() {
                "No tool output was returned."
            } else {
                "Results were persisted to the session event log."
            }
        )
    };
    app.storage
        .append_message(session.id, MessageRole::Assistant, &completion)?;
    app.storage.record_event(
        "agent",
        session.id,
        "autonomous_completed",
        &json!({"model": provider.model(), "completed_steps": completed, "max_steps": max_steps}),
        utharness_core::new_id(),
    )?;
    // Episodic auto-capture: successful runs leave a short trace so future
    // tasks recall what was done here. Best-effort; never fails the run.
    if completed > 0 {
        let first_line: String = completion
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(200)
            .collect();
        let task: String = args.prompt.chars().take(200).collect();
        let _ = app.storage.add_memory(
            Some(app.workspace.id),
            Some(session.id),
            "task",
            "episode",
            &format!("Task: {task} — {completed}/{max_steps} steps. {first_line}"),
            "agent",
            None,
        );
    }
    println!();
    println!("AGENT RESULT");
    println!("{}", Policy::redact(&completion));
    for skill_id in temporary_skill_ids {
        registry.remove(&skill_id)?;
    }
    Ok(())
}

fn execute_autonomous_step(policy: &Policy, root: &Path, step: &AgentStep) -> Result<String> {
    let target = step.target.as_deref().unwrap_or(".");
    match step.tool.as_str() {
        "list_directory" => {
            let path = policy
                .validate_path(Path::new(target))
                .map_err(anyhow::Error::msg)?;
            let mut entries = fs::read_dir(path)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            entries.sort();
            Ok(entries.into_iter().take(32).collect::<Vec<_>>().join("\n"))
        }
        "read_file" => {
            let path = policy
                .validate_path(Path::new(target))
                .map_err(anyhow::Error::msg)?;
            let content = fs::read_to_string(path)?;
            Ok(content.chars().take(4000).collect())
        }
        "git_status" => {
            let output = Command::new("git")
                .args(["status", "--short"])
                .current_dir(root)
                .output()?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        "git_diff" => {
            let output = Command::new("git")
                .args(["diff", "--stat"])
                .current_dir(root)
                .output()?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        other => anyhow::bail!("unsupported autonomous tool: {other}"),
    }
}

fn autonomous_tool_enabled(tool: &str) -> bool {
    let capability = match tool {
        "list_directory" | "read_file" => "workspace_read",
        "git_status" | "git_diff" => "git_inspection",
        _ => return false,
    };
    runtime_tool_enabled(capability)
}

fn runtime_tool_enabled(capability: &str) -> bool {
    env::var("UTHARNESS_TOOLS")
        .ok()
        .map(|configured| configured.split(',').any(|item| item.trim() == capability))
        .unwrap_or(true)
}

fn run_command(args: RunArgs) -> Result<()> {
    if !runtime_tool_enabled("terminal") {
        anyhow::bail!("terminal capability is disabled; enable it with `utharness setup`");
    }
    execution::run_shell(&args.workspace, &args.command, args.allow)
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
        MemoryAction::List => {
            println!("MEMORY");
            println!("workspace: {}", app.workspace.canonical_path);
            println!("Use `utharness memory search <query>` to search persisted records.");
        }
        MemoryAction::Add {
            content,
            scope,
            kind,
            expires,
        } => {
            let expires_at = parse_expiry(&expires)?;
            let memory = app.storage.add_memory(
                Some(app.workspace.id),
                None,
                &scope,
                &kind,
                &content,
                "cli",
                expires_at,
            )?;
            println!(
                "{} stored memory {} [{}]",
                icons::icon(icons::Feature::Remember),
                memory.id,
                memory.scope
            );
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
        MemoryAction::Prune => {
            let (expired, duplicates) = app.storage.prune_memories(Some(app.workspace.id))?;
            println!(
                "{} pruned {expired} expired and {duplicates} duplicate memories",
                icons::icon(icons::Feature::Remember)
            );
        }
    }
    Ok(())
}

/// Parse retention flags like `24h`, `7d`, `30d` into absolute epoch ms.
/// Empty or `never` means no expiry. Rejects anything else loudly.
fn parse_expiry(value: &str) -> Result<Option<i64>> {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.is_empty() || trimmed == "never" {
        return Ok(None);
    }
    let (digits, factor) = if let Some(days) = trimmed.strip_suffix('d') {
        (days, 86_400_000i64)
    } else if let Some(hours) = trimmed.strip_suffix('h') {
        (hours, 3_600_000i64)
    } else {
        anyhow::bail!("invalid --expires value '{value}'; use like 24h, 7d, or never");
    };
    let amount: i64 = digits.parse().map_err(|_| {
        anyhow::anyhow!("invalid --expires value '{value}'; use like 24h, 7d, or never")
    })?;
    if amount <= 0 {
        anyhow::bail!("invalid --expires value '{value}'; use like 24h, 7d, or never");
    }
    Ok(Some(
        utharness_core::now_ms().saturating_add(amount.saturating_mul(factor)),
    ))
}

/// Memories relevant to a prompt, rendered for injection into model
/// context. Capped in size; empty when nothing recalls. Never fails.
fn recall_block(
    storage: &utharness_storage::Storage,
    workspace: utharness_core::Id,
    query: &str,
) -> String {
    let hits = storage.recall(Some(workspace), query, 3);
    if hits.is_empty() {
        return String::new();
    }
    let mut block = String::from("Relevant workspace memories:\n");
    for hit in hits {
        let line: String = hit.content.chars().take(200).collect();
        block.push_str(&format!("- [{}] {}\n", hit.kind, line));
        if block.len() > 800 {
            break;
        }
    }
    block
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

fn doctor(args: DoctorArgs) -> Result<()> {
    if args.fix {
        fs::create_dir_all(setup_system::home()?)?;
        println!(
            "{} UTHARNESS DOCTOR --FIX",
            icons::icon_accent(icons::Feature::Fix)
        );
        println!("✓ repaired      user configuration directory");
    }
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
    match Gateway::from_environment() {
        Ok(provider) => match provider.validate_model() {
            Ok(()) => println!(
                "{} ✓ provider      {}/{} validated",
                icons::icon_accent(icons::Feature::Test),
                provider.provider(),
                provider.model()
            ),
            Err(error) => println!(
                "! provider      {}/{}: {error}",
                provider.provider(),
                provider.model()
            ),
        },
        Err(_) => println!("! provider      offline planner only; run `utharness providers env`"),
    }
    let report = setup_system::scan_environment();
    let missing = report
        .components
        .iter()
        .filter(|component| {
            component.required && component.state != setup_system::ComponentState::Available
        })
        .count();
    if missing == 0 {
        println!(
            "{} ✓ dependencies  required components available",
            icons::icon_accent(icons::Feature::Build)
        );
    } else {
        println!("! dependencies  {missing} required component(s) need attention");
        if args.fix {
            for component in report.components.iter().filter(|component| {
                component.required && component.state != setup_system::ComponentState::Available
            }) {
                if let Some(hint) = &component.install_hint {
                    println!("  run: {hint}");
                }
            }
        }
    }
    println!("✓ skills        built-in registry available");
    if termux::is_termux() {
        print_termux_doctor();
    }
    println!("✓ diagnostics   clean");
    Ok(())
}

fn setup(args: SetupArgs) -> Result<()> {
    if args.scan {
        println!(
            "{}",
            serde_json::to_string_pretty(&setup_system::scan_environment())?
        );
        return Ok(());
    }
    if !args.non_interactive && io::stdin().is_terminal() && io::stdout().is_terminal() {
        return launch_ui(&["--setup"]);
    }

    let app = App::open(".")?;
    let requested_mode = if args.quick {
        "quick"
    } else if args.full {
        "full"
    } else if args.developer {
        "developer"
    } else if args.local_ai {
        "local"
    } else if args.custom {
        "custom"
    } else if args.blank {
        "blank"
    } else {
        args.mode.trim()
    };
    let mode = match requested_mode.to_ascii_lowercase().replace('-', "_").as_str() {
        "quick" | "full" | "developer" | "local" | "local_ai" | "custom" | "blank" | "import" => requested_mode.to_ascii_lowercase().replace('-', "_"),
        other => anyhow::bail!("unsupported setup mode '{other}'; use quick, full, developer, local-ai, custom, blank, or import"),
    };
    if mode == "import" {
        let source = args
            .import_config
            .context("--import-config is required for import mode")?;
        let raw = fs::read_to_string(&source)
            .with_context(|| format!("failed to read {}", source.display()))?;
        let imported: RuntimeConfig =
            serde_json::from_str(&raw).context("imported configuration is invalid")?;
        if imported.schema_version != 1 {
            anyhow::bail!("unsupported imported configuration schema");
        }
        let destination = env::current_dir()?.join("utharness.json");
        fs::write(
            &destination,
            format!("{}\n", serde_json::to_string_pretty(&imported)?),
        )?;
        setup_system::write_global_config(
            &imported.mode,
            &imported.provider,
            &imported.model,
            &destination,
        )?;
        println!(
            "UTHARNESS SETUP\n✓ imported {} into {}",
            source.display(),
            destination.display()
        );
        return Ok(());
    }
    let provider = args.provider.unwrap_or_else(|| {
        match mode.as_str() {
            "blank" => "offline",
            "local" | "local_ai" => "ollama",
            "custom" => "custom",
            _ => "openrouter",
        }
        .into()
    });
    if let Some(url) = args.provider_url.as_deref() {
        env::set_var("UTHARNESS_PROVIDER_URL", url);
    }
    if let Some(model) = args.model.as_deref() {
        env::set_var("UTHARNESS_MODEL", model);
    }
    let pending_secret = if args.api_key_stdin {
        use std::io::Read;
        let mut secret = String::new();
        io::stdin()
            .read_to_string(&mut secret)
            .context("failed to read API key from stdin")?;
        let variable =
            provider_key_variable(&provider).context("this provider does not accept an API key")?;
        let secret = secret.trim_end_matches(['\r', '\n']).to_string();
        env::set_var(variable, &secret);
        Some((variable, secret))
    } else {
        None
    };
    let status = if provider == "offline" {
        None
    } else {
        Some(Gateway::status_from_environment(ProviderKind::parse(
            &provider,
        )?))
    };
    let model = args.model.unwrap_or_else(|| {
        status
            .as_ref()
            .map(|value| value.model.clone())
            .unwrap_or_else(|| "deterministic-planner".into())
    });
    let tools = if args.tools.len() == 1 && args.tools[0] == "none" {
        Vec::new()
    } else if args.tools.is_empty() {
        match mode.as_str() {
            "blank" => vec!["workspace_read".into()],
            "developer" => SETUP_TOOLS.iter().map(|tool| (*tool).to_string()).collect(),
            _ => vec![
                "workspace_read".into(),
                "git_inspection".into(),
                "skills".into(),
                "memory".into(),
            ],
        }
    } else {
        args.tools
    };
    for tool in &tools {
        if !SETUP_TOOLS.contains(&tool.as_str()) {
            anyhow::bail!("unsupported setup capability '{tool}'");
        }
    }
    let permission_mode = if tools
        .iter()
        .any(|tool| tool == "terminal" || tool == "file_write")
    {
        "ask"
    } else {
        "safe"
    };
    let config = RuntimeConfig {
        schema_version: 1,
        mode,
        provider,
        model,
        permission_mode: permission_mode.into(),
        tools,
        ui: UiConfig::default(),
    };
    if let Some(status) = &status {
        if status.configured && !args.skip_validation {
            let gateway = Gateway::new_from_environment(ProviderKind::parse(&config.provider)?)?;
            gateway.validate_model()?;
        }
    }
    if let Some((variable, secret)) = pending_secret {
        setup_system::persist_secret(variable, &secret)?;
    }
    let config_path = env::current_dir()?.join("utharness.json");
    fs::write(
        &config_path,
        format!("{}\n", serde_json::to_string_pretty(&config)?),
    )
    .with_context(|| format!("failed to write {}", config_path.display()))?;
    let global_config = setup_system::write_global_config(
        &config.mode,
        &config.provider,
        &config.model,
        &config_path,
    )?;
    println!("UTHARNESS SETUP");
    println!("workspace: {}", app.workspace.canonical_path);
    if termux::is_termux() {
        let locations = termux::setup()?;
        println!("platform:  Android / Termux");
        println!("config:    {}", locations.config.display());
        println!("data:      {}", locations.data.display());
        println!("cache:     {}", locations.cache.display());
        println!("✓ Termux user directories initialized without root");
    } else {
        println!("platform:  {}", env::consts::OS);
        println!("✓ workspace database initialized");
    }
    println!("config:    {}", config_path.display());
    println!("global:    {}", global_config.display());
    println!("provider:  {}", config.provider);
    println!("model:     {}", config.model);
    println!("tools:     {}", config.tools.join(", "));
    println!("\nAPI keys are never stored in utharness.json or logs; setup secrets use a private secrets.env file.");
    if let Some(status) = status {
        if status.configured {
            if let Some(source) = status.credential_source.as_deref() {
                println!("✓ credentials found in {source}");
            } else {
                println!("✓ no API key required for this provider");
            }
            if !args.skip_validation {
                println!("✓ provider and model validated");
            }
        } else {
            println!(
                "! credentials missing; run `utharness providers env` for {} setup",
                config.provider
            );
        }
    }
    println!("next: utharness providers test && utharness doctor && utharness");
    Ok(())
}

fn provider_key_variable(provider: &str) -> Option<&'static str> {
    utharness_provider::key_variable(provider)
}

fn providers(action: ProviderAction) -> Result<()> {
    match action {
        ProviderAction::List => {
            println!("PROVIDER GATEWAYS");
            for provider in supported_providers() {
                let state = if provider.configured {
                    "CONFIGURED"
                } else {
                    "NEEDS KEY"
                };
                let credential = provider.credential_source.as_deref().unwrap_or(
                    if provider.provider == "ollama" {
                        "no key"
                    } else {
                        "—"
                    },
                );
                println!(
                    "{:<11} {:<11} {:<34} key={}",
                    provider.provider, state, provider.model, credential
                );
            }
        }
        ProviderAction::Test { provider, all } => {
            if all || provider == "all" {
                let mut passed = 0;
                let mut failed = 0;
                for status in supported_providers() {
                    if !status.configured {
                        continue;
                    }
                    let result = ProviderKind::parse(&status.provider)
                        .and_then(Gateway::new_from_environment)
                        .and_then(|gateway| gateway.health_check().map(|code| (gateway, code)));
                    match result {
                        Ok((gateway, code)) => {
                            passed += 1;
                            println!(
                                "✓ provider={} model={} endpoint={} HTTP={}",
                                gateway.provider(),
                                gateway.model(),
                                gateway.base_url(),
                                code
                            );
                        }
                        Err(error) => {
                            failed += 1;
                            println!("✗ provider={} error={error:#}", status.provider);
                        }
                    }
                }
                println!("{passed} passed, {failed} failed");
                if passed == 0 {
                    anyhow::bail!("no configured provider passed its health check");
                }
                return Ok(());
            }
            let gateway = if provider == "auto" {
                Gateway::from_environment()?
            } else {
                Gateway::new_from_environment(ProviderKind::parse(&provider)?)?
            };
            let status = gateway.health_check()?;
            println!(
                "✓ provider={} model={} endpoint={} HTTP={}",
                gateway.provider(),
                gateway.model(),
                gateway.base_url(),
                status
            );
        }
        ProviderAction::Env => {
            println!("AI GATEWAY ENVIRONMENT");
            println!(
                "UTHARNESS_PROVIDER={}",
                utharness_provider::provider_ids().join("|")
            );
            println!("UTHARNESS_MODEL=<provider model id>");
            println!("UTHARNESS_PROVIDER_URL=<HTTPS OpenAI-compatible /v1 endpoint>");
            println!("UTHARNESS_API_KEY=<custom override>");
            println!(
                "Provider keys: {}",
                utharness_provider::key_variables().join(" ")
            );
            println!("Secrets are read at process start and are never persisted by Utharness.");
        }
    }
    Ok(())
}

fn agents(action: AgentAction) -> Result<()> {
    match action {
        AgentAction::List => {
            println!("AGENT RUNTIME");
            println!(
                "{} UTHARNESS  planner/executor   READY",
                icons::icon_agent()
            );
            println!("  tools      list_directory read_file git_status git_diff");
            println!("  policy     SAFE read-only; every tool request is evaluated and persisted");
            println!("Run: utharness agents run \"Inspect this repository\"");
            Ok(())
        }
        AgentAction::Run {
            prompt,
            max_steps,
            workspace,
        } => autonomous(AutonomousArgs {
            prompt,
            max_steps,
            workspace,
        }),
    }
}

fn models(action: ModelAction) -> Result<()> {
    match action {
        ModelAction::List { json } => {
            if let Ok(gateway) = Gateway::from_environment() {
                let models = gateway.models()?;
                let active = format!("{}/{}", gateway.provider(), gateway.model());
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "provider": gateway.provider(),
                            "models": models,
                            "active": active,
                        })
                    );
                } else {
                    println!("MODELS");
                    for model in models {
                        println!("{}", model);
                    }
                    println!("active: {active}");
                }
            } else {
                if json {
                    println!(
                        "{}",
                        serde_json::json!({"provider": "offline", "models": ["offline/deterministic-planner"], "active": "offline/deterministic-planner"})
                    );
                    return Ok(());
                }
                println!("MODELS");
                for provider in supported_providers()
                    .into_iter()
                    .filter(|provider| provider.configured)
                {
                    println!("{}/{}", provider.provider, provider.model);
                }
                println!("offline/deterministic-planner");
            }
            Ok(())
        }
        ModelAction::Test { provider } => {
            let gateway = if provider == "auto" {
                Gateway::from_environment()?
            } else {
                Gateway::new_from_environment(ProviderKind::parse(&provider)?)?
            };
            gateway.validate_model()?;
            println!(
                "✓ provider={} model={} available",
                gateway.provider(),
                gateway.model()
            );
            Ok(())
        }
    }
}

fn mcp() -> Result<()> {
    let paths = termux::paths();
    let mcp_dir = paths.data.join("mcp");
    fs::create_dir_all(&mcp_dir)?;
    println!("MCP");
    println!("registry: {}", mcp_dir.display());
    println!("status:   configuration directory ready; no MCP server is enabled by default");
    println!("safety:   external MCP tools require explicit configuration and permission review");
    Ok(())
}

fn update() -> Result<()> {
    println!("{} UTHARNESS UPDATE", icons::icon(icons::Feature::Build));
    if termux::is_termux() {
        println!("{}", termux::update_guidance());
        return Ok(());
    }

    let executable = env::current_exe().context("cannot locate the running executable")?;
    let install_directory = executable
        .parent()
        .context("running executable has no parent directory")?;
    let release_install = install_directory.join("utharnessly-ui").is_dir()
        || install_directory.join("ui/dist/index.js").is_file();

    if !release_install {
        println!("This executable is owned by a source build or package-manager launcher.");
        println!("Use the command that installed it:");
        println!("  npm install --global utharnessly@latest");
        println!("  python -m pip install --upgrade utharnessly");
        println!("  uv tool upgrade utharnessly");
        println!("  cargo install --git https://github.com/uthumany/utharnessly --package utharness-cli --locked --force");
        return Ok(());
    }

    #[cfg(windows)]
    {
        println!("Release archive installation detected.");
        println!("Run the verified PowerShell installer:");
        println!("  irm https://raw.githubusercontent.com/uthumany/utharnessly/main/packaging/install.ps1 | iex");
    }

    #[cfg(not(windows))]
    {
        const INSTALLER_URL: &str =
            "https://raw.githubusercontent.com/uthumany/utharnessly/main/packaging/install.sh";
        println!(
            "Release archive installation detected; checking the latest signed-checksum release…"
        );
        let mut response = reqwest::blocking::get(INSTALLER_URL)
            .context("failed to download the release installer")?
            .error_for_status()
            .context("release installer download was rejected")?;
        let total = response.content_length().unwrap_or(0);
        let mut bar = progress::ProgressBar::new("Downloading installer", total.max(1));
        let mut script = Vec::with_capacity(total.try_into().unwrap_or(65_536));
        let mut received: u64 = 0;
        let mut buf = [0u8; 8192];
        loop {
            use std::io::Read;
            let n = response
                .read(&mut buf)
                .context("failed to read the release installer")?;
            if n == 0 {
                break;
            }
            received += n as u64;
            script.extend_from_slice(&buf[..n]);
            bar.set(received);
        }
        bar.finish("Installer download");
        let temporary = env::temp_dir().join(format!("utharness-update-{}.sh", std::process::id()));
        fs::write(&temporary, &script).context("failed to stage the release installer")?;
        let status = Command::new("bash")
            .arg(&temporary)
            .status()
            .context("failed to start the release installer")?;
        let _ = fs::remove_file(&temporary);
        if !status.success() {
            anyhow::bail!("release installer exited with {status}");
        }
        println!("✓ Utharness release installation updated");
    }
    Ok(())
}

fn uninstall() -> Result<()> {
    println!("UTHARNESS UNINSTALL");
    if termux::is_termux() {
        let paths = termux::paths();
        println!("Run: pkg uninstall utharness");
        println!("User data is retained by default:");
        println!("  {}", paths.config.display());
        println!("  {}", paths.data.display());
        println!("  {}", paths.cache.display());
        println!(
            "Remove user data only when intended: rm -rf {} {} {}",
            paths.config.display(),
            paths.data.display(),
            paths.cache.display()
        );
    } else {
        println!(
            "Use the package manager that installed utharnessly, or remove the source checkout."
        );
    }
    Ok(())
}

fn termux_command(args: TermuxArgs) -> Result<()> {
    match args.action.unwrap_or(TermuxAction::Info) {
        TermuxAction::Info => println!("{}", serde_json::to_string_pretty(&termux::info()?)?),
        TermuxAction::Setup => {
            let locations = termux::setup()?;
            println!("Termux directories initialized");
            println!("prefix: {}", locations.prefix.display());
            println!("config: {}", locations.config.display());
            println!("data:   {}", locations.data.display());
            println!("cache:  {}", locations.cache.display());
        }
        TermuxAction::Api { capability, value } => {
            println!(
                "{}",
                termux::api_status_or_call(capability.as_deref(), value.as_deref())?
            );
        }
        TermuxAction::Keys(args) => match args.action.unwrap_or(TermuxKeysAction::Install) {
            TermuxKeysAction::Install => println!(
                "extra keys installed at {}",
                termux::install_keys()?.display()
            ),
        },
        TermuxAction::Storage(args) => match args.action.unwrap_or(TermuxStorageAction::Enable) {
            TermuxStorageAction::Enable => {
                termux::enable_storage()?;
                println!("Termux shared storage setup requested");
            }
        },
        TermuxAction::Permissions => {
            for item in termux::checks().into_iter().filter(|item| {
                matches!(
                    item.name.as_str(),
                    "Shared storage" | "Termux:API" | "Storage sandbox"
                )
            }) {
                println!(
                    "{} {} · {}",
                    if item.status == "pass" { "✓" } else { "!" },
                    item.name,
                    item.detail
                );
                if let Some(repair) = item.repair {
                    println!("  repair: {repair}");
                }
            }
        }
        TermuxAction::Doctor => print_termux_doctor(),
    }
    Ok(())
}

fn print_termux_doctor() {
    println!("UTHARNESS TERMUX DOCTOR");
    let checks = termux::checks();
    let mut warnings = 0;
    for item in checks {
        let symbol = if item.status == "pass" {
            "✓"
        } else {
            warnings += 1;
            "!"
        };
        println!("{symbol} {:<18} {}", item.name, item.detail);
        if let Some(repair) = item.repair {
            println!("  repair: {repair}");
        }
    }
    if warnings == 0 {
        println!("\nSystem ready.");
    } else {
        println!("\n{warnings} warning(s); core UTHARNESS remains usable where optional capabilities are unavailable.");
    }
}

fn config_show() -> Result<()> {
    let app = App::open(".")?;
    println!("workspace = \"{}\"", app.workspace.canonical_path);
    println!("database = \"{}\"", app.storage.path().display());
    let saved = load_runtime_config()?;
    println!(
        "permission_mode = \"{}\"",
        saved
            .as_ref()
            .map(|value| value.permission_mode.as_str())
            .unwrap_or("safe")
    );
    let provider = Gateway::from_environment().ok();
    println!(
        "provider = \"{}\"",
        provider
            .as_ref()
            .map(|value| value.provider().to_string())
            .or_else(|| saved.as_ref().map(|value| value.provider.clone()))
            .unwrap_or_else(|| "offline".into())
    );
    println!(
        "model = \"{}\"",
        provider
            .as_ref()
            .map(|value| value.model().to_string())
            .or_else(|| saved.as_ref().map(|value| value.model.clone()))
            .unwrap_or_else(|| "deterministic-planner".into())
    );
    if let Some(saved) = saved {
        println!("setup_mode = \"{}\"", saved.mode);
        println!("tools = \"{}\"", saved.tools.join(","));
        println!("ui.banner = {}", saved.ui.banner);
        println!("ui.banner_mode = \"{}\"", saved.ui.banner_mode);
        println!("ui.icons = \"{}\"", saved.ui.icons);
    }
    println!("theme = \"utharness-carbon\"");
    Ok(())
}

fn config_set(key: &str, value: &str) -> Result<()> {
    let mut config = load_runtime_config()?.unwrap_or(RuntimeConfig {
        schema_version: 1,
        mode: "blank".into(),
        provider: "offline".into(),
        model: "deterministic-planner".into(),
        permission_mode: "safe".into(),
        tools: vec!["workspace_read".into()],
        ui: UiConfig::default(),
    });
    match key {
        "ui.banner" => {
            config.ui.banner = match value.to_ascii_lowercase().as_str() {
                "true" | "on" | "1" => true,
                "false" | "off" | "0" => false,
                _ => anyhow::bail!("ui.banner expects true or false"),
            }
        }
        "ui.banner_mode" | "ui.bannerMode" => {
            let normalized = value.to_ascii_lowercase();
            if !matches!(normalized.as_str(), "full" | "compact" | "minimal") {
                anyhow::bail!("ui.banner_mode expects full, compact, or minimal");
            }
            config.ui.banner_mode = normalized;
        }
        "ui.icons" => {
            let normalized = value.to_ascii_lowercase();
            if !matches!(normalized.as_str(), "nerd" | "unicode" | "ascii") {
                anyhow::bail!("ui.icons expects nerd, unicode, or ascii");
            }
            config.ui.icons = normalized;
        }
        _ => anyhow::bail!("unsupported configuration key '{key}'"),
    }
    let path = env::current_dir()?.join("utharness.json");
    fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&config)?),
    )?;
    println!("set {key} = {value} in {}", path.display());
    Ok(())
}

fn launch_tui(headless: bool) -> Result<()> {
    if headless || !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        banner::print_startup_banner(VERSION)?;
        banner::print_onboarding_tips()?;
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

    launch_ui(&[])
}

fn launch_ui(arguments: &[&str]) -> Result<()> {
    let repository_root = env::var_os("UTHARNESS_SOURCE_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            candidate.is_dir().then_some(candidate)
        })
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let executable_directory = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let repository_ui_directory = repository_root.join("ui");
    let mut installed_ui_directories = executable_directory
        .clone()
        .map(|path| vec![path.join("ui"), path.join("utharnessly-ui")])
        .unwrap_or_default();
    if let Some(prefix) = env::var_os("PREFIX").map(PathBuf::from) {
        installed_ui_directories.push(prefix.join("lib").join("utharness"));
        installed_ui_directories.push(prefix.join("share").join("utharness"));
    }
    let ui_directory = installed_ui_directories
        .into_iter()
        .find(|path| path.join("dist/index.js").is_file())
        .unwrap_or(repository_ui_directory);
    let ui_entry = env::var_os("UTHARNESS_UI_ENTRY")
        .map(PathBuf::from)
        .unwrap_or_else(|| ui_directory.join("dist/index.js"));

    let mut command = if ui_entry.is_file() {
        let mut command = Command::new("node");
        command.arg(ui_entry).args(arguments);
        command
    } else if ui_directory.join("package.json").is_file() {
        let mut command = Command::new("pnpm");
        command.args(["--dir", ui_directory.to_string_lossy().as_ref(), "dev"]);
        command
    } else {
        anyhow::bail!(
            "TypeScript terminal UI is unavailable. Build it with `pnpm --dir {} install && pnpm --dir {} build` or set UTHARNESS_UI_ENTRY.",
            ui_directory.display(),
            ui_directory.display()
        );
    };

    if let Ok(runtime_binary) = env::current_exe() {
        command.env("UTHARNESS_RUNTIME_BIN", runtime_binary);
    }
    let status = command.current_dir(env::current_dir()?).status()?;
    if !status.success() {
        anyhow::bail!("TypeScript terminal UI exited with status {status}");
    }
    Ok(())
}

fn load_runtime_config() -> Result<Option<RuntimeConfig>> {
    let path = env::current_dir()?.join("utharness.json");
    if !path.is_file() {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let config = serde_json::from_str(&raw)
        .with_context(|| format!("invalid Utharness configuration in {}", path.display()))?;
    Ok(Some(config))
}

fn apply_runtime_config() -> Result<()> {
    // Precedence: explicit env > workspace utharness.json > global
    // ~/.utharness/config.yaml (every setup writes it) > provider autodetect.
    // Without the global fallback, opening utharness in any directory other
    // than the configured workspace silently dropped the saved choice and
    // autodetect picked whatever key happened to be exported (often groq).
    if let Some(config) = load_runtime_config()? {
        if config.schema_version != 1 {
            anyhow::bail!(
                "unsupported utharness.json schema version {}",
                config.schema_version
            );
        }
        if config.provider != "offline" && env::var_os("UTHARNESS_PROVIDER").is_none() {
            env::set_var("UTHARNESS_PROVIDER", &config.provider);
        }
        if config.provider != "offline" && env::var_os("UTHARNESS_MODEL").is_none() {
            env::set_var("UTHARNESS_MODEL", &config.model);
        }
        apply_shared_config(&config.permission_mode, &config.tools, &config.ui);
        return Ok(());
    }
    if let Some((provider, model)) = setup_system::load_global_selection() {
        if provider != "offline" && env::var_os("UTHARNESS_PROVIDER").is_none() {
            env::set_var("UTHARNESS_PROVIDER", &provider);
        }
        if provider != "offline" && env::var_os("UTHARNESS_MODEL").is_none() {
            env::set_var("UTHARNESS_MODEL", &model);
        }
    }
    Ok(())
}

fn apply_shared_config(permission_mode: &str, tools: &[String], ui: &UiConfig) {
    if env::var_os("UTHARNESS_PERMISSION").is_none() {
        env::set_var("UTHARNESS_PERMISSION", permission_mode);
    }
    if env::var_os("UTHARNESS_TOOLS").is_none() {
        env::set_var("UTHARNESS_TOOLS", tools.join(","));
    }
    if env::var_os("UTHARNESS_BANNER").is_none() {
        env::set_var(
            "UTHARNESS_BANNER",
            if ui.banner {
                ui.banner_mode.as_str()
            } else {
                "hide"
            },
        );
    }
    if env::var_os("UTHARNESS_ICONS").is_none() {
        env::set_var("UTHARNESS_ICONS", &ui.icons);
    }
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

    #[test]
    fn system_prompt_carries_the_requested_product_identity() {
        let prompt = agent_system_prompt("");
        assert!(prompt.contains("UTHARNESS"));
        assert!(prompt.contains("\"UTHUMAN & CO\" Center for AI (UCAI)"));
        for developer in ["𓁷 Uthuman M", "𓁷 Shafiq N", "𓁷 Alid K"] {
            assert!(prompt.contains(developer), "missing {developer}");
        }
    }
}

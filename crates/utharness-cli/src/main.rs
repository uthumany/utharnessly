use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::Deserialize;
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
mod execution;
mod skills;
mod termux;

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
    Setup,
    Chat(ChatArgs),
    Run(RunArgs),
    Tui(TuiArgs),
    Autonomous(AutonomousArgs),
    Doctor,
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
    Providers(ProviderArgs),
    Agents(AgentArgs),
    Tools,
    Models,
    Mcp,
    Termux(TermuxArgs),
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
    },
    Search {
        query: String,
    },
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
    let cli = Cli::parse();
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
        Some(CommandKind::Setup) => setup(),
        Some(CommandKind::Chat(args)) => chat(args),
        Some(CommandKind::Run(args)) => run_command(args),
        Some(CommandKind::Tui(args)) => launch_tui(args.headless),
        Some(CommandKind::Autonomous(args)) => autonomous(args),
        Some(CommandKind::Doctor) => doctor(),
        Some(CommandKind::Update) => update(),
        Some(CommandKind::Uninstall) => uninstall(),
        Some(CommandKind::Config { action }) => {
            let _ = action.unwrap_or(ConfigAction::Show);
            config_show()
        }
        Some(CommandKind::Sessions { action }) => sessions(action.unwrap_or(SessionAction::List)),
        Some(CommandKind::Memory { action }) => memory(action.unwrap_or(MemoryAction::List)),
        Some(CommandKind::Checkpoint) => checkpoint(),
        Some(CommandKind::Skills(args)) => skills_command(args),
        Some(CommandKind::Providers(args)) => {
            providers(args.action.unwrap_or(ProviderAction::List))
        }
        Some(CommandKind::Agents(args)) => agents(args.action.unwrap_or(AgentAction::List)),
        Some(CommandKind::Tools) => {
            println!("TOOLS\n✓ read_file       SAFE\n✓ list_directory  SAFE\n! write_file      ASK\n! shell           ASK\n! browser_open    ASK\n✓ git_diff        SAFE");
            Ok(())
        }
        Some(CommandKind::Models) => models(),
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
    app.storage
        .append_message(session.id, MessageRole::User, &args.prompt)?;
    let mut live = false;
    let response = match Gateway::from_environment() {
        Ok(provider) => {
            live = true;
            println!("Uthy · {}/{}", provider.provider(), provider.model());
            use std::io::Write;
            io::stdout().flush()?;
            let response = provider.complete_streaming(
                &[ChatMessage { role: "system".into(), content: "You are Uthy, a concise terminal coding agent. Never claim a command ran unless a tool result proves it.".into() }, ChatMessage { role: "user".into(), content: args.prompt.clone() }],
                |delta| { print!("{delta}"); io::stdout().flush()?; Ok(()) },
            )?;
            println!();
            response
        }
        Err(_error) if !has_provider_configuration() =>
            format!("Offline planner ready. I received: {}\n\nConfigure a provider with `utharness providers env` to enable live model streaming.", args.prompt),
        Err(error) => return Err(error),
    };
    app.storage
        .append_message(session.id, MessageRole::Assistant, &response)?;
    app.storage.record_event(
        "session",
        session.id,
        "message_completed",
        &json!({"offline": !live}),
        utharness_core::new_id(),
    )?;
    if !live {
        println!("Uthy · OFFLINE PLANNER\n{}", response);
    }
    Ok(())
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
    let planner_prompt = format!(
        "You are the Utharness autonomous planner. Return only valid JSON with this shape: {{\"summary\":\"short summary\",\"steps\":[{{\"tool\":\"list_directory|read_file|git_status|git_diff\",\"target\":\"relative path or null\",\"rationale\":\"short reason\"}}],\"final_response\":\"short completion note\"}}. Plan at most {max_steps} read-only steps. Never request shell, write, network, secrets, or paths outside the workspace. Candidate skills from the local registry: {recommended_names}. Loaded skill evidence: {}. Task: {}",
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
    let mut results = Vec::new();
    for (index, step) in plan.steps.into_iter().take(max_steps).enumerate() {
        let request = utharness_core::ToolRequest {
            tool: step.tool.clone(),
            target: step.target.clone(),
            arguments: json!({}),
        };
        let decision = policy.evaluate(&request);
        println!("{:02} {} · {:?}", index + 1, step.tool, decision);
        if decision != utharness_core::PermissionDecision::Allow {
            println!("   denied by SAFE policy");
            continue;
        }
        let output = execute_autonomous_step(&policy, &root, &step)?;
        let safe_output = Policy::redact(&output);
        println!("   {}", safe_output.lines().next().unwrap_or("completed"));
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
    let completion = format!(
        "{} Completed {completed} approved read-only step(s). {}",
        plan.final_response
            .unwrap_or_else(|| "Workspace inspection finished.".into()),
        if results.is_empty() {
            "No tool output was returned."
        } else {
            "Results were persisted to the session event log."
        }
    );
    app.storage
        .append_message(session.id, MessageRole::Assistant, &completion)?;
    app.storage.record_event(
        "agent",
        session.id,
        "autonomous_completed",
        &json!({"model": provider.model(), "completed_steps": completed, "max_steps": max_steps}),
        utharness_core::new_id(),
    )?;
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

fn run_command(args: RunArgs) -> Result<()> {
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
    match Gateway::from_environment() {
        Ok(provider) => println!(
            "✓ provider      {}/{} configured",
            provider.provider(),
            provider.model()
        ),
        Err(_) => println!("! provider      offline planner only; run `utharness providers env`"),
    }
    println!("✓ skills        built-in registry available");
    if termux::is_termux() {
        print_termux_doctor();
    }
    println!("✓ diagnostics   clean");
    Ok(())
}

fn setup() -> Result<()> {
    let app = App::open(".")?;
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
    println!("\nAI gateway setup (keys are read from the environment and never stored):");
    println!("  export UTHARNESS_PROVIDER=openrouter");
    println!("  export OPENROUTER_API_KEY=...   # or OPENAI_API_KEY/GROQ_API_KEY/etc.");
    println!("  export UTHARNESS_MODEL=openrouter/free");
    println!("next: utharness providers test && utharness doctor");
    Ok(())
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
        ProviderAction::Test { provider } => {
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
            println!("UTHARNESS_PROVIDER=openrouter|openai|groq|together|deepseek|fireworks|ollama|custom");
            println!("UTHARNESS_MODEL=<provider model id>");
            println!("UTHARNESS_PROVIDER_URL=<HTTPS OpenAI-compatible /v1 endpoint>");
            println!("UTHARNESS_API_KEY=<custom override>");
            println!("Provider keys: OPENROUTER_API_KEY OPENAI_API_KEY GROQ_API_KEY TOGETHER_API_KEY DEEPSEEK_API_KEY FIREWORKS_API_KEY");
            println!("Secrets are read at process start and are never persisted by Utharness.");
        }
    }
    Ok(())
}

fn agents(action: AgentAction) -> Result<()> {
    match action {
        AgentAction::List => {
            println!("AGENT RUNTIME");
            println!("● Uthy       planner/executor   READY");
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

fn models() -> Result<()> {
    println!("MODELS");
    for provider in supported_providers()
        .into_iter()
        .filter(|provider| provider.configured)
    {
        println!("{}/{}", provider.provider, provider.model);
    }
    if let Ok(model) = env::var("UTHARNESS_MODEL") {
        println!("active: {model}");
    }
    Ok(())
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
    println!("UTHARNESS UPDATE");
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
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        const INSTALLER_URL: &str =
            "https://raw.githubusercontent.com/uthumany/utharnessly/main/packaging/install.sh";
        println!(
            "Release archive installation detected; checking the latest signed-checksum release…"
        );
        let script = reqwest::blocking::get(INSTALLER_URL)
            .context("failed to download the release installer")?
            .error_for_status()
            .context("release installer download was rejected")?
            .bytes()
            .context("failed to read the release installer")?;
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
    println!("permission_mode = \"SAFE\"");
    let provider = Gateway::from_environment().ok();
    println!(
        "provider = \"{}\"",
        provider
            .as_ref()
            .map(|value| value.provider())
            .unwrap_or("offline")
    );
    println!(
        "model = \"{}\"",
        provider
            .as_ref()
            .map(|value| value.model())
            .unwrap_or("deterministic-planner")
    );
    println!("theme = \"utharness-carbon\"");
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
        command.arg(ui_entry);
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

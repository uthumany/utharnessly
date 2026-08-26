# Utharness Agent Terminal — Native Runtime

Utharness is a local-first autonomous AI agent terminal. This repository is the native Rust runtime foundation for the product: a persistent SQLite-backed workspace, a safe tool boundary, a scriptable CLI, and a Ratatui terminal shell. It is a standalone product and is not UTHARNESS OS.

> **Current milestone:** a working offline-first backend that can initialize a workspace, persist sessions and messages, search memory with SQLite FTS5, create checkpoints, run explicitly approved shell commands, expose diagnostics, and open a persistent terminal UI.

## Quick start

```bash
cargo build --release
cargo test --workspace

# Initialize the current repository
./target/release/utharness init

# Create a persisted session and send an offline planning request
./target/release/utharness sessions new "Repository review"
./target/release/utharness chat "Inspect this repository and propose a safe plan"

# Persist and search project memory
./target/release/utharness memory add "Shell writes require approval in ASK mode"
./target/release/utharness memory search "approval"

# Run diagnostics and inspect configuration
./target/release/utharness doctor
./target/release/utharness config show

# Open the full-screen TUI from an interactive terminal
./target/release/utharness tui

# Run a bounded OpenRouter-backed read-only autonomous inspection
export OPENROUTER_API_KEY="..."
export UTHARNESS_MODEL="openrouter/free"
./target/release/utharness autonomous "Inspect this workspace and report its Git status" --max-steps 3
```

The default storage location is `~/.local/share/utharness/utharness.db`. Set `UTHARNESS_HOME` or `UTHARNESS_DB` to use an isolated location for tests, CI, or portable deployments.

## Working capabilities

| Capability | Implementation |
| --- | --- |
| Native CLI | Clap commands for `init`, `chat`, `run`, `tui`, `autonomous`, `doctor`, `config`, `sessions`, `memory`, `checkpoint`, `skills`, `providers`, `agents`, and `tools`. |
| SQLite persistence | Bundled SQLite with foreign keys, WAL mode, migration SQL, sessions, messages, tasks, checkpoints, events, memories, FTS5, tool calls, permissions, and audit tables. |
| Offline chat | Deterministic offline planner response that persists both user and assistant messages without credentials. |
| OpenRouter autonomy | OpenAI-compatible OpenRouter client that asks for a JSON plan, caps steps, executes only SAFE read-only tools, redacts output, and persists agent events. |
| Memory | Workspace-scoped records and FTS5 search with triggers keeping the index synchronized. |
| Safety | SAFE default, explicit `--allow` for shell execution, workspace-scoped execution, destructive-command denylist, and secret redaction. |
| TUI | Ratatui/Crossterm alternate-screen interface with navigation, chat, task inspector, context meter, and keyboard exit handling. |
| Diagnostics | Database integrity, workspace, storage, shell, provider, permissions, skills, and clean-runtime checks. |
| Tests | Unit tests for domain and security rules, SQLite persistence tests, and a process-level CLI end-to-end test. |

## CLI commands

```text
utharness                         Open the TUI when attached to a terminal
utharness init [--workspace PATH] Initialize a local workspace
utharness chat PROMPT             Persist a prompt and offline planner response
utharness run --command CMD       Refuse shell execution unless --allow is supplied
utharness tui [--headless]        Open TUI or print a non-interactive status
utharness autonomous PROMPT        Plan and execute bounded SAFE read-only tools through OpenRouter
utharness doctor                  Run actionable local diagnostics
utharness config show             Print effective local configuration
utharness sessions list           List persisted sessions
utharness sessions new TITLE      Create a session
utharness memory add CONTENT      Store workspace memory
utharness memory search QUERY     Search indexed memory
utharness checkpoint              Create a session checkpoint
utharness skills                  List built-in skills
utharness providers               List provider routes
utharness agents                  List agent roles
utharness tools                   List registered tools and policy modes
```

Shell execution is intentionally opt-in:

```bash
utharness run --command "cargo test"          # denied in SAFE mode
utharness run --command "cargo test" --allow   # explicit approval required
```

## Architecture

The workspace is divided into four focused crates:

| Crate | Responsibility |
| --- | --- |
| `utharness-core` | Shared IDs, domain records, state machines, permission types, provider metadata, and diagnostic models. |
| `utharness-storage` | SQLite connection policy, embedded migrations, repositories, FTS5 search, and persistence tests. |
| `utharness-security` | Permission modes, workspace path validation, shell policy, and secret redaction. |
| `utharness-cli` | Clap commands, offline agent behavior, bounded autonomous execution, tool execution, diagnostics, and Ratatui TUI composition. |
| `utharness-provider` | OpenRouter/OpenAI-compatible HTTP client with typed JSON plan responses and timeout/error handling. |

The next backend milestones are provider adapters and streaming, task-graph persistence and leases, PTY/process supervision, Git and file tools, scheduler execution, skill loading, MCP, and the loopback Axum API for the browser control plane.

## TUI workspace

Focus Mode is now the default native TUI. It contains only the UTHY identity/header, conversation history, active task or warning cards, the fixed message composer, and the bottom status bar. Navigation, model selection, files, agents, tasks, memory, and logs are available as keyboard-triggered overlays instead of permanent panels. `Ctrl+B` toggles the original multi-pane layout as optional Workspace Mode.

Conversation history renders agent activity inline. Completed PLAN, SHELL, FILE, EDIT, DIFF, GIT, BROWSER, AGENT, MEMORY, SKILL, MCP, TEST, ERROR, and permission cards stay compact; the active operation expands with a marker, progress, and details. The message composer remains the strongest interactive element with a cyan focus border, placeholder `Type your message or @path/to/file`, multiline `Shift+Enter`, `Enter` to send, and context references for `@file`, `@folder`, `@agent`, `@skill`, and `@memory`.

The Focus Mode shortcut map is `Ctrl+K` command palette, `Ctrl+P` model picker, `Ctrl+O` file picker, `Ctrl+G` agent manager, `Ctrl+T` task inspector, `Ctrl+M` memory, `Ctrl+L` logs, `Ctrl+B` Workspace Mode, and `F1` help. `Tab` moves between chat and composer, `PageUp`/`PageDown` and arrow keys scroll conversation history, `Esc` closes overlays, and `Ctrl+C` exits. The bottom status bar shows workspace, permission mode, provider/model, Git branch, and context remaining. Narrow terminals collapse to a single chat surface while preserving the composer and essential status controls.

Semantic colors remain restrained: cyan marks focus, yellow marks warnings and progress, green marks success, red marks errors, purple marks agents, blue marks tools, and gray carries secondary information.

## Startup banner

Every `utharness` startup begins with the large **UTHY** block-letter banner before the setup or terminal workspace begins. The premium startup keeps the logo upper-left, adds layered block/shadow depth, and uses a gold-to-amber-to-coral ANSI gradient when color is available. It then shows four numbered getting-started tips: asking questions/editing files/executing commands, using `@file` for project context, creating `UTHARNESS.md` for project instructions, and using `/help` to explore commands. `utharness init` and `utharness tui` use the same startup path. Wide terminals render the Unicode-safe UTHY wordmark; medium terminals use a smaller ASCII-safe fallback; terminals below 42 columns switch to a compact layout. The banner stays responsive to terminal width and includes the current package version.

The banner paints immediately by default so the CLI identity appears without startup delay. Set `UTHARNESS_BANNER_ANIMATION=typein` to opt into the restrained line-by-line reveal, and `UTHARNESS_STARTUP_SPLASH_MS` to tune the short onboarding pause. ANSI theme colors are used when attached to a terminal. Set `UTHARNESS_THEME=midnight-cyan`, `UTHARNESS_THEME=ember`, or `UTHARNESS_THEME=mono-black` to select a built-in palette. Set `UTHARNESS_ASCII=1` or `NO_COLOR=1` to force ASCII/no-color output. Set `UTHARNESS_REDUCED_MOTION=1` or `REDUCED_MOTION=1` to disable animation. When the persistent TUI starts outside a Git project, it shows a single yellow setup card: “You are running Utharness outside a project workspace. Open a project directory for repository-aware agent features.”

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release
```

The repository intentionally has no committed credentials. Provider secrets should be supplied through the OS keyring or environment variables and must never be persisted in logs, checkpoints, or prompts. The autonomous command is deliberately bounded: it accepts a model-generated JSON plan but executes only the SAFE allowlist (`list_directory`, `read_file`, `git_status`, and `git_diff`), limits the number of steps, scopes paths to the workspace, and records redacted results.

## License

MIT. See [`LICENSE`](./LICENSE).

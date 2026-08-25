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
```

The default storage location is `~/.local/share/utharness/utharness.db`. Set `UTHARNESS_HOME` or `UTHARNESS_DB` to use an isolated location for tests, CI, or portable deployments.

## Working capabilities

| Capability | Implementation |
| --- | --- |
| Native CLI | Clap commands for `init`, `chat`, `run`, `tui`, `doctor`, `config`, `sessions`, `memory`, `checkpoint`, `skills`, `providers`, `agents`, and `tools`. |
| SQLite persistence | Bundled SQLite with foreign keys, WAL mode, migration SQL, sessions, messages, tasks, checkpoints, events, memories, FTS5, tool calls, permissions, and audit tables. |
| Offline chat | Deterministic offline planner response that persists both user and assistant messages without credentials. |
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
| `utharness-cli` | Clap commands, offline agent behavior, tool execution, diagnostics, and Ratatui TUI composition. |

The next backend milestones are provider adapters and streaming, task-graph persistence and leases, PTY/process supervision, Git and file tools, scheduler execution, skill loading, MCP, and the loopback Axum API for the browser control plane.

## TUI workspace

The native TUI now uses the full available viewport instead of leaving the right side empty. Wide terminals use a `16% / 60% / 24%` navigation, chat, and inspector split. Medium terminals promote chat and inspector to a `66% / 34%` layout, while tiny terminals collapse to a single chat surface. The inspector combines Task, Context, Agents, Tools, and Git tabs, and the composer has its own dedicated bordered region.

The keyboard model keeps focus visible. `Tab` moves between navigation, chat, and inspector; `Ctrl+B` collapses navigation; `Ctrl+1` through `Ctrl+5` jump inspector tabs; `h` and `l` switch inspector tabs when the inspector is focused; `Enter` sends the draft; `Esc` clears it; and `q` or `Ctrl+C` exits. The footer changes its hints with the active focus region.

## Startup banner

Every `utharness` startup begins with the large **UTHY** block-letter banner before the setup or terminal workspace begins. `utharness init` and `utharness tui` use the same startup path. Wide terminals render the Unicode-safe UTHY wordmark; medium terminals use a smaller ASCII-safe UTHY fallback; terminals below 42 columns switch to a compact three-line layout. The banner centers itself from the detected `COLUMNS` value or terminal size and includes the current package version.

The banner paints immediately by default so the CLI identity appears without startup delay. Set `UTHARNESS_BANNER_ANIMATION=typein` to opt into the restrained type-in effect. ANSI theme colors are used when attached to a terminal. Set `UTHARNESS_THEME=midnight-cyan`, `UTHARNESS_THEME=ember`, or `UTHARNESS_THEME=mono-black` to select a built-in palette. Set `UTHARNESS_ASCII=1` or `NO_COLOR=1` to force ASCII/no-color output. Set `UTHARNESS_REDUCED_MOTION=1` or `REDUCED_MOTION=1` to disable animation.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release
```

The repository intentionally has no committed credentials. Provider secrets should be added later through the OS keyring or environment variables and must never be persisted in logs, checkpoints, or prompts.

## License

MIT. See [`LICENSE`](./LICENSE).

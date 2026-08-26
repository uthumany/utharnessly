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

# Build and open the full-screen TypeScript/Ink TUI from an interactive terminal
pnpm --dir ui install
pnpm --dir ui build
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
| `utharness-cli` | Clap commands, offline agent behavior, bounded autonomous execution, tool execution, diagnostics, and the Rust-to-Ink launcher bridge. |
| `utharness-provider` | OpenRouter/OpenAI-compatible HTTP client with typed JSON plan responses and timeout/error handling. |

The next backend milestones are provider adapters and streaming, task-graph persistence and leases, PTY/process supervision, Git and file tools, scheduler execution, skill loading, MCP, and the loopback Axum API for the browser control plane.

## Reference-matched TypeScript terminal UI

The interactive terminal has been rebuilt rather than incrementally patched. The Rust command now launches `ui/dist/index.js` through Node 22, with a `pnpm --dir ui dev` fallback for source checkouts that have not built the bundle. The UI is a full-screen persistent React/Ink application using `@inkjs/ui`, `chalk`, `gradient-string`, `cli-spinners`, `figures`, `string-width`, `wrap-ansi`, `execa`, `chokidar`, and `zod`.

Every display uses one left-aligned content grid. The header, ASCII branding, tips, workspace warning, prompt, and status bar are fixed; only the conversation viewport is height-constrained and scrollable. The header presents `UTHARNESS AGENT — focus mode` and `Ctrl+K`/help affordances. The persistent banner uses wide, medium, and compact ASCII variants, retains the word `UTHARNESS` on every display, and applies the requested amber-to-orange-to-coral gradient in truecolor terminals. The prompt is a cyan rounded border with `Type your message or @path/to/file`, slash command suggestions, `@file`, `@folder`, `@url`, `@agent`, `@skill`, and `@memory` context suggestions, and command history navigation.

The conversation is composed of UTHY and YOU rows with timestamps, streaming token updates, running/completed tool cards, success/error/approval states, result summaries, and responsive wrapping. `Ctrl+K` opens the command palette; `PageUp`/`PageDown` scroll the conversation; arrow keys recall command history; mouse-wheel escape sequences adjust the viewport; and SIGWINCH triggers a redraw on resize. Runtime metadata is loaded through the native CLI boundary and filesystem/Git state is watched with Chokidar. If the native binary is available, prompt submissions invoke its persisted offline chat path through `execa`; otherwise the UI uses a bounded offline planner response.

The fixed terminal breakpoints are `40–59` compact, `60–79` narrow, `80–119` standard, `120–199` wide, and `200+` ultra-wide. The compact layout preserves the explicit UTHARNESS identity, reduced prompt copy, and abbreviated status. TrueColor, ANSI 256, ANSI 16, and monochrome/no-color environments are selected from `COLORTERM`, `TERM`, `UTHARNESS_COLOR`, `UTHARNESS_ASCII`, and `NO_COLOR`. The implementation relies on standard ANSI input and SIGWINCH behavior and is designed for Linux, macOS, Windows Terminal, WSL, SSH, and tmux; each host’s terminal emulator still controls the final glyph metrics.

## UI development and screenshots

```bash
cd ui
pnpm install
pnpm typecheck
pnpm test
pnpm build
pnpm screenshots
```

The PTY matrix exercises `40x18`, `60x20`, `80x24`, `120x36`, `160x40`, and `220x44`, plus a command-palette capture. Generated PNGs are kept under `ui/screenshots/` during development and are not required at runtime. The Rust bridge can be verified with `cargo build --release` followed by `./target/release/utharness tui` from an interactive terminal. Set `UTHARNESS_UI_ENTRY` to point the Rust launcher at a custom compiled UI bundle.

The UI is distributed as a normal Node package and supports the common command runners: `npm install && npm run build`, `npx tsx ui/src/index.tsx`, `pnpm --dir ui install && pnpm --dir ui build`, `yarn --cwd ui install && yarn --cwd ui build`, and `bun --cwd ui install && bun --cwd ui run build`. Deno can execute the source entrypoint with its Node-compatibility mode when dependencies are available; `uv` and `pipx` remain supported for surrounding Python-based PTY/automation workflows. Git users can clone the private repository, build the Rust binary, build `ui/dist/index.js`, and run `utharness tui` from a workspace.

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

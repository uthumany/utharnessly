# Changelog

## Unreleased

### Added

- Instant startup banner with large UTHY block lettering, ANSI themes, Unicode/ASCII fallback, compact narrow-terminal mode, version display, and opt-in type-in animation.
- Native Rust workspace with core, storage, security, and CLI crates.
- SQLite WAL persistence with embedded initial migration and FTS5 memory search.
- Persisted workspaces, sessions, messages, tasks, checkpoints, runtime events, tool calls, permission decisions, and audit records.
- SAFE and explicit approval shell execution with workspace scoping, denylist checks, and output redaction.
- Offline-first chat path that works without provider credentials.
- Clap CLI commands for initialization, chat, shell execution, TUI status, sessions, memory, checkpoints, skills, providers, agents, tools, configuration, and diagnostics.
- Ratatui/Crossterm interactive terminal shell.
- Unit, persistence, security, and process-level CLI integration tests.
- Cross-platform CI, security auditing, release packaging, and architecture documentation.

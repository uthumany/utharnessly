# Changelog

## Unreleased

### Added

- Premium UTHY startup experience with upper-left layered logo depth, gold-to-amber-to-coral ANSI gradient, responsive Unicode/ASCII fallbacks, numbered onboarding tips, and an outside-project warning card.
- Minimal initial chat surface with cyan composer focus and compact telemetry.

- Full-viewport Ratatui workspace with dominant chat, responsive navigation, tabbed inspector, live task timeline, tool cards, dedicated composer, contextual keyboard hints, and tiny-terminal fallback.
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

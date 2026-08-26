# Changelog

## Unreleased

### Added

- Native Rust workspace with focused core, storage, security, provider, and CLI crates.
- SQLite WAL persistence with embedded migrations, sessions, messages, tasks, checkpoints, runtime events, tool calls, permission decisions, audit records, and FTS5 memory search.
- SAFE default execution boundary with workspace scoping, destructive-command denial, output redaction, and explicit approval for shell execution.
- Offline-first chat and bounded OpenRouter-compatible autonomous inspection using a SAFE read-only tool allowlist.
- Responsive UTHY startup banner with layered logo depth, gold-to-amber-to-coral color treatment, Unicode/ASCII fallbacks, onboarding tips, project warning state, version display, reduced-motion support, and opt-in type-in animation.
- Focus Mode as the default Ratatui/Crossterm terminal experience, with conversation-first history, inline PLAN/SHELL/FILE/EDIT/DIFF/GIT/BROWSER/AGENT/MEMORY/SKILL/MCP/TEST/ERROR/permission cards, fixed cyan composer, live elapsed activity, telemetry, and optional Workspace Mode through `Ctrl+B`.
- Exact responsive TUI breakpoints: wide `120+` Focus/Workspace, full-width `80–119`, compact `60–79`, and minimal text mode below `60` columns.
- Keyboard-triggered command, model, provider, file, agent, task, memory, logs, skills, settings, permission, and help overlays. Task, memory, and log views read persisted SQLite state; the file picker reads the workspace filesystem; model/provider choices are process-local.
- Composer multiline editing, `@file`/`@folder`/`@url`/`@agent`/`@skill`/`@memory` references, slash autocomplete for `/model`, `/provider`, `/agents`, `/files`, `/git`, `/tasks`, `/memory`, `/skills`, `/theme`, `/settings`, and `/doctor`, mouse-aware scrolling/focus, resize handling, and independent chat scrolling.
- Terminal palette fallback from TrueColor to ANSI 256, ANSI 16, and monochrome based on terminal capability/environment settings.
- Unit, persistence, security, CLI process, breakpoint, slash-command, and palette fallback tests, with CI, security auditing, and release packaging workflows.

### Notes

Provider credentials are never committed or persisted by the runtime. The interactive TUI currently provides bounded offline planning and session journaling; generalized live provider streaming and broader autonomous tool execution remain subsequent backend milestones.

# Changelog

## Unreleased

### Added

- Native Rust workspace with focused core, storage, security, provider, and CLI crates.
- SQLite WAL persistence with embedded migrations, sessions, messages, tasks, checkpoints, runtime events, tool calls, permission decisions, audit records, and FTS5 memory search.
- SAFE default execution boundary with workspace scoping, destructive-command denial, output redaction, and explicit approval for shell execution.
- Offline-first chat and bounded OpenRouter-compatible autonomous inspection using a SAFE read-only tool allowlist.
- Responsive UTHY startup/banner system with layered logo depth, amber-to-orange-to-coral gradient treatment, wide/medium/compact ASCII variants, persistent uppercase UTHARNESS identity, onboarding tips, project warning state, version display, reduced-motion support, and limited-color fallback.
- Complete React/Ink terminal UI replacement under `ui/`, using `@inkjs/ui`, `chalk`, `gradient-string`, `cli-spinners`, `figures`, `string-width`, `wrap-ansi`, `execa`, `chokidar`, and `zod`.
- One left-aligned full-screen content grid with fixed header/banner/prompt/status chrome and a conversation-only scroll viewport. UTHY/YOU rows, timestamps, streaming tokens, running/completed tool cards, success/error/approval states, result summaries, and responsive wrapping are included.
- Exact responsive terminal matrix for `40–59`, `60–79`, `80–119`, `120–199`, and `200+` columns, with short-height budgeting that keeps branding, prompt, and status visible.
- Ctrl+K command palette, slash-command suggestions, `@` context suggestions, command history, PageUp/PageDown scrolling, mouse-wheel escape handling, SIGWINCH redraw support, and Chokidar workspace/Git refresh hooks.
- Rust CLI launch bridge that runs the built Ink bundle through Node 22 or falls back to `pnpm --dir ui dev` for source checkouts.
- UI unit tests, PTY screenshot matrix, Rust workspace tests, strict Clippy, release compilation, and process-level Rust-to-Ink bridge smoke coverage.

### Notes

Provider credentials are never committed or persisted by the runtime. The interactive UI provides bounded offline planning, native CLI handoff through `execa`, local streaming presentation, and SQLite-backed runtime metadata; broader provider token streaming and autonomous tool execution remain subsequent backend milestones.

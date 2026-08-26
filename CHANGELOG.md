# Changelog

All notable changes to utharnessly are documented here.

## [0.1.0] — 2026-08-27

### Added

- Public `utharnessly` repository identity with preserved Git history and canonical links.
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
- Linux x64, macOS x64, and Windows x64 release archives with bundled UI and SHA-256 checksums.
- POSIX and PowerShell release installers with explicit unsupported-target and source-build fallback behavior.
- npm package `utharnessly` and PyPI package `utharnessly`, each providing `utharness` and `utharnessly` launcher entry points with checksum-verified native runtime caching.
- UI unit tests, PTY screenshot matrix, Rust workspace tests, strict Clippy, release compilation, package builds, package installation simulations, and process-level Rust-to-Ink bridge smoke coverage.
- Cross-platform CI, package validation, security auditing, and tag-triggered release automation.
- Installation, platform, terminal, package-manager, troubleshooting, compatibility, and development documentation, with real screenshots under `docs/assets/screenshots/`.

### Notes and limitations

Provider credentials are never committed or persisted by the runtime. The interactive UI provides bounded offline planning, native CLI handoff through `execa`, local streaming presentation, and SQLite-backed runtime metadata; broader provider token streaming and autonomous tool execution remain subsequent backend milestones.

Published native release artifacts target Linux x64, macOS x64, and Windows x64. ARM, Android, iOS/iPadOS, FreeBSD, Homebrew, apt, Nix, winget, and other unlisted package ecosystems use source or remote-host guidance rather than fabricated product packages.

[0.1.0]: https://github.com/uthumany/utharnessly/releases/tag/v0.1.0

# Changelog

All notable changes to utharnessly are documented here.

## [Unreleased]

### Added

- Provider-neutral OpenAI-compatible gateway for OpenRouter, OpenAI, Groq, Together, DeepSeek, Fireworks, Ollama, and custom endpoints.
- Real-time SSE token streaming in `utharness chat`, provider health checks, environment setup output, and automatic provider selection from supported credential variables.
- Working `agents list` and `agents run` commands backed by the existing bounded SAFE inspection engine.
- Local mock-gateway unit and process-level integration tests covering stream ordering, authorization, persistence, and secret-safe output.

### Security

- Provider credentials remain process-environment-only and are never written to SQLite or printed by status commands.
- Plain HTTP provider URLs are rejected except for loopback development endpoints; rejected API response bodies are not echoed.

## [0.2.9] — 2026-08-29

### Added

- Responsive focus and workspace TUI modes with navigation, task inspector, role-separated transcripts, real Git telemetry, persistent preferences, multiline composer editing, and compact Termux layouts.
- Command, model, file, agent, task, memory, job, and log overlays with keyboard navigation and slash-command entry points.
- Deterministic PTY screenshot coverage at 40×15 through 160×50, including command-palette and workspace captures.

### Fixed

- Removed fabricated demo transcript and tool results from the runtime UI; startup and status content now reflect real local state.
- Hardened terminal color fallback, short-height layout, workspace column sizing, capture lifecycle, and asynchronous snapshot rendering.
- Updated the UI toolchain to pnpm 11 with explicit dependency build-script allowlisting.

### Verification

- UI type checking, nine behavior tests, production bundling, Rust formatting, strict Clippy, all workspace tests, and release compilation pass locally.
- NPM, PyPI, shell, Termux, and native release-package validation is coordinated under the 0.2.9 release line.

### Added

- Termux-native command family: `termux info`, `termux setup`, `termux api`, `termux keys install`, `termux storage enable`, `termux permissions`, and `termux doctor`.
- Android/Termux environment detection for architecture, `$PREFIX`, shell, terminal size/color, storage, optional Termux:API, Node, Python, Git, SSH, curl, OpenSSL, disk, RAM, DNS, and network status.
- No-root Termux path contract using `$PREFIX/bin/utharness`, `$PREFIX/lib/utharness`, `$PREFIX/share/utharness`, `~/.config/utharness`, `~/.local/share/utharness`, and `~/.cache/utharness`.
- Debian package builder, package lifecycle scripts, signed APT metadata builder, repository bootstrap script, package checksums, and release workflow support for aarch64 and x86_64.
- Termux mobile-first TUI breakpoint policy for under 50, 50–89, and 90+ columns with persistent branding and Termux navigation hints.

### Fixed

- Synchronized Rust, UI, NPM, and Python release metadata to the coordinated 0.2.7 line; added a CLI regression test that compares `utharness --version` with the Cargo package version.
- Corrected the live Termux bootstrap and repository-relative checksum verification paths in the release workflow and repository builder.

### Notes and limitations

The v0.2.7 Termux repository is live and has been verified from public Pages endpoints: both signed metadata forms, the published public key, both architecture indexes, package versions, and repository checksums pass. The sandbox does not contain an Android emulator or physical Android device, so Android-version-specific behavior, soft-keyboard behavior, and real Termux:API execution remain unverified. The coordinated 0.2.7 source and registry package publication is complete.

## [0.2.7] — 2026-08-27

### Fixed

- Synchronized Rust, UI, NPM, and Python release metadata and version reporting to 0.2.7.
- Added a CLI regression test covering `utharness --version` against the Cargo package version.
- Corrected Termux bootstrap exit behavior and repository-relative checksum generation.
- Published and verified signed Termux packages for Android `aarch64` and `x86_64`, with live GitHub Pages metadata, checksums, and release assets.

### Verification

- Hosted CI and Security passed for the release source.
- Rust, Ink UI, NPM, PyPI, CLI safety, mock-provider, package, and PTY bridge checks passed.
- Direct physical-device, Android-version, soft-keyboard, and Termux:API testing remains outside the available environment.

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

Published native release artifacts target Linux x64, macOS x64, and Windows x64; the v0.2.7 Termux release adds signed Android `aarch64` and `x86_64` packages. iOS/iPadOS, FreeBSD, desktop ARM variants, Homebrew, apt, Nix, winget, and other unlisted package ecosystems use source or remote-host guidance rather than fabricated product packages.

[0.2.7]: https://github.com/uthumany/utharnessly/releases/tag/v0.2.7
[0.2.9]: https://github.com/uthumany/utharnessly/releases/tag/v0.2.9
[0.1.0]: https://github.com/uthumany/utharnessly/releases/tag/v0.1.0

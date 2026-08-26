# utharnessly installation and platform guide

This document separates **verified installation paths** from package-manager or platform entries that do not currently have a published utharnessly package. The public repository contains a Rust runtime and a bundled React/Ink UI. The interactive UI requires **Node.js 22 or newer**; the native CLI requires a Rust toolchain when built from source.

## Verified installation paths

| Method | Status | Installation | Executable entry point | PATH and dependency checks | Update, uninstall, and clean reinstall |
| --- | --- | --- | --- | --- | --- |
| `curl` | Verified for published POSIX release archives | `curl -fsSL https://raw.githubusercontent.com/uthumany/utharnessly/main/packaging/install.sh \\| bash` | `utharness` | Installs to `~/.local/bin`; ensure that directory is on `PATH`. Requires `curl`, `tar`, and `sha256sum`; the UI requires Node 22. | Re-run the installer with `UTHARNESS_VERSION=vX.Y.Z`; remove `~/.local/bin/utharness` and `~/.local/bin/utharnessly-ui`, then rerun for a clean install. |
| `npm` | Verified for the UI package from a checkout | `npm --prefix ui install && npm --prefix ui run build` | `node ui/dist/index.js` or `npm --prefix ui start` | Requires Node 22 and npm. `npm --prefix ui run typecheck` checks the package. | `npm --prefix ui update`; `npm --prefix ui uninstall`; delete `ui/node_modules` and `ui/pnpm-lock.yaml` only when intentionally regenerating package-manager metadata. |
| `npx` | Verified for source execution | `npx --yes tsx ui/src/index.tsx` | `npx tsx ui/src/index.tsx` | Uses the package manifest and Node 22. Prefer a lockfile-backed install for repeatable builds. | Use `npx --yes tsx@latest` only for diagnosis; clean reinstall with `rm -rf ui/node_modules && npm --prefix ui install`. |
| `pnpm` / `pnpx` | Verified and CI-tested | `pnpm --dir ui install --frozen-lockfile && pnpm --dir ui build` | `pnpm --dir ui start`, `pnpx tsx ui/src/index.tsx`, or `./target/release/utharness tui` | Requires Node 22 and pnpm. The repository includes `ui/pnpm-lock.yaml`. | `pnpm --dir ui update`; `pnpm --dir ui remove PACKAGE`; clean reinstall with `rm -rf ui/node_modules && pnpm --dir ui install --frozen-lockfile`. |
| Bun / `bunx` | Source-compatible | `bun --cwd ui install && bun --cwd ui run build` | `bun --cwd ui run start` or `bunx tsx ui/src/index.tsx` | Requires a Bun release with Node/npm compatibility. The Rust bridge still expects a `node` executable for the bundled entrypoint, so run the UI directly with Bun or provide `UTHARNESS_UI_ENTRY`. | `bun --cwd ui update`; `rm -rf ui/node_modules && bun --cwd ui install`. |
| `Cargo` | Verified for the native runtime from source | `cargo build --release` | `./target/release/utharness`; run `./target/release/utharness tui` after building `ui/dist/index.js` | Requires Rust stable, Node 22, and pnpm for the UI bundle. A bare `cargo install` does not include the UI bundle and is therefore not advertised as a complete TUI installation. | `cargo update`; `cargo clean && cargo build --release`; remove `target/release/utharness` for uninstall. |
| Git | Verified | `git clone https://github.com/uthumany/utharnessly.git && cd utharnessly` | `./target/release/utharness` after the build steps | Requires Git, Rust stable, Node 22, and pnpm. The repository is public and the old URL redirects to the renamed repository. | `git pull --ff-only`; delete the checkout and clone again for a clean reinstall. |
| Homebrew | Source formula not yet published | Use the Git + Cargo + pnpm path until a signed Homebrew formula is published. | `utharness` after installing the release archive or source build | Do not use `brew install utharnessly` yet; that would claim a package that is not present in a Homebrew tap. | When a formula is published, use `brew upgrade utharnessly`, `brew uninstall utharnessly`, and `brew reinstall utharnessly`. |
| `apt` | Source/release archive only | Use the `curl` release installer or build from Git. No Debian repository is claimed. | `utharness` | `apt` can install prerequisites such as `curl`, `git`, `build-essential`, and `pkg-config`; it does not currently install the product. | Remove the installed user binary and UI directory, then rerun the chosen verified path. |
| Nix | Source shell only | `nix develop` is not currently provided; use the documented Git build with a Nix-provided Rust, Node, pnpm, and Git environment. | `./target/release/utharness` | No flake or Nixpkgs package is claimed until a reproducible derivation is maintained. | Re-enter the development shell and rebuild; remove `target/` for a clean source build. |
| Volta, mise, fnm, nvm | Runtime setup, not product installers | Install Node 22 with the manager, then use the pnpm or npm UI path. | `node ui/dist/index.js` | These managers can place Node and package-manager shims on `PATH`; verify with `node --version` and `pnpm --version`. | Use the manager’s version switch/remove command, then reinstall dependencies with the selected runtime. |
| Corepack | Verified as a pnpm bootstrap mechanism | `corepack enable && corepack prepare pnpm@latest --activate` followed by `pnpm --dir ui install --frozen-lockfile` | `pnpm --dir ui start` | Requires Node 22. Corepack manages pnpm; it does not install the Rust runtime. | `corepack prepare pnpm@latest --activate`; remove `ui/node_modules` for a clean dependency reinstall. |
| Rush, Lerna, cnpm | Workspace orchestration alternatives | Not required by this repository. Use the existing single-package `ui` workspace and its lockfile. | Use `pnpm --dir ui ...` | Adding an orchestrator only to claim compatibility would create an unnecessary dependency and is intentionally avoided. | Do not run a package-manager migration unless a maintained workspace configuration is added. |
| Deno | Direct source execution only | `deno run --allow-all --node-modules-dir=auto ui/src/index.tsx` on a Deno release with Node compatibility. | `deno run ... ui/src/index.tsx` | Deno compatibility depends on its Node/npm support and terminal behavior; the Rust launcher is Node-based. | Remove the Deno cache or use a fresh checkout; package builds remain Node/pnpm-based. |
| `uv`, `uvx`, `pip`, `pipx`, `python -m` | PTY/automation support, not the product runtime | Use these tools for the repository’s Python PTY capture helpers; they are not claimed as Python distributions of the Rust/Ink application. | `python3 ui/test/pty_capture.py` | Requires Python 3.11+ and a terminal emulator. The CLI itself remains Rust plus Node. | Recreate the Python environment or use the system interpreter; no Python package uninstall is needed for the core product. |
| `winget` | Windows prerequisite channel | Install Node 22 and Git with `winget`, then build from Git; no utharnessly winget manifest is claimed yet. | `utharness.exe` plus Node-launched UI | Use `winget install OpenJS.NodeJS.LTS Git.Git`; verify `node --version`, `git --version`, and `cargo --version`. | `winget upgrade`; uninstall prerequisites with their package IDs; rebuild the checkout for a clean product install. |

## Release archive installer

For a published POSIX release, the installer verifies the `SHA256SUMS` asset when available and installs the native executable plus bundled UI files into a user-owned directory:

```bash
curl -fsSL https://raw.githubusercontent.com/uthumany/utharnessly/main/packaging/install.sh | bash
utharness
```

PowerShell users can run:

```powershell
irm https://raw.githubusercontent.com/uthumany/utharnessly/main/packaging/install.ps1 | iex
utharness
```

The installer refuses unsupported operating systems and architectures instead of silently installing an incompatible artifact. When a matching release has not been published, it prints the source-build path.

## Operating systems and terminal environments

| Platform | Valid local workflow | Restricted or remote workflow |
| --- | --- | --- |
| Windows | Windows Terminal, WezTerm, Alacritty, Tabby, and Cmder can run the PowerShell installer or the Git source build. | Use WSL or a remote Linux host when Rust or Node tooling is unavailable. |
| Linux | Kitty, WezTerm, Alacritty, Ghostty, and Tilix are supported ANSI terminals for the source build or release archive. | Use SSH to a Linux release host or a container with Node 22 and the native binary. |
| Android | Termux is the closest supported local shell; install Git, Rust, Node, and pnpm there or SSH to a release host. Termius, ConnectBot, TermAI, and Moshi are SSH/client environments rather than native build targets. | Use SSH to a Linux/macOS host when the mobile client cannot run Rust or Node locally. |
| macOS | Ghostty, iTerm2, Warp, WezTerm, and Kitty can run the POSIX installer or source build. | Use a Linux container or SSH when local toolchains are restricted. |
| iOS/iPadOS | Blink Shell, Termius, Secure ShellFish, and compatible SSH clients should connect to a remote host. a-Shell and iSH are restricted shells and are not claimed as native Rust/Node targets. | Use the remote-host workflow; do not claim local installation where the platform cannot supply the required runtimes. |
| FreeBSD/Unix | Kitty, Alacritty, WezTerm, xterm, and Konsole can render the ANSI UI. Build from source if Rust, Node 22, and pnpm are available. | Use a Linux-compatible container or SSH when a dependency is unavailable. |
| Cross-platform | WezTerm, Alacritty, Termius, Tabby, and Hyper are terminal frontends; they do not install the product themselves. | Pair them with the platform-specific installer or a remote host. |

The UI uses standard ANSI escape sequences, Unicode/ASCII fallbacks, Node stdin handling, and SIGWINCH where available. Terminal glyph metrics and mouse reporting vary by emulator; the PTY test matrix is authoritative for Linux, while the remaining platforms should run the same package tests and source-build checks in their native CI environments.

## Troubleshooting

If `utharness tui` reports that the UI bundle is missing, run `pnpm --dir ui install --frozen-lockfile && pnpm --dir ui build`, then retry. If the executable is not found after installation, add `~/.local/bin` to `PATH` and open a new shell. If the screen is garbled, check `TERM`, run with `NO_COLOR=1`, or use a terminal with UTF-8 and ANSI support. If a provider is unavailable, use the offline planner; provider credentials are read from environment variables and are never stored by the installer.

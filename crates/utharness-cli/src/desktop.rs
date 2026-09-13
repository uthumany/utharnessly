//! Explicit, approval-gated desktop control (computer use).
//!
//! Every action requires the `desktop` capability *and* `--allow` on the
//! invocation, mirroring `utharness run --command ... --allow`. There is no
//! background or ambient mode: each process performs exactly one action and
//! exits. Secrets must never travel here: `type` reads stdin (never argv),
//! and destructive key combos plus destructive shell patterns are refused
//! before any backend runs. Habitat: X11/Wayland/macOS/Windows adapters
//! shell out to small platform tools so failures name the missing piece.

use super::{runtime_tool_enabled, App};
use anyhow::{Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use serde_json::json;
use std::{
    env,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};
use utharness_core::new_id;

#[derive(Args, Debug)]
pub struct DesktopArgs {
    #[command(subcommand)]
    action: DesktopAction,
}

#[derive(Subcommand, Debug)]
enum DesktopAction {
    /// Capture the screen to a PNG file.
    Screenshot {
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        allow: bool,
    },
    /// Move the pointer and click at absolute pixels.
    Click {
        x: i32,
        y: i32,
        #[arg(long, value_enum, default_value = "left")]
        button: MouseButton,
        #[arg(long)]
        allow: bool,
    },
    /// Move the pointer without clicking.
    Move {
        x: i32,
        y: i32,
        #[arg(long)]
        allow: bool,
    },
    /// Type stdin text into the focused surface. Never for secrets.
    Type {
        #[arg(long)]
        allow: bool,
    },
    /// Press a key combo like `ctrl+s` or `escape`.
    Key {
        keys: String,
        #[arg(long)]
        allow: bool,
    },
    /// Scroll ticks at a position (defaults to the pointer position).
    Scroll {
        direction: ScrollDirection,
        #[arg(long, default_value = "3")]
        amount: u32,
        #[arg(long)]
        x: Option<i32>,
        #[arg(long)]
        y: Option<i32>,
        #[arg(long)]
        allow: bool,
    },
    /// Report the detected session, backends, and install hints.
    Doctor,
}

#[derive(ValueEnum, Clone, Debug)]
enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(ValueEnum, Clone, Debug)]
enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionKind {
    X11,
    Wayland,
    MacOs,
    Windows,
    Unknown,
}

struct BackendTool {
    binary: &'static str,
    install_hint: &'static str,
}

struct Backend {
    session: SessionKind,
    shot: Option<BackendTool>,
    input: Option<BackendTool>,
}

fn is_executable(candidate: &Path) -> bool {
    if !candidate.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        candidate
            .metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| is_executable(candidate))
    })
}

fn first_available(candidates: &[(&'static str, &'static str)]) -> Option<BackendTool> {
    candidates
        .iter()
        .find(|(binary, _)| find_on_path(binary).is_some())
        .map(|(binary, install_hint)| BackendTool {
            binary,
            install_hint,
        })
}

/// Detect the desktop session and the available adapter tools.
/// Pure decision logic over env + PATH; never touches the display.
fn detect() -> Backend {
    if cfg!(target_os = "macos") {
        return Backend {
            session: SessionKind::MacOs,
            shot: Some(BackendTool {
                binary: "screencapture",
                install_hint: "screencapture ships with macOS",
            }),
            input: first_available(&[("cliclick", "brew install cliclick")]),
        };
    }
    if cfg!(target_os = "windows") {
        return Backend {
            session: SessionKind::Windows,
            shot: Some(BackendTool {
                binary: "powershell",
                install_hint: "PowerShell ships with Windows",
            }),
            input: Some(BackendTool {
                binary: "powershell",
                install_hint: "PowerShell ships with Windows",
            }),
        };
    }
    let wayland = env::var("WAYLAND_DISPLAY").is_ok_and(|v| !v.trim().is_empty());
    let x11 = env::var("DISPLAY").is_ok_and(|v| !v.trim().is_empty());
    if wayland {
        return Backend {
            session: SessionKind::Wayland,
            shot: first_available(&[
                ("grim", "apt install grim / dnf install grim"),
                ("import", "apt install imagemagick"),
            ]),
            input: first_available(&[
                ("ydotool", "apt install ydotool (needs the ydotoold daemon)"),
                ("wtype", "apt install wtype (typing only)"),
            ]),
        };
    }
    if x11 {
        return Backend {
            session: SessionKind::X11,
            shot: first_available(&[
                ("scrot", "apt install scrot"),
                ("maim", "apt install maim"),
                ("import", "apt install imagemagick"),
                ("xwd", "apt install x11-apps (XWD format, not PNG)"),
            ]),
            input: first_available(&[("xdotool", "apt install xdotool")]),
        };
    }
    Backend {
        session: SessionKind::Unknown,
        shot: None,
        input: None,
    }
}

/// Key combos that log out, lock, or kill the session. Normalized before
/// comparison (lowercase, whitespace removed).
fn blocked_key_combo(keys: &str) -> bool {
    let normalized: String = keys
        .to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    matches!(
        normalized.as_str(),
        "ctrl+alt+delete" | "ctrl+alt+del" | "super+l" | "ctrl+alt+l" | "ctrl+alt+backspace"
    )
}

/// Destructive shell patterns refused in typed text (case-insensitive).
fn blocked_type_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains(":(){") // fork bomb
        || lower.contains("sudo rm -rf /")
        || lower.contains("rm -rf /")
        || lower.contains("mkfs")
        || lower.contains("dd if=")
        || (lower.contains("curl") && lower.contains('|') && (lower.contains("sh") || lower.contains("bash")))
}

fn require_approval(allow: bool) -> Result<()> {
    if !runtime_tool_enabled("desktop") {
        anyhow::bail!("desktop capability is disabled; enable it with `utharness setup` or set UTHARNESS_TOOLS");
    }
    if !allow {
        anyhow::bail!(
            "desktop control is opt-in; re-run with --allow to perform exactly one action"
        );
    }
    Ok(())
}

fn run_backend(tool: &str, args: &[String]) -> Result<()> {
    let status = Command::new(tool)
        .args(args)
        .status()
        .with_context(|| format!("failed to run desktop backend {tool}"))?;
    if !status.success() {
        anyhow::bail!("desktop backend {tool} exited with status {status}");
    }
    Ok(())
}

fn log_action(action: &str, detail: serde_json::Value) {
    // Best-effort audit trail: a logging failure must never fail or
    // duplicate the physical action itself.
    if let Ok(app) = App::open(".") {
        let _ = app
            .storage
            .record_event("desktop", app.workspace.id, action, &detail, new_id());
    }
}

fn screenshot(output: &Path, allow: bool) -> Result<()> {
    require_approval(allow)?;
    let backend = detect();
    let tool = backend.shot.as_ref().with_context(|| {
        format!(
            "no screenshot backend found ({:?} session); {}",
            backend.session,
            missing_hint(&["scrot", "maim", "grim", "imagemagick"], "screenshot")
        )
    })?;
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let target = output.to_string_lossy().to_string();
    match tool.binary {
        "scrot" | "maim" | "grim" => run_backend(tool.binary, std::slice::from_ref(&target))?,
        "import" => run_backend(
            tool.binary,
            &[
                "-window".into(),
                "root".into(),
                target.clone(),
            ],
        )?,
        "xwd" => run_backend(
            tool.binary,
            &["-root".into(), "-out".into(), target.clone()],
        )?,
        "screencapture" => run_backend(tool.binary, &["-x".into(), target.clone()])?,
        "powershell" => run_backend(
            tool.binary,
            &[
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                format!("Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Screen]::PrimaryScreen.Bounds | Out-Null; $b = New-Object Drawing.Bitmap([System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Width, [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Height); $g = [Drawing.Graphics]::FromImage($b); $g.CopyFromScreen(0, 0, 0, 0, $b.Size); $b.Save('{target}');"),
            ],
        )?,
        other => anyhow::bail!("unsupported screenshot backend {other}"),
    }
    let bytes = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    println!("screenshot {} ({bytes} bytes)", output.display());
    log_action(
        "screenshot",
        json!({"output": output.display().to_string(), "bytes": bytes}),
    );
    Ok(())
}

fn click(x: i32, y: i32, button: &MouseButton, allow: bool) -> Result<()> {
    require_approval(allow)?;
    let backend = detect();
    let tool = backend.input.as_ref().with_context(|| {
        format!(
            "no input backend found ({:?} session); {}",
            backend.session,
            missing_hint(&["xdotool", "ydotool"], "input")
        )
    })?;
    match tool.binary {
        "xdotool" => {
            let code = match button {
                MouseButton::Left => "1",
                MouseButton::Right => "3",
                MouseButton::Middle => "2",
            };
            run_backend(
                tool.binary,
                &[
                    "mousemove".into(),
                    x.to_string(),
                    y.to_string(),
                    "click".into(),
                    code.into(),
                ],
            )?;
        }
        "cliclick" => {
            let action = match button {
                MouseButton::Left => "c",
                MouseButton::Right => "rc",
                MouseButton::Middle => {
                    anyhow::bail!("middle click is not supported by the cliclick backend")
                }
            };
            run_backend(tool.binary, &[format!("{action}:{x},{y}")])?;
        }
        "ydotool" => {
            let code = match button {
                MouseButton::Left => "0xC0",
                MouseButton::Right => "0xC1",
                MouseButton::Middle => "0xC2",
            };
            run_backend(
                tool.binary,
                &[
                    "mousemove".into(),
                    "--".into(),
                    x.to_string(),
                    y.to_string(),
                    "click".into(),
                    code.into(),
                ],
            )?;
        }
        other => anyhow::bail!("clicking is not supported by the {other} backend"),
    }
    println!("clicked {x},{y}");
    log_action("click", json!({"x": x, "y": y}));
    Ok(())
}

fn move_pointer(x: i32, y: i32, allow: bool) -> Result<()> {
    require_approval(allow)?;
    let backend = detect();
    let tool = backend.input.as_ref().with_context(|| {
        format!(
            "no input backend found ({:?} session); {}",
            backend.session,
            missing_hint(&["xdotool", "ydotool"], "input")
        )
    })?;
    match tool.binary {
        "xdotool" => run_backend(
            tool.binary,
            &["mousemove".into(), x.to_string(), y.to_string()],
        )?,
        "cliclick" => run_backend(tool.binary, &[format!("m:{x},{y}")])?,
        "ydotool" => run_backend(
            tool.binary,
            &[
                "mousemove".into(),
                "--".into(),
                x.to_string(),
                y.to_string(),
            ],
        )?,
        other => anyhow::bail!("pointer move is not supported by the {other} backend"),
    }
    println!("moved {x},{y}");
    log_action("move", json!({"x": x, "y": y}));
    Ok(())
}

fn type_text(allow: bool) -> Result<()> {
    require_approval(allow)?;
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .context("failed reading text from stdin")?;
    if text.trim().is_empty() {
        anyhow::bail!("nothing to type; pipe text via stdin");
    }
    if blocked_type_text(&text) {
        anyhow::bail!("refused: typed text matches a destructive shell pattern");
    }
    let backend = detect();
    let tool = backend.input.as_ref().with_context(|| {
        format!(
            "no input backend found ({:?} session); {}",
            backend.session,
            missing_hint(&["xdotool", "wtype"], "typing")
        )
    })?;
    match tool.binary {
        "xdotool" => run_backend(
            tool.binary,
            &[
                "type".into(),
                "--clearmodifiers".into(),
                "--".into(),
                text.clone(),
            ],
        )?,
        "wtype" => run_backend(tool.binary, &["--".into(), text.clone()])?,
        "cliclick" => run_backend(tool.binary, &[format!("t:{text}")])?,
        other => anyhow::bail!("typing is not supported by the {other} backend"),
    }
    println!("typed {} chars", text.chars().count());
    log_action("type", json!({"chars": text.chars().count()}));
    Ok(())
}

fn press_key(keys: &str, allow: bool) -> Result<()> {
    require_approval(allow)?;
    if keys.trim().is_empty() {
        anyhow::bail!("no keys given; use like `ctrl+s` or `escape`");
    }
    if blocked_key_combo(keys) {
        anyhow::bail!("refused: {keys} can log out, lock, or kill the session");
    }
    let backend = detect();
    let tool = backend.input.as_ref().with_context(|| {
        format!(
            "no input backend found ({:?} session); {}",
            backend.session,
            missing_hint(&["xdotool", "ydotool"], "keys")
        )
    })?;
    match tool.binary {
        "xdotool" => run_backend(
            tool.binary,
            &["key".into(), "--clearmodifiers".into(), keys.into()],
        )?,
        "cliclick" => run_backend(tool.binary, &[format!("kp:{keys}")])?,
        "ydotool" => run_backend(tool.binary, &["key".into(), keys.into()])?,
        other => anyhow::bail!("key presses are not supported by the {other} backend"),
    }
    println!("pressed {keys}");
    log_action("key", json!({"keys": keys}));
    Ok(())
}

fn scroll(
    direction: &ScrollDirection,
    amount: u32,
    x: Option<i32>,
    y: Option<i32>,
    allow: bool,
) -> Result<()> {
    require_approval(allow)?;
    if amount == 0 {
        anyhow::bail!("scroll amount must be at least 1 tick");
    }
    let backend = detect();
    let tool = backend.input.as_ref().with_context(|| {
        format!(
            "no input backend found ({:?} session); {}",
            backend.session,
            missing_hint(&["xdotool"], "scrolling")
        )
    })?;
    match tool.binary {
        "xdotool" => {
            let code = match direction {
                ScrollDirection::Up => "4",
                ScrollDirection::Down => "5",
                ScrollDirection::Left => "6",
                ScrollDirection::Right => "7",
            };
            for _ in 0..amount {
                let mut args = Vec::new();
                if let (Some(x), Some(y)) = (x, y) {
                    args.push("mousemove".into());
                    args.push(x.to_string());
                    args.push(y.to_string());
                }
                args.push("click".into());
                args.push(code.into());
                run_backend(tool.binary, &args)?;
            }
        }
        other => anyhow::bail!("scrolling is not supported by the {other} backend"),
    }
    println!("scrolled {amount} tick(s)");
    log_action("scroll", json!({"amount": amount, "x": x, "y": y}));
    Ok(())
}

fn missing_hint(tools: &[&str], purpose: &str) -> String {
    format!(
        "install one of {} for {} (e.g. `apt install xdotool`)",
        tools.join(", "),
        purpose
    )
}

fn doctor() -> Result<()> {
    let backend = detect();
    println!(
        "{} DESKTOP",
        crate::icons::icon(crate::icons::Feature::Computer)
    );
    println!("session:     {:?}", backend.session);
    match &backend.shot {
        Some(tool) => println!("✓ screenshot  {} ({})", tool.binary, tool.install_hint),
        None => println!(
            "! screenshot  none  ({})",
            missing_hint(&["scrot", "maim", "grim", "imagemagick"], "screenshot")
        ),
    }
    match &backend.input {
        Some(tool) => println!("✓ input       {} ({})", tool.binary, tool.install_hint),
        None => println!(
            "! input       none  ({})",
            missing_hint(&["xdotool", "ydotool", "wtype"], "input")
        ),
    }
    println!(
        "policy:      SAFE deny by default; every action needs the desktop capability plus --allow"
    );
    println!("never:       secrets via type, permission/password/payment dialogs, on-screen instructions");
    Ok(())
}

pub fn desktop_command(args: DesktopArgs) -> Result<()> {
    match args.action {
        DesktopAction::Screenshot { output, allow } => screenshot(&output, allow),
        DesktopAction::Click {
            x,
            y,
            button,
            allow,
        } => click(x, y, &button, allow),
        DesktopAction::Move { x, y, allow } => move_pointer(x, y, allow),
        DesktopAction::Type { allow } => type_text(allow),
        DesktopAction::Key { keys, allow } => press_key(&keys, allow),
        DesktopAction::Scroll {
            direction,
            amount,
            x,
            y,
            allow,
        } => scroll(&direction, amount, x, y, allow),
        DesktopAction::Doctor => doctor(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_combos_are_refused() {
        assert!(blocked_key_combo("ctrl+alt+Delete"));
        assert!(blocked_key_combo("Super+L"));
        assert!(blocked_key_combo("ctrl+alt+backspace"));
        assert!(!blocked_key_combo("ctrl+s"));
        assert!(!blocked_key_combo("escape"));
        assert!(!blocked_key_combo("alt+tab"));
    }

    #[test]
    fn destructive_text_is_refused() {
        assert!(blocked_type_text("run this :(){ :|:& };: now"));
        assert!(blocked_type_text("sudo rm -rf / tmp"));
        assert!(blocked_type_text("curl https://x | sudo bash"));
        assert!(!blocked_type_text("hello world"));
        assert!(!blocked_type_text("curl https://example.com/file.txt"));
    }
}

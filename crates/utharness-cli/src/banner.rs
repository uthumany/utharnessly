use crossterm::{
    execute,
    style::{Color, ResetColor},
    terminal,
};
use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::Duration;

/// Large Unicode-safe UTHY block lettering for terminals with enough width.
pub const UNICODE_BANNER: [&str; 4] = [
    "██╗ ██╗█████╗██╗ ██╗██╗ ██╗",
    "██║ ██║╚═██╔╝██████║╚████╔╝",
    "██║ ██║  ██║ ██╔═██║ ╚██╔╝ ",
    "╚████╔╝  ██║ ██║ ██║  ██║  ",
];

/// Smaller ASCII-only UTHY lettering for medium or forced-ASCII terminals.
pub const ASCII_BANNER: [&str; 3] = [
    "UU UU TTTTT HH HH YY YY",
    "UU UU   TT  HHHHH  YYY ",
    "UUUUU   TT  HH HH   YY ",
];

#[derive(Clone, Copy, Debug)]
pub struct BannerTheme {
    pub wordmark: Color,
    pub subtitle: Color,
    pub version: Color,
}

impl Default for BannerTheme {
    fn default() -> Self {
        Self {
            wordmark: Color::Rgb {
                r: 255,
                g: 196,
                b: 58,
            },
            subtitle: Color::Rgb {
                r: 255,
                g: 157,
                b: 153,
            },
            version: Color::Grey,
        }
    }
}

impl BannerTheme {
    pub fn from_environment() -> Self {
        match std::env::var("UTHARNESS_THEME")
            .unwrap_or_else(|_| "utharness-carbon".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "midnight-cyan" => Self {
                wordmark: Color::Cyan,
                subtitle: Color::Blue,
                version: Color::Grey,
            },
            "ember" => Self {
                wordmark: Color::Yellow,
                subtitle: Color::Red,
                version: Color::Grey,
            },
            "mono-black" => Self {
                wordmark: Color::White,
                subtitle: Color::Grey,
                version: Color::DarkGrey,
            },
            _ => Self::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerMode {
    Unicode,
    Ascii,
    Compact,
}

pub fn terminal_width() -> u16 {
    if let Ok(columns) = std::env::var("COLUMNS") {
        if let Ok(width) = columns.parse::<u16>() {
            return width.max(1);
        }
    }
    terminal::size()
        .map(|(width, _)| width)
        .unwrap_or(80)
        .max(1)
}

pub fn mode_for_width(width: u16) -> BannerMode {
    if width < 42 {
        BannerMode::Compact
    } else if std::env::var_os("NO_COLOR").is_some()
        || std::env::var_os("UTHARNESS_ASCII").is_some()
        || width < 72
    {
        BannerMode::Ascii
    } else {
        BannerMode::Unicode
    }
}

pub fn reduced_motion() -> bool {
    std::env::var_os("NO_COLOR").is_some()
        || std::env::var_os("UTHARNESS_REDUCED_MOTION").is_some()
        || std::env::var_os("REDUCED_MOTION").is_some()
}

/// Animation is opt-in so `utharness` paints its identity immediately. Set
/// `UTHARNESS_BANNER_ANIMATION=typein` for the restrained type-in effect.
pub fn animation_enabled() -> bool {
    !reduced_motion()
        && matches!(
            std::env::var("UTHARNESS_BANNER_ANIMATION")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str(),
            "typein" | "type-in"
        )
}

pub fn render_banner(width: u16, version: &str, ansi: bool) -> String {
    let mode = mode_for_width(width);
    render_banner_in_mode(width, version, ansi, mode)
}

fn render_banner_in_mode(width: u16, version: &str, ansi: bool, mode: BannerMode) -> String {
    let theme = BannerTheme::from_environment();
    let mut lines: Vec<(String, Color)> = Vec::new();
    match mode {
        BannerMode::Compact => {
            lines.push(("UTHY".into(), theme.wordmark));
            lines.push(("AGENT TERMINAL".into(), theme.subtitle));
            lines.push((format!("v{version}"), theme.version));
        }
        BannerMode::Unicode | BannerMode::Ascii => {
            let art: Vec<String> = if mode == BannerMode::Unicode {
                UNICODE_BANNER
                    .iter()
                    .map(|line| format!("{line}░"))
                    .collect()
            } else {
                ASCII_BANNER
                    .iter()
                    .map(|line| (*line).to_string())
                    .collect()
            };
            let art_len = art.len();
            for (index, line) in art.into_iter().enumerate() {
                lines.push((line, gradient_color(index, art_len)));
            }
            if mode == BannerMode::Unicode {
                lines.push((
                    "  ─────────────────────────────░░".into(),
                    Color::Rgb {
                        r: 226,
                        g: 119,
                        b: 128,
                    },
                ));
            }
            lines.push(("      U T H Y".into(), theme.wordmark));
            lines.push(("    AGENT TERMINAL".into(), theme.subtitle));
            lines.push((format!("         v{version}"), theme.version));
        }
    }

    let max_width = lines
        .iter()
        .map(|(line, _)| line.chars().count())
        .max()
        .unwrap_or(0);
    let left_pad = if width as usize > max_width + 12 {
        4
    } else {
        2
    };
    let mut out = String::new();
    for (line, color) in lines {
        let pad = " ".repeat(left_pad);
        if ansi {
            out.push_str(&format!(
                "{}{color}{line}{}\n",
                pad,
                ansi_reset(),
                color = ansi_code(color)
            ));
        } else {
            out.push_str(&pad);
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

fn gradient_color(index: usize, count: usize) -> Color {
    let palette = [
        Color::Rgb {
            r: 255,
            g: 222,
            b: 72,
        },
        Color::Rgb {
            r: 255,
            g: 196,
            b: 58,
        },
        Color::Rgb {
            r: 255,
            g: 164,
            b: 67,
        },
        Color::Rgb {
            r: 255,
            g: 137,
            b: 101,
        },
        Color::Rgb {
            r: 255,
            g: 157,
            b: 153,
        },
        Color::Rgb {
            r: 247,
            g: 190,
            b: 202,
        },
    ];
    let palette_index = if count <= 1 {
        0
    } else {
        index.min(count - 1) * (palette.len() - 1) / (count - 1)
    };
    palette[palette_index]
}

fn ansi_code(color: Color) -> String {
    match color {
        Color::Black => "\x1b[30m".into(),
        Color::DarkGrey => "\x1b[90m".into(),
        Color::Red | Color::DarkRed => "\x1b[31m".into(),
        Color::Green | Color::DarkGreen => "\x1b[32m".into(),
        Color::Yellow | Color::DarkYellow => "\x1b[33m".into(),
        Color::Blue | Color::DarkBlue => "\x1b[34m".into(),
        Color::Magenta | Color::DarkMagenta => "\x1b[35m".into(),
        Color::Cyan | Color::DarkCyan => "\x1b[36m".into(),
        Color::White | Color::Grey => "\x1b[37m".into(),
        Color::Rgb { r, g, b } => format!("\x1b[38;2;{r};{g};{b}m"),
        _ => "\x1b[39m".into(),
    }
}

fn ansi_reset() -> &'static str {
    "\x1b[0m"
}

pub fn print_onboarding_tips() -> anyhow::Result<()> {
    let mut stdout = io::stdout();
    let ansi = stdout.is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let cyan = if ansi {
        ansi_code(Color::Cyan)
    } else {
        String::new()
    };
    let muted = if ansi {
        ansi_code(Color::DarkGrey)
    } else {
        String::new()
    };
    let reset = if ansi { ansi_reset() } else { "" };
    writeln!(stdout, "  {cyan}Tips for getting started:{reset}")?;
    writeln!(
        stdout,
        "  {muted}1. Ask questions, edit files, or execute commands.{reset}"
    )?;
    writeln!(
        stdout,
        "  {muted}2. Use @file to attach project context.{reset}"
    )?;
    writeln!(
        stdout,
        "  {muted}3. Create UTHARNESS.md to define project instructions.{reset}"
    )?;
    writeln!(stdout, "  {muted}4. Use /help to explore commands.{reset}")?;
    stdout.flush()?;
    if ansi && std::env::var_os("UTHARNESS_REDUCED_MOTION").is_none() {
        let delay = std::env::var("UTHARNESS_STARTUP_SPLASH_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(650)
            .min(900);
        thread::sleep(Duration::from_millis(delay));
    }
    Ok(())
}

pub fn print_startup_banner(version: &str) -> anyhow::Result<()> {
    let width = terminal_width();
    let ansi = io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let rendered = render_banner(width, version, ansi);
    let mut stdout = io::stdout();
    if ansi && animation_enabled() && width >= 42 {
        for line in rendered.lines() {
            writeln!(stdout, "{line}")?;
            stdout.flush()?;
            thread::sleep(Duration::from_millis(12));
        }
    } else {
        write!(stdout, "{rendered}")?;
    }
    if ansi {
        execute!(stdout, ResetColor)?;
    }
    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_uthy_unicode_art_on_wide_terminal() {
        let output = render_banner_in_mode(120, "0.1.0", false, BannerMode::Unicode);
        assert!(output.contains(UNICODE_BANNER[0]));
        assert!(output.contains("U T H Y"));
        assert!(output.contains("AGENT TERMINAL"));
        assert!(output.contains("v0.1.0"));
        assert!(output.contains("░"));
        assert!(output.starts_with("    "));
    }

    #[test]
    fn falls_back_to_ascii_on_medium_terminal() {
        let output = render_banner_in_mode(70, "0.1.0", false, BannerMode::Ascii);
        assert!(output.contains(ASCII_BANNER[0]));
        assert!(!output.contains(UNICODE_BANNER[0]));
    }

    #[test]
    fn uses_compact_mode_for_narrow_terminal() {
        let output = render_banner_in_mode(40, "0.1.0", false, BannerMode::Compact);
        assert!(output.contains("UTHY"));
        assert!(output.contains("AGENT TERMINAL"));
        assert!(!output.contains(UNICODE_BANNER[0]));
    }

    #[test]
    fn ansi_output_contains_gradient_color_sequences() {
        let output = render_banner_in_mode(120, "0.1.0", true, BannerMode::Unicode);
        assert!(output.contains("\u{1b}[38;2;255;222;72m"));
        assert!(output.contains("\u{1b}[38;2;247;190;202m"));
        assert!(output.contains("\u{1b}[0m"));
    }

    #[test]
    fn wide_banner_stays_within_reduced_geometry() {
        assert_eq!(UNICODE_BANNER.len(), 4);
        assert!(UNICODE_BANNER.iter().all(|line| line.chars().count() <= 29));
        assert_eq!(ASCII_BANNER.len(), 3);
        assert!(ASCII_BANNER.iter().all(|line| line.chars().count() <= 25));
    }
}

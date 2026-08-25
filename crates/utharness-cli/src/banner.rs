use crossterm::{
    execute,
    style::{Color, ResetColor},
    terminal,
};
use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::Duration;

pub const UNICODE_BANNER: [&str; 6] = [
    "██╗   ██╗████████╗██╗  ██╗",
    "██║   ██║╚══██╔══╝██║  ██║",
    "██║   ██║   ██║   ███████║",
    "██║   ██║   ██║   ██╔══██║",
    "╚██████╔╝   ██║   ██║  ██║",
    " ╚═════╝    ╚═╝   ╚═╝  ╚═╝",
];

pub const ASCII_BANNER: [&str; 6] = [
    "UU   UU TTTTTTT HH  HH  AAA  RRRRR  NN  NN EEEEE SS",
    "UU   UU   TTT   HH  HH AAAAA RRRRR  NNN NN EE    SS",
    "UU   UU   TTT   HHHHHH AA AA RR RR  NNNNNN EEEE  SSS",
    "UU   UU   TTT   HH  HH AAAAA RRRR   NN NNN EE     SS",
    "UUUUUUU   TTT   HH  HH AA AA RR RR  NN  NN EEEEE SSSS",
    "                                                       ",
];

#[derive(Clone, Copy, Debug)]
pub struct BannerTheme {
    pub art: Color,
    pub wordmark: Color,
    pub subtitle: Color,
    pub version: Color,
}

impl Default for BannerTheme {
    fn default() -> Self {
        Self {
            art: Color::Cyan,
            wordmark: Color::Green,
            subtitle: Color::DarkGrey,
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
                art: Color::Cyan,
                wordmark: Color::Blue,
                subtitle: Color::DarkGrey,
                version: Color::Grey,
            },
            "ember" => Self {
                art: Color::Yellow,
                wordmark: Color::Red,
                subtitle: Color::DarkGrey,
                version: Color::Grey,
            },
            "mono-black" => Self {
                art: Color::White,
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

pub fn render_banner(width: u16, version: &str, ansi: bool) -> String {
    let mode = mode_for_width(width);
    let theme = BannerTheme::from_environment();
    let mut lines = Vec::new();
    match mode {
        BannerMode::Compact => {
            lines.push("UTHARNESS".to_string());
            lines.push("AGENT TERMINAL".to_string());
            lines.push(format!("v{version}"));
        }
        BannerMode::Unicode | BannerMode::Ascii => {
            let art = if mode == BannerMode::Unicode {
                &UNICODE_BANNER
            } else {
                &ASCII_BANNER
            };
            lines.extend(art.iter().map(|line| (*line).to_string()));
            lines.push("      U T H A R N E S S".to_string());
            lines.push("        AGENT TERMINAL".to_string());
            lines.push(format!("             v{version}"));
        }
    }
    let max_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for (index, line) in lines.iter().enumerate() {
        let pad = " ".repeat((width as usize).saturating_sub(max_width) / 2);
        let color = match mode {
            BannerMode::Compact if index == 0 => theme.wordmark,
            BannerMode::Compact if index == 1 => theme.subtitle,
            BannerMode::Compact => theme.version,
            _ if index < 6 => theme.art,
            _ if index == 6 => theme.wordmark,
            _ if index == 7 => theme.subtitle,
            _ => theme.version,
        };
        if ansi {
            out.push_str(&format!(
                "{}{}{}{}\n",
                pad,
                ansi_code(color),
                line,
                ansi_reset()
            ));
        } else {
            out.push_str(&pad);
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn ansi_code(color: Color) -> String {
    match color {
        Color::Black => "\x1b[30m".into(),
        Color::DarkGrey => "\x1b[90m".into(),
        Color::Red => "\x1b[31m".into(),
        Color::DarkRed => "\x1b[31m".into(),
        Color::Green => "\x1b[32m".into(),
        Color::DarkGreen => "\x1b[32m".into(),
        Color::Yellow => "\x1b[33m".into(),
        Color::DarkYellow => "\x1b[33m".into(),
        Color::Blue => "\x1b[34m".into(),
        Color::DarkBlue => "\x1b[34m".into(),
        Color::Magenta => "\x1b[35m".into(),
        Color::DarkMagenta => "\x1b[35m".into(),
        Color::Cyan => "\x1b[36m".into(),
        Color::DarkCyan => "\x1b[36m".into(),
        Color::White => "\x1b[37m".into(),
        Color::Grey => "\x1b[37m".into(),
        Color::Rgb { r, g, b } => format!("\x1b[38;2;{r};{g};{b}m"),
        _ => "\x1b[39m".into(),
    }
}

fn ansi_reset() -> &'static str {
    "\x1b[0m"
}

pub fn print_startup_banner(version: &str) -> anyhow::Result<()> {
    let width = terminal_width();
    let ansi = io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let rendered = render_banner(width, version, ansi);
    let animate = ansi && !reduced_motion() && width >= 42;
    let mut stdout = io::stdout();
    if animate {
        for line in rendered.lines() {
            for ch in line.chars() {
                write!(stdout, "{ch}")?;
                stdout.flush()?;
                thread::sleep(Duration::from_millis(3));
            }
            writeln!(stdout)?;
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
    fn renders_requested_unicode_art_on_wide_terminal() {
        let output = render_banner(120, "0.1.0", false);
        assert!(output.contains(UNICODE_BANNER[0]));
        assert!(output.contains("U T H A R N E S S"));
        assert!(output.contains("AGENT TERMINAL"));
        assert!(output.contains("v0.1.0"));
    }

    #[test]
    fn falls_back_to_ascii_on_medium_terminal() {
        let output = render_banner(70, "0.1.0", false);
        assert!(output.contains(ASCII_BANNER[0]));
        assert!(!output.contains(UNICODE_BANNER[0]));
    }

    #[test]
    fn uses_compact_mode_for_narrow_terminal() {
        let output = render_banner(40, "0.1.0", false);
        assert!(output.contains("UTHARNESS"));
        assert!(output.contains("AGENT TERMINAL"));
        assert!(!output.contains(UNICODE_BANNER[0]));
    }
}

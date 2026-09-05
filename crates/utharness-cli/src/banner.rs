use crossterm::terminal;
use std::io::{self, IsTerminal, Write};

const WORD: &str = "UTHARNESS";
/// The large wordmark is deliberately fixed-width: it is the supplied block-3D
/// design, sized to fit a 90-column terminal without horizontal scrolling.
const BLOCK_WORDMARK: [&str; 6] = [
    "██╗   ██╗████████╗██╗  ██╗ █████╗ ██████╗ ███╗   ██╗███████╗███████╗███████╗",
    "██║   ██║╚══██╔══╝██║  ██║██╔══██╗██╔══██╗████╗  ██║██╔════╝██╔════╝██╔════╝",
    "██║   ██║   ██║   ███████║███████║██████╔╝██╔██╗ ██║█████╗  ███████╗███████╗",
    "██║   ██║   ██║   ██╔══██║██╔══██║██╔══██╗██║╚██╗██║██╔══╝  ╚════██║╚════██║",
    "╚██████╔╝   ██║   ██║  ██║██║  ██║██║  ██║██║ ╚████║███████╗███████╗███████╗",
    " ╚═════╝    ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚══════╝╚══════╝╚══════╝",
];
const GRADIENT_START: (u8, u8, u8) = (34, 197, 94); // #22c55e
const GRADIENT_END: (u8, u8, u8) = (56, 189, 248); // #38bdf8
const COLORS: [(u8, u8, u8); 9] = [
    (180, 76, 255),
    (32, 214, 244),
    (85, 219, 36),
    (255, 210, 31),
    (255, 138, 22),
    (255, 61, 79),
    (52, 120, 246),
    (180, 76, 255),
    (180, 76, 255),
];
const GLYPHS: [[&str; 3]; 8] = [
    ["█ █", "█ █", "███"],
    ["███", " █ ", " █ "],
    ["█ █", "███", "█ █"],
    [" █ ", "███", "█ █"],
    ["██ ", "██ ", "█ █"],
    ["█ █", "███", "█ █"],
    ["███", "██ ", "███"],
    [" ██", " █ ", "██ "],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerLayout {
    Full,
    Compressed,
    Wrapped,
    Compact,
    Minimal,
    Hidden,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerPreference {
    Full,
    Compact,
    Minimal,
    Hidden,
}
impl BannerPreference {
    pub fn from_environment() -> Self {
        match std::env::var("UTHARNESS_BANNER")
            .unwrap_or_else(|_| "full".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "hide" | "hidden" | "off" | "false" => Self::Hidden,
            "minimal" => Self::Minimal,
            "compact" => Self::Compact,
            _ => Self::Full,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorDepth {
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

pub fn terminal_width() -> u16 {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .or_else(|| terminal::size().ok().map(|(w, _)| w))
        .unwrap_or(80)
        .max(1)
}
pub fn layout_for_width(width: u16, preference: BannerPreference) -> BannerLayout {
    if preference == BannerPreference::Hidden {
        return BannerLayout::Hidden;
    }
    if width < 40 || preference == BannerPreference::Minimal {
        return BannerLayout::Minimal;
    }
    if width < 60 || preference == BannerPreference::Compact {
        return BannerLayout::Compact;
    }
    if width < 90 {
        BannerLayout::Wrapped
    } else if width < 120 {
        BannerLayout::Compressed
    } else {
        BannerLayout::Full
    }
}
fn color_depth(ansi: bool) -> ColorDepth {
    if !ansi
        || std::env::var_os("NO_COLOR").is_some()
        || std::env::var("TERM").is_ok_and(|v| v == "dumb")
    {
        return ColorDepth::None;
    }
    if std::env::var("COLORTERM").is_ok_and(|v| v.contains("truecolor") || v.contains("24bit"))
        || std::env::var("UTHARNESS_COLOR").is_ok_and(|v| v == "truecolor")
    {
        ColorDepth::TrueColor
    } else if std::env::var("TERM").is_ok_and(|v| v.contains("256color"))
        || std::env::var("UTHARNESS_COLOR").is_ok_and(|v| v == "ansi256")
    {
        ColorDepth::Ansi256
    } else {
        ColorDepth::Ansi16
    }
}
fn start_color(index: usize, depth: ColorDepth) -> String {
    let (r, g, b) = COLORS[index.min(8)];
    match depth {
        ColorDepth::None => String::new(),
        ColorDepth::TrueColor => format!("\x1b[38;2;{r};{g};{b}m"),
        ColorDepth::Ansi256 => format!(
            "\x1b[38;5;{}m",
            [129, 45, 40, 220, 208, 203, 69, 129, 129][index.min(8)]
        ),
        ColorDepth::Ansi16 => format!(
            "\x1b[{}m",
            [35, 36, 32, 33, 33, 31, 34, 35, 35][index.min(8)]
        ),
    }
}
fn reset(depth: ColorDepth) -> &'static str {
    if depth == ColorDepth::None {
        ""
    } else {
        "\x1b[0m"
    }
}
fn paint(text: &str, index: usize, depth: ColorDepth) -> String {
    format!("{}{text}{}", start_color(index, depth), reset(depth))
}
fn gradient_color(column: usize, width: usize, depth: ColorDepth) -> String {
    let ratio = column as f32 / width.saturating_sub(1).max(1) as f32;
    let channel = |start: u8, end: u8| start as f32 + (end as f32 - start as f32) * ratio;
    let (r, g, b) = (
        channel(GRADIENT_START.0, GRADIENT_END.0).round() as u8,
        channel(GRADIENT_START.1, GRADIENT_END.1).round() as u8,
        channel(GRADIENT_START.2, GRADIENT_END.2).round() as u8,
    );
    match depth {
        ColorDepth::None => String::new(),
        ColorDepth::TrueColor => format!("\x1b[38;2;{r};{g};{b}m"),
        // Green → cyan/sky blue in the closest broadly supported palette.
        ColorDepth::Ansi256 => format!("\x1b[38;5;{}m", if ratio < 0.5 { 42 } else { 81 }),
        ColorDepth::Ansi16 => format!("\x1b[{}m", if ratio < 0.5 { 32 } else { 36 }),
    }
}
/// Paint one wordmark line as a horizontal ANSI gradient. Spaces deliberately
/// remain unpainted, matching CSS `background-clip: text` while keeping output
/// compact and preserving transparent terminal backgrounds.
fn gradient_wordmark_line(line: &str, depth: ColorDepth) -> String {
    if depth == ColorDepth::None {
        return line.into();
    }
    let width = line.chars().count();
    let mut output = String::with_capacity(line.len() * 3);
    for (column, character) in line.chars().enumerate() {
        if character == ' ' {
            output.push(character);
        } else {
            output.push_str(&gradient_color(column, width, depth));
            output.push(character);
        }
    }
    output.push_str(reset(depth));
    output
}
/// ANSI escape sequences have no terminal-cell width. This intentionally small
/// parser covers CSI SGR sequences emitted by the banner renderer.
fn visible_width(value: &str) -> usize {
    let mut characters = value.chars().peekable();
    let mut width = 0;
    while let Some(character) = characters.next() {
        if character == '\x1b' && characters.peek() == Some(&'[') {
            characters.next();
            while let Some(next) = characters.next() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}
fn centered_line(value: String, terminal_width: u16) -> String {
    let indent = (terminal_width as usize).saturating_sub(visible_width(&value)) / 2;
    format!("{}{}", " ".repeat(indent), value)
}
fn colored_word(depth: ColorDepth) -> String {
    WORD.chars()
        .enumerate()
        .map(|(i, c)| paint(&c.to_string(), i, depth))
        .collect()
}
fn art_line(row: usize, depth: ColorDepth, doubled: bool) -> String {
    WORD.chars()
        .enumerate()
        .map(|(i, c)| {
            let g = match c {
                'U' => 0,
                'T' => 1,
                'H' => 2,
                'A' => 3,
                'R' => 4,
                'N' => 5,
                'E' => 6,
                _ => 7,
            };
            let s = if doubled {
                GLYPHS[g][row].replace('█', "██")
            } else {
                GLYPHS[g][row].into()
            };
            format!("{}{} ", start_color(i, depth), s)
        })
        .collect::<String>()
        + reset(depth)
}
fn icons(ascii: bool) -> [&'static str; 7] {
    if ascii {
        ["[A]", "[M]", "[S]", "[C]", "[D]", "[T]", ">_"]
    } else {
        ["◉", "◇", "</>", "⎇", "▤", "⚒", ">_"]
    }
}

fn terminal_block_line(row: usize, depth: ColorDepth, ascii: bool) -> String {
    let (top, side, bottom) = if ascii {
        ("+----------+", "|", "+----------+")
    } else {
        ("╔══════════╗", "║", "╚══════════╝")
    };
    match row {
        0 => paint(top, 0, depth),
        2 => format!(
            "{}{} {}>_{}      {}{}",
            start_color(0, depth),
            side,
            start_color(2, depth),
            start_color(0, depth),
            side,
            reset(depth)
        ),
        5 => paint(bottom, 0, depth),
        _ => format!(
            "{}{}          {}{}",
            start_color(0, depth),
            side,
            side,
            reset(depth)
        ),
    }
}

pub fn render_banner(width: u16, version: &str, ansi: bool) -> String {
    render_banner_with(width, version, ansi, BannerPreference::from_environment())
}
pub fn render_banner_with(
    width: u16,
    version: &str,
    ansi: bool,
    preference: BannerPreference,
) -> String {
    let layout = layout_for_width(width, preference);
    if layout == BannerLayout::Hidden {
        return String::new();
    }
    let depth = color_depth(ansi);
    let ascii = std::env::var_os("UTHARNESS_ASCII").is_some()
        || std::env::var("TERM").is_ok_and(|v| v == "dumb");
    let mut lines = Vec::new();
    match layout {
        BannerLayout::Minimal => lines.push(format!(
            "{} {} v{version}",
            colored_word(depth),
            paint(">_", 2, depth)
        )),
        BannerLayout::Compact => {
            lines.push(format!(
                "+- {} {} -+",
                colored_word(depth),
                paint(">_", 2, depth)
            ));
            lines.push("[A] [M] [S] [C] [D] [T] >_".into());
            lines.push("AUTONOMOUS AGENT HARNESS".into());
        }
        BannerLayout::Wrapped | BannerLayout::Compressed | BannerLayout::Full => {
            // Keep borders and wordmark on one centered visual grid. The full
            // tier is 12-cell prompt block + two-cell gutter + 76-cell art.
            let content_width = match layout {
                BannerLayout::Full => 90,
                BannerLayout::Compressed => 76,
                BannerLayout::Wrapped => usize::min(width.saturating_sub(2) as usize, 58),
                _ => unreachable!(),
            };
            lines.push(centered_line("-".repeat(content_width), width));
            for row in 0..6 {
                let wordmark = if layout == BannerLayout::Wrapped {
                    // A 3-row fallback remains the only safely readable option
                    // below 90 columns.
                    art_line(row / 2, depth, false)
                } else {
                    gradient_wordmark_line(BLOCK_WORDMARK[row], depth)
                };
                let composed = if layout == BannerLayout::Full {
                    format!("{}  {wordmark}", terminal_block_line(row, depth, ascii))
                } else {
                    wordmark
                };
                lines.push(centered_line(composed, width));
            }
            lines.push(centered_line("-".repeat(content_width), width));
            let names = [
                "AGENTS", "MODELS", "SKILLS", "MCP", "MEMORY", "TOOLS", "TERMINAL",
            ];
            let glyphs = icons(ascii);
            let blocks = names
                .iter()
                .enumerate()
                .map(|(i, name)| paint(&format!("{} {name}", glyphs[i]), i, depth))
                .collect::<Vec<_>>();
            if layout == BannerLayout::Wrapped {
                lines.push(centered_line(blocks[..4].join("  "), width));
                lines.push(centered_line(blocks[4..].join("  "), width));
            } else {
                lines.push(centered_line(blocks.join("  "), width));
            }
            lines.push(centered_line(
                format!(
                    "{} AUTONOMOUS AI AGENT TERMINAL HARNESS {} v{version}",
                    paint(">", 2, depth),
                    paint("<", 2, depth)
                ),
                width,
            ));
        }
        BannerLayout::Hidden => {}
    }
    lines.into_iter().map(|line| format!("{line}\n")).collect()
}
pub fn print_onboarding_tips() -> anyhow::Result<()> {
    let mut out = io::stdout();
    writeln!(
        out,
        "Tips: ask questions · use @file for context · /help for commands"
    )?;
    out.flush()?;
    Ok(())
}
pub fn print_startup_banner(version: &str) -> anyhow::Result<()> {
    let mut out = io::stdout();
    if !out.is_terminal() {
        return Ok(());
    }
    write!(out, "{}", render_banner(terminal_width(), version, true))?;
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plain(w: u16) -> String {
        render_banner_with(w, "0.2.16", false, BannerPreference::Full)
    }
    #[test]
    fn maps_required_widths() {
        assert_eq!(
            [20, 30, 40, 60, 80, 100, 120, 160, 200]
                .map(|w| layout_for_width(w, BannerPreference::Full)),
            [
                BannerLayout::Minimal,
                BannerLayout::Minimal,
                BannerLayout::Compact,
                BannerLayout::Wrapped,
                BannerLayout::Wrapped,
                BannerLayout::Compressed,
                BannerLayout::Full,
                BannerLayout::Full,
                BannerLayout::Full
            ]
        );
    }
    #[test]
    fn every_plain_line_fits() {
        for w in [20, 30, 40, 60, 80, 100, 120, 160, 200] {
            assert!(
                plain(w)
                    .lines()
                    .all(|line| line.chars().count() <= w as usize),
                "overflow at {w}"
            );
        }
    }
    #[test]
    fn hierarchy_is_present() {
        let o = plain(120);
        assert!(o.contains("AGENTS"));
        assert!(o.contains("MODELS"));
        assert!(o.contains("AUTONOMOUS AI AGENT TERMINAL HARNESS"));
    }
    #[test]
    fn minimal_never_disappears() {
        assert!(plain(20).contains("UTHARNESS"));
        assert!(render_banner_with(120, "x", false, BannerPreference::Hidden).is_empty());
    }
    #[test]
    fn block_wordmark_uses_requested_geometry() {
        assert_eq!(BLOCK_WORDMARK.len(), 6);
        assert!(BLOCK_WORDMARK.iter().all(|line| line.chars().count() == 76));
        assert!(plain(90).contains(BLOCK_WORDMARK[0]));
    }
    #[test]
    fn full_banner_uses_one_centered_visual_grid() {
        let rendered = plain(120);
        let lines = rendered.lines().collect::<Vec<_>>();
        let separator = lines[0];
        let wordmark = lines[1];
        assert_eq!(visible_width(separator), 105);
        assert_eq!(separator.trim_start().chars().count(), 90);
        assert_eq!(separator.chars().take_while(|c| *c == ' ').count(), 15);
        assert_eq!(wordmark.chars().take_while(|c| *c == ' ').count(), 15);
        assert_eq!(visible_width(wordmark), 105);
    }
    #[test]
    fn truecolor_wordmark_uses_green_to_sky_gradient() {
        let rendered = gradient_wordmark_line(BLOCK_WORDMARK[0], ColorDepth::TrueColor);
        assert!(rendered.contains("38;2;34;197;94"));
        assert!(rendered.contains("38;2;56;189;248"));
    }
}

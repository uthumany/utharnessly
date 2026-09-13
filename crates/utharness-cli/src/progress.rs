//! Animated gradient progress bar on stderr.
//!
//! Mirrors the launcher's `@vr_patel/tui` ProgressBar contract (gradient
//! fill, percentage, item counts, ETA) for native CLI paths that cannot use
//! the Node package: installer downloads, long scans, bulk loads. Rendering
//! degrades with color depth: 24-bit gradient, 256-color steps, then ASCII
//! `#`/`-`. Under `NO_COLOR=1`, `TERM=dumb`, or `UTHARNESS_ASCII=1` only a
//! final `done in Xs` line prints. Reduced motion (`UTHARNESS_REDUCED_MOTION=1`)
//! refreshes at most twice per second.

use std::io::Write;
use std::time::{Duration, Instant};

/// Red → yellow → green stops, matching the launcher progress bar.
const STOPS: [(u8, u8, u8); 3] = [
    crate::icons::HEMATITE,
    (245, 197, 66),
    crate::icons::NILE_GREEN,
];

fn gradient(position: f64) -> (u8, u8, u8) {
    if position <= 0.0 {
        return STOPS[0];
    }
    if position >= 1.0 {
        return STOPS[2];
    }
    let segment = position * 2.0;
    let index = segment as usize;
    let mix = segment - index as f64;
    let (a, b) = (STOPS[index], STOPS[index + 1]);
    (
        (a.0 as f64 + (b.0 as f64 - a.0 as f64) * mix) as u8,
        (a.1 as f64 + (b.1 as f64 - a.1 as f64) * mix) as u8,
        (a.2 as f64 + (b.2 as f64 - a.2 as f64) * mix) as u8,
    )
}

fn color_depth() -> u8 {
    if crate::icons::ascii_mode() {
        return 0;
    }
    if std::env::var("UTHARNESS_COLOR").as_deref() == Ok("truecolor")
        || std::env::var("COLORTERM")
            .map(|v| v.contains("truecolor"))
            .unwrap_or(false)
    {
        return 24;
    }
    if std::env::var("UTHARNESS_COLOR").as_deref() == Ok("ansi256")
        || std::env::var("TERM")
            .map(|v| v.contains("256color"))
            .unwrap_or(false)
    {
        return 8;
    }
    4
}

fn ansi256(rgb: (u8, u8, u8)) -> u8 {
    16 + 36 * (rgb.0 / 51) + 6 * (rgb.1 / 51) + (rgb.2 / 51)
}

pub struct ProgressBar {
    title: String,
    total: u64,
    current: u64,
    width: usize,
    started: Instant,
    last_draw: Option<Instant>,
    finished: bool,
}

impl ProgressBar {
    pub fn new(title: impl Into<String>, total: u64) -> Self {
        let width = terminal_width().saturating_sub(45).clamp(10, 40);
        Self {
            title: title.into(),
            total: total.max(1),
            current: 0,
            width,
            started: Instant::now(),
            last_draw: None,
            finished: false,
        }
    }

    pub fn set(&mut self, current: u64) {
        self.current = current.min(self.total);
        self.maybe_draw(false);
    }

    pub fn finish(&mut self, message: &str) {
        self.current = self.total;
        self.maybe_draw(true);
        if !self.finished {
            self.finished = true;
            let elapsed = self.started.elapsed().as_secs_f64();
            eprintln!("{} done in {:.1}s", message, elapsed);
        }
    }

    fn maybe_draw(&mut self, force: bool) {
        if color_depth() == 0 {
            return;
        }
        let now = Instant::now();
        let interval = if std::env::var_os("UTHARNESS_REDUCED_MOTION").is_some() {
            Duration::from_millis(500)
        } else {
            Duration::from_millis(80) // ~12 FPS cap
        };
        if !force && self.last_draw.map(|t| now - t < interval).unwrap_or(false) {
            return;
        }
        self.last_draw = Some(now);
        let ratio = self.current as f64 / self.total as f64;
        let filled = (ratio * self.width as f64).round() as usize;
        let depth = color_depth();
        let mut bar = String::with_capacity(self.width + 16);
        for i in 0..self.width {
            if i < filled {
                let rgb = gradient(i as f64 / self.width as f64);
                if depth >= 24 {
                    bar.push_str(&format!("\x1b[38;2;{};{};{}m█\x1b[0m", rgb.0, rgb.1, rgb.2));
                } else if depth >= 8 {
                    bar.push_str(&format!("\x1b[38;5;{}m█\x1b[0m", ansi256(rgb)));
                } else {
                    bar.push('#');
                }
            } else {
                bar.push(if depth >= 4 { '▒' } else { '-' });
            }
        }
        let eta = if self.current > 0 && self.current < self.total {
            let per_item = self.started.elapsed().as_secs_f64() / self.current as f64;
            format!(" ETA {:.0}s", per_item * (self.total - self.current) as f64)
        } else {
            String::new()
        };
        eprint!(
            "\r{} [{}] {:>3}% {}/{}{}",
            self.title,
            bar,
            (ratio * 100.0) as u64,
            self.current,
            self.total,
            eta
        );
        let _ = std::io::stderr().flush();
        if force {
            eprintln!();
        }
    }
}

fn terminal_width() -> usize {
    crate::banner::terminal_width() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_walks_red_to_green() {
        assert_eq!(gradient(0.0), STOPS[0]);
        assert_eq!(gradient(1.0), STOPS[2]);
        let mid = gradient(0.5);
        assert_eq!(mid, STOPS[1]);
    }

    #[test]
    fn ascii_mode_reports_no_depth() {
        std::env::set_var("UTHARNESS_ASCII", "1");
        assert_eq!(color_depth(), 0);
        std::env::remove_var("UTHARNESS_ASCII");
    }

    #[test]
    fn progress_clamps_without_panic() {
        let mut bar = ProgressBar::new("test", 10);
        bar.set(99);
        assert_eq!(bar.current, 10);
    }
}

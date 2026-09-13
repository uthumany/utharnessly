//! Egyptian hieroglyph feature icons with plain-text fallbacks.
//!
//! Six implemented features each get one hieroglyph identity (Unicode
//! Egyptian Hieroglyphs block, 5.2+). Terminals render at cell size, not
//! pixels: callers show the full glyph in headers and the same single cell
//! in tight lists. When `NO_COLOR=1`, `TERM=dumb`, or `UTHARNESS_ASCII=1`
//! is set, callers get bracketed ASCII tags instead of tofu boxes.
//!
//! Icons print WHITE by default (divine-glyph convention); pass an
//! explicit color to use a feature triad instead.

/// Unified palette roles (gold leaf, hematite, nile). The full five-role
/// table (gold leaf #D4AF37, lapis #1D3557, hematite #E63946, papyrus
/// #F5E6D3, nile #2A9D8F) lives in the TUI `divine` export and module docs;
/// only wired roles become constants here so no constant is dead.
pub const GOLD_LEAF: (u8, u8, u8) = (212, 175, 55); // #D4AF37
pub const HEMATITE: (u8, u8, u8) = (230, 57, 70); // #E63946
pub const NILE_GREEN: (u8, u8, u8) = (42, 157, 143); // #2A9D8F
/// Per-feature triad accents (computer gold, scroll ochre, plan terracotta,
/// adze sand). Improve reuses gold leaf; test reuses hematite.
pub const COMPUTER_GOLD: (u8, u8, u8) = (196, 163, 90); // #C4A35A
pub const SCROLL_OCHRE: (u8, u8, u8) = (181, 137, 0); // #B58900
pub const PLAN_TERRA: (u8, u8, u8) = (224, 122, 95); // #E07A5F
pub const ADZE_SAND: (u8, u8, u8) = (233, 196, 106); // #E9C46A
/// Default icon ink: white glyphs on dark terminals.
pub const GLYPH_WHITE: (u8, u8, u8) = (255, 255, 255);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    Computer,
    Improve,
    Remember,
    Build,
    Test,
    Fix,
}

impl Feature {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Computer => "\u{13080}", // 𓂀 Eye of Horus
            Self::Improve => "\u{132F9}",  // 𓋹 Ankh
            Self::Remember => "\u{133DC}", // 𓏞 Papyrus scroll
            Self::Build => "\u{13250}",    // 𓉐 House/plan
            Self::Test => "\u{13123}",     // 𓄣 Heart
            Self::Fix => "\u{13349}",      // 𓌙 Adze on block
        }
    }

    pub fn fallback(self) -> &'static str {
        match self {
            Self::Computer => "[eye]",
            Self::Improve => "[ankh]",
            Self::Remember => "[scroll]",
            Self::Build => "[build]",
            Self::Test => "[test]",
            Self::Fix => "[fix]",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Computer => "computer use",
            Self::Improve => "self-improvement",
            Self::Remember => "persistent memory",
            Self::Build => "build",
            Self::Test => "test",
            Self::Fix => "fix loop",
        }
    }

    pub fn accent(self) -> (u8, u8, u8) {
        match self {
            Self::Computer => COMPUTER_GOLD,
            Self::Improve => GOLD_LEAF,
            Self::Remember => SCROLL_OCHRE,
            Self::Build => PLAN_TERRA,
            Self::Test => HEMATITE,
            Self::Fix => ADZE_SAND,
        }
    }
}

/// Plain-text mode when glyphs would render as tofu.
pub fn ascii_mode() -> bool {
    std::env::var_os("NO_COLOR").is_some()
        || std::env::var("TERM")
            .map(|term| term == "dumb")
            .unwrap_or(false)
        || std::env::var("UTHARNESS_ASCII")
            .map(|v| v == "1")
            .unwrap_or(false)
}

/// The display cell for a feature: glyph, or ASCII tag in plain mode.
pub fn cell(feature: Feature) -> &'static str {
    if ascii_mode() {
        feature.fallback()
    } else {
        feature.glyph()
    }
}

/// Wrap `text` in a 24-bit foreground escape. In plain mode, or when stdout
/// is piped (not a terminal), the text passes through undecorated so logs
/// and captures stay clean.
pub fn ink(text: &str, rgb: (u8, u8, u8)) -> String {
    use std::io::IsTerminal;
    if ascii_mode() || !std::io::stdout().is_terminal() {
        return text.to_string();
    }
    format!("\x1b[38;2;{};{};{}m{text}\x1b[0m", rgb.0, rgb.1, rgb.2)
}

/// Feature icon in white by the default convention.
pub fn icon(feature: Feature) -> String {
    ink(cell(feature), GLYPH_WHITE)
}

/// Feature icon in its triad accent for stage headers on live terminals.
pub fn icon_accent(feature: Feature) -> String {
    ink(cell(feature), feature.accent())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_are_single_hieroglyph_cells() {
        for feature in [
            Feature::Computer,
            Feature::Improve,
            Feature::Remember,
            Feature::Build,
            Feature::Test,
            Feature::Fix,
        ] {
            assert_eq!(feature.glyph().chars().count(), 1, "{feature:?}");
            assert!(feature.fallback().starts_with('['));
        }
    }

    #[test]
    fn plain_mode_passes_text_through_undecorated() {
        std::env::set_var("UTHARNESS_ASCII", "1");
        assert_eq!(cell(Feature::Computer), "[eye]");
        assert_eq!(ink("x", GLYPH_WHITE), "x");
        std::env::remove_var("UTHARNESS_ASCII");
        assert_eq!(cell(Feature::Computer), "\u{13080}");
    }
}

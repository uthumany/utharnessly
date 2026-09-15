//! Truthful response-pipeline feedback for interactive native chat.
//!
//! The percentage measures only local, observable request lifecycle steps. It
//! never estimates remote token generation or an ETA for model inference.

use std::io::{self, IsTerminal, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseStage {
    Understanding,
    Decomposing,
    Retrieving,
    Grounding,
    Planning,
    Reasoning,
    Routing,
    Executing,
    Observing,
    Evaluating,
    Synthesizing,
    Verifying,
    Refining,
    Finalizing,
    Responding,
}

impl ResponseStage {
    #[cfg(test)]
    pub const ALL: [Self; 15] = [
        Self::Understanding,
        Self::Decomposing,
        Self::Retrieving,
        Self::Grounding,
        Self::Planning,
        Self::Reasoning,
        Self::Routing,
        Self::Executing,
        Self::Observing,
        Self::Evaluating,
        Self::Synthesizing,
        Self::Verifying,
        Self::Refining,
        Self::Finalizing,
        Self::Responding,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Understanding => "Understanding",
            Self::Decomposing => "Decomposing",
            Self::Retrieving => "Retrieving",
            Self::Grounding => "Grounding",
            Self::Planning => "Planning",
            Self::Reasoning => "Reasoning",
            Self::Routing => "Routing",
            Self::Executing => "Executing",
            Self::Observing => "Observing",
            Self::Evaluating => "Evaluating",
            Self::Synthesizing => "Synthesizing",
            Self::Verifying => "Verifying",
            Self::Refining => "Refining",
            Self::Finalizing => "Finalizing",
            Self::Responding => "Responding",
        }
    }

    pub const fn percent(self) -> u8 {
        match self {
            Self::Understanding => 0,
            Self::Decomposing => 7,
            Self::Retrieving => 14,
            Self::Grounding => 21,
            Self::Planning => 28,
            Self::Reasoning => 35,
            Self::Routing => 42,
            Self::Executing => 49,
            Self::Observing => 63,
            Self::Evaluating => 72,
            Self::Synthesizing => 80,
            Self::Verifying => 88,
            Self::Refining => 94,
            Self::Finalizing => 98,
            Self::Responding => 100,
        }
    }

    pub const fn detail(self) -> &'static str {
        match self {
            Self::Reasoning => "provider is working; token progress is not measurable",
            Self::Executing => "request is running",
            Self::Observing => "provider output received",
            Self::Responding => "response complete",
            _ => "local request lifecycle",
        }
    }
}

pub struct ResponsePipeline {
    enabled: bool,
    last_percent: u8,
    finished: bool,
}

impl ResponsePipeline {
    pub fn new() -> Self {
        Self {
            enabled: io::stderr().is_terminal() && io::stdout().is_terminal(),
            last_percent: 0,
            finished: false,
        }
    }

    pub fn advance(&mut self, stage: ResponseStage) {
        let percent = stage.percent();
        if !self.enabled || self.finished || percent < self.last_percent {
            return;
        }
        self.last_percent = percent;
        let width = 14usize;
        let filled = (usize::from(percent) * width).div_ceil(100);
        let (fill, empty) = if crate::icons::ascii_mode() {
            ('#', '-')
        } else {
            ('█', '░')
        };
        let bar = format!(
            "{}{}",
            fill.to_string().repeat(filled),
            empty.to_string().repeat(width - filled)
        );
        eprint!(
            "\r{} UTHARNESS [{}] {:>3}% {} · {}",
            crate::icons::icon_agent(),
            bar,
            percent,
            stage.label(),
            stage.detail(),
        );
        let _ = io::stderr().flush();
        if stage == ResponseStage::Responding {
            self.finished = true;
            eprintln!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_stages_are_complete_and_ordered() {
        let labels: Vec<_> = ResponseStage::ALL
            .iter()
            .map(|stage| stage.label())
            .collect();
        assert_eq!(labels.len(), 15);
        assert_eq!(labels.first(), Some(&"Understanding"));
        assert_eq!(labels.last(), Some(&"Responding"));
        assert_eq!(ResponseStage::Understanding.percent(), 0);
        assert_eq!(ResponseStage::Responding.percent(), 100);
        assert!(ResponseStage::ALL
            .windows(2)
            .all(|pair| pair[0].percent() < pair[1].percent()));
    }
}

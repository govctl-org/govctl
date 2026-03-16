//! Unified semantic color theme.
//!
//! Single source of truth for status/phase color mappings.
//! All rendering backends (owo-colors, comfy-table, ratatui) adapt from this.
//!
//! Implements [[ADR-0005]] color scheme.

#![allow(dead_code)] // Variants/functions are used across feature-gated modules

/// Semantic color intent, independent of rendering backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticColor {
    /// Positive terminal states: normative, accepted, done, active, stable
    Success,
    /// In-progress / pending states: draft, proposed, queue, spec
    Warning,
    /// Ended / inactive states: deprecated, superseded, cancelled
    Muted,
    /// Informational accents: IDs, paths, test phase
    Info,
    /// Secondary accent: impl phase
    Accent,
    /// Default text, no special coloring
    Neutral,
}

/// Map an artifact status string to its semantic color.
pub fn status_semantic(status: &str) -> SemanticColor {
    match status {
        "normative" | "accepted" | "done" | "active" => SemanticColor::Success,
        "draft" | "proposed" | "queue" => SemanticColor::Warning,
        "deprecated" | "superseded" | "cancelled" => SemanticColor::Muted,
        _ => SemanticColor::Neutral,
    }
}

/// Map an RFC phase string to its semantic color.
pub fn phase_semantic(phase: &str) -> SemanticColor {
    match phase {
        "stable" => SemanticColor::Success,
        "test" => SemanticColor::Info,
        "impl" => SemanticColor::Accent,
        "spec" => SemanticColor::Warning,
        _ => SemanticColor::Neutral,
    }
}

/// Unicode status icon for display.
pub fn status_icon(status: &str) -> &'static str {
    match status {
        "normative" | "accepted" | "done" => "●",
        "active" => "◉",
        "draft" | "proposed" | "queue" => "○",
        "deprecated" | "superseded" | "cancelled" => "✗",
        _ => "•",
    }
}

// -- Backend adapters --------------------------------------------------------

impl SemanticColor {
    /// Convert to owo-colors ANSI color (for stderr/stdout CLI output).
    pub fn to_owo(self) -> owo_colors::AnsiColors {
        match self {
            Self::Success => owo_colors::AnsiColors::Green,
            Self::Warning => owo_colors::AnsiColors::Yellow,
            Self::Muted => owo_colors::AnsiColors::BrightBlack,
            Self::Info => owo_colors::AnsiColors::Cyan,
            Self::Accent => owo_colors::AnsiColors::Blue,
            Self::Neutral => owo_colors::AnsiColors::Default,
        }
    }

    /// Convert to comfy-table color (for table cell formatting).
    pub fn to_comfy(self) -> comfy_table::Color {
        match self {
            Self::Success => comfy_table::Color::Green,
            Self::Warning => comfy_table::Color::Yellow,
            Self::Muted => comfy_table::Color::DarkGrey,
            Self::Info => comfy_table::Color::Cyan,
            Self::Accent => comfy_table::Color::Blue,
            Self::Neutral => comfy_table::Color::White,
        }
    }

    /// Convert to ratatui color (for TUI widget styling).
    #[cfg(feature = "tui")]
    pub fn to_ratatui(self) -> ratatui::style::Color {
        match self {
            Self::Success => ratatui::style::Color::Green,
            Self::Warning => ratatui::style::Color::Yellow,
            Self::Muted => ratatui::style::Color::DarkGray,
            Self::Info => ratatui::style::Color::Cyan,
            Self::Accent => ratatui::style::Color::Blue,
            Self::Neutral => ratatui::style::Color::Reset,
        }
    }
}

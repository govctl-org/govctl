//! CLI output formatting with colors.
//!
//! Implements [[ADR-0005]] CLI output color scheme and formatting.
//!
//! Provides consistent, colorized output for all CLI commands.
//! Colors auto-disable when output is not a TTY (agent-friendly).

mod color;
mod diagnostics;
mod messages;

pub use color::{path_str, stdout_supports_color};
pub use diagnostics::diagnostic;
pub use messages::*;

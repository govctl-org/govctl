//! Diagnostic message type and error formatting.

mod code;

pub use self::code::{DiagnosticCode, DiagnosticLevel};

use std::fmt;

/// A diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub file: String,
    pub level: DiagnosticLevel,
}

pub type Diagnostics = Vec<Diagnostic>;
pub type DiagnosticResult<T> = Result<T, Diagnostic>;

impl Diagnostic {
    pub fn new(code: DiagnosticCode, message: impl Into<String>, file: impl Into<String>) -> Self {
        Self {
            level: code.level(),
            code,
            message: message.into(),
            file: file.into(),
        }
    }

    pub fn io_error(
        action: impl fmt::Display,
        err: impl fmt::Display,
        file: impl Into<String>,
    ) -> Self {
        Self::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to {action}: {err}"),
            file,
        )
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level_str = match self.level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Info => "info",
        };
        write!(
            f,
            "{}[{}]: {} ({})",
            level_str,
            self.code.code(),
            self.message,
            self.file
        )
    }
}

impl std::error::Error for Diagnostic {}

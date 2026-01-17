//! Diagnostic codes and error reporting.

use std::fmt;

/// Diagnostic severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
}

/// Diagnostic error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DiagnosticCode {
    // RFC errors (E01xx)
    E0101RfcSchemaInvalid,
    E0102RfcNotFound,
    E0103RfcIdMismatch,
    E0104RfcInvalidTransition,
    E0105RfcRefNotFound,
    E0106RfcSupersedesNotFound,

    // Clause errors (E02xx)
    E0201ClauseSchemaInvalid,
    E0202ClauseNotFound,
    E0203ClauseIdMismatch,
    E0204ClausePathInvalid,
    E0205ClauseDuplicate,
    E0206ClauseSupersededByUnknown,
    E0207ClauseSupersededByNotActive,

    // ADR errors (E03xx)
    E0301AdrSchemaInvalid,
    E0302AdrNotFound,
    E0303AdrInvalidTransition,
    E0304AdrRefNotFound,

    // Work Item errors (E04xx)
    E0401WorkSchemaInvalid,
    E0402WorkNotFound,
    E0403WorkInvalidTransition,
    E0404WorkRefNotFound,

    // Config errors (E05xx)
    E0501ConfigInvalid,
    E0502PathNotFound,

    // General errors (E09xx)
    E0901IoError,
    E0902JsonParseError,
    E0903YamlParseError,

    // Signature errors (E06xx)
    E0601SignatureMismatch,
    E0602SignatureMissing,

    // Warnings (W01xx)
    W0101RfcNoChangelog,
    W0102ClauseNoSince,
    W0103AdrNoRefs,
    W0104AdrParseSkipped,
    W0105WorkParseSkipped,
    W0106RenderedReadError,
}

impl DiagnosticCode {
    pub fn level(&self) -> DiagnosticLevel {
        match self {
            Self::W0101RfcNoChangelog
            | Self::W0102ClauseNoSince
            | Self::W0103AdrNoRefs
            | Self::W0104AdrParseSkipped
            | Self::W0105WorkParseSkipped
            | Self::W0106RenderedReadError => DiagnosticLevel::Warning,
            _ => DiagnosticLevel::Error,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::E0101RfcSchemaInvalid => "E0101",
            Self::E0102RfcNotFound => "E0102",
            Self::E0103RfcIdMismatch => "E0103",
            Self::E0104RfcInvalidTransition => "E0104",
            Self::E0105RfcRefNotFound => "E0105",
            Self::E0106RfcSupersedesNotFound => "E0106",
            Self::E0201ClauseSchemaInvalid => "E0201",
            Self::E0202ClauseNotFound => "E0202",
            Self::E0203ClauseIdMismatch => "E0203",
            Self::E0204ClausePathInvalid => "E0204",
            Self::E0205ClauseDuplicate => "E0205",
            Self::E0206ClauseSupersededByUnknown => "E0206",
            Self::E0207ClauseSupersededByNotActive => "E0207",
            Self::E0301AdrSchemaInvalid => "E0301",
            Self::E0302AdrNotFound => "E0302",
            Self::E0303AdrInvalidTransition => "E0303",
            Self::E0304AdrRefNotFound => "E0304",
            Self::E0401WorkSchemaInvalid => "E0401",
            Self::E0402WorkNotFound => "E0402",
            Self::E0403WorkInvalidTransition => "E0403",
            Self::E0404WorkRefNotFound => "E0404",
            Self::E0501ConfigInvalid => "E0501",
            Self::E0502PathNotFound => "E0502",
            Self::E0601SignatureMismatch => "E0601",
            Self::E0602SignatureMissing => "E0602",
            Self::E0901IoError => "E0901",
            Self::E0902JsonParseError => "E0902",
            Self::E0903YamlParseError => "E0903",
            Self::W0101RfcNoChangelog => "W0101",
            Self::W0102ClauseNoSince => "W0102",
            Self::W0103AdrNoRefs => "W0103",
            Self::W0104AdrParseSkipped => "W0104",
            Self::W0105WorkParseSkipped => "W0105",
            Self::W0106RenderedReadError => "W0106",
        }
    }
}

/// A diagnostic message
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub file: String,
    pub level: DiagnosticLevel,
}

impl Diagnostic {
    pub fn new(code: DiagnosticCode, message: impl Into<String>, file: impl Into<String>) -> Self {
        Self {
            level: code.level(),
            code,
            message: message.into(),
            file: file.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level_str = match self.level {
            DiagnosticLevel::Error => "error",
            DiagnosticLevel::Warning => "warning",
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

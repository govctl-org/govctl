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
    E0107SourceRefUnknown,
    E0108RfcBumpRequiresSummary,
    E0109RfcAlreadyExists,
    E0110RfcInvalidId,
    E0111RfcNoChangelog,

    // Clause errors (E02xx)
    E0201ClauseSchemaInvalid,
    E0202ClauseNotFound,
    E0203ClauseIdMismatch,
    E0204ClausePathInvalid,
    E0205ClauseDuplicate,
    E0206ClauseSupersededByUnknown,
    E0207ClauseSupersededByNotActive,
    E0208ClauseAlreadyDeprecated,
    E0209ClauseAlreadySuperseded,
    E0210ClauseInvalidIdFormat,

    // ADR errors (E03xx)
    E0301AdrSchemaInvalid,
    E0302AdrNotFound,
    E0303AdrInvalidTransition,
    E0304AdrRefNotFound,
    E0305AdrCannotDeprecate,

    // Work Item errors (E04xx)
    E0401WorkSchemaInvalid,
    E0402WorkNotFound,
    E0403WorkInvalidTransition,
    E0404WorkRefNotFound,
    E0405WorkDirNotFound,
    E0406WorkAmbiguousMatch,
    E0407WorkMissingCriteria,
    E0408WorkCriteriaMissingCategory,

    // Config errors (E05xx)
    E0501ConfigInvalid,
    E0502PathNotFound,

    // Signature errors (E06xx)
    E0601SignatureMismatch,
    E0602SignatureMissing,

    // Release errors (E07xx)
    E0701ReleaseInvalidSemver,
    E0702ReleaseDuplicate,
    E0703ReleaseNoUnreleasedItems,

    // CLI/Command errors (E08xx)
    E0801MissingRequiredArg,
    E0802ConflictingArgs,
    E0803UnknownField,
    E0804FieldNotEditable,
    E0805EmptyValue,
    E0806InvalidPattern,
    E0807AmbiguousMatch,
    E0808InvalidPrefix,
    E0809ChoreNotAllowed,
    E0810CannotAddToField,
    E0811CannotRemoveFromField,
    E0812FieldEmpty,
    E0813SupersedeNotSupported,

    // General errors (E09xx)
    E0901IoError,
    E0902JsonParseError,
    E0903YamlParseError,

    // Warnings (W01xx)
    W0101RfcNoChangelog,
    W0102ClauseNoSince,
    W0103AdrNoRefs,
    W0104AdrParseSkipped,
    W0105WorkParseSkipped,
    W0106RenderedReadError,
    W0107SourceRefOutdated,
    W0108WorkPlaceholderDescription,
}

impl DiagnosticCode {
    pub fn level(&self) -> DiagnosticLevel {
        match self {
            Self::W0101RfcNoChangelog
            | Self::W0102ClauseNoSince
            | Self::W0103AdrNoRefs
            | Self::W0104AdrParseSkipped
            | Self::W0105WorkParseSkipped
            | Self::W0106RenderedReadError
            | Self::W0107SourceRefOutdated
            | Self::W0108WorkPlaceholderDescription => DiagnosticLevel::Warning,
            _ => DiagnosticLevel::Error,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            // E01xx - RFC
            Self::E0101RfcSchemaInvalid => "E0101",
            Self::E0102RfcNotFound => "E0102",
            Self::E0103RfcIdMismatch => "E0103",
            Self::E0104RfcInvalidTransition => "E0104",
            Self::E0105RfcRefNotFound => "E0105",
            Self::E0106RfcSupersedesNotFound => "E0106",
            Self::E0107SourceRefUnknown => "E0107",
            Self::E0108RfcBumpRequiresSummary => "E0108",
            Self::E0109RfcAlreadyExists => "E0109",
            Self::E0110RfcInvalidId => "E0110",
            Self::E0111RfcNoChangelog => "E0111",
            // E02xx - Clause
            Self::E0201ClauseSchemaInvalid => "E0201",
            Self::E0202ClauseNotFound => "E0202",
            Self::E0203ClauseIdMismatch => "E0203",
            Self::E0204ClausePathInvalid => "E0204",
            Self::E0205ClauseDuplicate => "E0205",
            Self::E0206ClauseSupersededByUnknown => "E0206",
            Self::E0207ClauseSupersededByNotActive => "E0207",
            Self::E0208ClauseAlreadyDeprecated => "E0208",
            Self::E0209ClauseAlreadySuperseded => "E0209",
            Self::E0210ClauseInvalidIdFormat => "E0210",
            // E03xx - ADR
            Self::E0301AdrSchemaInvalid => "E0301",
            Self::E0302AdrNotFound => "E0302",
            Self::E0303AdrInvalidTransition => "E0303",
            Self::E0304AdrRefNotFound => "E0304",
            Self::E0305AdrCannotDeprecate => "E0305",
            // E04xx - Work Item
            Self::E0401WorkSchemaInvalid => "E0401",
            Self::E0402WorkNotFound => "E0402",
            Self::E0403WorkInvalidTransition => "E0403",
            Self::E0404WorkRefNotFound => "E0404",
            Self::E0405WorkDirNotFound => "E0405",
            Self::E0406WorkAmbiguousMatch => "E0406",
            Self::E0407WorkMissingCriteria => "E0407",
            Self::E0408WorkCriteriaMissingCategory => "E0408",
            // E05xx - Config
            Self::E0501ConfigInvalid => "E0501",
            Self::E0502PathNotFound => "E0502",
            // E06xx - Signature
            Self::E0601SignatureMismatch => "E0601",
            Self::E0602SignatureMissing => "E0602",
            // E07xx - Release
            Self::E0701ReleaseInvalidSemver => "E0701",
            Self::E0702ReleaseDuplicate => "E0702",
            Self::E0703ReleaseNoUnreleasedItems => "E0703",
            // E08xx - CLI/Command
            Self::E0801MissingRequiredArg => "E0801",
            Self::E0802ConflictingArgs => "E0802",
            Self::E0803UnknownField => "E0803",
            Self::E0804FieldNotEditable => "E0804",
            Self::E0805EmptyValue => "E0805",
            Self::E0806InvalidPattern => "E0806",
            Self::E0807AmbiguousMatch => "E0807",
            Self::E0808InvalidPrefix => "E0808",
            Self::E0809ChoreNotAllowed => "E0809",
            Self::E0810CannotAddToField => "E0810",
            Self::E0811CannotRemoveFromField => "E0811",
            Self::E0812FieldEmpty => "E0812",
            Self::E0813SupersedeNotSupported => "E0813",
            // E09xx - General
            Self::E0901IoError => "E0901",
            Self::E0902JsonParseError => "E0902",
            Self::E0903YamlParseError => "E0903",
            // W01xx - Warnings
            Self::W0101RfcNoChangelog => "W0101",
            Self::W0102ClauseNoSince => "W0102",
            Self::W0103AdrNoRefs => "W0103",
            Self::W0104AdrParseSkipped => "W0104",
            Self::W0105WorkParseSkipped => "W0105",
            Self::W0106RenderedReadError => "W0106",
            Self::W0107SourceRefOutdated => "W0107",
            Self::W0108WorkPlaceholderDescription => "W0108",
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

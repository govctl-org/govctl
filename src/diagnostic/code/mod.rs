//! Diagnostic code catalog.

mod metadata;

/// Diagnostic severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
}

/// Diagnostic error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// RFC refs or [[...]] targets ADR/WI — violates [[RFC-0000:C-REFERENCE-HIERARCHY]]
    E0112RfcReferenceHierarchy,
    E0113RfcBumpNoAmendment,
    E0114RfcPendingAmendment,

    // Clause errors (E02xx)
    E0201ClauseSchemaInvalid,
    E0202ClauseNotFound,
    E0203ClauseIdMismatch,
    E0204ClausePathInvalid,
    E0206ClauseSupersededByUnknown,
    E0207ClauseSupersededByNotActive,
    E0208ClauseAlreadyDeprecated,
    E0209ClauseAlreadySuperseded,
    E0210ClauseInvalidIdFormat,
    E0211ClauseStillReferenced,
    E0212ClauseSupersessionCycle,
    E0213ClauseSupersededByMissing,

    // ADR errors (E03xx)
    E0301AdrSchemaInvalid,
    E0302AdrNotFound,
    E0303AdrInvalidTransition,
    E0304AdrRefNotFound,
    E0305AdrCannotDeprecate,
    /// ADR refs or [[...]] targets WI-* — violates [[RFC-0000:C-REFERENCE-HIERARCHY]]
    E0306AdrReferenceHierarchy,
    E0307AdrProjectionConflict,

    // Work Item errors (E04xx)
    E0401WorkSchemaInvalid,
    E0402WorkNotFound,
    E0403WorkInvalidTransition,
    E0404WorkRefNotFound,
    E0405WorkDirNotFound,
    E0406WorkAmbiguousMatch,
    E0407WorkMissingCriteria,
    E0408WorkCriteriaMissingCategory,
    E0409WorkDependencyInvalid,
    E0410WorkDependencyNotFound,
    E0411WorkDependencyCycle,

    // Config errors (E05xx)
    E0501ConfigInvalid,
    E0502PathNotFound,
    E0503LockTimeout,
    E0504PathConflict,
    E0505MigrationRequired,

    // Signature errors (E06xx)
    E0601SignatureMismatch,
    E0602SignatureMissing,

    // Release errors (E07xx)
    E0701ReleaseInvalidSemver,
    E0702ReleaseDuplicate,
    E0703ReleaseNoUnreleasedItems,
    E0704ReleaseSchemaInvalid,
    E0705ReleaseRefNotFound,
    E0706ReleaseWorkNotDone,
    E0707ReleaseWorkDuplicate,
    E0708ReleaseHistoryEmpty,
    E0709ReleaseLatestMismatch,

    // Verification Guard errors (E10xx)
    E1001GuardSchemaInvalid,
    E1002GuardNotFound,
    E1003GuardDuplicate,
    E1004GuardCheckFailed,
    E1005GuardTimeout,
    E1006GuardInvalidTitle,
    E1007GuardStillReferenced,

    // Tag errors (E11xx)
    /// Tag format is invalid (must match ^[a-z][a-z0-9-]*$)
    E1101TagInvalidFormat,
    /// Tag already exists in config.toml [tags] allowed
    E1102TagAlreadyExists,
    /// Tag not found in config.toml [tags] allowed
    E1103TagNotFound,
    /// Tag is still referenced by one or more artifacts
    E1104TagStillReferenced,
    /// Artifact uses a tag not in config.toml [tags] allowed — per [[RFC-0002:C-RESOURCES]]
    E1105TagUnknown,

    // Loop state errors (E12xx)
    E1201LoopStateInvalid,
    E1202LoopStateNotFound,
    E1203LoopInvalidTransition,
    E1204LoopInvalidId,
    E1205LoopDependencyNotFound,
    E1206LoopDependencyCycle,
    E1208LoopResumeAmbiguous,
    E1209LoopWorkMismatch,
    E1210LoopExecutionFailed,

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
    E0814InvalidPath,
    E0815PathFieldNotFound,
    E0816PathIndexOutOfBounds,
    E0817PathTypeMismatch,
    E0818PathIndexConflict,
    E0819UnknownArtifactType,
    E0820InvalidFieldValue,
    E0821InvalidCommandScope,
    E0822UnsupportedOperation,

    // General errors (E09xx)
    E0901IoError,
    E0902JsonParseError,
    E0903UnexpectedError,

    // Warnings (W01xx)
    W0101RfcNoChangelog,
    W0102ClauseNoSince,
    W0103AdrNoRefs,
    W0106RenderedReadError,
    W0107SourceRefOutdated,
    W0108WorkPlaceholderDescription,
    W0109WorkNoActive,
    W0110SchemaOutdated,
    W0111ProjectSupportOutdated,
    /// Known artifact ID appears in governed prose without [[...]] syntax.
    W0112BareArtifactReference,

    // Informational diagnostics (I04xx)
    I0401WorkLegacyInlineHistory,
}

impl DiagnosticCode {
    pub fn level(&self) -> DiagnosticLevel {
        metadata::level(self)
    }

    pub fn code(&self) -> &'static str {
        metadata::code(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticCode, DiagnosticLevel};

    #[test]
    fn code_strings_match_representative_catalog_ids() {
        assert_eq!(DiagnosticCode::E0101RfcSchemaInvalid.code(), "E0101");
        assert_eq!(DiagnosticCode::E0201ClauseSchemaInvalid.code(), "E0201");
        assert_eq!(DiagnosticCode::E0301AdrSchemaInvalid.code(), "E0301");
        assert_eq!(DiagnosticCode::E0401WorkSchemaInvalid.code(), "E0401");
        assert_eq!(DiagnosticCode::E0501ConfigInvalid.code(), "E0501");
        assert_eq!(DiagnosticCode::E0701ReleaseInvalidSemver.code(), "E0701");
        assert_eq!(DiagnosticCode::E0709ReleaseLatestMismatch.code(), "E0709");
        assert_eq!(DiagnosticCode::E0801MissingRequiredArg.code(), "E0801");
        assert_eq!(DiagnosticCode::E0901IoError.code(), "E0901");
        assert_eq!(DiagnosticCode::E1001GuardSchemaInvalid.code(), "E1001");
        assert_eq!(DiagnosticCode::E1101TagInvalidFormat.code(), "E1101");
        assert_eq!(DiagnosticCode::E1201LoopStateInvalid.code(), "E1201");
        assert_eq!(DiagnosticCode::W0101RfcNoChangelog.code(), "W0101");
        assert_eq!(DiagnosticCode::W0111ProjectSupportOutdated.code(), "W0111");
        assert_eq!(DiagnosticCode::W0112BareArtifactReference.code(), "W0112");
        assert_eq!(DiagnosticCode::I0401WorkLegacyInlineHistory.code(), "I0401");
    }

    #[test]
    fn diagnostic_levels_match_error_warning_info_families() {
        assert_eq!(
            DiagnosticCode::E0101RfcSchemaInvalid.level(),
            DiagnosticLevel::Error
        );
        assert_eq!(
            DiagnosticCode::W0101RfcNoChangelog.level(),
            DiagnosticLevel::Warning
        );
        assert_eq!(
            DiagnosticCode::W0110SchemaOutdated.level(),
            DiagnosticLevel::Warning
        );
        assert_eq!(
            DiagnosticCode::W0111ProjectSupportOutdated.level(),
            DiagnosticLevel::Warning
        );
        assert_eq!(
            DiagnosticCode::W0112BareArtifactReference.level(),
            DiagnosticLevel::Warning
        );
        assert_eq!(
            DiagnosticCode::I0401WorkLegacyInlineHistory.level(),
            DiagnosticLevel::Info
        );
    }
}

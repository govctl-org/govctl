use super::{DiagnosticCode, DiagnosticLevel};

pub(super) fn level(code: &DiagnosticCode) -> DiagnosticLevel {
    match code {
        DiagnosticCode::W0101RfcNoChangelog
        | DiagnosticCode::W0102ClauseNoSince
        | DiagnosticCode::W0103AdrNoRefs
        | DiagnosticCode::W0106RenderedReadError
        | DiagnosticCode::W0107SourceRefOutdated
        | DiagnosticCode::W0108WorkPlaceholderDescription
        | DiagnosticCode::W0109WorkNoActive
        | DiagnosticCode::W0110SchemaOutdated
        | DiagnosticCode::W0111ProjectSupportOutdated
        | DiagnosticCode::W0112BareArtifactReference => DiagnosticLevel::Warning,
        DiagnosticCode::I0401WorkLegacyInlineHistory => DiagnosticLevel::Info,
        _ => DiagnosticLevel::Error,
    }
}

pub(super) fn code(code: &DiagnosticCode) -> &'static str {
    match code {
        // E01xx - RFC
        DiagnosticCode::E0101RfcSchemaInvalid => "E0101",
        DiagnosticCode::E0102RfcNotFound => "E0102",
        DiagnosticCode::E0103RfcIdMismatch => "E0103",
        DiagnosticCode::E0104RfcInvalidTransition => "E0104",
        DiagnosticCode::E0105RfcRefNotFound => "E0105",
        DiagnosticCode::E0106RfcSupersedesNotFound => "E0106",
        DiagnosticCode::E0107SourceRefUnknown => "E0107",
        DiagnosticCode::E0108RfcBumpRequiresSummary => "E0108",
        DiagnosticCode::E0109RfcAlreadyExists => "E0109",
        DiagnosticCode::E0110RfcInvalidId => "E0110",
        DiagnosticCode::E0111RfcNoChangelog => "E0111",
        DiagnosticCode::E0112RfcReferenceHierarchy => "E0112",
        // E02xx - Clause
        DiagnosticCode::E0201ClauseSchemaInvalid => "E0201",
        DiagnosticCode::E0202ClauseNotFound => "E0202",
        DiagnosticCode::E0203ClauseIdMismatch => "E0203",
        DiagnosticCode::E0204ClausePathInvalid => "E0204",
        DiagnosticCode::E0206ClauseSupersededByUnknown => "E0206",
        DiagnosticCode::E0207ClauseSupersededByNotActive => "E0207",
        DiagnosticCode::E0208ClauseAlreadyDeprecated => "E0208",
        DiagnosticCode::E0209ClauseAlreadySuperseded => "E0209",
        DiagnosticCode::E0210ClauseInvalidIdFormat => "E0210",
        DiagnosticCode::E0211ClauseStillReferenced => "E0211",
        // E03xx - ADR
        DiagnosticCode::E0301AdrSchemaInvalid => "E0301",
        DiagnosticCode::E0302AdrNotFound => "E0302",
        DiagnosticCode::E0303AdrInvalidTransition => "E0303",
        DiagnosticCode::E0304AdrRefNotFound => "E0304",
        DiagnosticCode::E0305AdrCannotDeprecate => "E0305",
        DiagnosticCode::E0306AdrReferenceHierarchy => "E0306",
        // E04xx - Work Item
        DiagnosticCode::E0401WorkSchemaInvalid => "E0401",
        DiagnosticCode::E0402WorkNotFound => "E0402",
        DiagnosticCode::E0403WorkInvalidTransition => "E0403",
        DiagnosticCode::E0404WorkRefNotFound => "E0404",
        DiagnosticCode::E0405WorkDirNotFound => "E0405",
        DiagnosticCode::E0406WorkAmbiguousMatch => "E0406",
        DiagnosticCode::E0407WorkMissingCriteria => "E0407",
        DiagnosticCode::E0408WorkCriteriaMissingCategory => "E0408",
        DiagnosticCode::E0409WorkDependencyInvalid => "E0409",
        DiagnosticCode::E0410WorkDependencyNotFound => "E0410",
        DiagnosticCode::E0411WorkDependencyCycle => "E0411",
        // E05xx - Config
        DiagnosticCode::E0501ConfigInvalid => "E0501",
        DiagnosticCode::E0502PathNotFound => "E0502",
        DiagnosticCode::E0503LockTimeout => "E0503",
        DiagnosticCode::E0504PathConflict => "E0504",
        DiagnosticCode::E0505MigrationRequired => "E0505",
        // E06xx - Signature
        DiagnosticCode::E0601SignatureMismatch => "E0601",
        DiagnosticCode::E0602SignatureMissing => "E0602",
        // E07xx - Release
        DiagnosticCode::E0701ReleaseInvalidSemver => "E0701",
        DiagnosticCode::E0702ReleaseDuplicate => "E0702",
        DiagnosticCode::E0703ReleaseNoUnreleasedItems => "E0703",
        DiagnosticCode::E0704ReleaseSchemaInvalid => "E0704",
        DiagnosticCode::E0705ReleaseRefNotFound => "E0705",
        // E10xx - Verification Guard
        DiagnosticCode::E1001GuardSchemaInvalid => "E1001",
        DiagnosticCode::E1002GuardNotFound => "E1002",
        DiagnosticCode::E1003GuardDuplicate => "E1003",
        DiagnosticCode::E1004GuardCheckFailed => "E1004",
        DiagnosticCode::E1005GuardTimeout => "E1005",
        DiagnosticCode::E1006GuardInvalidTitle => "E1006",
        DiagnosticCode::E1007GuardStillReferenced => "E1007",
        // E11xx - Tags
        DiagnosticCode::E1101TagInvalidFormat => "E1101",
        DiagnosticCode::E1102TagAlreadyExists => "E1102",
        DiagnosticCode::E1103TagNotFound => "E1103",
        DiagnosticCode::E1104TagStillReferenced => "E1104",
        DiagnosticCode::E1105TagUnknown => "E1105",
        // E12xx - Loop state
        DiagnosticCode::E1201LoopStateInvalid => "E1201",
        DiagnosticCode::E1202LoopStateNotFound => "E1202",
        DiagnosticCode::E1203LoopInvalidTransition => "E1203",
        DiagnosticCode::E1204LoopInvalidId => "E1204",
        DiagnosticCode::E1205LoopDependencyNotFound => "E1205",
        DiagnosticCode::E1206LoopDependencyCycle => "E1206",
        DiagnosticCode::E1208LoopResumeAmbiguous => "E1208",
        DiagnosticCode::E1209LoopWorkMismatch => "E1209",
        DiagnosticCode::E1210LoopExecutionFailed => "E1210",
        DiagnosticCode::E1211LoopInvalidMaxRounds => "E1211",
        // E08xx - CLI/Command
        DiagnosticCode::E0801MissingRequiredArg => "E0801",
        DiagnosticCode::E0802ConflictingArgs => "E0802",
        DiagnosticCode::E0803UnknownField => "E0803",
        DiagnosticCode::E0804FieldNotEditable => "E0804",
        DiagnosticCode::E0805EmptyValue => "E0805",
        DiagnosticCode::E0806InvalidPattern => "E0806",
        DiagnosticCode::E0807AmbiguousMatch => "E0807",
        DiagnosticCode::E0808InvalidPrefix => "E0808",
        DiagnosticCode::E0809ChoreNotAllowed => "E0809",
        DiagnosticCode::E0810CannotAddToField => "E0810",
        DiagnosticCode::E0811CannotRemoveFromField => "E0811",
        DiagnosticCode::E0812FieldEmpty => "E0812",
        DiagnosticCode::E0813SupersedeNotSupported => "E0813",
        DiagnosticCode::E0814InvalidPath => "E0814",
        DiagnosticCode::E0815PathFieldNotFound => "E0815",
        DiagnosticCode::E0816PathIndexOutOfBounds => "E0816",
        DiagnosticCode::E0817PathTypeMismatch => "E0817",
        DiagnosticCode::E0818PathIndexConflict => "E0818",
        DiagnosticCode::E0819UnknownArtifactType => "E0819",
        DiagnosticCode::E0820InvalidFieldValue => "E0820",
        DiagnosticCode::E0821InvalidCommandScope => "E0821",
        DiagnosticCode::E0822UnsupportedOperation => "E0822",
        // E09xx - General
        DiagnosticCode::E0901IoError => "E0901",
        DiagnosticCode::E0902JsonParseError => "E0902",
        DiagnosticCode::E0903UnexpectedError => "E0903",
        // W01xx - Warnings
        DiagnosticCode::W0101RfcNoChangelog => "W0101",
        DiagnosticCode::W0102ClauseNoSince => "W0102",
        DiagnosticCode::W0103AdrNoRefs => "W0103",
        DiagnosticCode::W0106RenderedReadError => "W0106",
        DiagnosticCode::W0107SourceRefOutdated => "W0107",
        DiagnosticCode::W0108WorkPlaceholderDescription => "W0108",
        DiagnosticCode::W0109WorkNoActive => "W0109",
        DiagnosticCode::W0110SchemaOutdated => "W0110",
        DiagnosticCode::W0111ProjectSupportOutdated => "W0111",
        DiagnosticCode::W0112BareArtifactReference => "W0112",
        // I04xx - Work Item info
        DiagnosticCode::I0401WorkLegacyInlineHistory => "I0401",
    }
}

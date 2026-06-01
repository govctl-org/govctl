use super::ArtifactType;
use super::engine as edit_engine;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::write::WriteOp;

mod get;
mod list;
mod set;

pub(super) use get::get_json_field;
pub(super) use list::{add_json_simple_list_field, remove_json_simple_list_field};
pub(super) use set::{set_clause_field, set_rfc_field};

#[derive(Debug, Clone, Copy)]
enum JsonTargetKind {
    Rfc,
    Clause,
}

struct SetJsonRequest<'a> {
    config: &'a Config,
    id: &'a str,
    target: &'a edit_engine::ResolvedTarget,
    value: &'a str,
    op: WriteOp,
    allow_forced_simple_set: bool,
    kind: JsonTargetKind,
}

impl JsonTargetKind {
    fn artifact(self) -> ArtifactType {
        match self {
            Self::Rfc => ArtifactType::Rfc,
            Self::Clause => ArtifactType::Clause,
        }
    }

    fn validate_kind(self) -> crate::validate::ArtifactKind {
        match self {
            Self::Rfc => crate::validate::ArtifactKind::Rfc,
            Self::Clause => crate::validate::ArtifactKind::Clause,
        }
    }

    fn nested_error(self) -> &'static str {
        match self {
            Self::Rfc => "RFC fields do not support nested paths",
            Self::Clause => "Clause fields do not support nested paths",
        }
    }

    fn unsupported_set_path_error(self) -> &'static str {
        match self {
            Self::Rfc => "RFC fields do not support this set path",
            Self::Clause => "Clause fields do not support this set path",
        }
    }

    fn unknown_field_error(self, field: &str) -> Diagnostic {
        match self {
            Self::Rfc => Diagnostic::new(
                DiagnosticCode::E0101RfcSchemaInvalid,
                format!("Unknown field: {field}"),
                "",
            ),
            Self::Clause => Diagnostic::new(
                DiagnosticCode::E0201ClauseSchemaInvalid,
                format!("Unknown field: {field}"),
                "",
            ),
        }
    }
}

fn require_simple_field<'a>(
    fp: &'a super::path::FieldPath,
    id: &str,
    message: &str,
) -> DiagnosticResult<&'a str> {
    fp.as_simple()
        .ok_or_else(|| Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, message, id))
}

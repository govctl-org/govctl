use super::ArtifactType;
use super::adapter::DocAdapter;
use super::deserialize_edit_doc;
use super::engine as edit_engine;
use super::matching::MatchOptions;
use super::runtime as edit_runtime;
use super::serialize_edit_doc;
use super::target_doc::{NestedGetMode, cannot_add_to_field_error, render_target_from_doc};
use super::target_doc_remove::{notify_removed, remove_target_from_doc};
use crate::cmd::output::print_json;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::write::WriteOp;

pub(super) mod rfc_changelog;
mod set;

pub(super) use set::{set_clause_field, set_rfc_field};

#[derive(Debug, Clone, Copy)]
enum DocTargetKind {
    Rfc,
    Clause,
}

struct SetDocRequest<'a> {
    config: &'a Config,
    id: &'a str,
    target: &'a edit_engine::ResolvedTarget,
    value: &'a str,
    op: WriteOp,
    allow_forced_simple_set: bool,
    kind: DocTargetKind,
}

impl DocTargetKind {
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

pub(super) fn get_doc_field<A>(
    config: &Config,
    id: &str,
    target: Option<&edit_engine::ResolvedTarget>,
    artifact: ArtifactType,
    nested_error: &str,
) -> DiagnosticResult<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let loaded = A::load(config, id)?;
    if let Some(target) = target {
        let doc = serialize_edit_doc(&loaded.data, id)?;
        println!(
            "{}",
            render_target_from_doc(
                artifact,
                &doc,
                target,
                id,
                NestedGetMode::Reject(nested_error),
            )?
        );
    } else {
        print_json(
            &loaded.data,
            DiagnosticCode::E0903UnexpectedError,
            "Failed to serialize editable document",
            id,
        )?;
    }
    Ok(())
}

pub(super) fn add_doc_simple_list_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    artifact: ArtifactType,
    nested_error: &str,
) -> DiagnosticResult<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let edit_engine::ResolvedTarget::Node {
        path,
        kind: edit_engine::TargetKind::List,
        origin: edit_engine::TargetOrigin::Simple,
        ..
    } = target
    else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            nested_error,
            id,
        ));
    };
    let simple = require_simple_field(path, id, nested_error)?;
    let mut loaded = A::load(config, id)?;
    let mut doc = serialize_edit_doc(&loaded.data, id)?;
    if !edit_runtime::add_simple_list_value(artifact, &mut doc, simple, value, id)? {
        return Err(cannot_add_to_field_error(id, simple));
    }
    loaded.data = deserialize_edit_doc(doc, id)?;
    A::write(config, &loaded, op)?;
    Ok(())
}

pub(super) fn remove_doc_simple_list_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
    nested_error: &str,
) -> DiagnosticResult<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut loaded = A::load(config, id)?;
    let mut doc = serialize_edit_doc(&loaded.data, id)?;
    let (display_field, removed) = remove_target_from_doc(artifact, &mut doc, id, target, opts)?;
    if !matches!(
        target,
        edit_engine::ResolvedTarget::Node {
            origin: edit_engine::TargetOrigin::Simple,
            ..
        } | edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Simple,
            ..
        }
    ) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            nested_error,
            id,
        ));
    }
    loaded.data = deserialize_edit_doc(doc, id)?;
    A::write(config, &loaded, op)?;
    notify_removed(id, &display_field, &removed, op);
    Ok(())
}

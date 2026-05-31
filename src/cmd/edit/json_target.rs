use super::ArtifactType;
use super::adapter::DocAdapter;
use super::engine as edit_engine;
use super::matching::MatchOptions;
use super::runtime as edit_runtime;
use super::target_doc::{cannot_add_to_field_error, notify_removed, remove_target_from_doc};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::write::{WriteOp, today};

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

pub(super) fn get_json_field<A>(
    config: &Config,
    id: &str,
    target: Option<&edit_engine::ResolvedTarget>,
    artifact: ArtifactType,
    nested_error: &str,
) -> anyhow::Result<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let loaded = A::load(config, id)?;
    if let Some(target) = target {
        let doc = serde_json::to_value(&loaded.data)?;
        match target {
            edit_engine::ResolvedTarget::Node {
                origin: edit_engine::TargetOrigin::Simple,
                path,
                ..
            } => {
                let simple = require_simple_field(path, id, nested_error)?;
                println!(
                    "{}",
                    edit_runtime::get_simple_field(artifact, &doc, simple, id)?
                );
            }
            edit_engine::ResolvedTarget::IndexedItem {
                origin: edit_engine::TargetOrigin::Simple,
                container_path,
                index,
                ..
            } => {
                let simple = require_simple_field(container_path, id, nested_error)?;
                println!(
                    "{}",
                    edit_runtime::get_simple_list_item(artifact, &doc, simple, *index, id)?
                );
            }
            _ => {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    nested_error,
                    id,
                )
                .into());
            }
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&loaded.data)?);
    }
    Ok(())
}

pub(super) fn set_rfc_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    allow_forced_simple_set: bool,
) -> anyhow::Result<()>
where
    A: DocAdapter<Data = crate::model::RfcSpec>,
{
    let request = SetJsonRequest {
        config,
        id,
        target,
        value,
        op,
        allow_forced_simple_set,
        kind: JsonTargetKind::Rfc,
    };
    set_json_field::<A, _>(request, |data| {
        data.updated = Some(today());
        Ok(())
    })
}

pub(super) fn set_clause_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    allow_forced_simple_set: bool,
) -> anyhow::Result<()>
where
    A: DocAdapter<Data = crate::model::ClauseSpec>,
{
    let request = SetJsonRequest {
        config,
        id,
        target,
        value,
        op,
        allow_forced_simple_set,
        kind: JsonTargetKind::Clause,
    };
    set_json_field::<A, _>(request, |_| Ok(()))
}

pub(super) fn add_json_simple_list_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    artifact: ArtifactType,
    nested_error: &str,
) -> anyhow::Result<()>
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
        return Err(
            Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, nested_error, id).into(),
        );
    };
    let simple = require_simple_field(path, id, nested_error)?;
    let mut loaded = A::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
    if !edit_runtime::add_simple_list_value(artifact, &mut doc, simple, value, id)? {
        return Err(cannot_add_to_field_error(id, simple));
    }
    loaded.data = serde_json::from_value(doc)?;
    A::write(config, &loaded, op)?;
    Ok(())
}

pub(super) fn remove_json_simple_list_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
    nested_error: &str,
) -> anyhow::Result<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut loaded = A::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
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
        return Err(
            Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, nested_error, id).into(),
        );
    }
    loaded.data = serde_json::from_value(doc)?;
    A::write(config, &loaded, op)?;
    notify_removed(id, &display_field, &removed, op);
    Ok(())
}

fn set_json_field<A, F>(request: SetJsonRequest<'_>, touch_loaded_data: F) -> anyhow::Result<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
    F: FnOnce(&mut A::Data) -> anyhow::Result<()>,
{
    let SetJsonRequest {
        config,
        id,
        target,
        value,
        op,
        allow_forced_simple_set,
        kind,
    } = request;

    let mut loaded = A::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
    match target {
        edit_engine::ResolvedTarget::Node {
            path,
            kind: edit_engine::TargetKind::Scalar,
            ..
        } => {
            let simple = require_simple_field(path, id, kind.nested_error())?;
            if !allow_forced_simple_set
                && !edit_runtime::supports_simple_set_field(kind.artifact(), simple)
            {
                return Err(kind.unknown_field_error(simple).into());
            }
            crate::validate::validate_field(config, id, kind.validate_kind(), simple, value)?;
            if allow_forced_simple_set {
                edit_runtime::set_simple_field_forced(
                    kind.artifact(),
                    &mut doc,
                    simple,
                    value,
                    id,
                )?;
            } else {
                edit_runtime::set_simple_field(kind.artifact(), &mut doc, simple, value, id)?;
            }
        }
        edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Simple,
            container_path,
            index,
            item_kind: edit_engine::TargetKind::Scalar,
            ..
        } => {
            let simple = container_path.as_simple().ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    "simple indexed container expected",
                    id,
                )
            })?;
            edit_runtime::set_simple_list_item(
                kind.artifact(),
                &mut doc,
                simple,
                *index,
                value,
                id,
            )?;
        }
        _ => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                kind.unsupported_set_path_error(),
                id,
            )
            .into());
        }
    }
    loaded.data = serde_json::from_value(doc)?;
    touch_loaded_data(&mut loaded.data)?;
    A::write(config, &loaded, op)?;
    Ok(())
}

fn require_simple_field<'a>(
    fp: &'a super::path::FieldPath,
    id: &str,
    message: &str,
) -> anyhow::Result<&'a str> {
    fp.as_simple()
        .ok_or_else(|| Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, message, id).into())
}

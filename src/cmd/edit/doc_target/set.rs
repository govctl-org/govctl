use super::super::{
    adapter::DocAdapter, deserialize_edit_doc, engine as edit_engine, refs,
    runtime as edit_runtime, serialize_edit_doc, unexpected_edit_state,
};
use super::{DocTargetKind, SetDocRequest, require_simple_field};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::write::{WriteOp, today};

pub(in crate::cmd::edit) fn set_rfc_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    allow_forced_simple_set: bool,
) -> DiagnosticResult<()>
where
    A: DocAdapter<Data = crate::model::RfcSpec>,
{
    let request = SetDocRequest {
        config,
        id,
        target,
        value,
        op,
        allow_forced_simple_set,
        kind: DocTargetKind::Rfc,
    };
    set_doc_field::<A, _>(request, |data| {
        data.updated = Some(today());
        Ok(())
    })
}

pub(in crate::cmd::edit) fn set_clause_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    allow_forced_simple_set: bool,
) -> DiagnosticResult<()>
where
    A: DocAdapter<Data = crate::model::ClauseSpec>,
{
    let request = SetDocRequest {
        config,
        id,
        target,
        value,
        op,
        allow_forced_simple_set,
        kind: DocTargetKind::Clause,
    };
    set_doc_field::<A, _>(request, |_| Ok(()))
}

fn set_doc_field<A, F>(request: SetDocRequest<'_>, touch_loaded_data: F) -> DiagnosticResult<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
    F: FnOnce(&mut A::Data) -> DiagnosticResult<()>,
{
    let SetDocRequest {
        config,
        id,
        target,
        value,
        op,
        allow_forced_simple_set,
        kind,
    } = request;

    let mut loaded = A::load(config, id)?;
    let mut doc = serialize_edit_doc(&loaded.data, id)?;
    if refs::is_refs_target(target) {
        refs::validate_ref_edit(config, kind.artifact(), id, value)?;
    }
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
                return Err(kind.unknown_field_error(simple));
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
            let simple = container_path
                .as_simple()
                .ok_or_else(|| unexpected_edit_state(id, "simple indexed container expected"))?;
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
            ));
        }
    }
    loaded.data = deserialize_edit_doc(doc, id)?;
    touch_loaded_data(&mut loaded.data)?;
    A::write(config, &loaded, op)?;
    Ok(())
}

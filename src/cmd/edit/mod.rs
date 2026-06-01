//! Edit command implementation - modify artifacts.
//!
//! Implements [[ADR-0007]] ergonomic array field matching for remove and tick commands.

pub mod adapter;
mod add;
mod artifact;
mod delete;
mod delete_referrers;
mod doc_adapter;
pub mod engine;
mod get;
mod json_target;
mod matching;
pub mod path;
mod remove;
mod request;
pub mod rules;
pub mod runtime;
mod set;
mod target_doc;
mod target_doc_remove;
mod tick;
mod toml_adapter;
mod toml_target;

use self::adapter::{ClauseTomlAdapter, DocAdapter};
pub use self::add::{AddFieldRequest, add_to_field};
pub use self::artifact::ArtifactType;
pub use self::get::get_field;
pub use self::remove::remove_from_field;
pub use self::request::{EditFieldRequest, OwnedEditAction};
use self::set::apply_set_field;
pub(crate) use self::set::set_field_direct;
pub use self::tick::tick_item;
use self::{engine as edit_engine, rules as edit_rules};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::ui;
use crate::write::WriteOp;
pub use delete::{delete_clause, delete_work_item};
pub use matching::{MatchOptions, MatchOptionsOwned};
use std::path::Path;

use self::request::{read_stdin, resolve_owned_value};

// Field normalization is centralized in edit_engine::plan_request.

pub(super) fn serialize_edit_doc<T: serde::Serialize>(
    value: &T,
    id: &str,
) -> DiagnosticResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0903UnexpectedError,
            format!("Failed to serialize editable document: {err}"),
            id,
        )
    })
}

pub(super) fn deserialize_edit_doc<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
    id: &str,
) -> DiagnosticResult<T> {
    serde_json::from_value(value).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            format!("Edited document failed schema conversion: {err}"),
            id,
        )
    })
}

pub(super) fn unexpected_edit_state(id: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0903UnexpectedError, message, id)
}

fn plan_edit_with_field_for_verb(
    id: &str,
    field: &str,
    verb: Option<edit_rules::Verb>,
) -> DiagnosticResult<edit_engine::TargetPlan> {
    let plan = match verb {
        Some(verb) => edit_engine::plan_mutation_request(id, field, verb)?,
        None => edit_engine::plan_request(id, Some(field))?,
    };
    plan.field_path.as_ref().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "Field path required",
            id,
        )
    })?;
    Ok(plan)
}

pub fn edit_clause(
    config: &Config,
    clause_id: &str,
    text: Option<&str>,
    text_file: Option<&Path>,
    stdin: bool,
    op: WriteOp,
) -> DiagnosticResult<Vec<Diagnostic>> {
    let mut clause_doc = ClauseTomlAdapter::load(config, clause_id)?;

    let new_text = match (text, text_file, stdin) {
        (Some(t), None, false) => t.to_string(),
        (None, Some(path), false) => std::fs::read_to_string(path).map_err(|err| {
            Diagnostic::io_error("read text file", err, path.display().to_string())
        })?,
        (None, None, true) => read_stdin()?,
        (None, None, false) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0801MissingRequiredArg,
                "Provide --text, --text-file, or --stdin",
                "input",
            ));
        }
        _ => unreachable!("clap arg group ensures mutual exclusivity"),
    };

    clause_doc.data.text = new_text;
    ClauseTomlAdapter::write(config, &clause_doc, op)?;

    if !op.is_preview() {
        ui::updated("clause", clause_id);
    }
    Ok(vec![])
}

pub fn edit_field(request: EditFieldRequest<'_>) -> DiagnosticResult<Vec<Diagnostic>> {
    let EditFieldRequest {
        config,
        id,
        path,
        action,
        category_override,
        pros,
        cons,
        reject_reason,
        op,
    } = request;

    match action {
        OwnedEditAction::Set { value, stdin } => {
            let value = resolve_owned_value(value.as_ref(), *stdin)?;
            let plan = plan_edit_with_field_for_verb(id, path, Some(edit_rules::Verb::Set))?;
            let artifact = plan.artifact;
            let target = plan.target.as_ref().ok_or_else(|| {
                unexpected_edit_state(id, "mutation planning should produce target")
            })?;
            apply_set_field(config, id, target, artifact, value.as_str(), op, true)?;
            if !op.is_preview() {
                ui::field_set(id, &target.display_path(), value.as_str());
            }
            Ok(vec![])
        }
        OwnedEditAction::Add { value, stdin } => {
            let value = resolve_owned_value(value.as_ref(), *stdin)?;
            let value = Some(Some(value));
            add_to_field(AddFieldRequest {
                config,
                id,
                field: path,
                value: value.as_ref(),
                stdin: false,
                category_override,
                pros,
                cons,
                reject_reason,
                op,
            })
        }
        OwnedEditAction::Remove { match_opts } => {
            remove_from_field(config, id, path, &match_opts.as_match_options(), op)
        }
        OwnedEditAction::Tick { match_opts, status } => tick_item(
            config,
            id,
            path,
            &match_opts.as_match_options(),
            *status,
            op,
        ),
    }
}

fn reject_match_flags_for_indexed_target(
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
) -> DiagnosticResult<()> {
    let pattern_provided = opts.pattern.is_some_and(|pattern| !pattern.is_empty());
    let edit_engine::ResolvedTarget::IndexedItem { index, .. } = target else {
        return Ok(());
    };
    if pattern_provided || opts.exact || opts.regex || opts.all {
        return Err(Diagnostic::new(
            DiagnosticCode::E0818PathIndexConflict,
            "Cannot combine indexed path (e.g., alt[0].cons[1]) with match flags (--at, --exact, --regex, --all, or pattern)",
            id,
        ));
    }
    if let Some(existing_at) = opts.at
        && existing_at != *index
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0818PathIndexConflict,
            "Cannot combine indexed path with a different --at value",
            id,
        ));
    }
    Ok(())
}

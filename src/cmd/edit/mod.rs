//! Edit command implementation - modify artifacts.
//!
//! Implements [[ADR-0007]] ergonomic array field matching for remove and tick commands.

pub mod adapter;
mod add;
mod artifact;
mod delete;
pub mod engine;
mod get;
mod json_target;
mod matching;
pub mod path;
mod remove;
pub mod rules;
pub mod runtime;
mod set;
mod target_doc;
mod tick;
mod toml_target;

use self::adapter::{ClauseTomlAdapter, DocAdapter};
pub use self::add::{AddFieldRequest, add_to_field};
pub use self::artifact::ArtifactType;
pub use self::get::get_field;
pub use self::remove::remove_from_field;
use self::set::apply_set_field;
pub(crate) use self::set::set_field_direct;
pub use self::tick::tick_item;
use self::{engine as edit_engine, rules as edit_rules};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::ui;
use crate::write::WriteOp;
use anyhow::Context;
pub use delete::{delete_clause, delete_work_item};
pub use matching::{MatchOptions, MatchOptionsOwned};
use std::io::Read;
use std::path::Path;

// Field normalization is centralized in edit_engine::plan_request.

#[derive(Debug, Clone)]
pub enum OwnedEditAction {
    Set {
        value: Option<Option<String>>,
        stdin: bool,
    },
    Add {
        value: Option<Option<String>>,
        stdin: bool,
    },
    Remove {
        match_opts: MatchOptionsOwned,
    },
    Tick {
        match_opts: MatchOptionsOwned,
        status: crate::TickStatus,
    },
}

fn read_stdin() -> anyhow::Result<String> {
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read from stdin")?;
    // Trim trailing newline that HEREDOC adds
    Ok(buffer.trim_end_matches('\n').to_string())
}

fn resolve_owned_value(value: Option<&Option<String>>, stdin: bool) -> anyhow::Result<String> {
    match (value, stdin) {
        (Some(Some(v)), false) => Ok(v.clone()),
        (Some(None), true) => read_stdin(),
        (Some(None), false) => Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "Provide a value or use --stdin",
            "input",
        )
        .into()),
        (Some(Some(_)), true) => Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "Cannot use both value and --stdin",
            "input",
        )
        .into()),
        (None, _) => Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "Provide a value or use --stdin",
            "input",
        )
        .into()),
    }
}

fn plan_edit_with_field_for_verb(
    id: &str,
    field: &str,
    verb: Option<edit_rules::Verb>,
) -> anyhow::Result<edit_engine::TargetPlan> {
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
) -> anyhow::Result<Vec<Diagnostic>> {
    let mut clause_doc = ClauseTomlAdapter::load(config, clause_id)?;

    let new_text = match (text, text_file, stdin) {
        (Some(t), None, false) => t.to_string(),
        (None, Some(path), false) => std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read text file: {}", path.display()))?,
        (None, None, true) => read_stdin()?,
        (None, None, false) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0801MissingRequiredArg,
                "Provide --text, --text-file, or --stdin",
                "input",
            )
            .into());
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

#[allow(clippy::too_many_arguments)]
pub fn edit_field(
    config: &Config,
    id: &str,
    path: &str,
    action: &OwnedEditAction,
    category_override: Option<crate::model::ChangelogCategory>,
    _scope_override: Option<&str>,
    pros: Option<Vec<String>>,
    cons: Option<Vec<String>>,
    reject_reason: Option<String>,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    match action {
        OwnedEditAction::Set { value, stdin } => {
            let value = resolve_owned_value(value.as_ref(), *stdin)?;
            let plan = plan_edit_with_field_for_verb(id, path, Some(edit_rules::Verb::Set))?;
            let artifact = plan.artifact;
            let target = plan.target.as_ref().ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    "mutation planning should produce target",
                    id,
                )
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
) -> anyhow::Result<()> {
    let pattern_provided = opts.pattern.is_some_and(|pattern| !pattern.is_empty());
    let edit_engine::ResolvedTarget::IndexedItem { index, .. } = target else {
        return Ok(());
    };
    if pattern_provided || opts.exact || opts.regex || opts.all {
        return Err(Diagnostic::new(
            DiagnosticCode::E0818PathIndexConflict,
            "Cannot combine indexed path (e.g., alt[0].cons[1]) with match flags (--at, --exact, --regex, --all, or pattern)",
            id,
        )
        .into());
    }
    if let Some(existing_at) = opts.at
        && existing_at != *index
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0818PathIndexConflict,
            "Cannot combine indexed path with a different --at value",
            id,
        )
        .into());
    }
    Ok(())
}

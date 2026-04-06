//! Edit command implementation - modify artifacts.
//!
//! Implements [[ADR-0007]] ergonomic array field matching for remove and tick commands.

pub mod adapter;
pub mod engine;
pub mod path;
pub mod rules;
pub mod runtime;

use self::adapter::{
    AdrTomlAdapter, ClauseJsonAdapter, GuardTomlAdapter, JsonAdapter, RfcJsonAdapter, TomlAdapter,
    WorkTomlAdapter,
};
use self::path::FieldPath;
use self::{engine as edit_engine, rules as edit_rules, runtime as edit_runtime};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrEntry, AdrSpec, GuardEntry, GuardSpec, WorkItemEntry, WorkItemSpec};
use crate::ui;
use crate::write::{WriteOp, delete_file, today};
use anyhow::Context;
use regex::Regex;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactType {
    Clause,
    Rfc,
    Adr,
    WorkItem,
    Guard,
}

impl ArtifactType {
    pub fn from_id(id: &str) -> Option<Self> {
        if id.contains(':') {
            Some(Self::Clause)
        } else if id.starts_with("RFC-") {
            Some(Self::Rfc)
        } else if id.starts_with("ADR-") {
            Some(Self::Adr)
        } else if id.starts_with("GUARD-") {
            Some(Self::Guard)
        } else if id.starts_with("WI-") || id.contains('-') {
            Some(Self::WorkItem)
        } else {
            None
        }
    }

    pub fn unknown_error(id: &str) -> anyhow::Error {
        Diagnostic::new(
            DiagnosticCode::E0819UnknownArtifactType,
            format!("Unknown artifact type: {id}"),
            id,
        )
        .into()
    }

    pub fn rule_key(self) -> &'static str {
        match self {
            Self::Clause => "clause",
            Self::Rfc => "rfc",
            Self::Adr => "adr",
            Self::WorkItem => "work",
            Self::Guard => "guard",
        }
    }
}

// Field normalization is centralized in edit_engine::plan_request.

#[derive(Debug, Clone, Default)]
pub struct MatchOptions<'a> {
    pub pattern: Option<&'a str>,
    pub at: Option<i32>,
    pub exact: bool,
    pub regex: bool,
    pub all: bool,
}

#[derive(Debug, Clone)]
pub enum OwnedEditAction {
    Set {
        value: Option<String>,
        stdin: bool,
    },
    Add {
        value: Option<String>,
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

#[derive(Debug, Clone, Default)]
pub struct MatchOptionsOwned {
    pub pattern: Option<String>,
    pub at: Option<i32>,
    pub exact: bool,
    pub regex: bool,
    pub all: bool,
}

impl MatchOptionsOwned {
    pub fn as_match_options(&self) -> MatchOptions<'_> {
        MatchOptions {
            pattern: self.pattern.as_deref(),
            at: self.at,
            exact: self.exact,
            regex: self.regex,
            all: self.all,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchUse {
    Remove,
    TickSingle,
}

fn resolve_match_indices(
    id: &str,
    field: &str,
    items: &[&str],
    opts: &MatchOptions,
    use_case: MatchUse,
) -> anyhow::Result<Vec<usize>> {
    if items.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0812FieldEmpty,
            format!("Field {}.{} is empty", id, field),
            id,
        )
        .into());
    }

    if opts.pattern.is_none() && opts.at.is_none() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            format!(
                "Remove from {}.{} requires a pattern or --at <index>",
                id, field
            ),
            id,
        )
        .into());
    }

    let indices: Vec<usize> = if let Some(idx) = opts.at {
        let len = items.len() as i32;
        let actual_idx = if idx < 0 { len + idx } else { idx };
        if actual_idx < 0 || actual_idx >= len {
            return Err(Diagnostic::new(
                DiagnosticCode::E0806InvalidPattern,
                format!(
                    "Index {} out of range (array has {} items)",
                    idx,
                    items.len()
                ),
                "array",
            )
            .into());
        }
        vec![actual_idx as usize]
    } else {
        let pattern = opts.pattern.unwrap_or("<index>");
        let matches = if opts.regex {
            let re = Regex::new(pattern).map_err(|e| {
                Diagnostic::new(
                    DiagnosticCode::E0806InvalidPattern,
                    format!("Invalid regex: {}", e),
                    id,
                )
            })?;
            items
                .iter()
                .enumerate()
                .filter(|(_, s)| re.is_match(s))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        } else if opts.exact {
            items
                .iter()
                .enumerate()
                .filter(|(_, s)| **s == pattern)
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        } else {
            let pattern_lower = pattern.to_lowercase();
            items
                .iter()
                .enumerate()
                .filter(|(_, s)| s.to_lowercase().contains(&pattern_lower))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        };

        if matches.is_empty() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0806InvalidPattern,
                format!("No items match '{}' in {}.{}", pattern, id, field),
                id,
            )
            .into());
        }
        matches
    };

    if indices.len() == 1 || (use_case == MatchUse::Remove && opts.all) {
        return Ok(indices);
    }

    let pattern = opts.pattern.unwrap_or("");
    let hint = if use_case == MatchUse::Remove {
        "Options:\n  • Use more specific pattern\n  • Use --at <index> to select one\n  • Use --all to remove all matches"
    } else {
        "Use more specific pattern or --at <index> to select one"
    };
    let mut msg = format!(
        "{} items match '{}' in {}.{}:\n",
        indices.len(),
        pattern,
        id,
        field
    );
    for &i in &indices {
        msg.push_str(&format!("  [{}] {}\n", i, items[i]));
    }
    msg.push('\n');
    msg.push_str(hint);
    Err(Diagnostic::new(DiagnosticCode::E0807AmbiguousMatch, msg, id).into())
}

fn read_stdin() -> anyhow::Result<String> {
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read from stdin")?;
    // Trim trailing newline that HEREDOC adds
    Ok(buffer.trim_end_matches('\n').to_string())
}

fn resolve_value(value: Option<&str>, stdin: bool) -> anyhow::Result<String> {
    match (value, stdin) {
        (Some(v), false) => Ok(v.to_string()),
        (None, true) => read_stdin(),
        (None, false) => Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "Provide a value or use --stdin",
            "input",
        )
        .into()),
        (Some(_), true) => Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "Cannot use both value and --stdin",
            "input",
        )
        .into()),
    }
}

fn cannot_add_to_field_error(id: &str, field: &str) -> anyhow::Error {
    Diagnostic::new(
        DiagnosticCode::E0810CannotAddToField,
        format!("Cannot add to field: {field} (not an array or unsupported)"),
        id,
    )
    .into()
}

fn cannot_remove_from_field_error(id: &str, field: &str) -> anyhow::Error {
    Diagnostic::new(
        DiagnosticCode::E0811CannotRemoveFromField,
        format!("Cannot remove from field: {field}"),
        id,
    )
    .into()
}

fn require_simple_field<'a>(fp: &'a FieldPath, id: &str, message: &str) -> anyhow::Result<&'a str> {
    fp.as_simple()
        .ok_or_else(|| Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, message, id).into())
}

fn plan_edit_with_field_for_verb(
    id: &str,
    field: &str,
    verb: Option<edit_rules::Verb>,
) -> anyhow::Result<(ArtifactType, FieldPath)> {
    let plan = match verb {
        Some(verb) => edit_engine::plan_mutation_request(id, field, verb)?,
        None => edit_engine::plan_request(id, Some(field))?,
    };
    let fp = plan.field_path.ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "Field path required",
            id,
        )
    })?;
    Ok((plan.artifact, fp))
}

struct AdrAddContext {
    pros: Option<Vec<String>>,
    cons: Option<Vec<String>>,
    reject_reason: Option<String>,
}

struct WorkAddContext<'a> {
    category_override: Option<crate::model::ChangelogCategory>,
    scope_override: Option<&'a str>,
}

trait TomlEditableEntry {
    type Spec: serde::Serialize + serde::de::DeserializeOwned;
    fn spec(&self) -> &Self::Spec;
    fn spec_mut(&mut self) -> &mut Self::Spec;
}
impl TomlEditableEntry for AdrEntry {
    type Spec = AdrSpec;
    fn spec(&self) -> &Self::Spec {
        &self.spec
    }
    fn spec_mut(&mut self) -> &mut Self::Spec {
        &mut self.spec
    }
}
impl TomlEditableEntry for WorkItemEntry {
    type Spec = WorkItemSpec;
    fn spec(&self) -> &Self::Spec {
        &self.spec
    }
    fn spec_mut(&mut self) -> &mut Self::Spec {
        &mut self.spec
    }
}
impl TomlEditableEntry for GuardEntry {
    type Spec = GuardSpec;
    fn spec(&self) -> &Self::Spec {
        &self.spec
    }
    fn spec_mut(&mut self) -> &mut Self::Spec {
        &mut self.spec
    }
}

const TICK_NESTED_PATH_ERROR: &str =
    "tick only supports checklist root paths or indexed checklist items";
const TICK_UNSUPPORTED_ARTIFACT_ERROR: &str = "Tick only works for work items: {id}";

pub fn edit_clause(
    config: &Config,
    clause_id: &str,
    text: Option<&str>,
    text_file: Option<&Path>,
    stdin: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let mut clause_doc = ClauseJsonAdapter::load(config, clause_id)?;

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
    ClauseJsonAdapter::write(config, &clause_doc, op)?;

    if !op.is_preview() {
        ui::updated("clause", clause_id);
    }
    Ok(vec![])
}

pub fn set_field(
    config: &Config,
    id: &str,
    field: &str,
    value: Option<&str>,
    stdin: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let (artifact, fp) = plan_edit_with_field_for_verb(id, field, Some(edit_rules::Verb::Set))?;
    let value = resolve_value(value, stdin)?;
    apply_set_field(config, id, &fp, artifact, value.as_str(), op, true)?;

    if !op.is_preview() {
        let display_field = fp.to_string();
        ui::field_set(id, &display_field, value.as_str());
    }

    Ok(vec![])
}

pub(crate) fn set_field_direct(
    config: &Config,
    id: &str,
    field: &str,
    value: &str,
    op: WriteOp,
) -> anyhow::Result<()> {
    let (artifact, fp) = plan_edit_with_field_for_verb(id, field, Some(edit_rules::Verb::Set))?;
    apply_set_field(config, id, &fp, artifact, value, op, false)
}

fn apply_set_field(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    artifact: ArtifactType,
    value: &str,
    op: WriteOp,
    enforce_verb_ownership: bool,
) -> anyhow::Result<()> {
    if enforce_verb_ownership {
        reject_verb_owned_set(artifact, fp, id)?;
    }
    match artifact {
        ArtifactType::Adr => set_toml_field::<AdrTomlAdapter>(
            config,
            id,
            fp,
            value,
            op,
            ArtifactType::Adr,
            !enforce_verb_ownership,
        )?,
        ArtifactType::WorkItem => {
            if fp.as_simple() == Some("notes") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0804FieldNotEditable,
                    "Use 'add' to append notes and 'remove' to delete them",
                    id,
                )
                .into());
            }
            set_toml_field::<WorkTomlAdapter>(
                config,
                id,
                fp,
                value,
                op,
                ArtifactType::WorkItem,
                !enforce_verb_ownership,
            )?
        }
        ArtifactType::Rfc => set_rfc_field(config, id, fp, value, op, !enforce_verb_ownership)?,
        ArtifactType::Clause => {
            set_clause_field(config, id, fp, value, op, !enforce_verb_ownership)?
        }
        ArtifactType::Guard => set_toml_field::<GuardTomlAdapter>(
            config,
            id,
            fp,
            value,
            op,
            ArtifactType::Guard,
            !enforce_verb_ownership,
        )?,
    }
    Ok(())
}

fn reject_verb_owned_set(artifact: ArtifactType, fp: &FieldPath, id: &str) -> anyhow::Result<()> {
    let path = fp.to_string();
    let msg = match artifact {
        ArtifactType::Rfc => match fp.as_simple() {
            Some("status") => Some(
                "RFC status is lifecycle-owned. Use `govctl rfc finalize`, `govctl rfc deprecate`, or `govctl rfc supersede`.",
            ),
            Some("phase") => Some("RFC phase is lifecycle-owned. Use `govctl rfc advance`."),
            Some("version") => Some("RFC version is lifecycle-owned. Use `govctl rfc bump`."),
            _ => None,
        },
        ArtifactType::Clause => match fp.as_simple() {
            Some("status") => Some(
                "Clause status is lifecycle-owned. Use `govctl clause deprecate` or `govctl clause supersede`.",
            ),
            Some("superseded_by") => {
                Some("Clause supersession is lifecycle-owned. Use `govctl clause supersede`.")
            }
            Some("since") => Some(
                "Clause 'since' is derived from RFC versioning. Use `govctl rfc bump` or `govctl rfc finalize`.",
            ),
            _ => None,
        },
        ArtifactType::Adr => {
            if fp.as_simple() == Some("status") || fp.as_simple() == Some("superseded_by") {
                Some(
                    "ADR lifecycle fields are verb-owned. Use `govctl adr accept`, `govctl adr reject`, or `govctl adr supersede`.",
                )
            } else {
                None
            }
        }
        ArtifactType::WorkItem => {
            if fp.as_simple() == Some("status") {
                Some("Work item status is lifecycle-owned. Use `govctl work move`.")
            } else if fp.segments.len() == 2
                && fp.segments[0].name == "acceptance_criteria"
                && fp.segments[1].name == "status"
            {
                Some("Acceptance criteria status is tick-owned. Use `govctl work tick`.")
            } else {
                None
            }
        }
        ArtifactType::Guard => None,
    };

    if let Some(message) = msg {
        return Err(Diagnostic::new(
            DiagnosticCode::E0804FieldNotEditable,
            format!("{message} (field: `{path}`)"),
            id,
        )
        .into());
    }

    Ok(())
}

pub fn get_field(
    config: &Config,
    id: &str,
    field: Option<&str>,
) -> anyhow::Result<Vec<Diagnostic>> {
    let plan = edit_engine::plan_request(id, field)?;
    match plan.artifact {
        ArtifactType::Adr => get_toml_field::<AdrTomlAdapter>(
            config,
            id,
            plan.field_path.as_ref(),
            ArtifactType::Adr,
        )?,
        ArtifactType::WorkItem => get_toml_field::<WorkTomlAdapter>(
            config,
            id,
            plan.field_path.as_ref(),
            ArtifactType::WorkItem,
        )?,
        ArtifactType::Rfc => get_json_field::<RfcJsonAdapter>(
            config,
            id,
            plan.field_path.as_ref(),
            ArtifactType::Rfc,
            "RFC fields do not support nested paths",
        )?,
        ArtifactType::Clause => get_json_field::<ClauseJsonAdapter>(
            config,
            id,
            plan.field_path.as_ref(),
            ArtifactType::Clause,
            "Clause fields do not support nested paths",
        )?,
        ArtifactType::Guard => get_toml_field::<GuardTomlAdapter>(
            config,
            id,
            plan.field_path.as_ref(),
            ArtifactType::Guard,
        )?,
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
    scope_override: Option<&str>,
    pros: Option<Vec<String>>,
    cons: Option<Vec<String>>,
    reject_reason: Option<String>,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    match action {
        OwnedEditAction::Set { value, stdin } => {
            set_field(config, id, path, value.as_deref(), *stdin, op)
        }
        OwnedEditAction::Add { value, stdin } => add_to_field(
            config,
            id,
            path,
            value.as_deref(),
            *stdin,
            category_override,
            scope_override,
            pros,
            cons,
            reject_reason,
            op,
        ),
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

fn get_toml_field<A>(
    config: &Config,
    id: &str,
    fp: Option<&FieldPath>,
    artifact: ArtifactType,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let entry = A::load(config, id)?;
    if let Some(fp) = fp {
        let doc = serde_json::to_value(entry.spec())?;
        if let Some(simple) = fp.as_simple() {
            println!(
                "{}",
                edit_runtime::get_simple_field(artifact, &doc, simple, id)?
            );
        } else {
            println!(
                "{}",
                edit_runtime::get_nested_field(artifact, &doc, fp, id)?
            );
        }
    } else {
        println!("{}", toml::to_string_pretty(entry.spec())?);
    }
    Ok(())
}

fn set_toml_field<A>(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    value: &str,
    op: WriteOp,
    artifact: ArtifactType,
    allow_forced_simple_set: bool,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    let mut doc = serde_json::to_value(entry.spec())?;
    if let Some(simple) = fp.as_simple() {
        if allow_forced_simple_set {
            edit_runtime::set_simple_field_forced(artifact, &mut doc, simple, value, id)?;
        } else {
            edit_runtime::set_simple_field(artifact, &mut doc, simple, value, id)?;
        }
    } else {
        edit_runtime::set_nested_field(artifact, &mut doc, fp, value, id)?;
    }
    *entry.spec_mut() = serde_json::from_value(doc)?;
    A::write(config, &entry, op)?;
    Ok(())
}

fn get_json_field<A>(
    config: &Config,
    id: &str,
    fp: Option<&FieldPath>,
    artifact: ArtifactType,
    nested_error: &str,
) -> anyhow::Result<()>
where
    A: JsonAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let loaded = A::load(config, id)?;
    if let Some(fp) = fp {
        let simple = require_simple_field(fp, id, nested_error)?;
        let doc = serde_json::to_value(&loaded.data)?;
        println!(
            "{}",
            edit_runtime::get_simple_field(artifact, &doc, simple, id)?
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&loaded.data)?);
    }
    Ok(())
}

fn set_rfc_field(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    value: &str,
    op: WriteOp,
    allow_forced_simple_set: bool,
) -> anyhow::Result<()> {
    let simple = require_simple_field(fp, id, "RFC fields do not support nested paths")?;
    if !allow_forced_simple_set
        && !edit_runtime::supports_simple_set_field(ArtifactType::Rfc, simple)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0101RfcSchemaInvalid,
            format!("Unknown field: {simple}"),
            "",
        )
        .into());
    }
    crate::validate::validate_field(
        config,
        id,
        crate::validate::ArtifactKind::Rfc,
        simple,
        value,
    )?;
    let mut loaded = RfcJsonAdapter::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
    if allow_forced_simple_set {
        edit_runtime::set_simple_field_forced(ArtifactType::Rfc, &mut doc, simple, value, id)?;
    } else {
        edit_runtime::set_simple_field(ArtifactType::Rfc, &mut doc, simple, value, id)?;
    }
    loaded.data = serde_json::from_value(doc)?;
    loaded.data.updated = Some(today());
    RfcJsonAdapter::write(config, &loaded, op)?;
    Ok(())
}

fn set_clause_field(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    value: &str,
    op: WriteOp,
    allow_forced_simple_set: bool,
) -> anyhow::Result<()> {
    let simple = require_simple_field(fp, id, "Clause fields do not support nested paths")?;
    if !allow_forced_simple_set
        && !edit_runtime::supports_simple_set_field(ArtifactType::Clause, simple)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0201ClauseSchemaInvalid,
            format!("Unknown field: {simple}"),
            "",
        )
        .into());
    }
    crate::validate::validate_field(
        config,
        id,
        crate::validate::ArtifactKind::Clause,
        simple,
        value,
    )?;
    let mut loaded = ClauseJsonAdapter::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
    if allow_forced_simple_set {
        edit_runtime::set_simple_field_forced(ArtifactType::Clause, &mut doc, simple, value, id)?;
    } else {
        edit_runtime::set_simple_field(ArtifactType::Clause, &mut doc, simple, value, id)?;
    }
    loaded.data = serde_json::from_value(doc)?;
    ClauseJsonAdapter::write(config, &loaded, op)?;
    Ok(())
}

fn adr_add_alternatives(
    entry: &mut AdrEntry,
    value: &str,
    ctx: &AdrAddContext,
) -> anyhow::Result<()> {
    use crate::model::Alternative;
    if entry
        .spec
        .content
        .alternatives
        .iter()
        .any(|a| a.text == value)
    {
        return Ok(());
    }

    entry.spec.content.alternatives.push(Alternative {
        text: value.to_string(),
        pros: ctx.pros.clone().unwrap_or_default(),
        cons: ctx.cons.clone().unwrap_or_default(),
        rejection_reason: ctx.reject_reason.clone(),
    });
    Ok(())
}

fn work_add_acceptance_criteria(
    entry: &mut WorkItemEntry,
    value: &str,
    ctx: &WorkAddContext,
) -> anyhow::Result<()> {
    use crate::model::ChecklistItem;
    use crate::write::parse_changelog_change;
    let parsed = parse_changelog_change(value)?;

    let final_category = if let Some(cat) = ctx.category_override {
        cat
    } else if parsed.explicit {
        parsed.category
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0408WorkCriteriaMissingCategory,
            format!(
                "Acceptance criteria requires category. Use prefix (e.g., 'fix: {}') or --category",
                parsed.message
            ),
            &entry.spec.govctl.id,
        )
        .into());
    };

    if !entry
        .spec
        .content
        .acceptance_criteria
        .iter()
        .any(|c| c.text == parsed.message)
    {
        entry
            .spec
            .content
            .acceptance_criteria
            .push(ChecklistItem::with_category(
                &parsed.message,
                final_category,
            ));
    }
    Ok(())
}

fn work_add_journal(
    entry: &mut WorkItemEntry,
    value: &str,
    ctx: &WorkAddContext,
) -> anyhow::Result<()> {
    use crate::model::JournalEntry;
    use crate::write::today;
    entry.spec.content.journal.push(JournalEntry {
        date: today(),
        scope: ctx.scope_override.map(String::from),
        content: value.to_string(),
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn add_to_field(
    config: &Config,
    id: &str,
    field: &str,
    value: Option<&str>,
    stdin: bool,
    category_override: Option<crate::model::ChangelogCategory>,
    scope_override: Option<&str>,
    pros: Option<Vec<String>>,
    cons: Option<Vec<String>>,
    reject_reason: Option<String>,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let (artifact, fp) = plan_edit_with_field_for_verb(id, field, Some(edit_rules::Verb::Add))?;
    let value = resolve_value(value, stdin)?;
    let value = value.as_str();
    match artifact {
        ArtifactType::Adr => {
            let mut entry = AdrTomlAdapter::load(config, id)?;
            if fp.is_simple() {
                let simple = fp.as_simple().unwrap();
                let mut doc = serde_json::to_value(entry.spec())?;
                if edit_runtime::add_simple_list_value(
                    ArtifactType::Adr,
                    &mut doc,
                    simple,
                    value,
                    id,
                )? {
                    *entry.spec_mut() = serde_json::from_value(doc)?;
                } else if simple == "alternatives" {
                    let ctx = AdrAddContext {
                        pros,
                        cons,
                        reject_reason,
                    };
                    adr_add_alternatives(&mut entry, value, &ctx)?;
                } else {
                    return Err(cannot_add_to_field_error(id, simple));
                }
            } else {
                let mut doc = serde_json::to_value(entry.spec())?;
                edit_runtime::add_nested_list_value(ArtifactType::Adr, &mut doc, &fp, value, id)?;
                *entry.spec_mut() = serde_json::from_value(doc)?;
            }
            AdrTomlAdapter::write(config, &entry, op)?;
        }
        ArtifactType::WorkItem => {
            if !fp.is_simple() {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    format!("Work item nested add is not supported for path '{fp}'"),
                    id,
                )
                .into());
            }
            let simple = fp.as_simple().unwrap();
            let mut entry = WorkTomlAdapter::load(config, id)?;
            let mut doc = serde_json::to_value(entry.spec())?;
            if edit_runtime::add_simple_list_value(
                ArtifactType::WorkItem,
                &mut doc,
                simple,
                value,
                id,
            )? {
                *entry.spec_mut() = serde_json::from_value(doc)?;
            } else {
                let ctx = WorkAddContext {
                    category_override,
                    scope_override,
                };
                if simple == "acceptance_criteria" {
                    work_add_acceptance_criteria(&mut entry, value, &ctx)?;
                } else if simple == "journal" {
                    work_add_journal(&mut entry, value, &ctx)?;
                } else {
                    return Err(cannot_add_to_field_error(id, simple));
                }
            }
            WorkTomlAdapter::write(config, &entry, op)?;
        }
        ArtifactType::Rfc => add_json_simple_list_field::<RfcJsonAdapter>(
            config,
            id,
            &fp,
            value,
            op,
            ArtifactType::Rfc,
            "RFC fields do not support nested paths for add",
        )?,
        ArtifactType::Clause => add_json_simple_list_field::<ClauseJsonAdapter>(
            config,
            id,
            &fp,
            value,
            op,
            ArtifactType::Clause,
            "Clause fields do not support nested paths for add",
        )?,
        ArtifactType::Guard => {
            if !fp.is_simple() {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    format!("Guard nested add is not supported for path '{fp}'"),
                    id,
                )
                .into());
            }
            let simple = fp.as_simple().unwrap();
            let mut entry = GuardTomlAdapter::load(config, id)?;
            let mut doc = serde_json::to_value(entry.spec())?;
            if edit_runtime::add_simple_list_value(
                ArtifactType::Guard,
                &mut doc,
                simple,
                value,
                id,
            )? {
                *entry.spec_mut() = serde_json::from_value(doc)?;
            } else {
                return Err(cannot_add_to_field_error(id, simple));
            }
            GuardTomlAdapter::write(config, &entry, op)?;
        }
    }

    if !op.is_preview() {
        let display_field = fp.to_string();
        ui::field_added(id, &display_field, value);
    }

    Ok(vec![])
}

fn add_json_simple_list_field<A>(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    value: &str,
    op: WriteOp,
    artifact: ArtifactType,
    nested_error: &str,
) -> anyhow::Result<()>
where
    A: JsonAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let simple = require_simple_field(fp, id, nested_error)?;
    let mut loaded = A::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
    if !edit_runtime::add_simple_list_value(artifact, &mut doc, simple, value, id)? {
        return Err(cannot_add_to_field_error(id, simple));
    }
    loaded.data = serde_json::from_value(doc)?;
    A::write(config, &loaded, op)?;
    Ok(())
}

fn notify_removed(id: &str, field: &str, removed: &[String], op: WriteOp) {
    if !op.is_preview() {
        for item in removed {
            ui::field_removed(id, field, item);
        }
    }
}

pub fn remove_from_field(
    config: &Config,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let (artifact, fp) = plan_edit_with_field_for_verb(id, field, Some(edit_rules::Verb::Remove))?;

    if !fp.is_simple() && fp.has_terminal_index() {
        let has_match_opts =
            opts.pattern.is_some() || opts.at.is_some() || opts.exact || opts.regex || opts.all;
        if has_match_opts {
            return Err(Diagnostic::new(
                DiagnosticCode::E0818PathIndexConflict,
                "Cannot combine indexed path (e.g., alt[0].cons[1]) with match flags (--at, --exact, --regex, --all, or pattern)",
                id,
            )
            .into());
        }
    }

    match artifact {
        ArtifactType::Adr => remove_toml_field::<AdrTomlAdapter>(
            config,
            id,
            &fp,
            field,
            opts,
            op,
            ArtifactType::Adr,
        )?,
        ArtifactType::WorkItem => remove_toml_field::<WorkTomlAdapter>(
            config,
            id,
            &fp,
            field,
            opts,
            op,
            ArtifactType::WorkItem,
        )?,
        ArtifactType::Rfc => remove_json_simple_list_field::<RfcJsonAdapter>(
            config,
            id,
            &fp,
            opts,
            op,
            ArtifactType::Rfc,
            "RFC fields do not support nested paths for remove",
        )?,
        ArtifactType::Clause => remove_json_simple_list_field::<ClauseJsonAdapter>(
            config,
            id,
            &fp,
            opts,
            op,
            ArtifactType::Clause,
            "Clause fields do not support nested paths for remove",
        )?,
        ArtifactType::Guard => remove_toml_field::<GuardTomlAdapter>(
            config,
            id,
            &fp,
            field,
            opts,
            op,
            ArtifactType::Guard,
        )?,
    }

    Ok(vec![])
}

pub fn tick_item(
    config: &Config,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    status: crate::TickStatus,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let (artifact, fp) = plan_edit_with_field_for_verb(id, field, Some(edit_rules::Verb::Tick))?;
    let mut indexed_match = MatchOptions::default();
    let (field, effective_opts) = if fp.is_simple() {
        (fp.as_simple().unwrap(), opts.clone())
    } else if fp.segments.len() == 1 && fp.segments[0].index.is_some() {
        if opts.pattern.is_some() || opts.at.is_some() || opts.exact || opts.regex || opts.all {
            return Err(Diagnostic::new(
                DiagnosticCode::E0818PathIndexConflict,
                "Cannot combine an indexed tick path with match flags",
                id,
            )
            .into());
        }
        indexed_match.at = fp.segments[0].index;
        (fp.segments[0].name.as_str(), indexed_match)
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            TICK_NESTED_PATH_ERROR,
            id,
        )
        .into());
    };

    let status_str = match (artifact, status) {
        (ArtifactType::WorkItem, crate::TickStatus::Done) => "done",
        (ArtifactType::WorkItem, crate::TickStatus::Pending) => "pending",
        (ArtifactType::WorkItem, crate::TickStatus::Cancelled) => "cancelled",
        (ArtifactType::Rfc | ArtifactType::Clause | ArtifactType::Adr | ArtifactType::Guard, _) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0813SupersedeNotSupported,
                TICK_UNSUPPORTED_ARTIFACT_ERROR.replace("{id}", id),
                id,
            )
            .into());
        }
    };
    let ticked_text = match artifact {
        ArtifactType::WorkItem => tick_toml_field::<WorkTomlAdapter>(
            config,
            id,
            field,
            &effective_opts,
            op,
            ArtifactType::WorkItem,
            status_str,
        )?,
        ArtifactType::Rfc | ArtifactType::Clause | ArtifactType::Adr | ArtifactType::Guard => {
            unreachable!("handled above")
        }
    };

    if !op.is_preview() {
        ui::ticked(&ticked_text, status_str);
    }

    Ok(vec![])
}

fn remove_simple_values_from_doc(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    field: &str,
    id: &str,
    opts: &MatchOptions,
) -> anyhow::Result<Option<Vec<String>>> {
    if let Some(removed) =
        edit_runtime::remove_simple_list_values_with_matcher(artifact, doc, field, id, |items| {
            resolve_match_indices(id, field, items, opts, MatchUse::Remove)
        })?
    {
        return Ok(Some(removed));
    }
    edit_runtime::remove_simple_status_list_values_with_matcher(artifact, doc, field, id, |items| {
        resolve_match_indices(id, field, items, opts, MatchUse::Remove)
    })
}

fn remove_toml_field<A>(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    field: &str,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    let mut doc = serde_json::to_value(entry.spec())?;

    let (display_field, removed) = if let Some(simple) = fp.as_simple() {
        let removed = remove_simple_values_from_doc(artifact, &mut doc, simple, id, opts)?
            .ok_or_else(|| cannot_remove_from_field_error(id, simple))?;
        (simple.to_string(), removed)
    } else if fp.segments.len() >= 2 {
        // Nested subfield remove: e.g., alt[0].pros or alt[0].cons[1]
        let removed = if fp.has_terminal_index() {
            let sub_idx = fp.segments.last().and_then(|seg| seg.index).unwrap();
            let mut container_fp = fp.clone();
            if let Some(last) = container_fp.segments.last_mut() {
                last.index = None;
            }
            edit_runtime::remove_nested_list_values(
                artifact,
                &mut doc,
                &container_fp,
                id,
                |items| {
                    let resolved = self::path::resolve_index(sub_idx, items.len())?;
                    Ok(vec![resolved])
                },
            )?
        } else {
            edit_runtime::remove_nested_list_values(artifact, &mut doc, fp, id, |items| {
                resolve_match_indices(id, field, items, opts, MatchUse::Remove)
            })?
        };
        (field.to_string(), removed)
    } else {
        // Root item remove: e.g., alt[0], journal[0], notes[0]
        let removed = edit_runtime::remove_nested_root_item(artifact, &mut doc, fp, id)?;
        (field.to_string(), vec![removed])
    };

    *entry.spec_mut() = serde_json::from_value(doc)?;
    A::write(config, &entry, op)?;
    notify_removed(id, &display_field, &removed, op);
    Ok(())
}

fn remove_json_simple_list_field<A>(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
    nested_error: &str,
) -> anyhow::Result<()>
where
    A: JsonAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let simple = require_simple_field(fp, id, nested_error)?;
    let mut loaded = A::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
    let removed = remove_simple_values_from_doc(artifact, &mut doc, simple, id, opts)?
        .ok_or_else(|| cannot_remove_from_field_error(id, simple))?;
    loaded.data = serde_json::from_value(doc)?;
    A::write(config, &loaded, op)?;
    notify_removed(id, simple, &removed, op);
    Ok(())
}

fn tick_toml_field<A>(
    config: &Config,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
    status_str: &str,
) -> anyhow::Result<String>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    if !edit_rules::simple_field_supports_verb(artifact.rule_key(), field, edit_rules::Verb::Tick) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0803UnknownField,
            format!("Unknown field for tick: {field}"),
            id,
        )
        .into());
    }
    let mut entry = A::load(config, id)?;
    let mut doc = serde_json::to_value(entry.spec())?;
    let ticked_text = edit_runtime::tick_simple_status_list_item_with_matcher(
        artifact,
        &mut doc,
        field,
        id,
        status_str,
        |items| resolve_match_indices(id, field, items, opts, MatchUse::TickSingle),
    )?
    .unwrap_or_else(|| unreachable!("validated fields with tick verb must be status lists"));
    *entry.spec_mut() = serde_json::from_value(doc)?;
    A::write(config, &entry, op)?;
    Ok(ticked_text)
}

fn confirm_delete_prompt(force: bool, op: WriteOp, prompt: &str) -> anyhow::Result<bool> {
    if force || op.is_preview() {
        return Ok(true);
    }
    use std::io::{self, Write};
    print!("{prompt} [y/N] ");
    io::stdout().flush()?;
    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    if !response.trim().eq_ignore_ascii_case("y") {
        ui::info("Deletion cancelled");
        return Ok(false);
    }
    Ok(true)
}

pub fn delete_clause(
    config: &Config,
    clause_id: &str,
    force: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    use crate::model::RfcStatus;

    let parts: Vec<&str> = clause_id.split(':').collect();
    if parts.len() != 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            "Invalid clause ID format. Expected RFC-NNNN:C-NAME",
            clause_id,
        )
        .into());
    }

    let rfc_id = parts[0];
    let clause_name = parts[1];

    let rfc_loaded = RfcJsonAdapter::load(config, rfc_id)?;
    if rfc_loaded.data.status != RfcStatus::Draft {
        return Err(Diagnostic::new(
            DiagnosticCode::E0110RfcInvalidId,
            format!(
                "Cannot delete clause: {} is {}. Only draft RFCs allow clause deletion.",
                rfc_id,
                rfc_loaded.data.status.as_ref()
            ),
            clause_id,
        )
        .into());
    }

    let clause_path = crate::load::find_clause_json(config, clause_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Clause not found: {}", clause_id),
            clause_id,
        )
    })?;

    let clause_file_name = clause_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0204ClausePathInvalid,
                format!("Invalid clause file path: {}", clause_path.display()),
                clause_id,
            )
        })?;

    if !confirm_delete_prompt(
        force,
        op,
        &format!("Delete clause {} from {}?", clause_name, rfc_id),
    )? {
        return Ok(vec![]);
    }

    let mut rfc = rfc_loaded.data.clone();
    let clause_rel_path = format!("clauses/{}", clause_file_name);

    let mut removed = false;
    for section in &mut rfc.sections {
        if let Some(pos) = section.clauses.iter().position(|c| c == &clause_rel_path) {
            section.clauses.remove(pos);
            removed = true;
            break;
        }
    }

    if !removed {
        return Err(Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!(
                "Clause {} not found in any section of {}",
                clause_name, rfc_id
            ),
            clause_id,
        )
        .into());
    }

    crate::write::write_rfc(
        &rfc_loaded.path,
        &rfc,
        op,
        Some(&config.display_path(&rfc_loaded.path)),
    )?;

    delete_file(&clause_path, op, Some(&config.display_path(&clause_path)))?;

    if !op.is_preview() {
        ui::success(format!("Deleted clause {}", clause_id));
    }

    Ok(vec![])
}

pub fn delete_work_item(
    config: &Config,
    id: &str,
    force: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    use crate::load::load_project_with_warnings;
    use crate::model::WorkItemStatus;

    let entry = WorkTomlAdapter::load(config, id)?;
    let wi = &entry.spec;

    if wi.govctl.status != WorkItemStatus::Queue {
        return Err(Diagnostic::new(
            DiagnosticCode::E0402WorkNotFound,
            format!(
                "Cannot delete work item: {} is {}. Only queued work items can be deleted. Use 'mv {} cancelled' for active items.",
                wi.govctl.id,
                wi.govctl.status.as_ref(),
                wi.govctl.id
            ),
            id,
        )
        .into());
    }

    // Check for references in other artifacts
    let load_result = match load_project_with_warnings(config) {
        Ok(result) => result,
        Err(_) => {
            return proceed_with_deletion(config, &entry.path, &wi.govctl.id, force, op);
        }
    };

    let index = &load_result.index;
    let mut referenced_by = Vec::new();

    for rfc in &index.rfcs {
        if rfc.rfc.refs.contains(&wi.govctl.id) {
            referenced_by.push(rfc.rfc.rfc_id.clone());
        }
    }

    for adr in &index.adrs {
        if adr.spec.govctl.refs.contains(&wi.govctl.id) {
            referenced_by.push(adr.spec.govctl.id.clone());
        }
    }

    for other_wi in &index.work_items {
        if other_wi.spec.govctl.id != wi.govctl.id
            && other_wi.spec.govctl.refs.contains(&wi.govctl.id)
        {
            referenced_by.push(other_wi.spec.govctl.id.clone());
        }
    }

    if !referenced_by.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0404WorkRefNotFound,
            format!(
                "Cannot delete work item: {} is referenced by: {}. Remove references first.",
                wi.govctl.id,
                referenced_by.join(", ")
            ),
            id,
        )
        .into());
    }

    proceed_with_deletion(config, &entry.path, &wi.govctl.id, force, op)
}

fn proceed_with_deletion(
    config: &Config,
    path: &std::path::Path,
    id: &str,
    force: bool,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    if !confirm_delete_prompt(force, op, &format!("Delete work item {}?", id))? {
        return Ok(vec![]);
    }

    delete_file(path, op, Some(&config.display_path(path)))?;

    if !op.is_preview() {
        ui::success(format!("Deleted work item {}", id));
    }

    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum MatchResult {
        None,
        Single(usize),
        Multiple(Vec<usize>),
    }

    fn find_matches(items: &[&str], opts: &MatchOptions) -> anyhow::Result<MatchResult> {
        if let Some(idx) = opts.at {
            let len = items.len() as i32;
            let resolved = if idx < 0 { len + idx } else { idx };
            if resolved < 0 || resolved >= len {
                return Err(anyhow::anyhow!(
                    "Index {} out of range (array has {} items)",
                    idx,
                    items.len()
                ));
            }
            return Ok(MatchResult::Single(resolved as usize));
        }

        let pattern = opts
            .pattern
            .ok_or_else(|| anyhow::anyhow!("Pattern or --at is required"))?;

        let indices = if opts.regex {
            let re = Regex::new(pattern).map_err(|e| anyhow::anyhow!("Invalid regex: {e}"))?;
            items
                .iter()
                .enumerate()
                .filter(|(_, s)| re.is_match(s))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        } else if opts.exact {
            items
                .iter()
                .enumerate()
                .filter(|(_, s)| **s == pattern)
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        } else {
            let pattern_lower = pattern.to_lowercase();
            items
                .iter()
                .enumerate()
                .filter(|(_, s)| s.to_lowercase().contains(&pattern_lower))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        };

        Ok(match indices.len() {
            0 => MatchResult::None,
            1 => MatchResult::Single(indices[0]),
            _ => MatchResult::Multiple(indices),
        })
    }

    fn remove_indices<T>(vec: &mut Vec<T>, indices: &[usize]) {
        let mut sorted: Vec<usize> = indices.to_vec();
        sorted.sort_by(|a, b| b.cmp(a)); // descending
        for i in sorted {
            vec.remove(i);
        }
    }

    // =========================================================================
    // ArtifactType::from_id Tests
    // =========================================================================

    #[test]
    fn test_artifact_type_clause() {
        assert_eq!(
            ArtifactType::from_id("RFC-0001:C-NAME"),
            Some(ArtifactType::Clause)
        );
        assert_eq!(
            ArtifactType::from_id("RFC-0000:C-SUMMARY"),
            Some(ArtifactType::Clause)
        );
    }

    #[test]
    fn test_artifact_type_rfc() {
        assert_eq!(ArtifactType::from_id("RFC-0001"), Some(ArtifactType::Rfc));
        assert_eq!(ArtifactType::from_id("RFC-9999"), Some(ArtifactType::Rfc));
    }

    #[test]
    fn test_artifact_type_adr() {
        assert_eq!(ArtifactType::from_id("ADR-0001"), Some(ArtifactType::Adr));
        assert_eq!(ArtifactType::from_id("ADR-0007"), Some(ArtifactType::Adr));
    }

    #[test]
    fn test_artifact_type_work_item_by_prefix() {
        assert_eq!(
            ArtifactType::from_id("WI-2026-01-17-001"),
            Some(ArtifactType::WorkItem)
        );
    }

    #[test]
    fn test_artifact_type_work_item_by_hyphen() {
        // Any ID with hyphen that doesn't match RFC/ADR/Clause is WorkItem
        assert_eq!(
            ArtifactType::from_id("2026-01-17-add-tests"),
            Some(ArtifactType::WorkItem)
        );
    }

    #[test]
    fn test_artifact_type_unknown() {
        assert_eq!(ArtifactType::from_id("UNKNOWN"), None);
        assert_eq!(ArtifactType::from_id("foo"), None);
    }

    // =========================================================================
    // find_matches Tests - Substring (Default)
    // =========================================================================

    #[test]
    fn test_find_matches_substring_single() {
        let items = vec!["apple", "banana", "cherry"];
        let opts = MatchOptions {
            pattern: Some("nan"),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 1),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_substring_case_insensitive() {
        let items = vec!["Apple", "BANANA", "Cherry"];
        let opts = MatchOptions {
            pattern: Some("banana"),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 1),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_substring_multiple() {
        let items = vec!["test-one", "test-two", "other"];
        let opts = MatchOptions {
            pattern: Some("test"),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Multiple(indices) => assert_eq!(indices, vec![0, 1]),
            _ => panic!("Expected multiple matches"),
        }
    }

    #[test]
    fn test_find_matches_substring_none() {
        let items = vec!["apple", "banana", "cherry"];
        let opts = MatchOptions {
            pattern: Some("xyz"),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::None => {}
            _ => panic!("Expected no match"),
        }
    }

    // =========================================================================
    // find_matches Tests - Exact Match
    // =========================================================================

    #[test]
    fn test_find_matches_exact_match() {
        let items = vec!["test", "testing", "test"];
        let opts = MatchOptions {
            pattern: Some("test"),
            exact: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Multiple(indices) => assert_eq!(indices, vec![0, 2]),
            _ => panic!("Expected multiple matches"),
        }
    }

    #[test]
    fn test_find_matches_exact_case_sensitive() {
        let items = vec!["Test", "test", "TEST"];
        let opts = MatchOptions {
            pattern: Some("test"),
            exact: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 1),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_exact_no_match() {
        let items = vec!["testing", "tested"];
        let opts = MatchOptions {
            pattern: Some("test"),
            exact: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::None => {}
            _ => panic!("Expected no match"),
        }
    }

    // =========================================================================
    // find_matches Tests - Index-based
    // =========================================================================

    #[test]
    fn test_find_matches_at_positive() {
        let items = vec!["a", "b", "c"];
        let opts = MatchOptions {
            at: Some(1),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 1),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_at_zero() {
        let items = vec!["first", "second"];
        let opts = MatchOptions {
            at: Some(0),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 0),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_at_negative() {
        let items = vec!["a", "b", "c"];
        let opts = MatchOptions {
            at: Some(-1),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 2), // last item
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_at_negative_two() {
        let items = vec!["a", "b", "c", "d"];
        let opts = MatchOptions {
            at: Some(-2),
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 2), // second to last
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_at_out_of_range() {
        let items = vec!["a", "b"];
        let opts = MatchOptions {
            at: Some(5),
            ..Default::default()
        };
        assert!(find_matches(&items, &opts).is_err());
    }

    #[test]
    fn test_find_matches_at_negative_out_of_range() {
        let items = vec!["a", "b"];
        let opts = MatchOptions {
            at: Some(-5),
            ..Default::default()
        };
        assert!(find_matches(&items, &opts).is_err());
    }

    // =========================================================================
    // find_matches Tests - Regex
    // =========================================================================

    #[test]
    fn test_find_matches_regex_single() {
        let items = vec!["RFC-0001", "ADR-0001", "WI-001"];
        let opts = MatchOptions {
            pattern: Some("RFC-.*"),
            regex: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Single(idx) => assert_eq!(idx, 0),
            _ => panic!("Expected single match"),
        }
    }

    #[test]
    fn test_find_matches_regex_multiple() {
        let items = vec!["test-1", "test-2", "other"];
        let opts = MatchOptions {
            pattern: Some("test-\\d+"),
            regex: true,
            ..Default::default()
        };
        match find_matches(&items, &opts).unwrap() {
            MatchResult::Multiple(indices) => assert_eq!(indices, vec![0, 1]),
            _ => panic!("Expected multiple matches"),
        }
    }

    #[test]
    fn test_find_matches_regex_invalid() {
        let items = vec!["a", "b"];
        let opts = MatchOptions {
            pattern: Some("[invalid"),
            regex: true,
            ..Default::default()
        };
        assert!(find_matches(&items, &opts).is_err());
    }

    // =========================================================================
    // remove_indices Tests
    // =========================================================================

    #[test]
    fn test_remove_indices_single() {
        let mut items = vec!["a", "b", "c"];
        remove_indices(&mut items, &[1]);
        assert_eq!(items, vec!["a", "c"]);
    }

    #[test]
    fn test_remove_indices_multiple() {
        let mut items = vec!["a", "b", "c", "d"];
        remove_indices(&mut items, &[1, 3]);
        assert_eq!(items, vec!["a", "c"]);
    }

    #[test]
    fn test_remove_indices_all() {
        let mut items = vec!["a", "b", "c"];
        remove_indices(&mut items, &[0, 1, 2]);
        assert!(items.is_empty());
    }

    #[test]
    fn test_remove_indices_preserves_order() {
        let mut items = vec!["1", "2", "3", "4", "5"];
        remove_indices(&mut items, &[0, 2, 4]);
        assert_eq!(items, vec!["2", "4"]);
    }

    #[test]
    fn test_remove_indices_empty() {
        let mut items = vec!["a", "b"];
        remove_indices(&mut items, &[]);
        assert_eq!(items, vec!["a", "b"]);
    }
}

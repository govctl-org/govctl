//! Edit command implementation - modify artifacts.
//!
//! Implements [[ADR-0007]] ergonomic array field matching for remove and tick commands.

use crate::cmd::edit_adapter::{
    AdrTomlAdapter, ClauseJsonAdapter, JsonAdapter, RfcJsonAdapter, TomlAdapter, WorkTomlAdapter,
};
use crate::cmd::path::{self, FieldPath};
use crate::cmd::{edit_engine, edit_rules, edit_runtime};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrEntry, AdrSpec, WorkItemEntry, WorkItemSpec};
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
}

impl ArtifactType {
    pub fn from_id(id: &str) -> Option<Self> {
        if id.contains(':') {
            Some(Self::Clause)
        } else if id.starts_with("RFC-") {
            Some(Self::Rfc)
        } else if id.starts_with("ADR-") {
            Some(Self::Adr)
        } else if id.starts_with("WI-") || id.contains('-') {
            Some(Self::WorkItem)
        } else {
            None
        }
    }

    pub fn unknown_error(id: &str) -> anyhow::Error {
        anyhow::anyhow!("Unknown artifact type: {id}")
    }

    pub fn rule_key(self) -> &'static str {
        match self {
            Self::Clause => "clause",
            Self::Rfc => "rfc",
            Self::Adr => "adr",
            Self::WorkItem => "work",
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
            let re = Regex::new(pattern).map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
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

fn plan_edit_with_field(id: &str, field: &str) -> anyhow::Result<(ArtifactType, FieldPath)> {
    let plan = edit_engine::plan_request(id, Some(field))?;
    let fp = plan
        .field_path
        .ok_or_else(|| anyhow::anyhow!("Field path required"))?;
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

const TICK_NESTED_PATH_ERROR: &str = "tick does not support nested paths";
const TICK_UNSUPPORTED_ARTIFACT_ERROR: &str = "Tick only works for work items and ADRs: {id}";
const ADR_NESTED_ERROR: &str = "ADR nested paths only support 'alternatives' (got '{}')";
const WORK_NESTED_GET_ERROR: &str =
    "Work item nested paths support journal, acceptance_criteria, or notes (got '{}')";
const WORK_NESTED_SET_ERROR: &str =
    "Work item nested set supports journal or acceptance_criteria (got '{}')";
const WORK_NESTED_REMOVE_ERROR: &str =
    "Work item nested remove supports notes, acceptance_criteria, or journal (got '{}')";

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
    let (artifact, fp) = plan_edit_with_field(id, field)?;
    let value = resolve_value(value, stdin)?;
    apply_set_field(config, id, &fp, artifact, value.as_str(), op)?;

    if !op.is_preview() {
        let display_field = format_path(&fp);
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
    let (artifact, fp) = plan_edit_with_field(id, field)?;
    apply_set_field(config, id, &fp, artifact, value, op)
}

fn apply_set_field(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    artifact: ArtifactType,
    value: &str,
    op: WriteOp,
) -> anyhow::Result<()> {
    match artifact {
        ArtifactType::Adr => set_toml_field::<AdrTomlAdapter>(
            config,
            id,
            fp,
            value,
            op,
            ArtifactType::Adr,
            adr_nested_set_alternatives,
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
                work_nested_set_dispatch,
            )?
        }
        ArtifactType::Rfc => set_json_field::<RfcJsonAdapter, _>(
            config,
            id,
            fp,
            value,
            op,
            ArtifactType::Rfc,
            crate::validate::ArtifactKind::Rfc,
            "RFC fields do not support nested paths",
            |spec| {
                spec.updated = Some(today());
            },
        )?,
        ArtifactType::Clause => set_json_field::<ClauseJsonAdapter, _>(
            config,
            id,
            fp,
            value,
            op,
            ArtifactType::Clause,
            crate::validate::ArtifactKind::Clause,
            "Clause fields do not support nested paths",
            |_spec| {},
        )?,
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
            adr_nested_get_alternatives,
        )?,
        ArtifactType::WorkItem => get_toml_field::<WorkTomlAdapter>(
            config,
            id,
            plan.field_path.as_ref(),
            ArtifactType::WorkItem,
            work_nested_get_dispatch,
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
    }

    Ok(vec![])
}

fn get_toml_field<A>(
    config: &Config,
    id: &str,
    fp: Option<&FieldPath>,
    artifact: ArtifactType,
    nested_handler: fn(&A::Entry, &FieldPath, &str) -> anyhow::Result<String>,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let entry = A::load(config, id)?;
    if let Some(fp) = fp {
        if let Some(simple) = fp.as_simple() {
            let doc = serde_json::to_value(entry.spec())?;
            println!(
                "{}",
                edit_runtime::get_simple_field(artifact, &doc, simple, id)?
            );
        } else {
            println!("{}", nested_handler(&entry, fp, id)?);
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
    nested_handler: fn(&mut A::Entry, &FieldPath, &str, &str) -> anyhow::Result<()>,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    if let Some(simple) = fp.as_simple() {
        let mut doc = serde_json::to_value(entry.spec())?;
        edit_runtime::set_simple_field(artifact, &mut doc, simple, value, id)?;
        *entry.spec_mut() = serde_json::from_value(doc)?;
    } else {
        nested_handler(&mut entry, fp, value, id)?;
    }
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

fn set_json_field<A, F>(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    value: &str,
    op: WriteOp,
    artifact: ArtifactType,
    kind: crate::validate::ArtifactKind,
    nested_error: &str,
    post_update: F,
) -> anyhow::Result<()>
where
    A: JsonAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
    F: FnOnce(&mut A::Data),
{
    let simple = require_simple_field(fp, id, nested_error)?;
    if !edit_runtime::supports_simple_set_field(artifact, simple) {
        let code = match kind {
            crate::validate::ArtifactKind::Rfc => DiagnosticCode::E0101RfcSchemaInvalid,
            crate::validate::ArtifactKind::Clause => DiagnosticCode::E0201ClauseSchemaInvalid,
            _ => DiagnosticCode::E0803UnknownField,
        };
        return Err(Diagnostic::new(code, format!("Unknown field: {simple}"), "").into());
    }
    crate::validate::validate_field(config, id, kind, simple, value)?;
    let mut loaded = A::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
    edit_runtime::set_simple_field(artifact, &mut doc, simple, value, id)?;
    loaded.data = serde_json::from_value(doc)?;
    post_update(&mut loaded.data);
    A::write(config, &loaded, op)?;
    Ok(())
}

macro_rules! define_object_scalar_access {
    ($get_fn:ident, $set_fn:ident, $ty:ty, get: { $($gname:literal => |$gobj:ident| $gexpr:expr),+ $(,)? }, set: { $($sname:literal => |$sobj:ident, $svalue:ident| $sexpr:expr),+ $(,)? }) => {
        fn $get_fn(item: &$ty, field: &str) -> Option<anyhow::Result<String>> {
            match field {
                $(
                    $gname => {
                        let $gobj = item;
                        Some(Ok($gexpr))
                    }
                ),+,
                _ => None,
            }
        }

        fn $set_fn(item: &mut $ty, field: &str, value: &str) -> Option<anyhow::Result<()>> {
            match field {
                $(
                    $sname => {
                        let $sobj = item;
                        let $svalue = value;
                        Some($sexpr)
                    }
                ),+,
                _ => None,
            }
        }
    };
}

define_object_scalar_access!(
    adr_alt_get_scalar,
    adr_alt_set_scalar,
    crate::model::Alternative,
    get: {
        "text" => |alt| alt.text.clone(),
        "status" => |alt| alt.status.as_ref().to_string(),
        "rejection_reason" => |alt| alt.rejection_reason.clone().unwrap_or_default()
    },
    set: {
        "text" => |alt, value| { alt.text = value.to_string(); Ok(()) },
        "status" => |alt, value| parse_enum_field(value, "Invalid alternative status").map(|status| { alt.status = status; }),
        "rejection_reason" => |alt, value| { set_option_string_field(&mut alt.rejection_reason, value); Ok(()) }
    }
);

fn adr_alt_list_field_mut<'a>(
    alt: &'a mut crate::model::Alternative,
    field: &str,
) -> Option<&'a mut Vec<String>> {
    match field {
        "pros" => Some(&mut alt.pros),
        "cons" => Some(&mut alt.cons),
        _ => None,
    }
}

fn adr_alt_index_and_subfield<'a>(
    fp: &'a FieldPath,
    id: &str,
    len: usize,
    verb: edit_rules::Verb,
) -> anyhow::Result<(usize, Option<&'a path::PathSegment>)> {
    if fp.segments[0].name != "alternatives" {
        return nested_root_field_not_found(id, fp.segments[0].name.as_str(), ADR_NESTED_ERROR);
    }
    let (idx, max_depth) = nested_index_and_depth(fp, len, "adr", "alternatives")?;
    if fp.segments.len() == 1 {
        let missing_subfield_error = match verb {
            edit_rules::Verb::Set => {
                Some("Cannot set an entire alternative; use a sub-field (e.g., alt[0].text)")
            }
            edit_rules::Verb::Add => {
                Some("Cannot add to alternative without sub-field (use alt[0].pros or alt[0].cons)")
            }
            _ => None,
        };
        if let Some(msg) = missing_subfield_error {
            return Err(Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, msg, id).into());
        }
        return Ok((idx, None));
    }
    ensure_nested_depth(fp, max_depth, "ADR alternatives", id)?;
    let seg1 = &fp.segments[1];
    let unsupported = match verb {
        edit_rules::Verb::Get => "Cannot get alternative field '{}'",
        edit_rules::Verb::Set => "Cannot set alternative field '{}'",
        edit_rules::Verb::Add => {
            "Cannot add to alternative field '{}' (only pros and cons support add)"
        }
        edit_rules::Verb::Remove => {
            "Cannot remove from alternative field '{}' (only pros and cons support remove)"
        }
        edit_rules::Verb::Tick => unreachable!("adr alternative nested does not support tick"),
    };
    ensure_nested_field_for_verb(
        "adr",
        "alternatives",
        seg1,
        verb,
        fp,
        id,
        format!(
            "Unknown alternative field: '{}' (expected text, status, pros, cons, or rejection_reason)",
            seg1.name
        ),
        unsupported.replace("{}", seg1.name.as_str()),
    )?;
    Ok((idx, Some(seg1)))
}

fn adr_nested_get_alternatives(
    entry: &AdrEntry,
    fp: &FieldPath,
    id: &str,
) -> anyhow::Result<String> {
    let alts = &entry.spec.content.alternatives;
    let (idx, seg1) = adr_alt_index_and_subfield(fp, id, alts.len(), edit_rules::Verb::Get)?;
    let alt = &alts[idx];
    let Some(seg1) = seg1 else {
        return Ok(toml::to_string_pretty(alt)?);
    };
    let items = match seg1.name.as_str() {
        "pros" => Some(alt.pros.as_slice()),
        "cons" => Some(alt.cons.as_slice()),
        _ => None,
    };
    if let Some(items) = items {
        if let Some(j) = seg1.index {
            return Ok(items[path::resolve_index(j, items.len())?].clone());
        }
        return Ok(items.join("\n"));
    }
    match adr_alt_get_scalar(alt, seg1.name.as_str()) {
        Some(v) => v,
        None => unreachable!("validated by edit rules"),
    }
}

fn adr_nested_set_alternatives(
    entry: &mut AdrEntry,
    fp: &FieldPath,
    value: &str,
    id: &str,
) -> anyhow::Result<()> {
    let alts = &mut entry.spec.content.alternatives;
    let (idx, seg1) = adr_alt_index_and_subfield(fp, id, alts.len(), edit_rules::Verb::Set)?;
    let seg1 = seg1.expect("set requires subfield by construction");
    let alt = &mut alts[idx];
    if let Some(items) = adr_alt_list_field_mut(alt, seg1.name.as_str()) {
        let j = seg1.index.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!(
                    "Use 'add' to append to {}, or provide an index (e.g., alt[0].{}[0])",
                    seg1.name, seg1.name
                ),
                id,
            )
        })?;
        let ji = path::resolve_index(j, items.len())?;
        items[ji] = value.to_string();
        return Ok(());
    }
    match adr_alt_set_scalar(alt, seg1.name.as_str(), value) {
        Some(v) => v,
        None => unreachable!("validated by edit rules"),
    }
}

fn adr_nested_add_alternatives(
    entry: &mut AdrEntry,
    fp: &FieldPath,
    value: &str,
    id: &str,
) -> anyhow::Result<()> {
    let alts = &mut entry.spec.content.alternatives;
    let (idx, seg1) = adr_alt_index_and_subfield(fp, id, alts.len(), edit_rules::Verb::Add)?;
    let seg1 = seg1.expect("add requires subfield by construction");
    let alt = &mut alts[idx];
    let items = adr_alt_list_field_mut(alt, seg1.name.as_str()).expect("validated by edit rules");
    if seg1.index.is_some() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!(
                "Cannot add to indexed path '{}' (use set/remove for a specific element)",
                format_path(fp)
            ),
            id,
        )
        .into());
    }
    if !items.iter().any(|item| item == value) {
        items.push(value.to_string());
    }
    Ok(())
}

fn adr_nested_remove_alternatives(
    entry: &mut AdrEntry,
    fp: &FieldPath,
    opts: &MatchOptions,
    id: &str,
) -> anyhow::Result<Vec<String>> {
    let alts = &mut entry.spec.content.alternatives;
    let (idx, seg1) = adr_alt_index_and_subfield(fp, id, alts.len(), edit_rules::Verb::Remove)?;
    let Some(seg1) = seg1 else {
        let removed = alts.remove(idx);
        return Ok(vec![removed.text]);
    };

    let alt = &mut alts[idx];
    let items = adr_alt_list_field_mut(alt, seg1.name.as_str()).expect("validated by edit rules");
    if let Some(j) = seg1.index {
        return Ok(vec![items.remove(path::resolve_index(j, items.len())?)]);
    }

    let texts: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let to_remove = resolve_match_indices(id, seg1.name.as_str(), &texts, opts, MatchUse::Remove)?;
    let removed: Vec<String> = to_remove.iter().map(|&i| items[i].clone()).collect();
    let mut sorted = to_remove;
    sorted.sort_by(|a, b| b.cmp(a));
    for i in sorted {
        items.remove(i);
    }
    Ok(removed)
}

#[derive(Clone, Copy)]
struct WorkNestedObjectSpec {
    root: &'static str,
    scope: &'static str,
    expected: &'static str,
    whole_item_error: &'static str,
}

const WORK_JOURNAL_SPEC: WorkNestedObjectSpec = WorkNestedObjectSpec {
    root: "journal",
    scope: "journal entries",
    expected: "content, date, or scope",
    whole_item_error: "Cannot set an entire journal entry; use a sub-field (e.g., journal[0].content)",
};

const WORK_ACCEPTANCE_CRITERIA_SPEC: WorkNestedObjectSpec = WorkNestedObjectSpec {
    root: "acceptance_criteria",
    scope: "acceptance criteria",
    expected: "text, status, or category",
    whole_item_error: "Cannot set an entire acceptance criterion; use a sub-field (e.g., ac[0].text)",
};

define_object_scalar_access!(
    journal_get_field,
    journal_set_field,
    crate::model::JournalEntry,
    get: {
        "content" => |item| item.content.clone(),
        "date" => |item| item.date.clone(),
        "scope" => |item| item.scope.clone().unwrap_or_default()
    },
    set: {
        "content" => |item, value| { item.content = value.to_string(); Ok(()) },
        "date" => |item, value| { item.date = value.to_string(); Ok(()) },
        "scope" => |item, value| { set_option_string_field(&mut item.scope, value); Ok(()) }
    }
);

define_object_scalar_access!(
    ac_get_field,
    ac_set_field,
    crate::model::ChecklistItem,
    get: {
        "text" => |item| item.text.clone(),
        "status" => |item| item.status.as_ref().to_string(),
        "category" => |item| item.category.as_ref().to_string()
    },
    set: {
        "text" => |item, value| { item.text = value.to_string(); Ok(()) },
        "status" => |item, value| parse_enum_field(value, "Invalid checklist status").map(|status| { item.status = status; }),
        "category" => |item, value| parse_enum_field(value, "Invalid category").map(|category| { item.category = category; })
    }
);

fn work_nested_get_dispatch(
    entry: &WorkItemEntry,
    fp: &FieldPath,
    id: &str,
) -> anyhow::Result<String> {
    match fp.segments[0].name.as_str() {
        "journal" => work_nested_get_object_item(
            &entry.spec.content.journal,
            fp,
            id,
            WORK_JOURNAL_SPEC,
            |item| Ok(toml::to_string_pretty(item)?),
            journal_get_field,
        ),
        "acceptance_criteria" => work_nested_get_object_item(
            &entry.spec.content.acceptance_criteria,
            fp,
            id,
            WORK_ACCEPTANCE_CRITERIA_SPEC,
            |item| Ok(format!("[{}] {}", item.status.as_ref(), item.text)),
            ac_get_field,
        ),
        "notes" => {
            ensure_no_subfields(
                fp,
                id,
                "Notes entries do not have sub-fields (use notes[index])",
            )?;
            let (idx, _max_depth) =
                nested_index_and_depth(fp, entry.spec.content.notes.len(), "work", "notes")?;
            Ok(entry.spec.content.notes[idx].clone())
        }
        root => nested_root_field_not_found(id, root, WORK_NESTED_GET_ERROR),
    }
}

fn work_nested_set_dispatch(
    entry: &mut WorkItemEntry,
    fp: &FieldPath,
    value: &str,
    id: &str,
) -> anyhow::Result<()> {
    match fp.segments[0].name.as_str() {
        "journal" => work_nested_set_object_item(
            &mut entry.spec.content.journal,
            fp,
            value,
            id,
            WORK_JOURNAL_SPEC,
            journal_set_field,
        ),
        "acceptance_criteria" => work_nested_set_object_item(
            &mut entry.spec.content.acceptance_criteria,
            fp,
            value,
            id,
            WORK_ACCEPTANCE_CRITERIA_SPEC,
            ac_set_field,
        ),
        root => nested_root_field_not_found(id, root, WORK_NESTED_SET_ERROR),
    }
}

fn work_nested_remove_dispatch(
    entry: &mut WorkItemEntry,
    fp: &FieldPath,
    _opts: &MatchOptions,
    id: &str,
) -> anyhow::Result<Vec<String>> {
    match fp.segments[0].name.as_str() {
        "journal" => remove_work_nested_root_item(
            &mut entry.spec.content.journal,
            fp,
            id,
            "journal",
            |item| item.content,
        ),
        "acceptance_criteria" => remove_work_nested_root_item(
            &mut entry.spec.content.acceptance_criteria,
            fp,
            id,
            "acceptance_criteria",
            |item| item.text,
        ),
        "notes" => remove_work_nested_root_item(
            &mut entry.spec.content.notes,
            fp,
            id,
            "notes",
            std::convert::identity,
        ),
        root => nested_root_field_not_found(id, root, WORK_NESTED_REMOVE_ERROR),
    }
}

fn nested_root_field_not_found<T>(id: &str, root: &str, fmt: &str) -> anyhow::Result<T> {
    Err(Diagnostic::new(
        DiagnosticCode::E0815PathFieldNotFound,
        fmt.replace("{}", root),
        id,
    )
    .into())
}

fn work_nested_get_object_item<T, FRoot, FGet>(
    items: &[T],
    fp: &FieldPath,
    id: &str,
    spec: WorkNestedObjectSpec,
    render_root: FRoot,
    get_field: FGet,
) -> anyhow::Result<String>
where
    FRoot: Fn(&T) -> anyhow::Result<String>,
    FGet: Fn(&T, &str) -> Option<anyhow::Result<String>>,
{
    let (idx, seg1) =
        work_nested_object_index_and_field(fp, id, items.len(), spec, edit_rules::Verb::Get)?;
    let item = &items[idx];
    let Some(seg1) = seg1 else {
        return render_root(item);
    };
    match get_field(item, seg1.name.as_str()) {
        Some(value) => value,
        None => unreachable!("validated by edit rules"),
    }
}

#[allow(clippy::too_many_arguments)]
fn work_nested_set_object_item<T, FSet>(
    items: &mut [T],
    fp: &FieldPath,
    value: &str,
    id: &str,
    spec: WorkNestedObjectSpec,
    set_field: FSet,
) -> anyhow::Result<()>
where
    FSet: Fn(&mut T, &str, &str) -> Option<anyhow::Result<()>>,
{
    let (idx, seg1) =
        work_nested_object_index_and_field(fp, id, items.len(), spec, edit_rules::Verb::Set)?;
    let Some(seg1) = seg1 else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            spec.whole_item_error,
            id,
        )
        .into());
    };
    let item = &mut items[idx];
    match set_field(item, seg1.name.as_str(), value) {
        Some(result) => result,
        None => unreachable!("validated by edit rules"),
    }
}

fn work_nested_object_index_and_field<'a>(
    fp: &'a FieldPath,
    id: &str,
    len: usize,
    spec: WorkNestedObjectSpec,
    verb: edit_rules::Verb,
) -> anyhow::Result<(usize, Option<&'a path::PathSegment>)> {
    let (idx, max_depth) = nested_index_and_depth(fp, len, "work", spec.root)?;
    if fp.segments.len() == 1 {
        return Ok((idx, None));
    }

    ensure_nested_depth(fp, max_depth, spec.scope, id)?;
    let seg1 = &fp.segments[1];
    let unsupported = match verb {
        edit_rules::Verb::Get => format!("Cannot get {} field '{}'", spec.root, seg1.name),
        edit_rules::Verb::Set => format!("Cannot set {} field '{}'", spec.root, seg1.name),
        _ => unreachable!("work nested object fields only support get/set"),
    };
    ensure_nested_field_for_verb(
        "work",
        spec.root,
        seg1,
        verb,
        fp,
        id,
        format!(
            "Unknown {} field: '{}' (expected {})",
            spec.root, seg1.name, spec.expected
        ),
        unsupported,
    )?;
    Ok((idx, Some(seg1)))
}

fn nested_index_and_depth(
    fp: &FieldPath,
    len: usize,
    artifact: &str,
    root: &str,
) -> anyhow::Result<(usize, usize)> {
    let seg0 = &fp.segments[0];
    let idx = path::require_index(seg0, len)?;
    let max_depth = edit_rules::nested_root_rule(artifact, root)
        .map(|r| r.max_depth)
        .unwrap_or(2);
    Ok((idx, max_depth))
}

fn remove_work_nested_root_item<T, F>(
    items: &mut Vec<T>,
    fp: &FieldPath,
    id: &str,
    root: &str,
    to_removed_text: F,
) -> anyhow::Result<Vec<String>>
where
    F: FnOnce(T) -> String,
{
    ensure_no_subfields(
        fp,
        id,
        "Work item nested remove only supports top-level indexed removal (e.g., notes[0], ac[0], journal[0])",
    )?;
    let (idx, _max_depth) = nested_index_and_depth(fp, items.len(), "work", root)?;
    let removed = items.remove(idx);
    Ok(vec![to_removed_text(removed)])
}

fn ensure_no_subfields(fp: &FieldPath, id: &str, message: &str) -> anyhow::Result<()> {
    if fp.segments.len() > 1 {
        return Err(Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, message, id).into());
    }
    Ok(())
}

fn set_option_string_field(dst: &mut Option<String>, value: &str) {
    *dst = (!value.is_empty()).then(|| value.to_string());
}

fn parse_enum_field<T>(value: &str, err_prefix: &str) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(&format!("\"{value}\""))
        .map_err(|_| anyhow::anyhow!("{err_prefix}: {value}"))
}

fn adr_add_alternatives(
    entry: &mut AdrEntry,
    value: &str,
    ctx: &AdrAddContext,
) -> anyhow::Result<()> {
    use crate::model::{Alternative, AlternativeStatus};
    if entry
        .spec
        .content
        .alternatives
        .iter()
        .any(|a| a.text == value)
    {
        return Ok(());
    }

    let status = if ctx.reject_reason.is_some() {
        AlternativeStatus::Rejected
    } else {
        AlternativeStatus::Considered
    };

    entry.spec.content.alternatives.push(Alternative {
        text: value.to_string(),
        status,
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
    let (artifact, fp) = plan_edit_with_field(id, field)?;
    let value = resolve_value(value, stdin)?;
    let value = value.as_str();
    match artifact {
        ArtifactType::Adr => {
            let ctx = AdrAddContext {
                pros,
                cons,
                reject_reason,
            };
            if fp.is_simple() {
                add_toml_simple_field::<AdrTomlAdapter, AdrAddContext>(
                    config,
                    id,
                    &fp,
                    value,
                    op,
                    ArtifactType::Adr,
                    &ctx,
                    |entry, simple, value, ctx| match simple {
                        "alternatives" => adr_add_alternatives(entry, value, ctx),
                        _ => unreachable!("validated by edit rules"),
                    },
                )?;
            } else {
                let mut entry = AdrTomlAdapter::load(config, id)?;
                adr_nested_add_alternatives(&mut entry, &fp, value, id)?;
                AdrTomlAdapter::write(config, &entry, op)?;
            }
        }
        ArtifactType::WorkItem => {
            if !fp.is_simple() {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    format!(
                        "Work item nested add is not supported for path '{}'",
                        format_path(&fp)
                    ),
                    id,
                )
                .into());
            }
            let ctx = WorkAddContext {
                category_override,
                scope_override,
            };
            add_toml_simple_field::<WorkTomlAdapter, WorkAddContext<'_>>(
                config,
                id,
                &fp,
                value,
                op,
                ArtifactType::WorkItem,
                &ctx,
                |entry, simple, value, ctx| match simple {
                    "acceptance_criteria" => work_add_acceptance_criteria(entry, value, ctx),
                    "journal" => work_add_journal(entry, value, ctx),
                    _ => unreachable!("validated by edit rules"),
                },
            )?;
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
    }

    if !op.is_preview() {
        let display_field = format_path(&fp);
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

fn add_toml_simple_field<A, C>(
    config: &Config,
    id: &str,
    fp: &FieldPath,
    value: &str,
    op: WriteOp,
    artifact: ArtifactType,
    ctx: &C,
    special_handler: fn(&mut A::Entry, &str, &str, &C) -> anyhow::Result<()>,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let simple = fp.as_simple().expect("simple path expected");
    let mut entry = A::load(config, id)?;
    let mut doc = serde_json::to_value(entry.spec())?;
    if edit_runtime::add_simple_list_value(artifact, &mut doc, simple, value, id)? {
        *entry.spec_mut() = serde_json::from_value(doc)?;
    } else {
        if !edit_rules::simple_field_supports_verb(
            artifact.rule_key(),
            simple,
            edit_rules::Verb::Add,
        ) {
            return Err(cannot_add_to_field_error(id, simple));
        }
        special_handler(&mut entry, simple, value, ctx)?;
    }
    A::write(config, &entry, op)?;
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
    let (artifact, fp) = plan_edit_with_field(id, field)?;

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
            adr_nested_remove_alternatives,
        )?,
        ArtifactType::WorkItem => remove_toml_field::<WorkTomlAdapter>(
            config,
            id,
            &fp,
            field,
            opts,
            op,
            ArtifactType::WorkItem,
            work_nested_remove_dispatch,
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
    let (artifact, fp) = plan_edit_with_field(id, field)?;
    if !fp.is_simple() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            TICK_NESTED_PATH_ERROR,
            id,
        )
        .into());
    }
    let field = fp.as_simple().unwrap();

    let status_str = match (artifact, status) {
        (ArtifactType::Adr, crate::TickStatus::Done) => "accepted",
        (ArtifactType::Adr, crate::TickStatus::Pending) => "considered",
        (ArtifactType::Adr, crate::TickStatus::Cancelled) => "rejected",
        (ArtifactType::WorkItem, crate::TickStatus::Done) => "done",
        (ArtifactType::WorkItem, crate::TickStatus::Pending) => "pending",
        (ArtifactType::WorkItem, crate::TickStatus::Cancelled) => "cancelled",
        (ArtifactType::Rfc | ArtifactType::Clause, _) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0813SupersedeNotSupported,
                TICK_UNSUPPORTED_ARTIFACT_ERROR.replace("{id}", id),
                id,
            )
            .into());
        }
    };
    let ticked_text = match artifact {
        ArtifactType::Adr => tick_toml_field::<AdrTomlAdapter>(
            config,
            id,
            field,
            opts,
            op,
            ArtifactType::Adr,
            status_str,
        )?,
        ArtifactType::WorkItem => tick_toml_field::<WorkTomlAdapter>(
            config,
            id,
            field,
            opts,
            op,
            ArtifactType::WorkItem,
            status_str,
        )?,
        ArtifactType::Rfc | ArtifactType::Clause => unreachable!("handled above"),
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
    nested_handler: fn(
        &mut A::Entry,
        &FieldPath,
        &MatchOptions,
        &str,
    ) -> anyhow::Result<Vec<String>>,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    if let Some(simple) = fp.as_simple() {
        let mut doc = serde_json::to_value(entry.spec())?;
        let removed = remove_simple_values_from_doc(artifact, &mut doc, simple, id, opts)?
            .ok_or_else(|| cannot_remove_from_field_error(id, simple))?;
        *entry.spec_mut() = serde_json::from_value(doc)?;
        A::write(config, &entry, op)?;
        notify_removed(id, simple, &removed, op);
    } else {
        let removed = nested_handler(&mut entry, fp, opts, id)?;
        A::write(config, &entry, op)?;
        notify_removed(id, field, &removed, op);
    }
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

fn ensure_nested_depth(
    fp: &FieldPath,
    max_depth: usize,
    scope: &str,
    id: &str,
) -> anyhow::Result<()> {
    if fp.segments.len() > max_depth {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!(
                "Path '{}' is too deep for {} (max {} segments)",
                format_path(fp),
                scope,
                max_depth
            ),
            id,
        )
        .into());
    }
    Ok(())
}

fn ensure_nested_field_for_verb(
    artifact: &str,
    root: &str,
    seg: &path::PathSegment,
    verb: edit_rules::Verb,
    fp: &FieldPath,
    id: &str,
    unknown_message: String,
    unsupported_message: String,
) -> anyhow::Result<edit_rules::FieldKind> {
    let rule =
        edit_rules::nested_field_rule(artifact, root, seg.name.as_str()).ok_or_else(|| {
            Diagnostic::new(DiagnosticCode::E0815PathFieldNotFound, unknown_message, id)
        })?;

    if !edit_rules::nested_field_supports_verb(artifact, root, seg.name.as_str(), verb) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            unsupported_message,
            id,
        )
        .into());
    }

    if matches!(rule.kind, edit_rules::FieldKind::Scalar) && seg.index.is_some() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!(
                "Field '{}' does not support index in path '{}'",
                seg.name,
                format_path(fp)
            ),
            id,
        )
        .into());
    }
    Ok(rule.kind)
}

fn format_path(fp: &FieldPath) -> String {
    fp.segments
        .iter()
        .map(|seg| match seg.index {
            Some(idx) => format!("{}[{idx}]", seg.name),
            None => seg.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(".")
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

    let clause_path = config
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{}.json", clause_name));

    if !clause_path.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Clause not found: {}", clause_id),
            clause_id,
        )
        .into());
    }

    if !confirm_delete_prompt(
        force,
        op,
        &format!("Delete clause {} from {}?", clause_name, rfc_id),
    )? {
        return Ok(vec![]);
    }

    let mut rfc = rfc_loaded.data.clone();
    let clause_rel_path = format!("clauses/{}.json", clause_name);

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

//! Conformance Case resource and trace commands.

use crate::cmd::confirmation::confirm_destructive_action;
use crate::cmd::edit::OwnedEditAction;
use crate::cmd::output::{
    print_get_output, print_json, print_toml, print_yaml, table_with_bold_headers,
};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{
    ClauseStatus, ConformanceContent, ConformanceEntry, ConformanceMeta, ConformanceSpec,
    RequirementBinding, RfcPhase, RfcStatus,
};
use crate::parse::{load_conformance_cases, write_conformance_case};
use crate::ui;
use crate::write::{WriteOp, create_dir_all};
use crate::{GetOutputFormat, ListOutputFormat, ShowOutputFormat, TraceOutputFormat};
use serde::Serialize;
use slug::slugify;
use std::io::{IsTerminal, Read};

pub struct NewCaseRequest<'a> {
    pub title: &'a str,
    pub path: &'a str,
    pub selector: &'a str,
    pub requirements: &'a [String],
    pub guards: &'a [String],
    pub id: Option<&'a str>,
}

pub fn new_case(
    config: &Config,
    request: NewCaseRequest<'_>,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    require_schema_v4(config)?;
    let mut cases = load_conformance_cases(config)?;
    let id = match request.id {
        Some(id) => {
            validate_id(id)?;
            id.to_string()
        }
        None => generated_id(request.title, &cases)?,
    };
    if cases.iter().any(|entry| entry.meta().id == id) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1303ConformanceDuplicate,
            format!("Conformance Case already exists: {id}"),
            &id,
        ));
    }
    let requirements = request
        .requirements
        .iter()
        .map(|value| parse_requirement(value))
        .collect::<DiagnosticResult<Vec<_>>>()?;
    let path = config.conformance_dir().join(format!("{id}.toml"));
    let entry = ConformanceEntry {
        spec: ConformanceSpec {
            govctl: ConformanceMeta {
                id: id.clone(),
                title: request.title.to_string(),
                tags: vec![],
            },
            case: ConformanceContent {
                path: request.path.to_string(),
                selector: request.selector.to_string(),
                requirements,
                guards: request.guards.to_vec(),
            },
        },
        path: path.clone(),
    };
    cases.push(entry.clone());
    validate_prospective(config, &cases)?;

    if !config.conformance_dir().exists() && !op.is_preview() {
        create_dir_all(
            &config.conformance_dir(),
            op,
            Some(&config.display_path(&config.conformance_dir())),
        )?;
    }
    write_conformance_case(&path, &entry.spec, op, Some(&config.display_path(&path)))?;
    if !op.is_preview() {
        ui::info(format!("Created Conformance Case: {id}"));
    }
    Ok(vec![])
}

pub fn list(
    config: &Config,
    filter: Option<&str>,
    limit: Option<usize>,
    output: Option<ListOutputFormat>,
    tags: &[String],
) -> DiagnosticResult<Diagnostics> {
    require_schema_v4(config)?;
    let mut cases = load_conformance_cases(config)?;
    cases.retain(|entry| {
        filter.is_none_or(|needle| {
            entry.meta().id.contains(needle) || entry.meta().title.contains(needle)
        }) && tags.iter().all(|tag| entry.meta().tags.contains(tag))
    });
    cases.sort_by(|left, right| left.meta().id.cmp(&right.meta().id));
    if let Some(limit) = limit {
        cases.truncate(limit);
    }
    let rows = cases
        .iter()
        .map(|entry| CaseSummary {
            id: entry.meta().id.clone(),
            title: entry.meta().title.clone(),
            path: entry.spec.case.path.clone(),
            selector: entry.spec.case.selector.clone(),
        })
        .collect::<Vec<_>>();
    match crate::cmd::list::output::resolve_list_output(output) {
        ListOutputFormat::Json => print_json(
            &rows,
            DiagnosticCode::E1301ConformanceSchemaInvalid,
            "Failed to serialize Conformance Case list as JSON",
            "conformance list",
        )?,
        ListOutputFormat::Yaml => print_yaml(
            &rows,
            DiagnosticCode::E1301ConformanceSchemaInvalid,
            "Failed to serialize Conformance Case list as YAML",
            "conformance list",
        )?,
        ListOutputFormat::Table => {
            let mut table = table_with_bold_headers(&["Case", "Title", "Path", "Selector"]);
            for row in rows {
                table.add_row([row.id, row.title, row.path, row.selector]);
            }
            println!("{table}");
        }
    }
    Ok(vec![])
}

pub fn get(
    config: &Config,
    id: &str,
    field: Option<&str>,
    output: Option<GetOutputFormat>,
) -> DiagnosticResult<Diagnostics> {
    require_schema_v4(config)?;
    let entry = load_by_id(config, id)?;
    let value = match field {
        None => serde_json::to_value(&entry.spec).map_err(serialization_error)?,
        Some("id") => serde_json::Value::String(entry.meta().id.clone()),
        Some("title") => serde_json::Value::String(entry.meta().title.clone()),
        Some("path") => serde_json::Value::String(entry.spec.case.path.clone()),
        Some("selector") => serde_json::Value::String(entry.spec.case.selector.clone()),
        Some("tags") => serde_json::to_value(&entry.meta().tags).map_err(serialization_error)?,
        Some("guards") => {
            serde_json::to_value(&entry.spec.case.guards).map_err(serialization_error)?
        }
        Some("requirements") => {
            serde_json::to_value(&entry.spec.case.requirements).map_err(serialization_error)?
        }
        Some(field) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0803UnknownField,
                format!("Unknown Conformance Case field: {field}"),
                id,
            ));
        }
    };
    if field == Some("requirements")
        && output.unwrap_or(GetOutputFormat::Plain) == GetOutputFormat::Plain
    {
        let plain = entry
            .spec
            .case
            .requirements
            .iter()
            .map(|requirement| {
                serde_json::Value::String(format!(
                    "{}@{}",
                    requirement.clause_ref, requirement.version
                ))
            })
            .collect();
        print_get_output(
            &serde_json::Value::Array(plain),
            true,
            output,
            DiagnosticCode::E1301ConformanceSchemaInvalid,
            id,
        )?;
    } else {
        print_get_output(
            &value,
            field.is_some(),
            output,
            DiagnosticCode::E1301ConformanceSchemaInvalid,
            id,
        )?;
    }
    Ok(vec![])
}

pub fn show(
    config: &Config,
    id: &str,
    output: ShowOutputFormat,
    history: bool,
) -> DiagnosticResult<Diagnostics> {
    require_schema_v4(config)?;
    let entry = load_by_id(config, id)?;
    if history && output.is_structured() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "--history cannot be combined with structured --output; use table or plain",
            id,
        ));
    }
    match output {
        ShowOutputFormat::Json => print_json(
            &entry.spec,
            DiagnosticCode::E1301ConformanceSchemaInvalid,
            "Failed to serialize Conformance Case JSON",
            id,
        )?,
        ShowOutputFormat::Yaml => print_yaml(
            &entry.spec,
            DiagnosticCode::E1301ConformanceSchemaInvalid,
            "Failed to serialize Conformance Case YAML",
            id,
        )?,
        ShowOutputFormat::Toml => print_toml(
            &entry.spec,
            DiagnosticCode::E1301ConformanceSchemaInvalid,
            "Failed to serialize Conformance Case TOML",
            id,
        )?,
        ShowOutputFormat::Table => print_case_table(&entry),
        ShowOutputFormat::Plain => print_case_plain(&entry),
    }
    Ok(vec![])
}

pub fn edit(
    config: &Config,
    id: &str,
    path: &str,
    action: &OwnedEditAction,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    require_schema_v4(config)?;
    if path.starts_with("govctl.") || path.starts_with("case.") {
        return Err(unsupported_path(id, path));
    }
    let mut cases = load_conformance_cases(config)?;
    let index = cases
        .iter()
        .position(|entry| entry.meta().id == id)
        .ok_or_else(|| not_found(id))?;
    apply_edit(id, &mut cases[index].spec, path, action)?;
    let changed = cases[index].clone();
    validate_prospective(config, &cases)?;
    write_conformance_case(
        &changed.path,
        &changed.spec,
        op,
        Some(&config.display_path(&changed.path)),
    )?;
    if !op.is_preview() {
        ui::field_set(id, path, "updated");
    }
    Ok(vec![])
}

pub fn delete(
    config: &Config,
    id: &str,
    force: bool,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    require_schema_v4(config)?;
    let mut cases = load_conformance_cases(config)?;
    let index = cases
        .iter()
        .position(|entry| entry.meta().id == id)
        .ok_or_else(|| not_found(id))?;
    let entry = cases.remove(index);
    let blockers = case_reference_blockers(config, id)?;
    if !blockers.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E1306ConformanceStillReferenced,
            format!(
                "Cannot delete Conformance Case '{id}': referenced by {}",
                blockers.join(", ")
            ),
            id,
        ));
    }
    validate_prospective(config, &cases)?;
    if !confirm_destructive_action(
        force,
        op,
        &format!("Delete Conformance Case {id}?"),
        "Deletion cancelled",
    )? {
        return Ok(vec![]);
    }
    crate::write::delete_file(&entry.path, op, Some(&config.display_path(&entry.path)))?;
    if !op.is_preview() {
        ui::info(format!("Deleted Conformance Case: {id}"));
    }
    Ok(vec![])
}

fn case_reference_blockers(config: &Config, id: &str) -> DiagnosticResult<Vec<String>> {
    let index = crate::load::load_project(config).map_err(|errors| {
        errors.into_iter().next().unwrap_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                "Failed to load project before Conformance Case deletion",
                id,
            )
        })
    })?;
    let mut blockers = Vec::new();
    for rfc in &index.rfcs {
        if rfc.rfc.refs.iter().any(|reference| reference == id) {
            blockers.push(rfc.rfc.rfc_id.clone());
        }
    }
    for adr in &index.adrs {
        if adr.meta().refs.iter().any(|reference| reference == id) {
            blockers.push(adr.meta().id.clone());
        }
    }
    for work in &index.work_items {
        if work.meta().refs.iter().any(|reference| reference == id) {
            blockers.push(work.meta().id.clone());
        }
    }
    for guard in crate::parse::load_guards(config)? {
        if guard.meta().refs.iter().any(|reference| reference == id) {
            blockers.push(guard.meta().id.clone());
        }
    }
    blockers.sort();
    blockers.dedup();
    Ok(blockers)
}

pub fn trace(
    config: &Config,
    target: Option<&str>,
    output: Option<TraceOutputFormat>,
) -> DiagnosticResult<Diagnostics> {
    require_schema_v4(config)?;
    let cases = load_conformance_cases(config)?;
    crate::validate::conformance::validate_cases(config, &cases)
        .into_iter()
        .next()
        .map_or(Ok(()), Err)?;
    let index = crate::load::load_project(config).map_err(|errors| {
        errors.into_iter().next().unwrap_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1305ConformanceGraphInvalid,
                "Failed to load project for trace query",
                "conformance trace",
            )
        })
    })?;
    let guards = crate::parse::load_guards(config)?;
    validate_trace_target(target, &index, &cases, &guards)?;

    let mut records = cases
        .iter()
        .filter(|entry| target_matches(target, entry))
        .map(|entry| trace_record(entry, &index))
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.id.cmp(&right.id));
    let result = TraceResult { cases: records };
    let output = output.unwrap_or_else(|| {
        if std::io::stdout().is_terminal() {
            TraceOutputFormat::Table
        } else {
            TraceOutputFormat::Json
        }
    });
    match output {
        TraceOutputFormat::Json => print_json(
            &result,
            DiagnosticCode::E1305ConformanceGraphInvalid,
            "Failed to serialize trace result",
            "conformance trace",
        )?,
        TraceOutputFormat::Table => {
            let mut table = table_with_bold_headers(&[
                "Case",
                "Title",
                "Tags",
                "Applicability",
                "Path",
                "Selector",
                "Requirements",
                "Guards",
            ]);
            for case in result.cases {
                table.add_row([
                    case.id,
                    case.title,
                    case.tags.join("\n"),
                    case.requirement_applicability.to_string(),
                    case.path,
                    case.selector,
                    case.requirements
                        .iter()
                        .map(|requirement| {
                            format!(
                                "{}@{} ({})",
                                requirement.clause_ref,
                                requirement.version,
                                requirement.requirement_applicability
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    case.guards.join("\n"),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(vec![])
}

fn apply_edit(
    id: &str,
    spec: &mut ConformanceSpec,
    path: &str,
    action: &OwnedEditAction,
) -> DiagnosticResult<()> {
    match action {
        OwnedEditAction::Set { value, stdin } => {
            let value = resolve_value(value.as_ref(), *stdin)?;
            match path {
                "title" => spec.govctl.title = value,
                "path" => spec.case.path = value,
                "selector" => spec.case.selector = value,
                _ => {
                    let Some(index) = requirement_version_index(path) else {
                        return Err(unsupported_path(id, path));
                    };
                    let requirement = spec.case.requirements.get_mut(index).ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0816PathIndexOutOfBounds,
                            format!("Requirement index {index} is out of bounds"),
                            id,
                        )
                    })?;
                    semver::Version::parse(&value).map_err(|_| {
                        Diagnostic::new(
                            DiagnosticCode::E0820InvalidFieldValue,
                            format!("Invalid semantic version: {value}"),
                            id,
                        )
                    })?;
                    requirement.version = value;
                }
            }
        }
        OwnedEditAction::Add { value, stdin } => {
            let value = resolve_value(value.as_ref(), *stdin)?;
            match path {
                "requirements" => spec.case.requirements.push(parse_requirement(&value)?),
                "guards" => spec.case.guards.push(value),
                "tags" => spec.govctl.tags.push(value),
                _ => return Err(unsupported_path(id, path)),
            }
        }
        OwnedEditAction::Remove { match_opts } => {
            if let Some(index) = requirement_index(path) {
                if match_opts.pattern.is_some() {
                    return Err(Diagnostic::new(
                        DiagnosticCode::E0818PathIndexConflict,
                        "Indexed requirement removal does not accept a value",
                        id,
                    ));
                }
                remove_index(&mut spec.case.requirements, index, id, "requirement")?;
            } else {
                match path {
                    "requirements" => {
                        if match_opts.regex || match_opts.all {
                            return Err(Diagnostic::new(
                                DiagnosticCode::E0802ConflictingArgs,
                                "Requirement removal supports only an exact value or indexed path",
                                id,
                            ));
                        }
                        let value = match_opts.pattern.as_deref().ok_or_else(|| {
                            Diagnostic::new(
                                DiagnosticCode::E0801MissingRequiredArg,
                                "Exact requirement removal requires a value",
                                id,
                            )
                        })?;
                        let parsed = parse_requirement(value)?;
                        remove_exact(
                            &mut spec.case.requirements,
                            |item| {
                                item.clause_ref == parsed.clause_ref
                                    && item.version == parsed.version
                            },
                            id,
                            value,
                        )?;
                    }
                    "guards" => {
                        remove_string_matches(&mut spec.case.guards, id, "guards", match_opts)?
                    }
                    "tags" => remove_string_matches(&mut spec.govctl.tags, id, "tags", match_opts)?,
                    _ => return Err(unsupported_path(id, path)),
                }
            }
        }
        OwnedEditAction::Tick { .. } => return Err(unsupported_path(id, path)),
    }
    if spec.case.requirements.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E1305ConformanceGraphInvalid,
            "A Conformance Case must retain at least one requirement",
            id,
        ));
    }
    Ok(())
}

fn remove_string_matches(
    values: &mut Vec<String>,
    id: &str,
    field: &str,
    options: &crate::cmd::edit::MatchOptionsOwned,
) -> DiagnosticResult<()> {
    let items = values.iter().map(String::as_str).collect::<Vec<_>>();
    let indices = crate::cmd::edit::matching::resolve_match_indices(
        id,
        field,
        &items,
        &options.as_match_options(),
        crate::cmd::edit::matching::MatchUse::Remove,
    )?;
    for index in indices.into_iter().rev() {
        values.remove(index);
    }
    Ok(())
}

fn validate_prospective(config: &Config, cases: &[ConformanceEntry]) -> DiagnosticResult<()> {
    let mut index = crate::load::load_project(config).map_err(|errors| {
        errors.into_iter().next().unwrap_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1305ConformanceGraphInvalid,
                "Failed to load project for Conformance Case validation",
                "conformance",
            )
        })
    })?;
    index.conformance_cases = cases.to_vec();
    crate::validate::validate_project(&index, config)
        .diagnostics
        .into_iter()
        .find(|diagnostic| diagnostic.level == crate::diagnostic::DiagnosticLevel::Error)
        .map_or(Ok(()), Err)
}

fn validate_trace_target(
    target: Option<&str>,
    index: &crate::model::ProjectIndex,
    cases: &[ConformanceEntry],
    guards: &[crate::model::GuardEntry],
) -> DiagnosticResult<()> {
    let Some(target) = target else {
        return Ok(());
    };
    let known = if target.starts_with("RFC-") && target.contains(':') {
        index
            .iter_clauses()
            .any(|(rfc, clause)| format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id) == target)
    } else if target.starts_with("RFC-") {
        index.rfcs.iter().any(|entry| entry.rfc.rfc_id == target)
    } else if target.starts_with("CONF-") {
        cases.iter().any(|entry| entry.meta().id == target)
    } else if target.starts_with("GUARD-") {
        guards.iter().any(|entry| entry.meta().id == target)
    } else {
        false
    };
    if known {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E1302ConformanceNotFound,
            format!("Unknown or unsupported conformance trace target: {target}"),
            target,
        ))
    }
}

fn target_matches(target: Option<&str>, entry: &ConformanceEntry) -> bool {
    let Some(target) = target else {
        return true;
    };
    if target == entry.meta().id {
        return true;
    }
    if target.starts_with("GUARD-") {
        return entry.spec.case.guards.iter().any(|guard| guard == target);
    }
    if target.contains(':') {
        return entry
            .spec
            .case
            .requirements
            .iter()
            .any(|binding| binding.clause_ref == target);
    }
    entry.spec.case.requirements.iter().any(|binding| {
        binding
            .clause_ref
            .split_once(':')
            .is_some_and(|(rfc, _)| rfc == target)
    })
}

fn trace_record(entry: &ConformanceEntry, index: &crate::model::ProjectIndex) -> TraceCase {
    let mut requirements = entry
        .spec
        .case
        .requirements
        .iter()
        .map(|binding| TraceRequirement {
            clause_ref: binding.clause_ref.clone(),
            version: binding.version.clone(),
            requirement_applicability: applicability(binding, index),
        })
        .collect::<Vec<_>>();
    requirements.sort_by(|left, right| {
        left.clause_ref
            .cmp(&right.clause_ref)
            .then_with(|| left.version.cmp(&right.version))
    });
    let mut tags = entry.meta().tags.clone();
    tags.sort();
    tags.dedup();
    let mut guards = entry.spec.case.guards.clone();
    guards.sort();
    guards.dedup();
    let requirement_applicability = requirements
        .iter()
        .map(|requirement| requirement.requirement_applicability)
        .min()
        .unwrap_or(Applicability::Stale);
    TraceCase {
        id: entry.meta().id.clone(),
        title: entry.meta().title.clone(),
        tags,
        path: entry.spec.case.path.clone(),
        selector: entry.spec.case.selector.clone(),
        requirements,
        guards,
        requirement_applicability,
    }
}

fn applicability(
    binding: &RequirementBinding,
    index: &crate::model::ProjectIndex,
) -> Applicability {
    let Some((rfc_id, clause_id)) = binding.clause_ref.split_once(':') else {
        return Applicability::Stale;
    };
    let Some(rfc) = index.rfcs.iter().find(|entry| entry.rfc.rfc_id == rfc_id) else {
        return Applicability::Stale;
    };
    let Some(clause) = rfc
        .clauses
        .iter()
        .find(|entry| entry.spec.clause_id == clause_id)
    else {
        return Applicability::Stale;
    };
    if clause.spec.status != ClauseStatus::Active || binding.version != rfc.rfc.version {
        return Applicability::Stale;
    }
    match (rfc.rfc.status, rfc.rfc.phase, clause.spec.since.is_some()) {
        (RfcStatus::Draft, _, _) => Applicability::Provisional,
        (RfcStatus::Normative, RfcPhase::Spec, true) => Applicability::Candidate,
        (RfcStatus::Normative, RfcPhase::Impl | RfcPhase::Test | RfcPhase::Stable, true) => {
            Applicability::Current
        }
        _ => Applicability::Stale,
    }
}

fn load_by_id(config: &Config, id: &str) -> DiagnosticResult<ConformanceEntry> {
    crate::artifact_catalog::load_conformance_by_id(config, id)
}

fn require_schema_v4(config: &Config) -> DiagnosticResult<()> {
    if config.schema.version >= 4 {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E0505MigrationRequired,
            "Conformance Case commands require schema version 4. Run `govctl migrate`.",
            "gov/config.toml",
        ))
    }
}

fn generated_id(title: &str, cases: &[ConformanceEntry]) -> DiagnosticResult<String> {
    let slug = slugify(title).to_uppercase().replace('_', "-");
    let base = format!("CONF-{slug}");
    validate_id(&base)?;
    if !cases.iter().any(|entry| entry.meta().id == base) {
        return Ok(base);
    }
    for suffix in 2.. {
        let candidate = format!("{base}-{suffix}");
        if !cases.iter().any(|entry| entry.meta().id == candidate) {
            return Ok(candidate);
        }
    }
    unreachable!("the unbounded numeric suffix space must contain an unused ID")
}

fn validate_id(id: &str) -> DiagnosticResult<()> {
    let valid = id.strip_prefix("CONF-").is_some_and(|tail| {
        !tail.is_empty()
            && tail
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_uppercase())
            && tail.chars().all(|character| {
                character.is_ascii_uppercase() || character.is_ascii_digit() || character == '-'
            })
    });
    if valid {
        Ok(())
    } else {
        Err(Diagnostic::new(
            DiagnosticCode::E1304ConformanceInvalidId,
            format!("Invalid Conformance Case ID: {id}"),
            id,
        ))
    }
}

fn parse_requirement(value: &str) -> DiagnosticResult<RequirementBinding> {
    let (clause_ref, version) = value.rsplit_once('@').ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            "Requirement must use CLAUSE-ID@VERSION",
            value,
        )
    })?;
    if !clause_ref.starts_with("RFC-") || !clause_ref.contains(":C-") {
        return Err(Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            "Requirement must use a fully qualified RFC Clause ID",
            value,
        ));
    }
    semver::Version::parse(version).map_err(|_| {
        Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            format!("Invalid requirement version: {version}"),
            value,
        )
    })?;
    Ok(RequirementBinding {
        clause_ref: clause_ref.to_string(),
        version: version.to_string(),
    })
}

fn resolve_value(value: Option<&Option<String>>, stdin: bool) -> DiagnosticResult<String> {
    match (value, stdin) {
        (Some(Some(value)), false) => Ok(value.clone()),
        (Some(None), true) => {
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|error| Diagnostic::io_error("read stdin", error, "stdin"))?;
            Ok(buffer.trim_end_matches('\n').to_string())
        }
        (Some(None), false) | (None, _) => Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "Provide a value or use --stdin",
            "conformance edit",
        )),
        (Some(Some(_)), true) => Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "Cannot combine a value with --stdin",
            "conformance edit",
        )),
    }
}

fn requirement_index(path: &str) -> Option<usize> {
    path.strip_prefix("requirements[")?
        .strip_suffix(']')?
        .parse()
        .ok()
}

fn requirement_version_index(path: &str) -> Option<usize> {
    path.strip_prefix("requirements[")?
        .strip_suffix("].version")?
        .parse()
        .ok()
}

fn remove_index<T>(
    values: &mut Vec<T>,
    index: usize,
    id: &str,
    label: &str,
) -> DiagnosticResult<()> {
    if index >= values.len() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0816PathIndexOutOfBounds,
            format!("{label} index {index} is out of bounds"),
            id,
        ));
    }
    values.remove(index);
    Ok(())
}

fn remove_exact<T>(
    values: &mut Vec<T>,
    predicate: impl Fn(&T) -> bool,
    id: &str,
    value: &str,
) -> DiagnosticResult<()> {
    let index = values.iter().position(predicate).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0806InvalidPattern,
            format!("No exact value '{value}' exists"),
            id,
        )
    })?;
    values.remove(index);
    Ok(())
}

fn print_case_table(entry: &ConformanceEntry) {
    let mut table = table_with_bold_headers(&["Field", "Value"]);
    table.add_row(["ID", entry.meta().id.as_str()]);
    table.add_row(["Title", entry.meta().title.as_str()]);
    table.add_row(["Tags", &entry.meta().tags.join("\n")]);
    table.add_row(["Path", entry.spec.case.path.as_str()]);
    table.add_row(["Selector", entry.spec.case.selector.as_str()]);
    table.add_row([
        "Requirements",
        &entry
            .spec
            .case
            .requirements
            .iter()
            .map(|binding| format!("{}@{}", binding.clause_ref, binding.version))
            .collect::<Vec<_>>()
            .join("\n"),
    ]);
    table.add_row(["Guards", &entry.spec.case.guards.join("\n")]);
    println!("{table}");
}

fn print_case_plain(entry: &ConformanceEntry) {
    println!("ID: {}", entry.meta().id);
    println!("Title: {}", entry.meta().title);
    println!("Tags: {}", entry.meta().tags.join(", "));
    println!("Path: {}", entry.spec.case.path);
    println!("Selector: {}", entry.spec.case.selector);
    println!(
        "Requirements: {}",
        entry
            .spec
            .case
            .requirements
            .iter()
            .map(|binding| format!("{}@{}", binding.clause_ref, binding.version))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("Guards: {}", entry.spec.case.guards.join(", "));
}

fn not_found(id: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E1302ConformanceNotFound,
        format!("Conformance Case not found: {id}"),
        id,
    )
}

fn unsupported_path(id: &str, path: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0803UnknownField,
        format!("Unsupported Conformance Case edit path: {path}"),
        id,
    )
}

fn serialization_error(error: serde_json::Error) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E1301ConformanceSchemaInvalid,
        format!("Failed to serialize Conformance Case output: {error}"),
        "conformance",
    )
}

#[derive(Serialize)]
struct CaseSummary {
    id: String,
    title: String,
    path: String,
    selector: String,
}

#[derive(Serialize)]
struct TraceResult {
    cases: Vec<TraceCase>,
}

#[derive(Serialize)]
struct TraceCase {
    id: String,
    title: String,
    tags: Vec<String>,
    path: String,
    selector: String,
    requirements: Vec<TraceRequirement>,
    guards: Vec<String>,
    requirement_applicability: Applicability,
}

#[derive(Serialize)]
struct TraceRequirement {
    #[serde(rename = "ref")]
    clause_ref: String,
    version: String,
    requirement_applicability: Applicability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
enum Applicability {
    Stale,
    Provisional,
    Candidate,
    Current,
}

impl std::fmt::Display for Applicability {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Stale => "stale",
            Self::Provisional => "provisional",
            Self::Candidate => "candidate",
            Self::Current => "current",
        };
        formatter.write_str(value)
    }
}

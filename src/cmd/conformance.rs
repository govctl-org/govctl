//! Conformance Case resource and trace commands.

use crate::cmd::confirmation::confirm_destructive_action;
use crate::cmd::edit::OwnedEditAction;
use crate::cmd::output::{
    print_get_output, print_json, print_toml, print_yaml, table_with_bold_headers,
};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{ConformanceContent, ConformanceEntry, ConformanceMeta, ConformanceSpec};
use crate::parse::{load_conformance_cases, write_conformance_case};
use crate::ui;
use crate::write::{WriteOp, create_dir_all};
use crate::{GetOutputFormat, ListOutputFormat, ShowOutputFormat, TraceOutputFormat};
use serde::Serialize;
use slug::slugify;
use std::io::IsTerminal;

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
        .map(|value| crate::cmd::edit::parse_conformance_requirement(value))
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
    crate::cmd::edit::edit_field(crate::cmd::edit::EditFieldRequest {
        config,
        id,
        path,
        action,
        op,
    })
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
        .map(|entry| crate::model::derive_conformance_trace(entry, &index))
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
    cases: Vec<crate::model::ConformanceTraceCase>,
}

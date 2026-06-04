use super::artifact_not_found;
use crate::OutputFormat;
use crate::cmd::output::print_json;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::{find_clause_toml, find_rfc_toml, load_clause, load_rfc, split_clause_id};
use crate::render::{expand_inline_refs, render_adr, render_clause, render_rfc, render_work_item};
use crate::terminal_md::render_terminal_md;
use serde::Serialize;

fn print_show_output<T>(
    config: &Config,
    output: OutputFormat,
    json_value: &T,
    json_error_code: DiagnosticCode,
    json_error_message: &str,
    id: &str,
    render_markdown: impl FnOnce() -> DiagnosticResult<String>,
) -> DiagnosticResult<()>
where
    T: Serialize,
{
    match output {
        OutputFormat::Json => {
            print_json(json_value, json_error_code, json_error_message, id)?;
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let raw = render_markdown()?;
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }
    Ok(())
}

/// Show RFC content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_rfc(config: &Config, id: &str, output: OutputFormat) -> DiagnosticResult<Diagnostics> {
    // [[RFC-0002:C-CRUD-VERBS]]: read-by-ID must error when no RFC exists for
    // the requested stable resource ID from [[RFC-0002:C-RESOURCES]].
    let path = find_rfc_toml(config, id).ok_or_else(|| {
        artifact_not_found(
            config,
            DiagnosticCode::E0102RfcNotFound,
            "RFC",
            id,
            config.rfc_dir(),
        )
    })?;
    let rfc = load_rfc(config, &path).map_err(Diagnostic::from)?;
    // [[RFC-0000:C-RFC-DEF]] makes the TOML `[govctl].id` authoritative; the
    // resolved path must not satisfy a different requested RFC ID.
    if rfc.rfc.rfc_id != id {
        return Err(artifact_not_found(
            config,
            DiagnosticCode::E0102RfcNotFound,
            "RFC",
            id,
            config.rfc_dir(),
        ));
    }

    print_show_output(
        config,
        output,
        &rfc.rfc,
        DiagnosticCode::E0101RfcSchemaInvalid,
        "Failed to serialize RFC JSON",
        id,
        || render_rfc(&rfc),
    )?;

    Ok(vec![])
}

/// Show ADR content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_adr(config: &Config, id: &str, output: OutputFormat) -> DiagnosticResult<Diagnostics> {
    let adr = crate::artifact_catalog::load_adr_by_id(config, id)?;

    print_show_output(
        config,
        output,
        &adr.spec,
        DiagnosticCode::E0301AdrSchemaInvalid,
        "Failed to serialize ADR JSON",
        id,
        || render_adr(&adr),
    )?;

    Ok(vec![])
}

/// Show work item content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_work(config: &Config, id: &str, output: OutputFormat) -> DiagnosticResult<Diagnostics> {
    let item = crate::artifact_catalog::load_work_item_by_id(config, id)?;

    print_show_output(
        config,
        output,
        &item.spec,
        DiagnosticCode::E0401WorkSchemaInvalid,
        "Failed to serialize work item JSON",
        id,
        || render_work_item(&item),
    )?;

    Ok(vec![])
}

/// Show clause content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_clause(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> DiagnosticResult<Diagnostics> {
    let (rfc_id, clause_name) = split_clause_id(id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Invalid clause ID format: {id} (expected RFC-NNNN:C-NAME)"),
            id,
        )
    })?;

    let path = find_clause_toml(config, id).ok_or_else(|| {
        artifact_not_found(
            config,
            DiagnosticCode::E0202ClauseNotFound,
            "Clause",
            id,
            config.clause_dir(rfc_id),
        )
    })?;
    let clause = load_clause(config, &path).map_err(Diagnostic::from)?;
    // [[RFC-0002:C-RESOURCES]] defines `RFC-NNNN:C-NAME` as self-scoping, and
    // [[RFC-0000:C-CLAUSE-DEF]] makes the clause TOML ID authoritative.
    if clause.spec.clause_id != clause_name {
        return Err(artifact_not_found(
            config,
            DiagnosticCode::E0202ClauseNotFound,
            "Clause",
            id,
            config.clause_dir(rfc_id),
        ));
    }

    print_show_output(
        config,
        output,
        &clause.spec,
        DiagnosticCode::E0201ClauseSchemaInvalid,
        "Failed to serialize clause structured output",
        id,
        || {
            let mut raw = String::new();
            render_clause(&mut raw, rfc_id, &clause);
            Ok(raw)
        },
    )?;

    Ok(vec![])
}

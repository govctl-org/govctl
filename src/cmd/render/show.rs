use super::display_path_string;
use crate::OutputFormat;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::load_rfcs;
use crate::parse::{load_adrs, load_work_items};
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
            let json = serde_json::to_string_pretty(json_value).map_err(|err| {
                Diagnostic::new(json_error_code, format!("{json_error_message}: {err}"), id)
            })?;
            println!("{json}");
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
    let rfcs = load_rfcs(config).map_err(Diagnostic::from)?;

    let rfc = rfcs
        .into_iter()
        .find(|r| r.rfc.rfc_id == id)
        .ok_or_else(|| {
            let scope = display_path_string(config, config.rfc_dir());
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {id}"),
                scope,
            )
        })?;

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
    let adrs = load_adrs(config)?;

    let adr = adrs
        .into_iter()
        .find(|a| a.spec.govctl.id == id)
        .ok_or_else(|| {
            let scope = display_path_string(config, config.adr_dir());
            Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {id}"),
                scope,
            )
        })?;

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
    let items = load_work_items(config)?;

    let item = items
        .into_iter()
        .find(|w| w.spec.govctl.id == id)
        .ok_or_else(|| {
            let scope = config
                .display_path(&config.work_dir())
                .display()
                .to_string();
            Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {id}"),
                scope,
            )
        })?;

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
    let parts: Vec<&str> = id.split(':').collect();
    if parts.len() != 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Invalid clause ID format: {id} (expected RFC-NNNN:C-NAME)"),
            id,
        ));
    }
    let rfc_id = parts[0];
    let clause_name = parts[1];

    let rfcs = load_rfcs(config).map_err(Diagnostic::from)?;

    let rfc = rfcs
        .into_iter()
        .find(|r| r.rfc.rfc_id == rfc_id)
        .ok_or_else(|| {
            let scope = display_path_string(config, config.rfc_dir());
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {rfc_id}"),
                scope,
            )
        })?;

    let clause = rfc
        .clauses
        .into_iter()
        .find(|c| c.spec.clause_id == clause_name)
        .ok_or_else(|| {
            let scope = config
                .display_path(&config.rfc_dir().join(rfc_id).join("clauses"))
                .display()
                .to_string();
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Clause not found: {id}"),
                scope,
            )
        })?;

    print_show_output(
        config,
        output,
        &clause.spec,
        DiagnosticCode::E0201ClauseSchemaInvalid,
        "Failed to serialize clause JSON",
        id,
        || {
            let mut raw = String::new();
            render_clause(&mut raw, rfc_id, &clause);
            Ok(raw)
        },
    )?;

    Ok(vec![])
}

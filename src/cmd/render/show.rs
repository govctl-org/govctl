use super::artifact_not_found;
use crate::ShowOutputFormat;
use crate::cmd::output::{print_json, print_toml, print_yaml};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::{
    find_clause_toml, find_rfc_toml, load_clause, load_rfc, load_rfcs, split_clause_id,
};
use crate::render::{
    RenderProjection, expand_inline_refs, render_adr_with_projection,
    render_clause_with_projection, render_rfc_with_projection, render_work_item_with_projection,
};
use crate::terminal_md::render_terminal_md;
use serde::Serialize;

struct ShowOutputRequest<'a, T> {
    output: ShowOutputFormat,
    history: bool,
    structured_value: &'a T,
    structured_error_code: DiagnosticCode,
    structured_error_message: &'a str,
    id: &'a str,
}

fn print_show_output<T>(
    config: &Config,
    request: ShowOutputRequest<'_, T>,
    render_human: impl FnOnce(RenderProjection) -> DiagnosticResult<String>,
) -> DiagnosticResult<()>
where
    T: Serialize,
{
    if request.history && request.output.is_structured() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "--history cannot be combined with structured --output; use table or plain",
            request.id,
        ));
    }

    match request.output {
        ShowOutputFormat::Json => {
            print_json(
                request.structured_value,
                request.structured_error_code,
                request.structured_error_message,
                request.id,
            )?;
        }
        ShowOutputFormat::Yaml => {
            print_yaml(
                request.structured_value,
                request.structured_error_code,
                request.structured_error_message,
                request.id,
            )?;
        }
        ShowOutputFormat::Toml => {
            print_toml(
                request.structured_value,
                request.structured_error_code,
                request.structured_error_message,
                request.id,
            )?;
        }
        ShowOutputFormat::Table | ShowOutputFormat::Plain => {
            let projection = if request.history {
                RenderProjection::Archive
            } else {
                RenderProjection::Current
            };
            let raw = render_human(projection)?;
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }
    Ok(())
}

/// Show RFC content to stdout (no file written).
///
/// Per [[RFC-0002:C-SHOW-PROJECTION]], human output defaults to the current
/// projection while structured output remains complete.
pub fn show_rfc(
    config: &Config,
    id: &str,
    output: ShowOutputFormat,
    history: bool,
) -> DiagnosticResult<Diagnostics> {
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
        ShowOutputRequest {
            output,
            history,
            structured_value: &rfc.rfc,
            structured_error_code: DiagnosticCode::E0101RfcSchemaInvalid,
            structured_error_message: "Failed to serialize RFC structured output",
            id,
        },
        |projection| {
            let superseded_by = if projection == RenderProjection::Current
                && rfc.rfc.status == crate::model::RfcStatus::Deprecated
            {
                load_rfcs(config)
                    .map_err(Diagnostic::from)?
                    .into_iter()
                    .find(|candidate| candidate.rfc.supersedes.as_deref() == Some(id))
                    .map(|candidate| candidate.rfc.rfc_id)
            } else {
                None
            };
            render_rfc_with_projection(&rfc, projection, superseded_by.as_deref())
        },
    )?;

    Ok(vec![])
}

/// Show ADR content to stdout (no file written).
///
/// Per [[RFC-0002:C-SHOW-PROJECTION]], human output defaults to the current
/// projection while structured output remains complete.
pub fn show_adr(
    config: &Config,
    id: &str,
    output: ShowOutputFormat,
    history: bool,
) -> DiagnosticResult<Diagnostics> {
    let adr = crate::artifact_catalog::load_adr_by_id(config, id)?;

    print_show_output(
        config,
        ShowOutputRequest {
            output,
            history,
            structured_value: &adr.spec,
            structured_error_code: DiagnosticCode::E0301AdrSchemaInvalid,
            structured_error_message: "Failed to serialize ADR structured output",
            id,
        },
        |projection| render_adr_with_projection(&adr, projection),
    )?;

    Ok(vec![])
}

/// Show work item content to stdout (no file written).
///
/// Work Item current and archival projections are content-equivalent per
/// [[RFC-0002:C-SHOW-PROJECTION]].
pub fn show_work(
    config: &Config,
    id: &str,
    output: ShowOutputFormat,
    history: bool,
) -> DiagnosticResult<Diagnostics> {
    let item = crate::artifact_catalog::load_work_item_by_id(config, id)?;

    print_show_output(
        config,
        ShowOutputRequest {
            output,
            history,
            structured_value: &item.spec,
            structured_error_code: DiagnosticCode::E0401WorkSchemaInvalid,
            structured_error_message: "Failed to serialize work item structured output",
            id,
        },
        |projection| render_work_item_with_projection(&item, projection),
    )?;

    Ok(vec![])
}

/// Show clause content to stdout (no file written).
///
/// Per [[RFC-0002:C-SHOW-PROJECTION]], human output defaults to the current
/// projection while structured output remains complete.
pub fn show_clause(
    config: &Config,
    id: &str,
    output: ShowOutputFormat,
    history: bool,
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
        ShowOutputRequest {
            output,
            history,
            structured_value: &clause.spec,
            structured_error_code: DiagnosticCode::E0201ClauseSchemaInvalid,
            structured_error_message: "Failed to serialize clause structured output",
            id,
        },
        |projection| {
            let mut raw = String::new();
            render_clause_with_projection(&mut raw, rfc_id, &clause, projection);
            Ok(raw)
        },
    )?;

    Ok(vec![])
}

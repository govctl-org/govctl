use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::split_clause_id;
use crate::model::{ClauseKind, ClauseSpec, ClauseStatus, ClauseWire, RfcWire, SectionSpec};
use crate::schema::{ArtifactSchema, with_schema_header};
use crate::ui;
use crate::write::{WriteOp, write_file};

pub(super) fn create(
    config: &Config,
    clause_id: &str,
    title: &str,
    section: &str,
    kind: ClauseKind,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let (rfc_id, clause_name) = split_clause_id(clause_id).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0210ClauseInvalidIdFormat,
            "Invalid clause ID format. Expected RFC-NNNN:C-NAME",
            clause_id,
        )
    })?;

    let rfc_path = config.rfc_dir().join(rfc_id).join("rfc.toml");
    if !rfc_path.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0102RfcNotFound,
            format!("RFC not found: {rfc_id}"),
            rfc_id,
        ));
    }

    let mut rfc = crate::write::read_rfc(config, &rfc_path)?;

    let clause = ClauseSpec {
        clause_id: clause_name.to_string(),
        title: title.to_string(),
        kind,
        status: ClauseStatus::Active,
        text: "TODO: Add clause text here.".to_string(),
        anchors: vec![],
        superseded_by: None,
        since: None, // Will be set by rfc bump
        tags: vec![],
    };

    let clause_path = config
        .rfc_dir()
        .join(rfc_id)
        .join("clauses")
        .join(format!("{clause_name}.toml"));

    let display_clause_path = config.display_path(&clause_path);
    let wire: ClauseWire = clause.into();
    let body = toml::to_string_pretty(&wire).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0201ClauseSchemaInvalid,
            format!("Failed to serialize clause TOML: {err}"),
            display_clause_path.display().to_string(),
        )
    })?;
    let content = with_schema_header(ArtifactSchema::Clause, &body);
    write_file(&clause_path, &content, op, Some(&display_clause_path))?;

    let clause_rel_path = format!("clauses/{clause_name}.toml");
    if let Some(sec) = rfc.sections.iter_mut().find(|s| s.title == section) {
        if !sec.clauses.contains(&clause_rel_path) {
            sec.clauses.push(clause_rel_path.clone());
        }
    } else {
        rfc.sections.push(SectionSpec {
            title: section.to_string(),
            clauses: vec![clause_rel_path.clone()],
        });
    }

    let display_rfc_path = config.display_path(&rfc_path);
    let wire: RfcWire = rfc.into();
    let body = toml::to_string_pretty(&wire).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0101RfcSchemaInvalid,
            format!("Failed to serialize RFC TOML: {err}"),
            display_rfc_path.display().to_string(),
        )
    })?;
    let rfc_content = with_schema_header(ArtifactSchema::Rfc, &body);
    write_file(&rfc_path, &rfc_content, op, Some(&display_rfc_path))?;

    if !op.is_preview() {
        ui::created("clause", &config.display_path(&clause_path));
        ui::sub_info(format!(
            "Added to section '{}', path: {}",
            section, clause_rel_path
        ));
    }

    Ok(vec![])
}

use super::write_new_artifact_toml;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::load::split_clause_id;
use crate::model::{
    ClauseKind, ClauseSpec, ClauseStatus, ClauseWire, RfcPhase, RfcStatus, RfcWire, SectionSpec,
};
use crate::schema::ArtifactSchema;
use crate::ui;
use crate::write::{WriteOp, with_file_transaction};

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

    let rfc_path = config.rfc_source_path(rfc_id, "toml");
    if !rfc_path.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0102RfcNotFound,
            format!("RFC not found: {rfc_id}"),
            rfc_id,
        ));
    }

    let mut rfc = crate::write::read_rfc(config, &rfc_path)?;

    if rfc.status == RfcStatus::Deprecated {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!("Cannot create a clause in deprecated RFC: {rfc_id}"),
            clause_id,
        ));
    }

    let since = match (rfc.status, rfc.phase) {
        (RfcStatus::Normative, RfcPhase::Spec) => Some(rfc.version.clone()),
        _ => None,
    };

    let clause = ClauseSpec {
        clause_id: clause_name.to_string(),
        title: title.to_string(),
        kind,
        status: ClauseStatus::Active,
        text: "TODO: Add clause text here.".to_string(),
        anchors: vec![],
        superseded_by: None,
        since,
        tags: vec![],
    };

    let clause_path = config.clause_source_path(rfc_id, clause_name, "toml");

    let clause_wire: ClauseWire = clause.into();

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

    let rfc_wire: RfcWire = rfc.into();
    with_file_transaction(&[clause_path.as_path(), rfc_path.as_path()], op, || {
        write_new_artifact_toml(
            config,
            &clause_path,
            &clause_wire,
            ArtifactSchema::Clause,
            DiagnosticCode::E0201ClauseSchemaInvalid,
            "clause",
            op,
        )?;
        write_new_artifact_toml(
            config,
            &rfc_path,
            &rfc_wire,
            ArtifactSchema::Rfc,
            DiagnosticCode::E0101RfcSchemaInvalid,
            "RFC",
            op,
        )
    })?;

    if !op.is_preview() {
        ui::created("clause", &config.display_path(&clause_path));
        ui::sub_info(format!(
            "Added to section '{}', path: {}",
            section, clause_rel_path
        ));
    }

    Ok(vec![])
}

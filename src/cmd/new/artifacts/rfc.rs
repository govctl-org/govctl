use super::write_new_artifact_toml;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{ChangelogEntry, RfcPhase, RfcSpec, RfcStatus, RfcWire, SectionSpec};
use crate::schema::ArtifactSchema;
use crate::ui;
use crate::write::{WriteOp, create_dir_all, today};

pub(super) fn create(
    config: &Config,
    title: &str,
    manual_id: Option<&str>,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let rfcs_dir = config.rfc_dir();

    let rfc_id = match manual_id {
        Some(id) => {
            if !crate::load::valid_rfc_id(id) {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0110RfcInvalidId,
                    format!("Invalid RFC ID: {id} (expected RFC-NNNN)"),
                    id,
                ));
            }
            if rfcs_dir.join(id).exists() {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0109RfcAlreadyExists,
                    format!("RFC already exists: {id}"),
                    id,
                ));
            }
            id.to_string()
        }
        None => {
            let max_num = std::fs::read_dir(&rfcs_dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    crate::load::valid_rfc_id(&name_str)
                        .then(|| name_str.strip_prefix("RFC-")?.parse::<u32>().ok())
                        .flatten()
                })
                .max()
                .unwrap_or(0);

            if max_num == 9999 {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0110RfcInvalidId,
                    "RFC ID namespace exhausted at RFC-9999",
                    rfcs_dir.display().to_string(),
                ));
            }
            format!("RFC-{:04}", max_num + 1)
        }
    };

    let rfc_dir = config.rfc_artifact_dir(&rfc_id);
    let clauses_dir = config.clause_dir(&rfc_id);

    if rfc_dir.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0109RfcAlreadyExists,
            format!("RFC already exists: {}", rfc_dir.display()),
            rfc_dir.display().to_string(),
        ));
    }

    let display_clauses_dir = config.display_path(&clauses_dir);
    create_dir_all(&clauses_dir, op, Some(&display_clauses_dir))?;

    let rfc = RfcSpec {
        rfc_id: rfc_id.to_string(),
        title: title.to_string(),
        version: "0.1.0".to_string(),
        status: RfcStatus::Draft,
        phase: RfcPhase::Spec,
        owners: vec![config.project.default_owner.clone()],
        created: today(),
        updated: None,
        supersedes: None,
        refs: vec![],
        tags: vec![],
        sections: vec![
            SectionSpec {
                title: "Summary".to_string(),
                clauses: vec![],
            },
            SectionSpec {
                title: "Specification".to_string(),
                clauses: vec![],
            },
        ],
        changelog: vec![ChangelogEntry {
            version: "0.1.0".to_string(),
            date: today(),
            notes: Some("Initial draft".to_string()),
            added: vec![],
            changed: vec![],
            deprecated: vec![],
            removed: vec![],
            fixed: vec![],
            security: vec![],
        }],
        signature: None, // Sealed when the RFC first advances from spec to impl.
    };

    let rfc_toml = config.rfc_source_path(&rfc_id, "toml");
    let wire: RfcWire = rfc.into();
    write_new_artifact_toml(
        config,
        &rfc_toml,
        &wire,
        ArtifactSchema::Rfc,
        DiagnosticCode::E0101RfcSchemaInvalid,
        "RFC",
        op,
    )?;

    if !op.is_preview() {
        ui::created("RFC", &config.display_path(&rfc_toml));
        ui::sub_info(format!(
            "Clauses dir: {}",
            config.display_path(&clauses_dir).display()
        ));
    }

    Ok(vec![])
}

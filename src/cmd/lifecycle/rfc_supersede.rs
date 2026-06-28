use super::paths::{require_replacement_rfc_toml_path, require_rfc_toml_path};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::RfcStatus;
use crate::ui;
use crate::validate::is_valid_status_transition;
use crate::write::{WriteOp, read_rfc, today, with_file_transaction, write_rfc};

pub(super) fn supersede_rfc(
    config: &Config,
    rfc_id: &str,
    by: &str,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    if rfc_id == by {
        return Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "RFC cannot supersede itself",
            rfc_id,
        ));
    }

    let rfc_path = require_rfc_toml_path(config, rfc_id)?;
    let replacement_path = require_replacement_rfc_toml_path(config, by)?;
    let mut source = read_rfc(config, &rfc_path)?;
    let mut replacement = read_rfc(config, &replacement_path)?;

    validate_supersede_transition(&source, &replacement, rfc_id, by)?;

    let today = today();
    source.status = RfcStatus::Deprecated;
    source.updated = Some(today.clone());
    replacement.supersedes = Some(rfc_id.to_string());
    replacement.updated = Some(today);

    with_file_transaction(
        &[rfc_path.as_path(), replacement_path.as_path()],
        op,
        || {
            write_rfc(
                &rfc_path,
                &source,
                op,
                Some(&config.display_path(&rfc_path)),
            )?;
            write_rfc(
                &replacement_path,
                &replacement,
                op,
                Some(&config.display_path(&replacement_path)),
            )
        },
    )?;

    if !op.is_preview() {
        ui::superseded("RFC", rfc_id, by);
    }
    Ok(vec![])
}

fn validate_supersede_transition(
    source: &crate::model::RfcSpec,
    replacement: &crate::model::RfcSpec,
    rfc_id: &str,
    by: &str,
) -> DiagnosticResult<()> {
    if !is_valid_status_transition(source.status, RfcStatus::Deprecated) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Invalid RFC transition: {} -> deprecated",
                source.status.as_ref()
            ),
            rfc_id,
        ));
    }
    if replacement.status == RfcStatus::Deprecated {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!("Replacement RFC is deprecated: {by}"),
            by,
        ));
    }
    if replacement
        .supersedes
        .as_deref()
        .is_some_and(|old| old != rfc_id)
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0104RfcInvalidTransition,
            format!(
                "Replacement RFC already supersedes {}",
                replacement.supersedes.as_deref().unwrap_or_default()
            ),
            by,
        ));
    }

    Ok(())
}

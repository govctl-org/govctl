use super::ops::FileOp;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::RfcWire;
use crate::schema::{ArtifactSchema, with_schema_header};

/// Rebaseline legacy projection hashes as content-only amendment signatures.
/// Implements [[RFC-0000:C-PHASE-LIFECYCLE]] through the versioned migration
/// pipeline required by [[RFC-0002:C-GLOBAL-COMMANDS]].
pub(super) fn plan_rfc_signature_upgrade(config: &Config) -> DiagnosticResult<Vec<FileOp>> {
    let rfcs = crate::load::load_rfcs(config).map_err(Diagnostic::from)?;
    let mut ops = Vec::new();

    for mut rfc in rfcs {
        let signature = crate::signature::compute_rfc_content_signature(&rfc)?;
        if rfc.rfc.signature.as_deref() == Some(signature.as_str()) {
            continue;
        }

        rfc.rfc.signature = Some(signature);
        let path = rfc.path;
        let wire: RfcWire = rfc.rfc.into();
        let body = toml::to_string_pretty(&wire).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0101RfcSchemaInvalid,
                format!("Failed to serialize RFC TOML during signature migration: {err}"),
                config.display_path(&path).display().to_string(),
            )
        })?;
        ops.push(FileOp::Write {
            path,
            content: with_schema_header(ArtifactSchema::Rfc, &body),
        });
    }

    Ok(ops)
}

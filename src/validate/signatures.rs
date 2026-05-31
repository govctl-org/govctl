use super::ValidationResult;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;
use crate::signature::{compute_rfc_signature, extract_signature};

/// Validate RFC rendered markdown signatures (per ADR-0003)
pub(super) fn validate_rfc_signatures(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let output_dir = config.rfc_output();

    for rfc in &index.rfcs {
        let md_path = output_dir.join(format!("{}.md", rfc.rfc.rfc_id));

        // Skip if rendered file doesn't exist yet
        if !md_path.exists() {
            continue;
        }

        // Read rendered markdown
        let md_content = match std::fs::read_to_string(&md_path) {
            Ok(content) => content,
            Err(e) => {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::W0106RenderedReadError,
                    format!(
                        "Could not read rendered markdown: {} (hint: run `govctl rfc render`)",
                        e
                    ),
                    config.display_path(&md_path).display().to_string(),
                ));
                continue;
            }
        };

        let md_path_display = config.display_path(&md_path).display().to_string();

        // Extract signature from rendered markdown
        let Some(existing_sig) = extract_signature(&md_content) else {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0602SignatureMissing,
                format!(
                    "Rendered markdown missing signature. Run 'govctl render' to regenerate: {}",
                    rfc.rfc.rfc_id
                ),
                md_path_display.clone(),
            ));
            continue;
        };

        // Compute expected signature from source
        let expected_sig = match compute_rfc_signature(rfc) {
            Ok(sig) => sig,
            Err(e) => {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E0601SignatureMismatch,
                    format!("Failed to compute signature for {}: {}", rfc.rfc.rfc_id, e),
                    md_path_display.clone(),
                ));
                continue;
            }
        };

        // Compare
        if existing_sig != expected_sig {
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::E0601SignatureMismatch,
                format!(
                    "Signature mismatch: rendered markdown was edited directly or source changed. Run 'govctl render' to regenerate: {}",
                    rfc.rfc.rfc_id
                ),
                md_path_display,
            ));
        }
    }
}

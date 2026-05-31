use super::ValidationResult;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;

/// Validate that all artifact tags are in the allowed set and well-formed.
///
/// Per [[RFC-0002:C-RESOURCES]] controlled-vocabulary tags: every tag used by an
/// artifact must be listed in config.toml [tags] allowed, and each tag must match
/// the format `^[a-z][a-z0-9-]*$`.
pub(super) fn validate_artifact_tags(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    let allowed = &config.tags.allowed;

    let mut check_tags = |tags: &[String], artifact_id: &str, path_display: &str| {
        for tag in tags {
            // Validate format
            let tag_re = match crate::cmd::tag::tag_re() {
                Ok(r) => r,
                Err(e) => {
                    result.diagnostics.push(Diagnostic::new(
                        DiagnosticCode::E0806InvalidPattern,
                        format!("Failed to compile tag regex: {e}"),
                        path_display,
                    ));
                    continue;
                }
            };
            if !tag_re.is_match(tag) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E1101TagInvalidFormat,
                    format!(
                        "Artifact '{artifact_id}' has invalid tag format '{tag}': must match ^[a-z][a-z0-9-]*$"
                    ),
                    path_display,
                ));
                continue;
            }
            // Validate against allowed set (deny-all when empty)
            if !allowed.contains(tag) {
                result.diagnostics.push(Diagnostic::new(
                    DiagnosticCode::E1105TagUnknown,
                    format!(
                        "Artifact '{artifact_id}' uses unknown tag '{tag}' (not in config.toml [tags] allowed)"
                    ),
                    path_display,
                ));
            }
        }
    };

    for rfc in &index.rfcs {
        let path = config.display_path(&rfc.path).display().to_string();
        check_tags(&rfc.rfc.tags, &rfc.rfc.rfc_id, &path);
    }

    for (rfc, clause) in index.iter_clauses() {
        let clause_id = format!("{}:{}", rfc.rfc.rfc_id, clause.spec.clause_id);
        let path = config.display_path(&clause.path).display().to_string();
        check_tags(&clause.spec.tags, &clause_id, &path);
    }

    for adr in &index.adrs {
        let path = config.display_path(&adr.path).display().to_string();
        check_tags(&adr.spec.govctl.tags, &adr.meta().id, &path);
    }

    for work in &index.work_items {
        let path = config.display_path(&work.path).display().to_string();
        check_tags(&work.spec.govctl.tags, &work.meta().id, &path);
    }

    if let Ok(guard_result) = crate::parse::load_guards_with_warnings(config) {
        for guard in &guard_result.items {
            let path = config.display_path(&guard.path).display().to_string();
            check_tags(&guard.spec.govctl.tags, &guard.meta().id, &path);
        }
    }
}

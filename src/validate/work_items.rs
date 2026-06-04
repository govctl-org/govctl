use super::ValidationResult;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ProjectIndex;

/// Check if a work item description is a placeholder or empty
fn is_placeholder_description(desc: &str) -> bool {
    let trimmed = desc.trim();

    // Empty or whitespace-only
    if trimmed.is_empty() {
        return true;
    }

    // Exact template match
    if trimmed.contains("Describe the work to be done")
        && trimmed.contains("What is the goal?")
        && trimmed.contains("What are the acceptance criteria?")
    {
        return true;
    }

    // Common placeholder patterns (case-insensitive)
    let lower = trimmed.to_lowercase();
    let placeholder_patterns = ["todo", "tbd", "fill in later", "placeholder", "fixme"];

    // Only flag if the entire description is just a placeholder word
    placeholder_patterns
        .iter()
        .any(|p| lower == *p || lower == format!("[{}]", p) || lower == format!("<{}>", p))
}

/// Validate work item descriptions for placeholder content (per ADR-0010)
pub(super) fn validate_work_item_descriptions(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    for work in &index.work_items {
        let desc = &work.spec.content.description;
        if is_placeholder_description(desc) {
            let path_display = config.display_path(&work.path).display().to_string();
            result.diagnostics.push(Diagnostic::new(
                DiagnosticCode::W0108WorkPlaceholderDescription,
                format!(
                    "Work item has placeholder description (hint: `govctl work set {} description \"...\"`)",
                    work.meta().id
                ),
                path_display,
            ));
        }
    }
}

/// Report legacy inline execution history for migration awareness per [[ADR-0047]].
pub(super) fn validate_work_item_legacy_inline_history(
    index: &ProjectIndex,
    config: &Config,
    result: &mut ValidationResult,
) {
    for work in &index.work_items {
        if work.spec.content.journal.is_empty() {
            continue;
        }

        let path_display = config.display_path(&work.path).display().to_string();
        result.diagnostics.push(Diagnostic::new(
            DiagnosticCode::I0401WorkLegacyInlineHistory,
            "Work item contains legacy inline execution history; move durable takeaways to notes and keep new execution trace in loop state.",
            path_display,
        ));
    }
}

//! Check/lint command implementation.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project;
use crate::validate::validate_project;

/// Validate all governed documents
pub fn check_all(config: &Config) -> anyhow::Result<Vec<Diagnostic>> {
    // Load project
    let index = match load_project(config) {
        Ok(idx) => idx,
        Err(diags) => return Ok(diags),
    };

    // Validate
    let result = validate_project(&index, config);

    // Print summary
    eprintln!("Checked:");
    eprintln!("  {} RFCs", result.rfc_count);
    eprintln!("  {} clauses", result.clause_count);
    eprintln!("  {} ADRs", result.adr_count);
    eprintln!("  {} work items", result.work_count);
    eprintln!();

    if result.diagnostics.is_empty() {
        eprintln!("✓ All checks passed");
    }

    Ok(result.diagnostics)
}

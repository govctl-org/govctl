//! Check/lint command implementation.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_project_with_warnings;
use crate::scan::scan_source_refs;
use crate::ui;
use crate::validate::validate_project;

/// Validate all governed documents
pub fn check_all(config: &Config) -> anyhow::Result<Vec<Diagnostic>> {
    // Load project (with warnings for parse errors)
    let load_result = match load_project_with_warnings(config) {
        Ok(result) => result,
        Err(diags) => return Ok(diags),
    };

    let index = load_result.index;
    let mut all_diagnostics = load_result.warnings;

    // Validate governance artifacts
    let result = validate_project(&index, config);
    all_diagnostics.extend(result.diagnostics);

    // Scan source code for references (if enabled)
    let scan_result = scan_source_refs(config, &index);
    all_diagnostics.extend(scan_result.diagnostics);

    // Print summary (colorized)
    ui::check_header();
    ui::check_count(result.rfc_count, "RFCs");
    ui::check_count(result.clause_count, "clauses");
    ui::check_count(result.adr_count, "ADRs");
    ui::check_count(result.work_count, "work items");

    // Show source scan summary if enabled
    if config.source_scan.enabled {
        ui::check_count(scan_result.files_scanned, "source files scanned");
        ui::check_count(scan_result.refs_found, "references found");
    }

    eprintln!();

    if all_diagnostics.is_empty() {
        ui::success("All checks passed");
    }

    Ok(all_diagnostics)
}

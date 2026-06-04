//! Check/lint command implementation.

use crate::config::Config;
use crate::diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticLevel, DiagnosticResult, Diagnostics,
};
use crate::load::load_project_with_warnings;
use crate::model::WorkItemStatus;
use crate::parse::{load_guards_with_warnings, load_releases, load_work_items};
use crate::scan::scan_source_refs;
use crate::schema::installed_schema_diagnostics;
use crate::ui;
use crate::validate::{validate_project, validate_releases};
use crate::verification;

/// Validate all governed documents
pub fn check_all(config: &Config) -> DiagnosticResult<Diagnostics> {
    let mut all_diagnostics = Vec::new();

    // Pre-load support checks implement [[RFC-0002:C-GLOBAL-COMMANDS]].
    // They run before artifact loading because stale local schemas can reject
    // newer artifact fields before users see the migrate hint.
    let current_schema = config.schema.version;
    let latest_schema = crate::cmd::migrate::CURRENT_SCHEMA_VERSION;
    if current_schema < latest_schema {
        all_diagnostics.push(Diagnostic::new(
            DiagnosticCode::W0110SchemaOutdated,
            format!(
                "Schema version {} is outdated (latest: {}). Run `govctl migrate` to upgrade.",
                current_schema, latest_schema
            ),
            "gov/config.toml",
        ));
    }
    all_diagnostics.extend(installed_schema_diagnostics(config));
    all_diagnostics.extend(crate::cmd::project_support::local_state_gitignore_diagnostics());

    // Load project (with warnings for parse errors)
    let load_result = match load_project_with_warnings(config) {
        Ok(result) => result,
        Err(diags) => {
            all_diagnostics.extend(diags);
            return Ok(all_diagnostics);
        }
    };

    let index = load_result.index;
    all_diagnostics.extend(load_result.warnings);

    // Validate governance artifacts
    let result = validate_project(&index, config);
    all_diagnostics.extend(result.diagnostics);

    let mut guard_count = 0usize;
    match load_guards_with_warnings(config) {
        Ok(result) => {
            guard_count = result.items.len();
            all_diagnostics.extend(result.warnings);
            let (guards_by_id, guard_diags) = verification::build_guard_index(result.items);
            all_diagnostics.extend(guard_diags);
            all_diagnostics.extend(verification::validate_guard_configuration(
                config,
                &guards_by_id,
                &index.work_items,
            ));
        }
        Err(diag) => all_diagnostics.push(diag),
    }

    // Validate releases separately until they are part of the full project index.
    match load_releases(config) {
        Ok(releases) => {
            all_diagnostics.extend(validate_releases(&releases, &index, config));
        }
        Err(diag) => all_diagnostics.push(diag),
    }

    // Scan source code for references (if enabled)
    let scan_result = scan_source_refs(config, &index);
    all_diagnostics.extend(scan_result.diagnostics);

    // Print summary (colorized)
    ui::check_header();
    ui::check_count(result.rfc_count, "RFCs");
    ui::check_count(result.clause_count, "clauses");
    ui::check_count(result.adr_count, "ADRs");
    ui::check_count(result.work_count, "work items");
    ui::check_count(guard_count, "verification guards");

    // Show source scan summary if enabled
    if config.source_scan.enabled {
        ui::check_count(scan_result.files_scanned, "source files scanned");
        ui::check_count(scan_result.refs_found, "references found");
    }

    eprintln!();

    let has_blocking_diagnostics = all_diagnostics.iter().any(|diag| {
        matches!(
            diag.level,
            DiagnosticLevel::Error | DiagnosticLevel::Warning
        )
    });
    if !has_blocking_diagnostics {
        ui::success("All checks passed");
    }

    Ok(all_diagnostics)
}

/// Fast-path: assert that at least one active work item exists.
pub fn check_has_active(config: &Config) -> DiagnosticResult<Diagnostics> {
    let items = load_work_items(config)?;
    let has_active = items
        .iter()
        .any(|w| w.meta().status == WorkItemStatus::Active);

    if has_active {
        Ok(vec![])
    } else {
        Ok(vec![Diagnostic::new(
            DiagnosticCode::W0109WorkNoActive,
            "No active work item (hint: `govctl work new --active \"<title>\"`)",
            "",
        )])
    }
}

//! Guard resource commands per [[RFC-0002:C-RESOURCES]].

use super::guard_refs::{guard_reference_blockers, load_guard_by_id};
use crate::OutputFormat;
use crate::cmd::output::{print_json, print_toml};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{GuardCheck, GuardMeta, GuardSpec};
use crate::parse::{load_guards, write_guard};
use crate::ui;
use crate::write::{WriteOp, create_dir_all};
use slug::slugify;

/// Create a new verification guard.
pub fn new_guard(config: &Config, title: &str, op: WriteOp) -> DiagnosticResult<Diagnostics> {
    let guard_dir = config.guard_dir();
    if !guard_dir.exists() && !op.is_preview() {
        create_dir_all(&guard_dir, op, Some(&config.display_path(&guard_dir)))?;
    }

    // Generate ID from title: slugify, uppercase, prefix with GUARD-
    let slug = slugify(title).to_uppercase().replace('_', "-");
    if slug.is_empty() || !slug.starts_with(|c: char| c.is_ascii_uppercase()) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1006GuardInvalidTitle,
            format!(
                "Invalid guard title: must produce a slug starting with a letter (got \"{title}\")"
            ),
            title,
        ));
    }
    let id = format!("GUARD-{slug}");

    // Check for duplicate
    if !op.is_preview() {
        let existing = load_guards(config)?;
        if existing.iter().any(|g| g.spec.govctl.id == id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1003GuardDuplicate,
                format!("Guard already exists: {id}"),
                &id,
            ));
        }
    }

    let filename = slug.to_lowercase().replace('_', "-");
    let path = guard_dir.join(format!("{filename}.toml"));

    let spec = GuardSpec {
        govctl: GuardMeta::new(id.clone(), title),
        check: GuardCheck {
            command: "echo 'GUARD NOT CONFIGURED: replace this command' && exit 1".to_string(),
            timeout_secs: 300,
            pattern: None,
        },
    };

    write_guard(&path, &spec, op, Some(&config.display_path(&path)))?;

    if !op.is_preview() {
        ui::info(format!(
            "Created guard: {}",
            config.display_path(&path).display()
        ));
        ui::hint(format!(
            "To add to project defaults: edit gov/config.toml and add \"{id}\" to verification.default_guards"
        ));
    }

    Ok(vec![])
}

/// Delete a verification guard with safety checks.
pub fn delete_guard(
    config: &Config,
    id: &str,
    _force: bool,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let guard = load_guard_by_id(config, id)?;
    // Safety checks always run — --force only skips confirmation, not reference checks
    let blockers = guard_reference_blockers(config, id)?;

    if !blockers.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E1007GuardStillReferenced,
            format!(
                "Cannot delete guard '{}': still referenced:\n{}",
                id,
                blockers
                    .iter()
                    .map(|b| format!("  - {b}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            id,
        ));
    }

    let path = guard.path.clone();
    crate::write::delete_file(&path, op, Some(&config.display_path(&path)))?;

    if !op.is_preview() {
        ui::info(format!("Deleted guard: {id}"));
    }

    Ok(vec![])
}

/// Show guard content to stdout.
pub fn show_guard(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> DiagnosticResult<Diagnostics> {
    let guard = load_guard_by_id(config, id)?;

    match output {
        OutputFormat::Json => {
            print_json(
                &guard.spec,
                DiagnosticCode::E1001GuardSchemaInvalid,
                "Failed to serialize guard JSON",
                id,
            )?;
        }
        OutputFormat::Table | OutputFormat::Plain => {
            print_toml(
                &guard.spec,
                DiagnosticCode::E1001GuardSchemaInvalid,
                "Failed to serialize guard TOML",
                id,
            )?;
        }
    }

    Ok(vec![])
}

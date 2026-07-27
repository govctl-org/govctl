//! Guard resource commands per [[RFC-0002:C-RESOURCES]].

use super::guard_refs::{guard_reference_blockers, load_guard_by_id};
use crate::ShowOutputFormat;
use crate::cmd::confirmation::confirm_destructive_action;
use crate::cmd::output::{print_json, print_toml, print_yaml, table_with_bold_headers};
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

    let existing = load_guards(config)?;
    if existing.iter().any(|g| g.spec.govctl.id == id) {
        return Err(Diagnostic::new(
            DiagnosticCode::E1003GuardDuplicate,
            format!("Guard already exists: {id}"),
            &id,
        ));
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
            "To require for an affected work item: govctl work edit <WI-ID> verification.required_guards --add {id}"
        ));
        ui::hint(format!(
            "Only if every work item needs this check, add \"{id}\" to verification.default_guards in gov/config.toml"
        ));
    }

    Ok(vec![])
}

/// Delete a verification guard with safety checks.
pub fn delete_guard(
    config: &Config,
    id: &str,
    force: bool,
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
    if !confirm_destructive_action(
        force,
        op,
        &format!("Delete guard {id}?"),
        "Deletion cancelled",
    )? {
        return Ok(vec![]);
    }
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
    output: ShowOutputFormat,
    history: bool,
) -> DiagnosticResult<Diagnostics> {
    let guard = load_guard_by_id(config, id)?;

    if history && output.is_structured() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "--history cannot be combined with structured --output; use table or plain",
            id,
        ));
    }

    match output {
        ShowOutputFormat::Json => {
            print_json(
                &guard.spec,
                DiagnosticCode::E1001GuardSchemaInvalid,
                "Failed to serialize guard JSON",
                id,
            )?;
        }
        ShowOutputFormat::Yaml => {
            print_yaml(
                &guard.spec,
                DiagnosticCode::E1001GuardSchemaInvalid,
                "Failed to serialize guard YAML",
                id,
            )?;
        }
        ShowOutputFormat::Toml => {
            print_toml(
                &guard.spec,
                DiagnosticCode::E1001GuardSchemaInvalid,
                "Failed to serialize guard TOML",
                id,
            )?;
        }
        ShowOutputFormat::Table => {
            let mut table = table_with_bold_headers(&["Field", "Value"]);
            table.add_row(["ID", guard.meta().id.as_str()]);
            table.add_row(["Title", guard.meta().title.as_str()]);
            table.add_row(["Refs", &guard.meta().refs.join("\n")]);
            table.add_row(["Tags", &guard.meta().tags.join("\n")]);
            table.add_row(["Command", guard.spec.check.command.as_str()]);
            table.add_row(["Timeout", &guard.spec.check.timeout_secs.to_string()]);
            table.add_row([
                "Pattern",
                guard.spec.check.pattern.as_deref().unwrap_or_default(),
            ]);
            println!("{table}");
        }
        ShowOutputFormat::Plain => {
            println!("ID: {}", guard.meta().id);
            println!("Title: {}", guard.meta().title);
            println!("Refs: {}", guard.meta().refs.join(", "));
            println!("Tags: {}", guard.meta().tags.join(", "));
            println!("Command: {}", guard.spec.check.command);
            println!("Timeout: {}", guard.spec.check.timeout_secs);
            println!(
                "Pattern: {}",
                guard.spec.check.pattern.as_deref().unwrap_or_default()
            );
        }
    }

    Ok(vec![])
}

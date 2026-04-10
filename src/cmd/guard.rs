//! Guard resource commands per [[RFC-0002:C-RESOURCES]].

use crate::OutputFormat;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{GuardCheck, GuardMeta, GuardSpec};
use crate::parse::{load_guards, write_guard};
use crate::ui;
use crate::write::WriteOp;
use slug::slugify;

/// Create a new verification guard.
pub fn new_guard(config: &Config, title: &str, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    let guard_dir = config.guard_dir();
    if !guard_dir.exists() && !op.is_preview() {
        std::fs::create_dir_all(&guard_dir)?;
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
        )
        .into());
    }
    let id = format!("GUARD-{slug}");

    // Check for duplicate
    if !op.is_preview() {
        let existing = load_guards(config).map_err(anyhow::Error::from)?;
        if existing.iter().any(|g| g.spec.govctl.id == id) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1003GuardDuplicate,
                format!("Guard already exists: {id}"),
                &id,
            )
            .into());
        }
    }

    let filename = slug.to_lowercase().replace('_', "-");
    let path = guard_dir.join(format!("{filename}.toml"));

    let spec = GuardSpec {
        govctl: GuardMeta {
            schema: 0,
            id: id.clone(),
            title: title.to_string(),
            refs: vec![],
            tags: vec![],
        },
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
) -> anyhow::Result<Vec<Diagnostic>> {
    let guards = load_guards(config).map_err(anyhow::Error::from)?;
    let guard = guards
        .iter()
        .find(|g| g.spec.govctl.id == id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1002GuardNotFound,
                format!("Guard not found: {id}"),
                id,
            )
        })?;

    // Safety checks always run — --force only skips confirmation, not reference checks
    let mut blockers = Vec::new();

    if config.verification.default_guards.contains(&id.to_string()) {
        blockers.push("Listed in verification.default_guards in gov/config.toml".to_string());
    }

    let work_items = crate::parse::load_work_items(config)?;
    for wi in &work_items {
        if wi
            .spec
            .verification
            .required_guards
            .contains(&id.to_string())
        {
            blockers.push(format!("Referenced by work item {}", wi.spec.govctl.id));
        }
        for waiver in &wi.spec.verification.waivers {
            if waiver.guard == id {
                blockers.push(format!("Waiver in work item {}", wi.spec.govctl.id));
            }
        }
    }

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
        )
        .into());
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
) -> anyhow::Result<Vec<Diagnostic>> {
    let guards = load_guards(config).map_err(anyhow::Error::from)?;
    let guard = guards
        .iter()
        .find(|g| g.spec.govctl.id == id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E1002GuardNotFound,
                format!("Guard not found: {id}"),
                id,
            )
        })?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&guard.spec)?;
            println!("{json}");
        }
        OutputFormat::Table | OutputFormat::Plain => {
            println!("{}", toml::to_string_pretty(&guard.spec)?);
        }
    }

    Ok(vec![])
}

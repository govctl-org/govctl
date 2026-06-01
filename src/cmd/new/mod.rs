//! New command implementation - create artifacts.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::schema::ARTIFACT_SCHEMA_TEMPLATES;
use crate::ui;
use crate::write::{WriteOp, create_dir_all, write_file};
use std::path::PathBuf;

mod artifacts;
mod skills;
pub use artifacts::create;
pub use skills::sync_skills;

fn schema_version_for_init() -> u32 {
    std::env::var("GOVCTL_SCHEMA_VERSION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(crate::cmd::migrate::CURRENT_SCHEMA_VERSION)
}
/// Initialize govctl project
pub fn init_project(config: &Config, force: bool, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    let config_path = config.gov_root.join("config.toml");

    if config_path.exists() && !force && !op.is_preview() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0501ConfigInvalid,
            format!(
                "{} already exists (use -f to overwrite)",
                config_path.display()
            ),
            config_path.display().to_string(),
        )
        .into());
    }

    let dirs: Vec<_> = vec![
        config.gov_root.clone(),
        config.rfc_dir(),
        config.schema_dir(),
        config.rfc_output(),
        config.adr_dir(),
        config.work_dir(),
        config.guard_dir(),
        config.templates_dir(),
    ];

    for dir in &dirs {
        create_dir_all(dir, op, Some(&config.display_path(dir)))?;
        if !op.is_preview() {
            ui::created_path(&config.display_path(dir));
        }
    }

    // Write config after gov_root exists
    write_file(
        &config_path,
        &Config::default_toml(schema_version_for_init()),
        op,
        Some(&config.display_path(&config_path)),
    )?;
    if !op.is_preview() {
        ui::created_path(&config.display_path(&config_path));
    }

    // Install bundled artifact JSON Schemas under gov/schema/.
    let schema_dir = config.schema_dir();
    for template in ARTIFACT_SCHEMA_TEMPLATES {
        let path = schema_dir.join(template.filename);
        let display_path = config.display_path(&path);
        write_file(&path, template.content, op, Some(&display_path))?;
        if !op.is_preview() {
            ui::created_path(&display_path);
        }
    }

    // Ensure .gitignore contains local govctl state entries
    ensure_gitignore_lock_entry(op)?;

    if !op.is_preview() {
        ui::success("Project initialized");
        ui::hint(
            "To install agent skills locally: govctl init-skills\n  \
             Or install the govctl plugin:    /plugin install govctl@govctl",
        );
    }
    Ok(vec![])
}

/// Ensure .gitignore contains local govctl state entries.
fn ensure_gitignore_lock_entry(op: WriteOp) -> anyhow::Result<()> {
    const LOCAL_STATE_ENTRIES: &[&str] = &[".govctl.lock", ".govctl/"];
    let gitignore_path = PathBuf::from(".gitignore");

    if gitignore_path.exists() {
        let content = std::fs::read_to_string(&gitignore_path).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0901IoError,
                format!("Failed to read .gitignore: {err}"),
                gitignore_path.display().to_string(),
            )
        })?;
        let missing_entries: Vec<&str> = LOCAL_STATE_ENTRIES
            .iter()
            .copied()
            .filter(|entry| !content.lines().any(|line| line.trim() == *entry))
            .collect();

        if missing_entries.is_empty() {
            return Ok(());
        }

        let missing_content = missing_entries.join("\n");
        let new_content = if content.ends_with('\n') {
            format!("{content}{missing_content}\n")
        } else {
            format!("{content}\n{missing_content}\n")
        };
        write_file(&gitignore_path, &new_content, op, None)?;
        if !op.is_preview() {
            ui::info(format!(
                "Added local govctl state entries to .gitignore: {}",
                missing_entries.join(", ")
            ));
        }
    } else {
        let content = format!("# govctl local state\n{}\n", LOCAL_STATE_ENTRIES.join("\n"));
        write_file(&gitignore_path, &content, op, None)?;
        if !op.is_preview() {
            ui::created_path(&gitignore_path);
        }
    }
    Ok(())
}

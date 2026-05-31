//! Versioned migration pipeline for governance artifact storage.
//!
//! Each migration is a step from schema version N to N+1.
//! The current version is tracked in `gov/config.toml` under `[schema] version`.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::schema::ARTIFACT_SCHEMA_TEMPLATES;
use crate::ui;
use crate::write::{WriteOp, write_file};
use std::fs;

mod ops;
mod releases;
mod rewrite;
mod rfc_json;

use ops::{FileOp, execute_ops, preview_ops};
use releases::plan_release_upgrade;
use rewrite::plan_toml_rewrites;
use rfc_json::plan_rfc_json_to_toml;

/// Latest schema version. Bump when adding a new migration step.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// A versioned migration step.
struct MigrationStep {
    from: u32,
    to: u32,
    name: &'static str,
    plan_fn: fn(&Config) -> anyhow::Result<Vec<FileOp>>,
}

/// All registered migrations, ordered by version.
const MIGRATIONS: &[MigrationStep] = &[MigrationStep {
    from: 1,
    to: 2,
    name: "structured wire format and schema headers",
    plan_fn: plan_v1_to_v2,
}];

// =============================================================================
// Public API
// =============================================================================

pub fn migrate(config: &Config, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    // Always sync bundled JSON Schemas regardless of schema version. [[ADR-0035]]
    let schemas_synced = sync_schemas(config, op)?;

    let current = config.schema.version;
    if current >= CURRENT_SCHEMA_VERSION {
        if schemas_synced > 0 {
            ui::success(format!(
                "Synced {schemas_synced} schema file(s); already at schema version {CURRENT_SCHEMA_VERSION}"
            ));
        } else {
            ui::info(format!(
                "Repository already at schema version {CURRENT_SCHEMA_VERSION}"
            ));
        }
        return Ok(vec![]);
    }

    let pending: Vec<&MigrationStep> = MIGRATIONS
        .iter()
        .filter(|s| s.from >= current && s.to <= CURRENT_SCHEMA_VERSION)
        .collect();

    let mut all_ops = Vec::new();
    let mut step_names = Vec::new();
    for step in &pending {
        let ops = (step.plan_fn)(config)?;
        step_names.push(format!("v{} -> v{}: {}", step.from, step.to, step.name));
        all_ops.extend(ops);
    }

    if op.is_preview() {
        if all_ops.is_empty() {
            ui::info(format!(
                "No file changes needed; version bump {} -> {CURRENT_SCHEMA_VERSION}",
                current
            ));
        } else {
            preview_ops(config, &all_ops);
        }
    } else {
        if !all_ops.is_empty() {
            execute_ops(config, &all_ops)?;
        }
        bump_config_version(config, CURRENT_SCHEMA_VERSION)?;
        for name in &step_names {
            ui::sub_info(name);
        }
        let writes = all_ops
            .iter()
            .filter(|o| matches!(o, FileOp::Write { .. }))
            .count();
        let deletes = all_ops
            .iter()
            .filter(|o| matches!(o, FileOp::Delete { .. }))
            .count();
        if writes > 0 || deletes > 0 {
            let mut parts = vec![format!("{writes} file(s) written")];
            if deletes > 0 {
                parts.push(format!("{deletes} file(s) deleted"));
            }
            ui::success(format!("Migrated: {}", parts.join(", ")));
        } else {
            ui::success(format!("Schema version bumped to {CURRENT_SCHEMA_VERSION}"));
        }
    }

    Ok(vec![])
}

/// Overwrite bundled JSON Schema files into `gov/schema/`. [[ADR-0035]]
/// Returns the number of schema files that were created or updated.
fn sync_schemas(config: &Config, op: WriteOp) -> anyhow::Result<usize> {
    let schema_dir = config.schema_dir();
    if !schema_dir.exists() {
        crate::write::create_dir_all(&schema_dir, op, Some(&config.display_path(&schema_dir)))?;
    }
    let mut count = 0;
    for template in ARTIFACT_SCHEMA_TEMPLATES {
        let path = schema_dir.join(template.filename);
        if path.exists()
            && let Ok(existing) = fs::read_to_string(&path)
            && existing == template.content
        {
            continue;
        }
        let display = config.display_path(&path);
        write_file(&path, template.content, op, Some(&display))?;
        count += 1;
    }
    Ok(count)
}

/// Bump `[schema] version` in config.toml preserving the rest of the file.
fn bump_config_version(config: &Config, new_version: u32) -> anyhow::Result<()> {
    let path = config.gov_root.join("config.toml");
    let content = fs::read_to_string(&path)?;

    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut in_schema = false;
    let mut found = false;

    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_schema = trimmed == "[schema]";
        }
        if in_schema && trimmed.starts_with("version") && trimmed.contains('=') {
            *line = format!("version = {new_version}");
            found = true;
            break;
        }
    }

    if !found {
        lines.push(String::new());
        lines.push("[schema]".to_string());
        lines.push(format!("version = {new_version}"));
    }

    let mut output = lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    fs::write(&path, output)?;
    Ok(())
}

// =============================================================================
// v1 -> v2: structured wire format + schema headers
// =============================================================================

fn plan_v1_to_v2(config: &Config) -> anyhow::Result<Vec<FileOp>> {
    let mut ops = Vec::new();

    // 1. JSON RFC/clause -> TOML wire format
    let rfc_root = config.rfc_dir();
    let mut converted_rfc_dirs = Vec::new();
    if rfc_root.exists() {
        let mut dirs: Vec<_> = fs::read_dir(&rfc_root)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();

        for dir in dirs {
            if let Some((rfc_ops, rfc_id)) = plan_rfc_json_to_toml(config, &dir)? {
                ops.extend(rfc_ops);
                converted_rfc_dirs.push(rfc_id);
            }
        }
    }

    // 2. Release metadata normalization
    let mut skip_releases = false;
    if let Some(release_ops) = plan_release_upgrade(config)? {
        ops.extend(release_ops);
        skip_releases = true;
    }

    // 3. Rewrite all artifacts: add #:schema headers + strip govctl.schema
    let rewrite_ops = plan_toml_rewrites(config, &converted_rfc_dirs, skip_releases)?;
    ops.extend(rewrite_ops);

    Ok(ops)
}

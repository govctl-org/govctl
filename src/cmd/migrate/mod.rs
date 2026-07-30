//! Versioned migration pipeline for governance artifact storage.
//!
//! Each migration is a step from schema version N to N+1.
//! The current version is tracked in `gov/config.toml` under `[schema] version`.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticResult, Diagnostics};
use crate::schema::ARTIFACT_SCHEMA_TEMPLATES;
use crate::ui;
use crate::write::{WriteOp, with_file_transaction, write_file};
use std::fs;

mod ops;

use ops::{FileOp, execute_ops, preview_ops};

/// Latest schema version. Bump when adding a new migration step.
pub const CURRENT_SCHEMA_VERSION: u32 = 5;
/// Oldest project schema accepted by this binary.
pub const MIN_SUPPORTED_SCHEMA_VERSION: u32 = 3;

pub(crate) fn validate_supported_schema_version(
    version: u32,
    location: impl Into<String>,
) -> DiagnosticResult<()> {
    if version < MIN_SUPPORTED_SCHEMA_VERSION {
        return Err(Diagnostic::new(
            crate::diagnostic::DiagnosticCode::E0505MigrationRequired,
            format!(
                "Project schema version {version} is unsupported (minimum: {MIN_SUPPORTED_SCHEMA_VERSION}). Migrate this repository with a compatible earlier govctl version before upgrading."
            ),
            location,
        ));
    }
    if version > CURRENT_SCHEMA_VERSION {
        return Err(Diagnostic::new(
            crate::diagnostic::DiagnosticCode::E0505MigrationRequired,
            format!(
                "Project schema version {version} is newer than this govctl supports (latest: {CURRENT_SCHEMA_VERSION}). Upgrade govctl before using this repository."
            ),
            location,
        ));
    }
    Ok(())
}

/// A versioned migration step.
struct MigrationStep {
    from: u32,
    to: u32,
    name: &'static str,
    plan_fn: fn(&Config) -> DiagnosticResult<Vec<FileOp>>,
}

/// All registered migrations, ordered by version.
const MIGRATIONS: &[MigrationStep] = &[
    MigrationStep {
        from: 3,
        to: 4,
        name: "enable Conformance Case resources",
        plan_fn: plan_v3_to_v4,
    },
    MigrationStep {
        from: 4,
        to: 5,
        name: "adopt source scan ignore files",
        plan_fn: plan_v4_to_v5,
    },
];

fn plan_v3_to_v4(config: &Config) -> DiagnosticResult<Vec<FileOp>> {
    validate_prospective_conformance_cases(config)?;
    Ok(vec![])
}

fn validate_prospective_conformance_cases(config: &Config) -> DiagnosticResult<()> {
    let cases = crate::parse::load_conformance_cases(config)?;
    if cases.is_empty() {
        return Ok(());
    }
    crate::validate::conformance::validate_cases(config, &cases)
        .into_iter()
        .next()
        .map_or(Ok(()), Err)
}

fn plan_v4_to_v5(config: &Config) -> DiagnosticResult<Vec<FileOp>> {
    let Some(patterns) = config.source_scan.legacy_exclude.as_ref() else {
        return Ok(vec![]);
    };
    if patterns.is_empty() {
        return Ok(vec![]);
    }

    let mut migrated = String::new();
    for (index, pattern) in patterns.iter().enumerate() {
        if pattern.contains(['\r', '\n']) {
            return Err(Diagnostic::new(
                crate::diagnostic::DiagnosticCode::E0501ConfigInvalid,
                format!(
                    "Cannot migrate source_scan.exclude[{index}]: ignore rules cannot contain carriage returns or line feeds"
                ),
                config
                    .display_path(&config.gov_root.join("config.toml"))
                    .display()
                    .to_string(),
            ));
        }
        if pattern.starts_with(['!', '#']) {
            migrated.push('\\');
        }
        migrated.push_str(pattern);
        migrated.push('\n');
    }

    let path = config.project_root().join(".govignore");
    let existing = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(Diagnostic::io_error(
                "read .govignore for migration",
                error,
                config.display_path(&path).display().to_string(),
            ));
        }
    };
    migrated.push_str(&existing);
    Ok(vec![FileOp::Write {
        path,
        content: migrated,
    }])
}

// =============================================================================
// Public API
// =============================================================================

pub fn migrate(config: &Config, op: WriteOp) -> DiagnosticResult<Diagnostics> {
    validate_supported_schema_version(
        config.schema.version,
        config
            .display_path(&config.gov_root.join("config.toml"))
            .display()
            .to_string(),
    )?;
    crate::load::reject_legacy_json_storage(config)?;

    let mut support_paths = support_paths_to_sync(config)?;
    if crate::cmd::project_support::local_state_gitignore_needs_sync(config)? {
        support_paths.push(config.project_root().join(".gitignore"));
    }
    let support_path_refs = support_paths
        .iter()
        .map(std::path::PathBuf::as_path)
        .collect::<Vec<_>>();
    let schema_dir = config.schema_dir();
    let schema_dir_existed = schema_dir.exists();
    let result = with_file_transaction(&support_path_refs, op, || migrate_inner(config, op));
    if result.is_err() && !schema_dir_existed && schema_dir.is_dir() {
        let _ = std::fs::remove_dir(&schema_dir);
    }
    result
}

fn support_paths_to_sync(config: &Config) -> DiagnosticResult<Vec<std::path::PathBuf>> {
    let mut paths = Vec::new();
    for template in ARTIFACT_SCHEMA_TEMPLATES {
        let path = config.schema_dir().join(template.filename);
        match fs::read_to_string(&path) {
            Ok(existing) if existing == template.content => {}
            Ok(_) => paths.push(path),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => paths.push(path),
            Err(err) => {
                return Err(Diagnostic::io_error(
                    "read schema file",
                    err,
                    config.display_path(&path).display().to_string(),
                ));
            }
        }
    }
    Ok(paths)
}

fn migrate_inner(config: &Config, op: WriteOp) -> DiagnosticResult<Diagnostics> {
    let current = config.schema.version;
    let pending: Vec<&MigrationStep> = MIGRATIONS
        .iter()
        .filter(|s| s.from >= current && s.to <= CURRENT_SCHEMA_VERSION)
        .collect();

    // Migration planning is the graph/schema preflight and must complete before
    // support-file synchronization creates or changes anything.
    let mut all_ops = Vec::new();
    let mut step_names = Vec::new();
    for step in &pending {
        let ops = (step.plan_fn)(config)?;
        step_names.push(format!("v{} -> v{}: {}", step.from, step.to, step.name));
        all_ops.extend(ops);
    }

    // Always sync bundled JSON Schemas regardless of schema version. [[ADR-0035]]
    let schemas_synced = sync_schemas(config, op)?;
    let gitignore_entries_synced =
        crate::cmd::project_support::ensure_local_state_gitignore_entries(config, op)?;

    if current >= CURRENT_SCHEMA_VERSION {
        if schemas_synced > 0 || gitignore_entries_synced > 0 {
            let mut parts = Vec::new();
            if schemas_synced > 0 {
                parts.push(format!("{schemas_synced} schema file(s)"));
            }
            if gitignore_entries_synced > 0 {
                let label = if gitignore_entries_synced == 1 {
                    "gitignore entry"
                } else {
                    "gitignore entries"
                };
                parts.push(format!("{gitignore_entries_synced} {label}"));
            }
            let message = if op.is_preview() {
                format!(
                    "Would sync {}; already at schema version {CURRENT_SCHEMA_VERSION}",
                    parts.join(", ")
                )
            } else {
                format!(
                    "Synced {}; already at schema version {CURRENT_SCHEMA_VERSION}",
                    parts.join(", ")
                )
            };
            if op.is_preview() {
                ui::info(message);
            } else {
                ui::success(message);
            }
        } else {
            ui::info(format!(
                "Repository already at schema version {CURRENT_SCHEMA_VERSION}"
            ));
        }
        return Ok(vec![]);
    }

    let config_path = config.gov_root.join("config.toml");
    all_ops.push(plan_config_version_bump(config, CURRENT_SCHEMA_VERSION)?);

    if op.is_preview() {
        preview_ops(config, &all_ops);
    } else {
        execute_ops(config, &all_ops)?;
        for name in &step_names {
            ui::sub_info(name);
        }
        let writes = all_ops
            .iter()
            .filter(|o| matches!(o, FileOp::Write { path, .. } if path != &config_path))
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
fn sync_schemas(config: &Config, op: WriteOp) -> DiagnosticResult<usize> {
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

/// Plan a `[schema] version` bump in config.toml preserving the rest of the file.
fn plan_config_version_bump(config: &Config, new_version: u32) -> DiagnosticResult<FileOp> {
    let path = config.gov_root.join("config.toml");
    let display_path = config.display_path(&path).display().to_string();
    let content = fs::read_to_string(&path)
        .map_err(|err| Diagnostic::io_error("read config for migration", err, &display_path))?;
    let mut document = content.parse::<toml_edit::DocumentMut>().map_err(|error| {
        Diagnostic::new(
            crate::diagnostic::DiagnosticCode::E0501ConfigInvalid,
            format!("Failed to parse config for migration: {error}"),
            &display_path,
        )
    })?;
    let schema = document
        .entry("schema")
        .or_insert(toml_edit::table())
        .as_table_mut()
        .ok_or_else(|| {
            Diagnostic::new(
                crate::diagnostic::DiagnosticCode::E0501ConfigInvalid,
                "Config field schema must be a table",
                &display_path,
            )
        })?;
    let version = schema
        .entry("version")
        .or_insert(toml_edit::value(i64::from(new_version)));
    let decor = version.as_value().map(|value| value.decor().clone());
    *version = toml_edit::value(i64::from(new_version));
    if let (Some(decor), Some(value)) = (decor, version.as_value_mut()) {
        *value.decor_mut() = decor;
    }
    if new_version >= 5
        && let Some(source_scan) = document
            .get_mut("source_scan")
            .and_then(toml_edit::Item::as_table_mut)
    {
        source_scan.remove("exclude");
    }
    Ok(FileOp::Write {
        path,
        content: document.to_string(),
    })
}

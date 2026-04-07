//! Versioned migration pipeline for governance artifact storage.
//!
//! Each migration is a step from schema version N to N+1.
//! The current version is tracked in `gov/config.toml` under `[schema] version`.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseSpec, ClauseWire, ReleasesFile, RfcSpec, RfcWire};
use crate::schema::{
    ARTIFACT_SCHEMA_TEMPLATES, ArtifactSchema, validate_toml_value, with_schema_header,
};
use crate::ui;
use crate::write::{WriteOp, read_clause, read_rfc, write_file};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Latest schema version. Bump when adding a new migration step.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

// =============================================================================
// Core types
// =============================================================================

/// A single file operation produced by a migration step.
#[derive(Debug, Clone)]
pub enum FileOp {
    Write { path: PathBuf, content: String },
    Delete { path: PathBuf },
}

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

// =============================================================================
// Generic execution engine
// =============================================================================

fn preview_ops(config: &Config, ops: &[FileOp]) {
    for op in ops {
        match op {
            FileOp::Write { path, content } => {
                ui::dry_run_file_preview(&config.display_path(path), content);
            }
            FileOp::Delete { path } => {
                ui::info(format!(
                    "[DRY RUN] Would delete: {}",
                    config.display_path(path).display()
                ));
            }
        }
    }
}

fn execute_ops(config: &Config, ops: &[FileOp]) -> anyhow::Result<()> {
    let gov_root = &config.gov_root;
    let stage_root = gov_root.join(".migrate-stage");
    let backup_root = gov_root.join(".migrate-backup");

    if stage_root.exists() || backup_root.exists() {
        let mut conflicts = Vec::new();
        if stage_root.exists() {
            conflicts.push(config.display_path(&stage_root).display().to_string());
        }
        if backup_root.exists() {
            conflicts.push(config.display_path(&backup_root).display().to_string());
        }
        return Err(Diagnostic::new(
            DiagnosticCode::E0504PathConflict,
            format!(
                "Migration staging directories already exist: {}",
                conflicts.join(", ")
            ),
            config.display_path(gov_root).display().to_string(),
        )
        .into());
    }

    fs::create_dir_all(&stage_root)?;
    fs::create_dir_all(&backup_root)?;

    // Stage: write all new content to staging area
    if let Err(err) = materialize_stage(&stage_root, ops) {
        let _ = fs::remove_dir_all(&stage_root);
        let _ = fs::remove_dir_all(&backup_root);
        return Err(err);
    }

    // Commit: backup originals then apply staged content
    let result = commit_ops(&stage_root, &backup_root, ops);
    let _ = fs::remove_dir_all(&stage_root);
    if result.is_ok() {
        let _ = fs::remove_dir_all(&backup_root);
    }
    result
}

fn materialize_stage(stage_root: &Path, ops: &[FileOp]) -> anyhow::Result<()> {
    for (i, op) in ops.iter().enumerate() {
        if let FileOp::Write { content, .. } = op {
            let staged = stage_root.join(format!("{i}"));
            fs::write(staged, content)?;
        }
    }
    Ok(())
}

fn commit_ops(stage_root: &Path, backup_root: &Path, ops: &[FileOp]) -> anyhow::Result<()> {
    let mut applied: Vec<usize> = Vec::new();

    let result = (|| -> anyhow::Result<()> {
        for (i, op) in ops.iter().enumerate() {
            let backup_path = backup_root.join(format!("{i}"));
            match op {
                FileOp::Write { path, .. } => {
                    if path.exists() {
                        fs::copy(path, &backup_path)?;
                    }
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let staged = stage_root.join(format!("{i}"));
                    fs::copy(&staged, path)?;
                }
                FileOp::Delete { path } => {
                    if path.exists() {
                        fs::copy(path, &backup_path)?;
                        fs::remove_file(path)?;
                    }
                }
            }
            applied.push(i);
        }
        Ok(())
    })();

    if result.is_err() {
        for &i in applied.iter().rev() {
            let backup_path = backup_root.join(format!("{i}"));
            if !backup_path.exists() {
                continue;
            }
            match &ops[i] {
                FileOp::Write { path, .. } | FileOp::Delete { path } => {
                    let _ = fs::copy(&backup_path, path);
                }
            }
        }
    }

    result
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

fn plan_rfc_json_to_toml(
    config: &Config,
    rfc_dir: &Path,
) -> anyhow::Result<Option<(Vec<FileOp>, String)>> {
    let rfc_json = rfc_dir.join("rfc.json");
    let rfc_toml = rfc_dir.join("rfc.toml");

    if rfc_toml.exists() {
        if rfc_json.exists() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0101RfcSchemaInvalid,
                format!(
                    "Mixed RFC storage detected in {}: both rfc.json and rfc.toml exist",
                    config.display_path(rfc_dir).display()
                ),
                config.display_path(rfc_dir).display().to_string(),
            )
            .into());
        }
        return Ok(None);
    }

    if !rfc_json.exists() {
        return Ok(None);
    }

    for entry in fs::read_dir(rfc_dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == "rfc.json" || file_name == "clauses" {
            continue;
        }
        return Err(Diagnostic::new(
            DiagnosticCode::E0101RfcSchemaInvalid,
            format!(
                "Unexpected file in RFC directory during migration: {}",
                file_name
            ),
            config.display_path(&entry.path()).display().to_string(),
        )
        .into());
    }

    let mut rfc: RfcSpec = read_rfc(config, &rfc_json)?;
    let clauses_dir = rfc_dir.join("clauses");
    let mut clause_map: BTreeMap<String, ClauseSpec> = BTreeMap::new();
    let mut ops = Vec::new();

    if clauses_dir.exists() {
        for entry in fs::read_dir(&clauses_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!(
                        "Mixed clause storage in {}: TOML clause exists before migration",
                        config.display_path(&clauses_dir).display()
                    ),
                    config.display_path(&path).display().to_string(),
                )
                .into());
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!("Unexpected file in clauses directory: {name}"),
                    config.display_path(&path).display().to_string(),
                )
                .into());
            }
            let clause = read_clause(config, &path)?;
            clause_map.insert(name, clause);
        }
    }

    for section in &mut rfc.sections {
        for clause_path in &mut section.clauses {
            if clause_path.contains("..") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0204ClausePathInvalid,
                    format!("Invalid clause path: {clause_path}"),
                    config.display_path(&rfc_json).display().to_string(),
                )
                .into());
            }
            if !clause_path.ends_with(".json") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0204ClausePathInvalid,
                    format!("Mixed clause path formats not supported: {clause_path}"),
                    config.display_path(&rfc_json).display().to_string(),
                )
                .into());
            }
            let file_name = Path::new(clause_path)
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::E0204ClausePathInvalid,
                        format!("Invalid clause path: {clause_path}"),
                        config.display_path(&rfc_json).display().to_string(),
                    )
                })?;
            if !clause_map.contains_key(file_name) {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0202ClauseNotFound,
                    format!("Referenced clause missing: {clause_path}"),
                    config.display_path(&rfc_json).display().to_string(),
                )
                .into());
            }
            *clause_path = clause_path.trim_end_matches(".json").to_string() + ".toml";
        }
    }

    let rfc_id = rfc.rfc_id.clone();
    let rfc_wire: RfcWire = rfc.into();
    let rfc_body = toml::to_string_pretty(&rfc_wire)?;
    let rfc_raw: toml::Value = toml::from_str(&rfc_body)?;
    validate_toml_value(
        ArtifactSchema::Rfc,
        config,
        &rfc_dir.join("rfc.toml"),
        &rfc_raw,
    )?;

    ops.push(FileOp::Write {
        path: rfc_dir.join("rfc.toml"),
        content: with_schema_header(ArtifactSchema::Rfc, &rfc_body),
    });
    ops.push(FileOp::Delete { path: rfc_json });

    for (file_name, clause) in clause_map {
        let toml_name = file_name.trim_end_matches(".json").to_string() + ".toml";
        let clause_wire: ClauseWire = clause.into();
        let body = toml::to_string_pretty(&clause_wire)?;
        let raw: toml::Value = toml::from_str(&body)?;
        validate_toml_value(
            ArtifactSchema::Clause,
            config,
            &clauses_dir.join(&toml_name),
            &raw,
        )?;

        ops.push(FileOp::Write {
            path: clauses_dir.join(&toml_name),
            content: with_schema_header(ArtifactSchema::Clause, &body),
        });
        ops.push(FileOp::Delete {
            path: clauses_dir.join(&file_name),
        });
    }

    Ok(Some((ops, rfc_id)))
}

fn plan_release_upgrade(config: &Config) -> anyhow::Result<Option<Vec<FileOp>>> {
    let path = config.releases_path();
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)?;
    let mut raw: toml::Value = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0704ReleaseSchemaInvalid,
            format!("Invalid releases.toml: {e}"),
            config.display_path(&path).display().to_string(),
        )
    })?;

    let needs_upgrade = {
        let table = raw.as_table();
        let govctl = table
            .and_then(|t| t.get("govctl"))
            .and_then(toml::Value::as_table);
        let has_schema = govctl
            .and_then(|g| g.get("schema"))
            .and_then(toml::Value::as_integer)
            == Some(1);
        !has_schema
    };

    if !needs_upgrade {
        return Ok(None);
    }

    // Normalize: ensure [govctl] schema = 1
    if let Some(root) = raw.as_table_mut() {
        let govctl = root
            .entry("govctl".to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
        if let Some(table) = govctl.as_table_mut() {
            table
                .entry("schema".to_string())
                .or_insert(toml::Value::Integer(1));
        }
    }

    validate_toml_value(ArtifactSchema::Release, config, &path, &raw)?;
    let releases: ReleasesFile = raw.try_into().map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0704ReleaseSchemaInvalid,
            format!("Invalid releases structure: {e}"),
            config.display_path(&path).display().to_string(),
        )
    })?;
    let body = toml::to_string_pretty(&releases)?;
    Ok(Some(vec![FileOp::Write {
        path,
        content: with_schema_header(ArtifactSchema::Release, &body),
    }]))
}

/// Strip `schema = N` lines from a `[govctl]` section in raw TOML text.
fn strip_govctl_schema(content: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();
    let mut in_govctl = false;
    lines.retain(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_govctl = trimmed == "[govctl]";
        }
        !(in_govctl && trimmed.starts_with("schema") && trimmed.contains('='))
    });
    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Check if a TOML file needs rewrite (missing header or has `govctl.schema`).
fn needs_rewrite(content: &str) -> bool {
    if !content.starts_with("#:schema ") {
        return true;
    }
    let mut in_govctl = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_govctl = trimmed == "[govctl]";
        }
        if in_govctl && trimmed.starts_with("schema") && trimmed.contains('=') {
            return true;
        }
    }
    false
}

/// Rewrite a TOML file: ensure `#:schema` header and strip `govctl.schema`.
fn rewrite_toml(content: &str, schema: ArtifactSchema) -> String {
    let cleaned = strip_govctl_schema(content);
    if cleaned.starts_with("#:schema ") {
        cleaned
    } else {
        with_schema_header(schema, &cleaned)
    }
}

/// Collect TOML files in a directory that need rewriting.
fn collect_rewrites(dir: &Path, schema: ArtifactSchema) -> Vec<FileOp> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut ops: Vec<(PathBuf, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path)
            && needs_rewrite(&content)
        {
            ops.push((path, rewrite_toml(&content, schema)));
        }
    }
    ops.sort_by(|a, b| a.0.cmp(&b.0));
    ops.into_iter()
        .map(|(path, content)| FileOp::Write { path, content })
        .collect()
}

/// Plan header + schema-strip rewrites for all TOML artifacts.
fn plan_toml_rewrites(
    config: &Config,
    skip_rfc_ids: &[String],
    skip_releases: bool,
) -> anyhow::Result<Vec<FileOp>> {
    let mut ops = Vec::new();

    ops.extend(collect_rewrites(&config.adr_dir(), ArtifactSchema::Adr));
    ops.extend(collect_rewrites(
        &config.work_dir(),
        ArtifactSchema::WorkItem,
    ));
    ops.extend(collect_rewrites(&config.guard_dir(), ArtifactSchema::Guard));

    let rfc_root = config.rfc_dir();
    if rfc_root.exists() {
        for entry in fs::read_dir(&rfc_root)?.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if skip_rfc_ids.iter().any(|id| id == dir_name) {
                continue;
            }
            let rfc_toml = dir.join("rfc.toml");
            if rfc_toml.exists()
                && let Ok(content) = fs::read_to_string(&rfc_toml)
                && needs_rewrite(&content)
            {
                ops.push(FileOp::Write {
                    path: rfc_toml,
                    content: rewrite_toml(&content, ArtifactSchema::Rfc),
                });
            }
            ops.extend(collect_rewrites(
                &dir.join("clauses"),
                ArtifactSchema::Clause,
            ));
        }
    }

    if !skip_releases {
        let releases_path = config.releases_path();
        if releases_path.exists()
            && let Ok(content) = fs::read_to_string(&releases_path)
            && needs_rewrite(&content)
        {
            ops.push(FileOp::Write {
                path: releases_path,
                content: rewrite_toml(&content, ArtifactSchema::Release),
            });
        }
    }

    Ok(ops)
}

//! Deterministic repository-local storage migration.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ClauseSpec, ReleasesFile, RfcSpec};
use crate::schema::{ArtifactSchema, validate_toml_value};
use crate::ui;
use crate::write::{WriteOp, delete_file, read_clause, read_rfc};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct PlannedFile {
    relative_path: PathBuf,
    target_path: PathBuf,
    content: String,
}

#[derive(Debug)]
struct RfcMigrationPlan {
    rfc_id: String,
    source_dir: PathBuf,
    legacy_paths: Vec<PathBuf>,
    files: Vec<PlannedFile>,
}

#[derive(Debug)]
struct MigrationPlan {
    rfcs: Vec<RfcMigrationPlan>,
    release_upgrade: Option<String>,
}

impl MigrationPlan {
    fn is_noop(&self) -> bool {
        self.rfcs.is_empty() && self.release_upgrade.is_none()
    }

    fn clause_count(&self) -> usize {
        self.rfcs
            .iter()
            .map(|plan| {
                plan.files
                    .iter()
                    .filter(|file| *file.relative_path != *"rfc.toml")
                    .count()
            })
            .sum()
    }
}

pub fn migrate(config: &Config, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
    let plan = build_plan(config)?;

    if plan.is_noop() {
        ui::info("Repository already migrated");
        return Ok(vec![]);
    }

    if op.is_preview() {
        preview_plan(config, &plan)?;
    } else {
        execute_plan(config, &plan)?;
        ui::success(format!(
            "Migrated {} RFC(s), {} clause file(s){}",
            plan.rfcs.len(),
            plan.clause_count(),
            if plan.release_upgrade.is_some() {
                ", and upgraded releases.toml metadata"
            } else {
                ""
            }
        ));
    }

    Ok(vec![])
}

fn build_plan(config: &Config) -> anyhow::Result<MigrationPlan> {
    let mut rfcs = Vec::new();
    let rfc_root = config.rfc_dir();

    if rfc_root.exists() {
        let mut dirs: Vec<_> = fs::read_dir(&rfc_root)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        dirs.sort();

        for dir in dirs {
            if let Some(plan) = build_rfc_plan(config, &dir)? {
                rfcs.push(plan);
            }
        }
    }

    let release_upgrade = build_release_upgrade(config)?;

    Ok(MigrationPlan {
        rfcs,
        release_upgrade,
    })
}

fn build_rfc_plan(config: &Config, rfc_dir: &Path) -> anyhow::Result<Option<RfcMigrationPlan>> {
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
    let mut legacy_paths = vec![rfc_json.clone()];

    if clauses_dir.exists() {
        for entry in fs::read_dir(&clauses_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!(
                        "Mixed clause storage detected in {}: TOML clause exists before migration",
                        config.display_path(&clauses_dir).display()
                    ),
                    config.display_path(&path).display().to_string(),
                )
                .into());
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0201ClauseSchemaInvalid,
                    format!(
                        "Unexpected file in clauses directory during migration: {}",
                        name
                    ),
                    config.display_path(&path).display().to_string(),
                )
                .into());
            }

            let clause = read_clause(config, &path)?;
            clause_map.insert(name, clause);
            legacy_paths.push(path);
        }
    }

    for section in &mut rfc.sections {
        for clause_path in &mut section.clauses {
            if clause_path.contains("..") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0204ClausePathInvalid,
                    format!("Invalid clause path during migration: {}", clause_path),
                    config.display_path(&rfc_json).display().to_string(),
                )
                .into());
            }
            if !clause_path.ends_with(".json") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0204ClausePathInvalid,
                    format!(
                        "Mixed clause path formats are not supported during migration: {}",
                        clause_path
                    ),
                    config.display_path(&rfc_json).display().to_string(),
                )
                .into());
            }
            let file_name = Path::new(clause_path)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid clause path: {}", clause_path))?;
            if !clause_map.contains_key(file_name) {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0202ClauseNotFound,
                    format!(
                        "Referenced clause missing during migration: {}",
                        clause_path
                    ),
                    config.display_path(&rfc_json).display().to_string(),
                )
                .into());
            }
            *clause_path = clause_path.trim_end_matches(".json").to_string() + ".toml";
        }
    }

    let rfc_content = toml::to_string_pretty(&rfc)?;
    let rfc_raw: toml::Value = toml::from_str(&rfc_content)?;
    validate_toml_value(
        ArtifactSchema::Rfc,
        config,
        &rfc_dir.join("rfc.toml"),
        &rfc_raw,
    )?;

    let mut files = vec![PlannedFile {
        relative_path: PathBuf::from("rfc.toml"),
        target_path: rfc_dir.join("rfc.toml"),
        content: rfc_content,
    }];

    for (file_name, clause) in clause_map {
        let toml_name = file_name.trim_end_matches(".json").to_string() + ".toml";
        let content = toml::to_string_pretty(&clause)?;
        let raw: toml::Value = toml::from_str(&content)?;
        validate_toml_value(
            ArtifactSchema::Clause,
            config,
            &clauses_dir.join(&toml_name),
            &raw,
        )?;
        files.push(PlannedFile {
            relative_path: PathBuf::from("clauses").join(&toml_name),
            target_path: clauses_dir.join(&toml_name),
            content,
        });
    }

    let rfc_id = rfc.rfc_id.clone();

    Ok(Some(RfcMigrationPlan {
        rfc_id,
        source_dir: rfc_dir.to_path_buf(),
        legacy_paths,
        files,
    }))
}

fn build_release_upgrade(config: &Config) -> anyhow::Result<Option<String>> {
    let releases_path = config.releases_path();
    if !releases_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&releases_path)?;
    let mut raw: toml::Value = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0704ReleaseSchemaInvalid,
            format!("Invalid releases.toml: {}", e),
            config.display_path(&releases_path).display().to_string(),
        )
    })?;

    if !needs_release_upgrade(&raw) {
        return Ok(None);
    }

    normalize_release_value(&mut raw);
    validate_toml_value(ArtifactSchema::Release, config, &releases_path, &raw)?;
    let releases: ReleasesFile = raw.try_into().map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0704ReleaseSchemaInvalid,
            format!("Invalid normalized releases structure: {}", e),
            config.display_path(&releases_path).display().to_string(),
        )
    })?;
    Ok(Some(toml::to_string_pretty(&releases)?))
}

fn needs_release_upgrade(raw: &toml::Value) -> bool {
    let Some(table) = raw.as_table() else {
        return true;
    };
    let Some(govctl) = table.get("govctl").and_then(toml::Value::as_table) else {
        return true;
    };
    !matches!(
        govctl.get("schema").and_then(toml::Value::as_integer),
        Some(1)
    )
}

fn normalize_release_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };
    let govctl = root
        .entry("govctl".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    if let Some(table) = govctl.as_table_mut() {
        table
            .entry("schema".to_string())
            .or_insert(toml::Value::Integer(1));
    }
}

fn preview_plan(config: &Config, plan: &MigrationPlan) -> anyhow::Result<()> {
    for rfc in &plan.rfcs {
        for file in &rfc.files {
            ui::dry_run_file_preview(&config.display_path(&file.target_path), &file.content);
        }
        for legacy in &rfc.legacy_paths {
            delete_file(legacy, WriteOp::Preview, Some(&config.display_path(legacy)))?;
        }
    }

    if let Some(content) = &plan.release_upgrade {
        let path = config.releases_path();
        ui::dry_run_file_preview(&config.display_path(&path), content);
    }

    Ok(())
}

fn execute_plan(config: &Config, plan: &MigrationPlan) -> anyhow::Result<()> {
    let gov_root = &config.paths.gov_root;
    let stage_root = gov_root.join(".migrate-stage");
    let backup_root = gov_root.join(".migrate-backup");

    if stage_root.exists() || backup_root.exists() {
        return Err(anyhow::anyhow!(
            "Migration staging directories already exist under {}",
            config.display_path(gov_root).display()
        ));
    }

    fs::create_dir_all(&stage_root)?;
    fs::create_dir_all(&backup_root)?;

    if let Err(err) = materialize_stage(&stage_root, plan) {
        let _ = fs::remove_dir_all(&stage_root);
        let _ = fs::remove_dir_all(&backup_root);
        return Err(err);
    }

    let result = commit_stage(config, &stage_root, &backup_root, plan);
    if result.is_ok() {
        let _ = fs::remove_dir_all(&stage_root);
        let _ = fs::remove_dir_all(&backup_root);
    }
    result
}

fn materialize_stage(stage_root: &Path, plan: &MigrationPlan) -> anyhow::Result<()> {
    for rfc in &plan.rfcs {
        let stage_dir = stage_root.join("rfc").join(&rfc.rfc_id);
        for file in &rfc.files {
            let target = stage_dir.join(&file.relative_path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(target, &file.content)?;
        }
    }

    if let Some(content) = &plan.release_upgrade {
        fs::write(stage_root.join("releases.toml"), content)?;
    }

    Ok(())
}

fn commit_stage(
    config: &Config,
    stage_root: &Path,
    backup_root: &Path,
    plan: &MigrationPlan,
) -> anyhow::Result<()> {
    let mut moved_rfcs: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut release_backup: Option<PathBuf> = None;
    let mut release_target_written = false;
    let release_target = config.releases_path();

    let commit_result = (|| -> anyhow::Result<()> {
        if !plan.rfcs.is_empty() {
            fs::create_dir_all(backup_root.join("rfc"))?;
        }

        for rfc in &plan.rfcs {
            let source_dir = rfc.source_dir.clone();
            let backup_dir = backup_root.join("rfc").join(&rfc.rfc_id);
            let staged_dir = stage_root.join("rfc").join(&rfc.rfc_id);

            fs::rename(&source_dir, &backup_dir)?;
            fs::rename(&staged_dir, &source_dir)?;
            moved_rfcs.push((source_dir, backup_dir));
        }

        if plan.release_upgrade.is_some() {
            let staged_release = stage_root.join("releases.toml");
            if release_target.exists() {
                let backup_path = backup_root.join("releases.toml");
                fs::rename(&release_target, &backup_path)?;
                release_backup = Some(backup_path);
            }
            fs::rename(&staged_release, &release_target)?;
            release_target_written = true;
        }

        Ok(())
    })();

    if commit_result.is_err() {
        rollback_commit(
            &moved_rfcs,
            release_backup.as_ref(),
            &release_target,
            release_target_written,
        );
    }

    commit_result
}

fn rollback_commit(
    moved_rfcs: &[(PathBuf, PathBuf)],
    release_backup: Option<&PathBuf>,
    release_target: &Path,
    release_target_written: bool,
) {
    if release_target_written {
        let _ = fs::remove_file(release_target);
        if let Some(backup) = release_backup {
            let _ = fs::rename(backup, release_target);
        }
    }

    for (final_dir, backup_dir) in moved_rfcs.iter().rev() {
        let _ = fs::remove_dir_all(final_dir);
        let _ = fs::rename(backup_dir, final_dir);
    }
}

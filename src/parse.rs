//! TOML parsing for ADR, Work Item, and Release files.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::ReleasesMeta;
use crate::model::{
    AdrEntry, AdrSpec, GuardEntry, GuardSpec, ReleasesFile, WorkItemEntry, WorkItemSpec,
};
use crate::schema::{ArtifactSchema, validate_toml_value, with_schema_header};
use crate::ui;
use crate::write::WriteOp;
use std::path::Path;

/// Result of loading items: successfully loaded items plus any warnings
pub struct LoadResult<T> {
    pub items: Vec<T>,
    pub warnings: Vec<Diagnostic>,
}

/// Load all ADRs from the adr directory
pub fn load_adrs(config: &Config) -> Result<Vec<AdrEntry>, Diagnostic> {
    load_adrs_with_warnings(config).map(|r| r.items)
}

/// Load all ADRs, returning both items and parse warnings
pub fn load_adrs_with_warnings(config: &Config) -> Result<LoadResult<AdrEntry>, Diagnostic> {
    let adr_dir = config.adr_dir();
    if !adr_dir.exists() {
        return Ok(LoadResult {
            items: vec![],
            warnings: vec![],
        });
    }

    let mut adrs = Vec::new();
    let mut warnings = Vec::new();
    let entries = std::fs::read_dir(&adr_dir).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            adr_dir.display().to_string(),
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match load_adr(config, &path) {
                Ok(adr) => adrs.push(adr),
                Err(e) => warnings.push(e),
            }
        }
    }

    // Sort by ID for deterministic output
    adrs.sort_by(|a, b| a.spec.govctl.id.cmp(&b.spec.govctl.id));

    Ok(LoadResult {
        items: adrs,
        warnings,
    })
}

/// Load a single ADR from TOML file
pub fn load_adr(config: &Config, path: &Path) -> Result<AdrEntry, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let raw: toml::Value = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0301AdrSchemaInvalid,
            format!("Invalid TOML: {e}"),
            path.display().to_string(),
        )
    })?;
    validate_toml_value(ArtifactSchema::Adr, config, path, &raw)?;
    let spec: AdrSpec = raw.try_into().map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0301AdrSchemaInvalid,
            format!("Invalid ADR structure: {e}"),
            path.display().to_string(),
        )
    })?;

    Ok(AdrEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Write an ADR to TOML file
pub fn write_adr(
    path: &Path,
    spec: &AdrSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    let body = toml::to_string_pretty(spec).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize TOML: {e}"),
            path.display().to_string(),
        )
    })?;
    let content = with_schema_header(ArtifactSchema::Adr, &body);

    match op {
        WriteOp::Execute => {
            std::fs::write(path, &content).map_err(|e| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    e.to_string(),
                    path.display().to_string(),
                )
            })?;
        }
        WriteOp::Preview => {
            let output_path = display_path.unwrap_or(path);
            ui::dry_run_file_preview(output_path, &content);
        }
    }

    Ok(())
}

/// Load all work items from the work directory
pub fn load_work_items(config: &Config) -> Result<Vec<WorkItemEntry>, Diagnostic> {
    load_work_items_with_warnings(config).map(|r| r.items)
}

/// Load all work items, returning both items and parse warnings
pub fn load_work_items_with_warnings(
    config: &Config,
) -> Result<LoadResult<WorkItemEntry>, Diagnostic> {
    let work_dir = config.work_dir();
    if !work_dir.exists() {
        return Ok(LoadResult {
            items: vec![],
            warnings: vec![],
        });
    }

    let mut items = Vec::new();
    let mut warnings = Vec::new();
    let entries = std::fs::read_dir(&work_dir).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            work_dir.display().to_string(),
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match load_work_item(config, &path) {
                Ok(item) => items.push(item),
                Err(e) => warnings.push(e),
            }
        }
    }

    // Sort by ID for deterministic output
    items.sort_by(|a, b| a.spec.govctl.id.cmp(&b.spec.govctl.id));

    Ok(LoadResult { items, warnings })
}

/// Load all verification guards from the guard directory.
#[allow(dead_code)]
pub fn load_guards(config: &Config) -> Result<Vec<GuardEntry>, Diagnostic> {
    load_guards_with_warnings(config).map(|r| r.items)
}

/// Load all verification guards, returning both items and parse warnings.
pub fn load_guards_with_warnings(config: &Config) -> Result<LoadResult<GuardEntry>, Diagnostic> {
    let guard_dir = config.guard_dir();
    if !guard_dir.exists() {
        return Ok(LoadResult {
            items: vec![],
            warnings: vec![],
        });
    }

    let mut items = Vec::new();
    let mut warnings = Vec::new();
    let entries = std::fs::read_dir(&guard_dir).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            guard_dir.display().to_string(),
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match load_guard(config, &path) {
                Ok(item) => items.push(item),
                Err(e) => warnings.push(e),
            }
        }
    }

    items.sort_by(|a, b| a.spec.govctl.id.cmp(&b.spec.govctl.id));

    Ok(LoadResult { items, warnings })
}

/// Load a single verification guard from TOML file.
pub fn load_guard(config: &Config, path: &Path) -> Result<GuardEntry, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let raw: toml::Value = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E1001GuardSchemaInvalid,
            format!("Invalid TOML: {e}"),
            path.display().to_string(),
        )
    })?;
    validate_toml_value(ArtifactSchema::Guard, config, path, &raw)?;
    let spec: GuardSpec = raw.try_into().map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E1001GuardSchemaInvalid,
            format!("Invalid verification guard structure: {e}"),
            path.display().to_string(),
        )
    })?;

    Ok(GuardEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Load a single work item from TOML file
pub fn load_work_item(config: &Config, path: &Path) -> Result<WorkItemEntry, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let raw: toml::Value = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0401WorkSchemaInvalid,
            format!("Invalid TOML: {e}"),
            path.display().to_string(),
        )
    })?;
    validate_toml_value(ArtifactSchema::WorkItem, config, path, &raw)?;
    let spec: WorkItemSpec = raw.try_into().map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0401WorkSchemaInvalid,
            format!("Invalid work item structure: {e}"),
            path.display().to_string(),
        )
    })?;

    Ok(WorkItemEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Write a work item to TOML file
pub fn write_work_item(
    path: &Path,
    spec: &WorkItemSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    let body = toml::to_string_pretty(spec).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize TOML: {e}"),
            path.display().to_string(),
        )
    })?;
    let content = with_schema_header(ArtifactSchema::WorkItem, &body);

    match op {
        WriteOp::Execute => {
            std::fs::write(path, &content).map_err(|e| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    e.to_string(),
                    path.display().to_string(),
                )
            })?;
        }
        WriteOp::Preview => {
            let output_path = display_path.unwrap_or(path);
            ui::dry_run_file_preview(output_path, &content);
        }
    }

    Ok(())
}

/// Load releases from gov/releases.toml
/// Returns empty ReleasesFile if file doesn't exist.
/// Validates that all versions are valid semver.
pub fn load_releases(config: &Config) -> Result<ReleasesFile, Diagnostic> {
    let path = config.releases_path();
    if !path.exists() {
        return Ok(ReleasesFile::default());
    }

    let content = std::fs::read_to_string(&path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let mut raw: toml::Value = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0704ReleaseSchemaInvalid,
            format!("Invalid releases.toml: {e}"),
            path.display().to_string(),
        )
    })?;
    normalize_release_value(&mut raw);
    validate_toml_value(ArtifactSchema::Release, config, &path, &raw)?;
    let releases: ReleasesFile = raw.try_into().map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0704ReleaseSchemaInvalid,
            format!("Invalid release structure: {e}"),
            path.display().to_string(),
        )
    })?;

    // Validate all versions are valid semver
    for release in &releases.releases {
        semver::Version::parse(&release.version).map_err(|_| {
            Diagnostic::new(
                DiagnosticCode::E0701ReleaseInvalidSemver,
                format!("Invalid semver version: {}", release.version),
                path.display().to_string(),
            )
        })?;
    }

    Ok(releases)
}

/// Validate a version string as semver
pub fn validate_version(version: &str) -> Result<semver::Version, String> {
    semver::Version::parse(version).map_err(|_| format!("Invalid semver: {version}"))
}

fn normalize_release_value(raw: &mut toml::Value) {
    let Some(root) = raw.as_table_mut() else {
        return;
    };

    if !root.contains_key("govctl") {
        let mut govctl = toml::map::Map::new();
        govctl.insert("schema".to_string(), toml::Value::Integer(1));
        root.insert("govctl".to_string(), toml::Value::Table(govctl));
        return;
    }

    let Some(govctl) = root.get_mut("govctl").and_then(toml::Value::as_table_mut) else {
        return;
    };
    govctl
        .entry("schema".to_string())
        .or_insert(toml::Value::Integer(ReleasesMeta::default().schema.into()));
}

/// Write releases to gov/releases.toml
pub fn write_releases(
    config: &Config,
    releases: &ReleasesFile,
    op: WriteOp,
) -> Result<(), Diagnostic> {
    let path = config.releases_path();
    let path_display = config.display_path(&path);
    let body = toml::to_string_pretty(releases).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize releases: {e}"),
            path_display.display().to_string(),
        )
    })?;
    let content = with_schema_header(ArtifactSchema::Release, &body);

    match op {
        WriteOp::Execute => {
            std::fs::write(&path, &content).map_err(|e| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    e.to_string(),
                    path_display.display().to_string(),
                )
            })?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(&path_display, &content);
        }
    }

    Ok(())
}

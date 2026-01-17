//! TOML parsing for ADR, Work Item, and Release files.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{AdrEntry, AdrSpec, ReleasesFile, WorkItemEntry, WorkItemSpec};
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
            match load_adr(&path) {
                Ok(adr) => adrs.push(adr),
                Err(e) => {
                    // Record warning instead of silently skipping
                    warnings.push(Diagnostic::new(
                        DiagnosticCode::W0104AdrParseSkipped,
                        format!("Skipped ADR (parse error): {}", e.message),
                        path.display().to_string(),
                    ));
                }
            }
        }
    }

    Ok(LoadResult {
        items: adrs,
        warnings,
    })
}

/// Load a single ADR from TOML file
pub fn load_adr(path: &Path) -> Result<AdrEntry, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let spec: AdrSpec = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0301AdrSchemaInvalid,
            format!("Invalid TOML: {e}"),
            path.display().to_string(),
        )
    })?;

    Ok(AdrEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Write an ADR to TOML file
pub fn write_adr(path: &Path, spec: &AdrSpec, op: WriteOp) -> Result<(), Diagnostic> {
    let content = toml::to_string_pretty(spec).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize TOML: {e}"),
            path.display().to_string(),
        )
    })?;

    match op {
        WriteOp::Execute => {
            std::fs::write(path, content).map_err(|e| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    e.to_string(),
                    path.display().to_string(),
                )
            })?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(path, &content);
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
            match load_work_item(&path) {
                Ok(item) => items.push(item),
                Err(e) => {
                    // Record warning instead of silently skipping
                    warnings.push(Diagnostic::new(
                        DiagnosticCode::W0105WorkParseSkipped,
                        format!("Skipped work item (parse error): {}", e.message),
                        path.display().to_string(),
                    ));
                }
            }
        }
    }

    Ok(LoadResult { items, warnings })
}

/// Load a single work item from TOML file
pub fn load_work_item(path: &Path) -> Result<WorkItemEntry, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            e.to_string(),
            path.display().to_string(),
        )
    })?;

    let spec: WorkItemSpec = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0401WorkSchemaInvalid,
            format!("Invalid TOML: {e}"),
            path.display().to_string(),
        )
    })?;

    Ok(WorkItemEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Write a work item to TOML file
pub fn write_work_item(path: &Path, spec: &WorkItemSpec, op: WriteOp) -> Result<(), Diagnostic> {
    let content = toml::to_string_pretty(spec).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize TOML: {e}"),
            path.display().to_string(),
        )
    })?;

    match op {
        WriteOp::Execute => {
            std::fs::write(path, content).map_err(|e| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    e.to_string(),
                    path.display().to_string(),
                )
            })?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(path, &content);
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

    let releases: ReleasesFile = toml::from_str(&content).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Invalid releases.toml: {e}"),
            path.display().to_string(),
        )
    })?;

    // Validate all versions are valid semver
    for release in &releases.releases {
        semver::Version::parse(&release.version).map_err(|_| {
            Diagnostic::new(
                DiagnosticCode::E0901IoError,
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

/// Write releases to gov/releases.toml
pub fn write_releases(
    config: &Config,
    releases: &ReleasesFile,
    op: WriteOp,
) -> Result<(), Diagnostic> {
    let path = config.releases_path();
    let content = toml::to_string_pretty(releases).map_err(|e| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize releases: {e}"),
            path.display().to_string(),
        )
    })?;

    match op {
        WriteOp::Execute => {
            std::fs::write(&path, content).map_err(|e| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    e.to_string(),
                    path.display().to_string(),
                )
            })?;
        }
        WriteOp::Preview => {
            ui::dry_run_file_preview(&path, &content);
        }
    }

    Ok(())
}

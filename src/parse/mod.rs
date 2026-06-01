//! TOML parsing for ADR, Work Item, Guard, and Release files.

mod toml_io;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{
    AdrEntry, AdrSpec, GuardEntry, GuardSpec, ReleasesFile, WorkItemEntry, WorkItemSpec,
};
use crate::schema::ArtifactSchema;
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
    toml_io::load_toml_dir(
        &adr_dir,
        |path| load_adr(config, path),
        |adrs| adrs.sort_by(|a, b| a.spec.govctl.id.cmp(&b.spec.govctl.id)),
    )
}

/// Load a single ADR from TOML file
pub fn load_adr(config: &Config, path: &Path) -> Result<AdrEntry, Diagnostic> {
    let spec = toml_io::load_toml_spec(
        config,
        path,
        ArtifactSchema::Adr,
        DiagnosticCode::E0301AdrSchemaInvalid,
        "Invalid TOML",
        "Invalid ADR structure",
        |_| {},
    )?;

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
    toml_io::write_toml_spec(
        path,
        ArtifactSchema::Adr,
        spec,
        op,
        display_path,
        "Failed to serialize TOML",
    )
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
    toml_io::load_toml_dir(
        &work_dir,
        |path| load_work_item(config, path),
        |items| items.sort_by(|a, b| a.spec.govctl.id.cmp(&b.spec.govctl.id)),
    )
}

/// Load all verification guards from the guard directory.
pub fn load_guards(config: &Config) -> Result<Vec<GuardEntry>, Diagnostic> {
    load_guards_with_warnings(config).map(|r| r.items)
}

/// Load all verification guards, returning both items and parse warnings.
pub fn load_guards_with_warnings(config: &Config) -> Result<LoadResult<GuardEntry>, Diagnostic> {
    let guard_dir = config.guard_dir();
    toml_io::load_toml_dir(
        &guard_dir,
        |path| load_guard(config, path),
        |items| items.sort_by(|a, b| a.spec.govctl.id.cmp(&b.spec.govctl.id)),
    )
}

/// Load a single verification guard from TOML file.
pub fn load_guard(config: &Config, path: &Path) -> Result<GuardEntry, Diagnostic> {
    let spec = toml_io::load_toml_spec(
        config,
        path,
        ArtifactSchema::Guard,
        DiagnosticCode::E1001GuardSchemaInvalid,
        "Invalid TOML",
        "Invalid verification guard structure",
        |_| {},
    )?;

    Ok(GuardEntry {
        spec,
        path: path.to_path_buf(),
    })
}

/// Load a single work item from TOML file
pub fn load_work_item(config: &Config, path: &Path) -> Result<WorkItemEntry, Diagnostic> {
    let spec = toml_io::load_toml_spec(
        config,
        path,
        ArtifactSchema::WorkItem,
        DiagnosticCode::E0401WorkSchemaInvalid,
        "Invalid TOML",
        "Invalid work item structure",
        strip_legacy_inline_history_for_schema,
    )?;

    Ok(WorkItemEntry {
        spec,
        path: path.to_path_buf(),
    })
}

fn strip_legacy_inline_history_for_schema(raw: &mut toml::Value) {
    let Some(content) = raw
        .as_table_mut()
        .and_then(|root| root.get_mut("content"))
        .and_then(toml::Value::as_table_mut)
    else {
        return;
    };
    content.remove("journal");
}

/// Write a work item to TOML file
pub fn write_work_item(
    path: &Path,
    spec: &WorkItemSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    toml_io::write_toml_spec(
        path,
        ArtifactSchema::WorkItem,
        spec,
        op,
        display_path,
        "Failed to serialize TOML",
    )
}

/// Write a verification guard to TOML file.
pub fn write_guard(
    path: &Path,
    spec: &GuardSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    toml_io::write_toml_spec(
        path,
        ArtifactSchema::Guard,
        spec,
        op,
        display_path,
        "Failed to serialize TOML",
    )
}

/// Load releases from gov/releases.toml
/// Returns empty ReleasesFile if file doesn't exist.
/// Validates that all versions are valid semver.
pub fn load_releases(config: &Config) -> Result<ReleasesFile, Diagnostic> {
    let path = config.releases_path();
    if !path.exists() {
        return Ok(ReleasesFile::default());
    }

    let releases: ReleasesFile = toml_io::load_toml_spec(
        config,
        &path,
        ArtifactSchema::Release,
        DiagnosticCode::E0704ReleaseSchemaInvalid,
        "Invalid releases.toml",
        "Invalid release structure",
        |_| {},
    )?;

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

/// Write releases to gov/releases.toml
pub fn write_releases(
    config: &Config,
    releases: &ReleasesFile,
    op: WriteOp,
) -> Result<(), Diagnostic> {
    let path = config.releases_path();
    let path_display = config.display_path(&path);
    toml_io::write_toml_spec(
        &path,
        ArtifactSchema::Release,
        releases,
        op,
        Some(&path_display),
        "Failed to serialize releases",
    )
}

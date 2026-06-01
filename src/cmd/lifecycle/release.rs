use std::collections::HashSet;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{Release, WorkItemStatus};
use crate::parse::{load_releases, load_work_items, validate_version, write_releases};
use crate::ui;
use crate::write::{WriteOp, today};

/// Cut a release - collect unreleased work items into a version
/// Per [[ADR-0014]], stores release info in gov/releases.toml
pub fn cut_release(
    config: &Config,
    version: &str,
    date: Option<&str>,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let releases_path = config.releases_path();
    let releases_path_str = config.display_path(&releases_path).display().to_string();

    // Validate version is valid semver
    validate_version(version).map_err(|_| {
        Diagnostic::new(
            DiagnosticCode::E0701ReleaseInvalidSemver,
            format!("Invalid semver version: {version}"),
            &releases_path_str,
        )
    })?;

    // Load existing releases
    let mut releases_file = load_releases(config)?;

    // Check for duplicate version
    if releases_file.releases.iter().any(|r| r.version == version) {
        let diag = Diagnostic::new(
            DiagnosticCode::E0702ReleaseDuplicate,
            format!("Release {version} already exists"),
            &releases_path_str,
        );
        return Err(diag.into());
    }

    // Get all work item IDs already in releases
    let released_ids: HashSet<_> = releases_file
        .releases
        .iter()
        .flat_map(|r| r.refs.iter().cloned())
        .collect();

    // Load all done work items
    let work_items = load_work_items(config)?;
    let unreleased: Vec<_> = work_items
        .iter()
        .filter(|w| w.spec.govctl.status == WorkItemStatus::Done)
        .filter(|w| !released_ids.contains(&w.spec.govctl.id))
        .collect();

    if unreleased.is_empty() {
        let diag = Diagnostic::new(
            DiagnosticCode::E0703ReleaseNoUnreleasedItems,
            "No unreleased work items to include in release",
            &releases_path_str,
        );
        return Err(diag.into());
    }

    // Create new release
    let release_date = date.map(|d| d.to_string()).unwrap_or_else(today);
    let mut refs: Vec<_> = unreleased
        .iter()
        .map(|w| w.spec.govctl.id.clone())
        .collect();
    refs.sort(); // Ensure deterministic ordering across platforms

    let release = Release {
        version: version.to_string(),
        date: release_date.clone(),
        refs: refs.clone(),
    };

    // Insert at the beginning (newest first)
    releases_file.releases.insert(0, release);

    // Write releases file
    write_releases(config, &releases_file, op)?;

    if !op.is_preview() {
        ui::release_created(version, &release_date, refs.len());
    }

    Ok(vec![])
}

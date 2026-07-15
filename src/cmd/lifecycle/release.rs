use std::collections::HashSet;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult, Diagnostics};
use crate::model::{Release, WorkItemStatus};
use crate::parse::{load_releases, load_work_items, validate_version, write_releases};
use crate::ui;
use crate::write::{WriteOp, delete_file, today};
use chrono::NaiveDate;

/// Cut a release - collect unreleased work items into a version
/// Per [[ADR-0014]], stores release info in gov/releases.toml
pub fn cut_release(
    config: &Config,
    version: &str,
    date: Option<&str>,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let releases_path = config.releases_path();
    let releases_path_str = config.display_path(&releases_path).display().to_string();

    validate_version(version).map_err(|_| {
        Diagnostic::new(
            DiagnosticCode::E0701ReleaseInvalidSemver,
            format!("Invalid semver version: {version}"),
            &releases_path_str,
        )
    })?;

    let mut releases_file = load_releases(config)?;

    if releases_file.releases.iter().any(|r| r.version == version) {
        let diag = Diagnostic::new(
            DiagnosticCode::E0702ReleaseDuplicate,
            format!("Release {version} already exists"),
            &releases_path_str,
        );
        return Err(diag);
    }

    let released_ids: HashSet<_> = releases_file
        .releases
        .iter()
        .flat_map(|r| r.refs.iter().cloned())
        .collect();

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
        return Err(diag);
    }

    let release_date = date.map(|d| d.to_string()).unwrap_or_else(today);
    if NaiveDate::parse_from_str(&release_date, "%Y-%m-%d").is_err() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0704ReleaseSchemaInvalid,
            format!(
                "Invalid release date: {release_date}. Expected a valid calendar date in YYYY-MM-DD format"
            ),
            &releases_path_str,
        ));
    }
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

    // Releases are stored newest-first for changelog rendering.
    releases_file.releases.insert(0, release);

    write_releases(config, &releases_file, op)?;

    if !op.is_preview() {
        ui::release_created(version, &release_date, refs.len());
    }

    Ok(vec![])
}

/// Undo the newest local release cut.
///
/// The expected version is an intent guard for [[RFC-0000:C-RELEASE-DEF]].
pub fn undo_release(
    config: &Config,
    expected_version: &str,
    op: WriteOp,
) -> DiagnosticResult<Diagnostics> {
    let releases_path = config.releases_path();
    let releases_path_str = config.display_path(&releases_path).display().to_string();

    validate_version(expected_version).map_err(|_| {
        Diagnostic::new(
            DiagnosticCode::E0701ReleaseInvalidSemver,
            format!("Invalid semver version: {expected_version}"),
            &releases_path_str,
        )
    })?;

    let mut releases_file = load_releases(config)?;
    let newest = releases_file.releases.first().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0708ReleaseHistoryEmpty,
            "Cannot undo a release because the release history is empty",
            &releases_path_str,
        )
    })?;

    if newest.version != expected_version {
        return Err(Diagnostic::new(
            DiagnosticCode::E0709ReleaseLatestMismatch,
            format!(
                "Cannot undo release {expected_version}: newest release is {}",
                newest.version
            ),
            &releases_path_str,
        ));
    }

    let removed = releases_file.releases.remove(0);
    if releases_file.releases.is_empty() {
        delete_file(
            &releases_path,
            op,
            Some(&config.display_path(&releases_path)),
        )?;
    } else {
        write_releases(config, &releases_file, op)?;
    }

    if !op.is_preview() {
        ui::release_undone(&removed.version, removed.refs.len());
        ui::sub_info("Run `govctl render changelog --force` to rebuild CHANGELOG.md");
    }

    Ok(vec![])
}

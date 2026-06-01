use std::collections::HashSet;

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ReleasesFile, WorkItemEntry, WorkItemStatus};
use crate::parse::{load_releases, load_work_items};
use crate::ui;

mod preserve;
mod sections;

/// Render CHANGELOG.md from completed work items
/// Per [[ADR-0014]], groups by release version and changelog category.
///
/// Default behavior: only updates the Unreleased section, preserving manually
/// edited released sections. Use `force=true` to regenerate the entire file.
pub fn render_changelog(
    config: &Config,
    dry_run: bool,
    force: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let releases_file = load_releases(config)?;
    let work_items = load_work_items(config)?;

    // Get all released work item IDs
    let released_ids: HashSet<_> = releases_file
        .releases
        .iter()
        .flat_map(|r| r.refs.iter().cloned())
        .collect();

    // Find unreleased done work items
    let unreleased: Vec<_> = work_items
        .iter()
        .filter(|w| w.spec.govctl.status == WorkItemStatus::Done)
        .filter(|w| !released_ids.contains(&w.spec.govctl.id))
        .collect();

    let changelog_path = std::path::PathBuf::from("CHANGELOG.md");

    let output = if force {
        // Force mode: regenerate entire file
        render_changelog_full(config, &releases_file, &work_items, &unreleased)?
    } else {
        // Default mode: update Unreleased section + add missing releases, preserve existing
        render_changelog_incremental(
            config,
            &changelog_path,
            &releases_file,
            &work_items,
            &unreleased,
        )?
    };

    let unreleased_count = unreleased.len();

    if dry_run {
        ui::dry_run_file_preview(&changelog_path, &output);
    } else {
        std::fs::write(&changelog_path, &output).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0901IoError,
                format!("Failed to write changelog: {err}"),
                changelog_path.display().to_string(),
            )
        })?;
        ui::changelog_rendered(
            &changelog_path,
            releases_file.releases.len(),
            unreleased_count,
        );
    }

    Ok(vec![])
}

/// Generate the complete changelog from scratch (force mode)
fn render_changelog_full(
    config: &Config,
    releases_file: &ReleasesFile,
    work_items: &[WorkItemEntry],
    unreleased: &[&WorkItemEntry],
) -> anyhow::Result<String> {
    let work_item_map = sections::work_item_map(work_items);

    let mut output = String::new();
    output.push_str(sections::CHANGELOG_HEADER);

    // Unreleased section
    let unreleased_expanded =
        sections::render_unreleased_section(unreleased, &config.source_scan.pattern);
    output.push_str(unreleased_expanded.trim_end());
    output.push('\n');

    // Released sections (newest first per releases.toml order)
    for release in &releases_file.releases {
        let release_expanded =
            sections::render_release_section(release, &work_item_map, &config.source_scan.pattern);
        output.push('\n');
        output.push_str(release_expanded.trim_end());
        output.push('\n');
    }

    Ok(format!("{}\n", output.trim_end()))
}

/// Update Unreleased section and add missing releases, preserving existing sections (default mode)
fn render_changelog_incremental(
    config: &Config,
    changelog_path: &std::path::Path,
    releases_file: &ReleasesFile,
    work_items: &[WorkItemEntry],
    unreleased: &[&WorkItemEntry],
) -> anyhow::Result<String> {
    let existing = if changelog_path.exists() {
        std::fs::read_to_string(changelog_path).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0901IoError,
                format!("Failed to read changelog: {err}"),
                changelog_path.display().to_string(),
            )
        })?
    } else {
        String::new()
    };

    let work_item_map = sections::work_item_map(work_items);
    let mut existing_changelog = preserve::split_existing_changelog(&existing);

    // Generate new Unreleased section and expand inline refs
    let unreleased_expanded =
        sections::render_unreleased_section(unreleased, &config.source_scan.pattern);

    // Build output: header + unreleased + (new releases + existing releases merged)
    let mut output = existing_changelog.header;
    output.push_str(unreleased_expanded.trim_end());
    output.push('\n');

    // Add releases from releases.toml that don't exist yet
    for release in &releases_file.releases {
        if !preserve::contains_version_variant(&existing_changelog.releases, &release.version) {
            let release_expanded = sections::render_release_section(
                release,
                &work_item_map,
                &config.source_scan.pattern,
            );
            existing_changelog.releases.insert(
                release.version.clone(),
                release_expanded.trim_end().to_string(),
            );
        }
        // If exists, we keep the existing section (preserve manual edits)
    }

    for version in preserve::versions_newest_first(&existing_changelog.releases) {
        if let Some(section) = existing_changelog.releases.get(&version) {
            output.push('\n');
            output.push_str(section.trim_end());
            output.push('\n');
        }
    }

    Ok(format!("{}\n", output.trim_end()))
}

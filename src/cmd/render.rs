//! Render command implementation.

use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::load::load_rfcs;
use crate::model::{ChangelogCategory, ChecklistStatus, WorkItemStatus};
use crate::parse::{load_adrs, load_releases, load_work_items};
use crate::render::{write_adr_md, write_rfc, write_work_item_md};
use crate::ui;
use std::collections::{HashMap, HashSet};

/// Render RFC markdown from JSON source
pub fn render(
    config: &Config,
    rfc_id: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfcs = load_rfcs(config).map_err(|e| {
        let diag: Diagnostic = e.into();
        anyhow::anyhow!("{}", diag)
    })?;

    if rfcs.is_empty() {
        ui::not_found("RFC", &config.rfc_dir());
        return Ok(vec![]);
    }

    // Filter to specific RFC if provided
    let rfcs_to_render: Vec<_> = if let Some(id) = rfc_id {
        rfcs.into_iter().filter(|r| r.rfc.rfc_id == id).collect()
    } else {
        rfcs
    };

    if rfcs_to_render.is_empty() {
        if let Some(id) = rfc_id {
            anyhow::bail!("RFC not found: {id}");
        }
    }

    for rfc in &rfcs_to_render {
        write_rfc(config, rfc, dry_run)?;
    }

    if !dry_run {
        ui::render_summary(rfcs_to_render.len(), "RFC");
    }

    Ok(vec![])
}

/// Render all ADRs to markdown
pub fn render_adrs(config: &Config, dry_run: bool) -> anyhow::Result<Vec<Diagnostic>> {
    let adrs = load_adrs(config)?;

    if adrs.is_empty() {
        ui::info("No ADRs found");
        return Ok(vec![]);
    }

    for adr in &adrs {
        write_adr_md(config, adr, dry_run)?;
    }

    if !dry_run {
        ui::render_summary(adrs.len(), "ADR");
    }

    Ok(vec![])
}

/// Render all Work Items to markdown
pub fn render_work_items(config: &Config, dry_run: bool) -> anyhow::Result<Vec<Diagnostic>> {
    let items = load_work_items(config)?;

    if items.is_empty() {
        ui::info("No work items found");
        return Ok(vec![]);
    }

    for item in &items {
        write_work_item_md(config, item, dry_run)?;
    }

    if !dry_run {
        ui::render_summary(items.len(), "work item");
    }

    Ok(vec![])
}

/// Render CHANGELOG.md from completed work items
/// Per [[ADR-0014]], groups by release version and changelog category.
pub fn render_changelog(config: &Config, dry_run: bool) -> anyhow::Result<Vec<Diagnostic>> {
    let releases_file = load_releases(config).map_err(|d| anyhow::anyhow!("{}", d.message))?;
    let work_items = load_work_items(config).map_err(|d| anyhow::anyhow!("{}", d.message))?;

    // Build work item lookup by ID
    let work_item_map: HashMap<_, _> = work_items
        .iter()
        .map(|w| (w.spec.govctl.id.clone(), w))
        .collect();

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

    let mut output = String::new();
    output.push_str("# Changelog\n\n");
    output.push_str("All notable changes to this project will be documented in this file.\n\n");
    output.push_str(
        "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\n",
    );
    output.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");

    // Render unreleased section if there are unreleased items
    let unreleased_count = if !unreleased.is_empty() {
        output.push_str("## [Unreleased]\n\n");
        render_changelog_section(&mut output, &unreleased);
        unreleased.len()
    } else {
        0
    };

    // Render each release (already sorted newest first)
    for release in &releases_file.releases {
        output.push_str(&format!("## [{}] - {}\n\n", release.version, release.date));

        let items: Vec<_> = release
            .refs
            .iter()
            .filter_map(|id| work_item_map.get(id).copied())
            .collect();

        if items.is_empty() {
            output.push_str("*No changes recorded.*\n\n");
        } else {
            render_changelog_section(&mut output, &items);
        }
    }

    // Write CHANGELOG.md
    let changelog_path = std::path::PathBuf::from("CHANGELOG.md");
    if dry_run {
        ui::dry_run_file_preview(&changelog_path, &output);
    } else {
        std::fs::write(&changelog_path, &output)?;
        ui::changelog_rendered(
            &changelog_path,
            releases_file.releases.len(),
            unreleased_count,
        );
    }

    Ok(vec![])
}

/// Render a changelog section from work items, grouped by category
fn render_changelog_section(output: &mut String, items: &[&crate::model::WorkItemEntry]) {
    // Collect all done criteria grouped by category
    let mut by_category: HashMap<ChangelogCategory, Vec<(String, String)>> = HashMap::new();

    for item in items {
        for criterion in &item.spec.content.acceptance_criteria {
            if criterion.status == ChecklistStatus::Done {
                by_category
                    .entry(criterion.category)
                    .or_default()
                    .push((criterion.text.clone(), item.spec.govctl.id.clone()));
            }
        }
    }

    // Output in standard Keep a Changelog order
    let categories = [
        (ChangelogCategory::Added, "Added"),
        (ChangelogCategory::Changed, "Changed"),
        (ChangelogCategory::Deprecated, "Deprecated"),
        (ChangelogCategory::Removed, "Removed"),
        (ChangelogCategory::Fixed, "Fixed"),
        (ChangelogCategory::Security, "Security"),
    ];

    for (cat, label) in categories {
        if let Some(entries) = by_category.get(&cat) {
            output.push_str(&format!("### {}\n\n", label));
            for (text, work_id) in entries {
                output.push_str(&format!("- {} ({})\n", text, work_id));
            }
            output.push('\n');
        }
    }
}

//! Render command implementation.

use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::load::load_rfcs;
use crate::model::{ChangelogCategory, ChecklistStatus, WorkItemStatus};
use crate::parse::{load_adrs, load_releases, load_work_items};
use crate::render::{expand_inline_refs_from_root, write_adr_md, write_rfc, write_work_item_md};
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

    if rfcs_to_render.is_empty()
        && let Some(id) = rfc_id
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0102RfcNotFound,
            format!("RFC not found: {id}"),
            id,
        )
        .into());
    }

    for rfc in &rfcs_to_render {
        write_rfc(config, rfc, dry_run)?;
    }

    if !dry_run {
        ui::render_summary(rfcs_to_render.len(), "RFC");
    }

    Ok(vec![])
}

/// Render ADRs to markdown
///
/// If `adr_id` is provided, renders only that ADR. Otherwise renders all.
pub fn render_adrs(
    config: &Config,
    adr_id: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let adrs = load_adrs(config)?;

    if adrs.is_empty() {
        ui::info("No ADRs found");
        return Ok(vec![]);
    }

    // Filter to specific ADR if provided
    let adrs_to_render: Vec<_> = if let Some(id) = adr_id {
        adrs.into_iter()
            .filter(|a| a.spec.govctl.id == id)
            .collect()
    } else {
        adrs
    };

    if adrs_to_render.is_empty()
        && let Some(id) = adr_id
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0302AdrNotFound,
            format!("ADR not found: {id}"),
            id,
        )
        .into());
    }

    for adr in &adrs_to_render {
        write_adr_md(config, adr, dry_run)?;
    }

    if !dry_run {
        ui::render_summary(adrs_to_render.len(), "ADR");
    }

    Ok(vec![])
}

/// Render Work Items to markdown
///
/// If `work_id` is provided, renders only that work item. Otherwise renders all.
pub fn render_work_items(
    config: &Config,
    work_id: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Vec<Diagnostic>> {
    let items = load_work_items(config)?;

    if items.is_empty() {
        ui::info("No work items found");
        return Ok(vec![]);
    }

    // Filter to specific work item if provided
    let items_to_render: Vec<_> = if let Some(id) = work_id {
        items
            .into_iter()
            .filter(|w| w.spec.govctl.id == id)
            .collect()
    } else {
        items
    };

    if items_to_render.is_empty()
        && let Some(id) = work_id
    {
        return Err(Diagnostic::new(
            DiagnosticCode::E0402WorkNotFound,
            format!("Work item not found: {id}"),
            id,
        )
        .into());
    }

    for item in &items_to_render {
        write_work_item_md(config, item, dry_run)?;
    }

    if !dry_run {
        ui::render_summary(items_to_render.len(), "work item");
    }

    Ok(vec![])
}

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
    let releases_file = load_releases(config).map_err(|d| anyhow::anyhow!("{}", d.message))?;
    let work_items = load_work_items(config).map_err(|d| anyhow::anyhow!("{}", d.message))?;

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
        std::fs::write(&changelog_path, &output)?;
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
    releases_file: &crate::model::ReleasesFile,
    work_items: &[crate::model::WorkItemEntry],
    unreleased: &[&crate::model::WorkItemEntry],
) -> anyhow::Result<String> {
    // Build work item lookup by ID
    let work_item_map: HashMap<_, _> = work_items
        .iter()
        .map(|w| (w.spec.govctl.id.clone(), w))
        .collect();

    let mut output = String::new();

    // Header
    output.push_str("# Changelog\n\n");
    output.push_str("All notable changes to this project will be documented in this file.\n\n");
    output.push_str(
        "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\n",
    );
    output.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");

    // Unreleased section
    let mut unreleased_content = String::new();
    unreleased_content.push_str("## [Unreleased]\n\n");
    if !unreleased.is_empty() {
        render_changelog_section(&mut unreleased_content, unreleased);
    }
    // Expand inline refs in unreleased section
    let unreleased_expanded =
        expand_inline_refs_from_root(&unreleased_content, &config.source_scan.pattern, "docs");
    output.push_str(unreleased_expanded.trim_end());
    output.push('\n');

    // Released sections (newest first per releases.toml order)
    for release in &releases_file.releases {
        let mut release_content = String::new();
        release_content.push_str(&format!("## [{}] - {}\n\n", release.version, release.date));

        let items: Vec<_> = release
            .refs
            .iter()
            .filter_map(|id| work_item_map.get(id).copied())
            .collect();

        if items.is_empty() {
            release_content.push_str("*No changes recorded.*\n");
        } else {
            render_changelog_section(&mut release_content, &items);
        }

        // Expand inline refs in this release section
        let release_expanded =
            expand_inline_refs_from_root(&release_content, &config.source_scan.pattern, "docs");
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
    releases_file: &crate::model::ReleasesFile,
    work_items: &[crate::model::WorkItemEntry],
    unreleased: &[&crate::model::WorkItemEntry],
) -> anyhow::Result<String> {
    // Read existing changelog if it exists
    let existing = if changelog_path.exists() {
        std::fs::read_to_string(changelog_path)?
    } else {
        String::new()
    };

    // Build work item lookup by ID
    let work_item_map: HashMap<_, _> = work_items
        .iter()
        .map(|w| (w.spec.govctl.id.clone(), w))
        .collect();

    let unreleased_header = "## [Unreleased]";
    let release_pattern = "\n## [";

    // Parse existing changelog into header and released sections
    let (header, existing_released) = if existing.is_empty() {
        let header = "# Changelog\n\n\
            All notable changes to this project will be documented in this file.\n\n\
            The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\n\
            and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n";
        (header.to_string(), String::new())
    } else if let Some(unreleased_pos) = existing.find(unreleased_header) {
        let header = existing[..unreleased_pos].to_string();
        let after_unreleased = &existing[unreleased_pos + unreleased_header.len()..];
        let released = if let Some(pos) = after_unreleased.find(release_pattern) {
            after_unreleased[pos + 1..].to_string() // skip leading \n
        } else {
            String::new()
        };
        (header, released)
    } else if let Some(first_release_pos) = existing.find(release_pattern) {
        let header = existing[..first_release_pos + 1].to_string();
        let released = existing[first_release_pos + 1..].to_string();
        (header, released)
    } else {
        (existing.clone(), String::new())
    };

    // Parse ALL existing release sections into a map (to preserve ALL versions)
    // Each section: "## [version] - date\n\ncontent"
    let mut existing_releases: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();

    if !existing_released.is_empty() {
        let mut current_pos = 0;
        let content = &existing_released;

        while current_pos < content.len() {
            // Find the next "## [" marker
            if !content[current_pos..].starts_with("## [") {
                break;
            }

            // Find the end of this section (next "## [" or EOF)
            let rest = &content[current_pos..];
            let section_end = rest[4..] // skip "## ["
                .find("\n## [")
                .map(|p| p + 4 + 1) // adjust for the skip and the newline
                .unwrap_or(rest.len());

            let section = &rest[..section_end];

            // Extract version from the section header (handles both "## [X.Y.Z]" and "## [vX.Y.Z]")
            let version = if let Some(bracket_end) = section.find(']') {
                section[4..bracket_end].to_string() // skip "## ["
            } else {
                current_pos += section.len();
                continue;
            };

            existing_releases.insert(version, section.to_string());
            current_pos += section.len();
        }
    };

    // Generate new Unreleased section and expand inline refs
    let mut unreleased_content = String::new();
    unreleased_content.push_str("## [Unreleased]\n\n");
    if !unreleased.is_empty() {
        render_changelog_section(&mut unreleased_content, unreleased);
    }
    // Expand inline refs in unreleased section only (preserves older versions)
    let unreleased_expanded =
        expand_inline_refs_from_root(&unreleased_content, &config.source_scan.pattern, "docs");

    // Build output: header + unreleased + (new releases + existing releases merged)
    let mut output = header;
    output.push_str(unreleased_expanded.trim_end());
    output.push('\n');

    // Add releases from releases.toml that don't exist yet
    for release in &releases_file.releases {
        // Check both "X.Y.Z" and "vX.Y.Z" variants
        let version_variants = if release.version.starts_with('v') {
            vec![release.version.clone(), release.version[1..].to_string()]
        } else {
            vec![release.version.clone(), format!("v{}", release.version)]
        };

        let exists = version_variants
            .iter()
            .any(|v| existing_releases.contains_key(v));

        if !exists {
            // This release is missing - generate it and add to map
            let mut release_content = String::new();
            release_content.push_str(&format!("## [{}] - {}\n\n", release.version, release.date));

            let items: Vec<_> = release
                .refs
                .iter()
                .filter_map(|id| work_item_map.get(id).copied())
                .collect();

            if items.is_empty() {
                release_content.push_str("*No changes recorded.*\n");
            } else {
                render_changelog_section(&mut release_content, &items);
            }

            // Expand inline refs in new release section
            let release_expanded =
                expand_inline_refs_from_root(&release_content, &config.source_scan.pattern, "docs");
            existing_releases.insert(
                release.version.clone(),
                release_expanded.trim_end().to_string(),
            );
        }
        // If exists, we keep the existing section (preserve manual edits)
    }

    // Output all releases in reverse semver order (newest first)
    // Sort by version number (parse and compare)
    let mut versions: Vec<String> = existing_releases.keys().cloned().collect();
    versions.sort_by(|a, b| {
        // Compare versions: strip 'v' prefix if present, then compare semantically
        let a_stripped = a.strip_prefix('v').unwrap_or(a);
        let b_stripped = b.strip_prefix('v').unwrap_or(b);

        // Parse version parts
        let a_parts: Vec<u32> = a_stripped
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        let b_parts: Vec<u32> = b_stripped
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        // Compare in reverse order (newest first)
        b_parts.cmp(&a_parts)
    });

    for version in versions {
        if let Some(section) = existing_releases.get(&version) {
            output.push('\n');
            output.push_str(section.trim_end());
            output.push('\n');
        }
    }

    Ok(format!("{}\n", output.trim_end()))
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

// =============================================================================
// Show Commands (stdout rendering per ADR-0022)
// =============================================================================

use crate::OutputFormat;
use crate::render::{expand_inline_refs, render_adr, render_clause, render_rfc, render_work_item};
use crate::terminal_md::render_terminal_md;

/// Show RFC content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_rfc(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    let rfcs = load_rfcs(config).map_err(|e| {
        let diag: Diagnostic = e.into();
        anyhow::anyhow!("{}", diag)
    })?;

    let rfc = rfcs
        .into_iter()
        .find(|r| r.rfc.rfc_id == id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {id}"),
                id,
            )
        })?;

    match output {
        OutputFormat::Json => {
            // Output the raw RFC data as JSON
            let json = serde_json::to_string_pretty(&rfc.rfc)?;
            println!("{json}");
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let raw = render_rfc(&rfc)?;
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }

    Ok(vec![])
}

/// Show ADR content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_adr(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    let adrs = load_adrs(config)?;

    let adr = adrs
        .into_iter()
        .find(|a| a.spec.govctl.id == id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0302AdrNotFound,
                format!("ADR not found: {id}"),
                id,
            )
        })?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&adr.spec)?;
            println!("{json}");
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let raw = render_adr(&adr)?;
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }

    Ok(vec![])
}

/// Show work item content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_work(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    let items = load_work_items(config)?;

    let item = items
        .into_iter()
        .find(|w| w.spec.govctl.id == id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0402WorkNotFound,
                format!("Work item not found: {id}"),
                id,
            )
        })?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&item.spec)?;
            println!("{json}");
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let raw = render_work_item(&item)?;
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }

    Ok(vec![])
}

/// Show clause content to stdout (no file written).
///
/// Per [[ADR-0022]], outputs markdown by default or JSON with --output json.
pub fn show_clause(
    config: &Config,
    id: &str,
    output: OutputFormat,
) -> anyhow::Result<Vec<Diagnostic>> {
    // Parse clause ID: RFC-NNNN:C-NAME
    let parts: Vec<&str> = id.split(':').collect();
    if parts.len() != 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0202ClauseNotFound,
            format!("Invalid clause ID format: {id} (expected RFC-NNNN:C-NAME)"),
            id,
        )
        .into());
    }
    let rfc_id = parts[0];
    let clause_name = parts[1];

    let rfcs = load_rfcs(config).map_err(|e| {
        let diag: Diagnostic = e.into();
        anyhow::anyhow!("{}", diag)
    })?;

    let rfc = rfcs
        .into_iter()
        .find(|r| r.rfc.rfc_id == rfc_id)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0102RfcNotFound,
                format!("RFC not found: {rfc_id}"),
                rfc_id,
            )
        })?;

    let clause = rfc
        .clauses
        .into_iter()
        .find(|c| c.spec.clause_id == clause_name)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0202ClauseNotFound,
                format!("Clause not found: {id}"),
                id,
            )
        })?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&clause.spec)?;
            println!("{json}");
        }
        OutputFormat::Table | OutputFormat::Plain => {
            let mut raw = String::new();
            render_clause(&mut raw, rfc_id, &clause);
            let expanded = expand_inline_refs(&raw, &config.source_scan.pattern);
            print!("{}", render_terminal_md(&expanded));
        }
    }

    Ok(vec![])
}

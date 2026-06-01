use std::collections::HashMap;

use crate::model::{ChangelogCategory, ChecklistStatus, Release, WorkItemEntry};
use crate::render::expand_inline_refs_from_root;

pub(super) const CHANGELOG_HEADER: &str = "# Changelog\n\n\
All notable changes to this project will be documented in this file.\n\n\
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),\n\
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n";

pub(super) fn work_item_map(work_items: &[WorkItemEntry]) -> HashMap<String, &WorkItemEntry> {
    work_items
        .iter()
        .map(|work_item| (work_item.spec.govctl.id.clone(), work_item))
        .collect()
}

pub(super) fn render_unreleased_section(
    items: &[&WorkItemEntry],
    source_scan_pattern: &str,
) -> String {
    let mut content = String::new();
    content.push_str("## [Unreleased]\n\n");
    if !items.is_empty() {
        render_changelog_section(&mut content, items);
    }
    expand_inline_refs_from_root(&content, source_scan_pattern, "docs")
        .trim_end()
        .to_string()
}

pub(super) fn render_release_section(
    release: &Release,
    work_item_map: &HashMap<String, &WorkItemEntry>,
    source_scan_pattern: &str,
) -> String {
    let mut content = String::new();
    content.push_str(&format!("## [{}] - {}\n\n", release.version, release.date));

    let items: Vec<_> = release
        .refs
        .iter()
        .filter_map(|id| work_item_map.get(id).copied())
        .collect();

    if items.is_empty() {
        content.push_str("*No changes recorded.*\n");
    } else {
        render_changelog_section(&mut content, &items);
    }

    expand_inline_refs_from_root(&content, source_scan_pattern, "docs")
        .trim_end()
        .to_string()
}

fn render_changelog_section(output: &mut String, items: &[&WorkItemEntry]) {
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

    let categories = [
        (ChangelogCategory::Added, "Added"),
        (ChangelogCategory::Changed, "Changed"),
        (ChangelogCategory::Deprecated, "Deprecated"),
        (ChangelogCategory::Removed, "Removed"),
        (ChangelogCategory::Fixed, "Fixed"),
        (ChangelogCategory::Security, "Security"),
    ];

    for (category, label) in categories {
        if let Some(entries) = by_category.get(&category) {
            output.push_str(&format!("### {}\n\n", label));
            for (text, work_id) in entries {
                output.push_str(&format!("- {} ({})\n", text, work_id));
            }
            output.push('\n');
        }
    }
}

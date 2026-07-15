use super::super::color::use_colors;
use owo_colors::OwoColorize;
use std::path::Path;

pub fn created(kind: &str, path: &Path) {
    if use_colors() {
        eprintln!("{} {}: {}", "Created".green(), kind, path.display().cyan());
    } else {
        eprintln!("Created {}: {}", kind, path.display());
    }
}

pub fn field_set(id: &str, field: &str, value: &str) {
    if use_colors() {
        eprintln!(
            "Set {}.{} = {}",
            id.cyan().bold(),
            field.yellow(),
            value.white()
        );
    } else {
        eprintln!("Set {}.{} = {}", id, field, value);
    }
}

pub fn field_added(id: &str, field: &str, value: &str) {
    if use_colors() {
        eprintln!(
            "Added '{}' to {}.{}",
            value.white(),
            id.cyan().bold(),
            field.yellow()
        );
    } else {
        eprintln!("Added '{}' to {}.{}", value, id, field);
    }
}

pub fn field_removed(id: &str, field: &str, value: &str) {
    if use_colors() {
        eprintln!(
            "Removed '{}' from {}.{}",
            value.white(),
            id.cyan().bold(),
            field.yellow()
        );
    } else {
        eprintln!("Removed '{}' from {}.{}", value, id, field);
    }
}

pub fn moved(filename: &str, status: &str) {
    if use_colors() {
        eprintln!("Moved {} to {}", filename.cyan(), status.green().bold());
    } else {
        eprintln!("Moved {} to {}", filename, status);
    }
}

pub fn phase_advanced(id: &str, phase: &str) {
    if use_colors() {
        eprintln!("Advanced {} to phase: {}", id.cyan().bold(), phase.green());
    } else {
        eprintln!("Advanced {} to phase: {}", id, phase);
    }
}

pub fn version_bumped(id: &str, version: &str) {
    if use_colors() {
        eprintln!("Bumped {} to {}", id.cyan().bold(), version.green().bold());
    } else {
        eprintln!("Bumped {} to {}", id, version);
    }
}

pub fn changelog_change_added(id: &str, version: &str, change: &str) {
    if use_colors() {
        eprintln!(
            "Added change to {} v{}: {}",
            id.cyan().bold(),
            version.green(),
            change
        );
    } else {
        eprintln!("Added change to {} v{}: {}", id, version, change);
    }
}

pub fn ticked(item: &str, status: &str) {
    if use_colors() {
        eprintln!("Marked '{}' as {}", item.white(), status.green());
    } else {
        eprintln!("Marked '{}' as {}", item, status);
    }
}

pub fn accepted(kind: &str, id: &str) {
    if use_colors() {
        eprintln!("Accepted {}: {}", kind, id.cyan().bold());
    } else {
        eprintln!("Accepted {}: {}", kind, id);
    }
}

pub fn rejected(kind: &str, id: &str) {
    if use_colors() {
        eprintln!("Rejected {}: {}", kind, id.cyan().bold());
    } else {
        eprintln!("Rejected {}: {}", kind, id);
    }
}

pub fn deprecated(kind: &str, id: &str) {
    if use_colors() {
        eprintln!("Deprecated {}: {}", kind, id.yellow().bold());
    } else {
        eprintln!("Deprecated {}: {}", kind, id);
    }
}

pub fn superseded(kind: &str, id: &str, by: &str) {
    if use_colors() {
        eprintln!("Superseded {}: {}", kind, id.yellow().bold());
        eprintln!("  Replaced by: {}", by.cyan().bold());
    } else {
        eprintln!("Superseded {}: {}", kind, id);
        eprintln!("  Replaced by: {}", by);
    }
}

pub fn updated(kind: &str, id: &str) {
    if use_colors() {
        eprintln!("Updated {}: {}", kind, id.cyan().bold());
    } else {
        eprintln!("Updated {}: {}", kind, id);
    }
}

pub fn finalized(id: &str, status: &str) {
    if use_colors() {
        eprintln!(
            "Finalized {} to status: {}",
            id.cyan().bold(),
            status.green()
        );
    } else {
        eprintln!("Finalized {} to status: {}", id, status);
    }
}

pub fn release_created(version: &str, date: &str, work_item_count: usize) {
    if use_colors() {
        eprintln!(
            "Created release {} ({}) with {} work items",
            version.cyan().bold(),
            date,
            work_item_count.to_string().green()
        );
    } else {
        eprintln!(
            "Created release {} ({}) with {} work items",
            version, date, work_item_count
        );
    }
}

pub fn release_undone(version: &str, work_item_count: usize) {
    if use_colors() {
        eprintln!(
            "Undid release {} ({} work items are now unreleased)",
            version.cyan().bold(),
            work_item_count.to_string().green()
        );
    } else {
        eprintln!(
            "Undid release {} ({} work items are now unreleased)",
            version, work_item_count
        );
    }
}

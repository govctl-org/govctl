use super::super::color::use_colors;
use super::super::path_str;
use owo_colors::OwoColorize;
use std::fmt::Display;
use std::path::Path;

pub fn success(msg: impl Display) {
    if use_colors() {
        eprintln!("{} {}", "✓".green(), msg);
    } else {
        eprintln!("✓ {}", msg);
    }
}

pub fn info(msg: impl Display) {
    eprintln!("{}", msg);
}

pub fn hint(msg: impl Display) {
    if use_colors() {
        eprintln!("{} {}", "hint:".dimmed(), msg.to_string().dimmed());
    } else {
        eprintln!("hint: {}", msg);
    }
}

pub fn rendered(path: &Path) {
    if use_colors() {
        eprintln!("{}: {}", "Rendered".green(), path.display().cyan());
    } else {
        eprintln!("Rendered: {}", path.display());
    }
}

pub fn not_found(kind: &str, location: &Path) {
    if use_colors() {
        eprintln!("No {}s found in {}", kind, location.display().cyan());
    } else {
        eprintln!("No {}s found in {}", kind, location.display());
    }
}

pub fn check_header() {
    if use_colors() {
        eprintln!("{}:", "Checked".bold());
    } else {
        eprintln!("Checked:");
    }
}

pub fn check_count(count: usize, kind: &str) {
    if use_colors() {
        eprintln!("  {} {}", count.to_string().cyan().bold(), kind);
    } else {
        eprintln!("  {} {}", count, kind);
    }
}

pub fn render_summary(count: usize, kind: &str) {
    if use_colors() {
        eprintln!(
            "{} Rendered {} {}(s)",
            "✓".green(),
            count.to_string().cyan().bold(),
            kind
        );
    } else {
        eprintln!("✓ Rendered {} {}(s)", count, kind);
    }
}

pub fn created_path(path: &Path) {
    if use_colors() {
        eprintln!("{}: {}", "Created".green(), path.display().cyan());
    } else {
        eprintln!("Created: {}", path.display());
    }
}

pub fn changelog_rendered(path: &Path, release_count: usize, unreleased_count: usize) {
    if use_colors() {
        eprintln!(
            "Rendered CHANGELOG to {} ({} releases, {} unreleased)",
            path_str(path).cyan(),
            release_count,
            unreleased_count
        );
    } else {
        eprintln!(
            "Rendered CHANGELOG to {} ({} releases, {} unreleased)",
            path.display(),
            release_count,
            unreleased_count
        );
    }
}

pub fn sub_info(msg: impl Display) {
    eprintln!("  {}", msg);
}

pub fn error(msg: impl Display) {
    if use_colors() {
        eprintln!("{}: {}", "Error".red().bold(), msg);
    } else {
        eprintln!("Error: {}", msg);
    }
}

pub fn dry_run_preview(path: &Path) {
    if use_colors() {
        eprintln!("{}: {}", "Would write".yellow(), path.display().cyan());
    } else {
        eprintln!("Would write: {}", path.display());
    }
    eprintln!("--- Content preview ---");
}

pub fn preview_line(line: &str) {
    eprintln!("{}", line);
}

pub fn preview_truncated() {
    eprintln!("...");
}

pub fn dry_run_file_preview(path: &Path, content: &str) {
    if use_colors() {
        eprintln!("{}: {}", "Would write".yellow(), path.display().cyan());
    } else {
        eprintln!("Would write: {}", path.display());
    }
    for line in content.lines().take(20) {
        eprintln!("  {}", line);
    }
    if content.lines().count() > 20 {
        eprintln!("  ...");
    }
}

pub fn dry_run_mkdir(path: &Path) {
    if use_colors() {
        eprintln!("{}: {}", "Would create dir".yellow(), path.display().cyan());
    } else {
        eprintln!("Would create dir: {}", path.display());
    }
}
